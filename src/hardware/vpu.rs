use crate::hardware::exu::{Access, ExecutionUnit, Plan, Resource, Window};
use crate::isa::{EReg, Exu, Instruction, MReg, MRegPair, RegFile, RegRef};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneBox {
    AddSub,
    Mul,
    // vmaximum and vredmax
    PairMax,
    // vminimum and vredmin
    PairMin,
    Recip,
    Sqrt,
    // vexp and vexp2
    Exp,
    SinCos,
    Log,
    Tanh,
    SquareCube,
    Relu,
    Mov,
    RowSum,
    RowMax,
    RowMin,
    ColSum,
    Pack,
    Unpack,
    Vli,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VpuConfig {
    // rows in a tensor register
    pub rows: u32,
    // cycles from an MRF read request to its data
    pub sram_latency: u32,
    // cycles through a lane box, from a row entering to its result
    pub lane_latency: u32,
    // cycles through the row-sum tree
    pub row_sum_latency: u32,
    // cycles through the fp8 unpacker
    pub unpack_latency: u32,
}

impl VpuConfig {
    pub fn default(rows: u32) -> Self {
        assert!(rows > 0, "a tensor register has at least one row");
        Self {
            rows,
            sram_latency: 1,
            lane_latency: 1,
            row_sum_latency: rows.ilog2() + 1,
            unpack_latency: 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VectorUnit {
    pub cfg: VpuConfig,
}

impl VectorUnit {
    pub fn new(cfg: VpuConfig) -> Self {
        Self { cfg }
    }

    fn unary(&self, vd: MRegPair, vs: MRegPair, lane: LaneBox) -> Vec<Plan> {
        let out = self.cfg.sram_latency + self.cfg.lane_latency;
        self.either_slot(|read, write| {
            let mut plan = self.reads(&pair(vs), 0, &[read, Resource::LaneBox(lane)]);
            plan.extend(self.writes(&pair(vd), out, &[write]));
            plan
        })
    }

    // both sources stream in lockstep, one per read port; results leave on write port 0 only
    fn binary(&self, vd: MRegPair, vs1: MRegPair, vs2: MRegPair, lane: LaneBox) -> Vec<Plan> {
        let out = self.cfg.sram_latency + self.cfg.lane_latency;
        let (read0, read1, lane) = (self.read_port(0), self.read_port(1), Resource::LaneBox(lane));
        let mut plan = if vs1 == vs2 {
            // one SRAM read feeds both operands
            self.reads(&pair(vs1), 0, &[read0, read1, lane])
        } else {
            let mut plan = self.reads(&pair(vs1), 0, &[read0, lane]);
            plan.extend(self.reads(&pair(vs2), 0, &[read1]));
            plan
        };
        plan.extend(self.writes(&pair(vd), out, &[self.write_port(0)]));
        vec![plan]
    }

    // both registers of the tile stream at once
    //  both halves of each result row leave together on both write ports
    fn row_reduce(&self, vd: MRegPair, vs: MRegPair, lane: LaneBox, latency: u32) -> Vec<Plan> {
        let out = self.cfg.sram_latency + latency;
        let ([vs_lo, vs_hi], [vd_lo, vd_hi]) = (pair(vs), pair(vd));
        let mut plan = self.reads(&[vs_lo], 0, &[self.read_port(0), Resource::LaneBox(lane)]);
        plan.extend(self.reads(&[vs_hi], 0, &[self.read_port(1)]));
        plan.extend(self.writes(&[vd_lo], out, &[self.write_port(0)]));
        plan.extend(self.writes(&[vd_hi], out, &[self.write_port(1)]));
        vec![plan]
    }

    // source streams in once,the result is ready when its last row has been through
    // the lane box, which holds the result while it streams out. this is MORE OPTIMAL than RTL
    fn col_reduce(&self, vd: MRegPair, vs: MRegPair, lane: LaneBox) -> Vec<Plan> {
        let c = &self.cfg;
        let out = (2 * c.rows - 1) + c.sram_latency + c.lane_latency;
        let lane = Resource::LaneBox(lane);
        self.either_slot(|read, write| {
            let mut plan = self.reads(&pair(vs), 0, &[read, lane]);
            plan.extend(self.writes(&pair(vd), out, &[write, lane]));
            plan
        })
    }

    // result row k joins rows 2k and 2k+1 of the source stream
    // writes every other cycle
    fn pack(&self, vd: MReg, vs: MRegPair, scale: EReg) -> Vec<Plan> {
        let c = &self.cfg;
        let out = 1 + c.sram_latency + c.lane_latency;
        self.either_slot(|read, write| {
            let mut plan = vec![Access::read(ereg(scale), Window::once(0))];
            plan.extend(self.reads(&pair(vs), 0, &[read, Resource::LaneBox(LaneBox::Pack)]));
            plan.push(Access::write(mreg(vd), Window::stream(out, c.rows, 2)).via(write));
            plan
        })
    }

    // each source row becomes two result rows so the unpacker's output is doubly busy
    fn unpack(&self, vd: MRegPair, vs: MReg, scale: EReg) -> Vec<Plan> {
        let c = &self.cfg;
        let out = c.sram_latency + c.unpack_latency;
        self.either_slot(|read, write| {
            let mut plan = vec![Access::read(ereg(scale), Window::once(0))];
            plan.extend(self.reads(&[mreg(vs)], 0, &[read]));
            plan.extend(self.writes(&pair(vd), out, &[write, Resource::LaneBox(LaneBox::Unpack)]));
            plan
        })
    }

    // no source: the lane box writes the immediate straight out
    fn vli(&self, vd: &[RegRef]) -> Vec<Plan> {
        let out = self.cfg.lane_latency;
        self.either_slot(|_, write| self.writes(vd, out, &[write, Resource::LaneBox(LaneBox::Vli)]))
    }

    // A single-slot op runs on slot 0 if it fits, else slot 1. A slot is a read port and a write
    // port: its lane-box results can only leave through its own write port.
    fn either_slot(&self, plan: impl Fn(Resource, Resource) -> Plan) -> Vec<Plan> {
        (0..2).map(|i| plan(self.read_port(i), self.write_port(i))).collect()
    }

    fn reads(&self, regs: &[RegRef], at: u32, via: &[Resource]) -> Plan {
        self.streams(regs, at, via, false)
    }

    fn writes(&self, regs: &[RegRef], at: u32, via: &[Resource]) -> Plan {
        self.streams(regs, at, via, true)
    }

    // `regs` streamed one after another from `at`, a row per cycle, each holding `via`
    fn streams(&self, regs: &[RegRef], at: u32, via: &[Resource], write: bool) -> Plan {
        let rows = self.cfg.rows;
        (0..)
            .zip(regs)
            .map(|(k, &reg)| {
                let win = Window::stream(at + k * rows, rows, 1);
                let access = if write { Access::write(reg, win) } else { Access::read(reg, win) };
                via.iter().fold(access, |access, &res| access.via(res))
            })
            .collect()
    }

    fn read_port(&self, i: u8) -> Resource {
        Resource::MrfReadPort(Exu::Vector, i)
    }

    fn write_port(&self, i: u8) -> Resource {
        Resource::MrfWritePort(Exu::Vector, i)
    }
}

impl ExecutionUnit for VectorUnit {
    fn plan(&self, insn: &Instruction) -> Vec<Plan> {
        assert_eq!(insn.spec().exu, Exu::Vector, "{insn} does not issue to the VPU");
        use Instruction as I;
        use LaneBox as L;
        let c = &self.cfg;
        match *insn {
            I::VaddBf16 { vd, vs1, vs2 } | I::VsubBf16 { vd, vs1, vs2 } => {
                self.binary(vd, vs1, vs2, L::AddSub)
            }
            I::VmulBf16 { vd, vs1, vs2 } => self.binary(vd, vs1, vs2, L::Mul),
            I::VmaximumBf16 { vd, vs1, vs2 } => self.binary(vd, vs1, vs2, L::PairMax),
            I::VminimumBf16 { vd, vs1, vs2 } => self.binary(vd, vs1, vs2, L::PairMin),

            I::Vmov { vd, vs1 } => self.unary(vd, vs1, L::Mov),
            I::VrecipBf16 { vd, vs1 } => self.unary(vd, vs1, L::Recip),
            I::VsqrtBf16 { vd, vs1 } => self.unary(vd, vs1, L::Sqrt),
            I::VexpBf16 { vd, vs1 } | I::Vexp2Bf16 { vd, vs1 } => self.unary(vd, vs1, L::Exp),
            I::VsinBf16 { vd, vs1 } | I::VcosBf16 { vd, vs1 } => self.unary(vd, vs1, L::SinCos),
            I::Vlog2Bf16 { vd, vs1 } => self.unary(vd, vs1, L::Log),
            I::VtanhBf16 { vd, vs1 } => self.unary(vd, vs1, L::Tanh),
            I::VsquareBf16 { vd, vs1 } | I::VcubeBf16 { vd, vs1 } => {
                self.unary(vd, vs1, L::SquareCube)
            }
            I::VreluBf16 { vd, vs1 } => self.unary(vd, vs1, L::Relu),

            I::VredsumRowBf16 { vd, vs1 } => self.row_reduce(vd, vs1, L::RowSum, c.row_sum_latency),
            I::VredmaxRowBf16 { vd, vs1 } => self.row_reduce(vd, vs1, L::RowMax, c.lane_latency),
            I::VredminRowBf16 { vd, vs1 } => self.row_reduce(vd, vs1, L::RowMin, c.lane_latency),

            I::VredsumBf16 { vd, vs1 } => self.col_reduce(vd, vs1, L::ColSum),
            I::VredmaxBf16 { vd, vs1 } => self.col_reduce(vd, vs1, L::PairMax),
            I::VredminBf16 { vd, vs1 } => self.col_reduce(vd, vs1, L::PairMin),

            I::VpackBf16Fp8 { vd, vs2, es1 } => self.pack(vd, vs2, es1),
            I::VunpackFp8Bf16 { vd, vs2, es1 } => self.unpack(vd, vs2, es1),

            I::VliAll { vd, .. } | I::VliRow { vd, .. } => self.vli(&pair(vd)),
            I::VliCol { vd, .. } | I::VliOne { vd, .. } => self.vli(&[mreg(vd)]),

            _ => unimplemented!("no VPU timing for {insn}"),
        }
    }
}

fn mreg(r: MReg) -> RegRef {
    RegRef { file: RegFile::Mrf, idx: r.idx() }
}

// the VPU names a BF16 tile by its low register, which must be even
fn pair(p: MRegPair) -> [RegRef; 2] {
    assert!(p.idx() % 2 == 0, "the VPU needs an even register pair, not {p}");
    [mreg(p.lo()), mreg(p.hi())]
}

fn ereg(e: EReg) -> RegRef {
    RegRef { file: RegFile::Erf, idx: e.idx() }
}
