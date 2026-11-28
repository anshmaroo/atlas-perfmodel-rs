use crate::hardware::exu::{Access, ExecutionUnit, Plan, Resource, Window};
use crate::isa::{Acc, EReg, Exu, Instruction, MReg, MRegPair, Mxu, RegFile, RegRef, WBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MxuConfig {
    // rows in a tensor register
    pub rows: u32,
    // cycles between consecutive rows in or out of the unit (1 = a row per cycle)
    pub stride: u32,
    // cycles from issue to the first row being read
    pub read_at: u32,
    // cycles from an SRAM read request (MRF or accumulator) to its data (1 default)
    pub sram_latency: u32,
    // cycles from an activation row entering the array to its result row leaving it
    pub pipeline_depth: u32,
    // cycles between neighbouring weight columns' first use: 1 when operands skew through a
    // systolic array, 0 when every column feeds every beat
    pub weight_skew: u32,
}

impl MxuConfig {
    // From the mxu0 and mxu1 sequencers at 32 rows.
    pub fn default(which: Mxu, rows: u32) -> Self {
        assert!(rows > 0, "a tensor register has at least one row");
        let (pipeline_depth, weight_skew) = match which {
            // PE(i, j) sees activation row r at feed + r + i + j
            Mxu::Mxu0 => (2 * (rows - 1), 1),
            Mxu::Mxu1 => (2, 0),
        };
        Self { rows, stride: 1, read_at: 0, sram_latency: 1, pipeline_depth, weight_skew }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MatrixUnit {
    pub which: Mxu,
    pub cfg: MxuConfig,
}

impl MatrixUnit {
    pub fn new(which: Mxu, cfg: MxuConfig) -> Self {
        Self { which, cfg }
    }

    pub fn exu(&self) -> Exu {
        match self.which {
            Mxu::Mxu0 => Exu::MatrixSystolic,
            Mxu::Mxu1 => Exu::MatrixInner,
        }
    }

    fn push_weight(&self, vd: WBuf, vs1: MReg, port: Resource) -> Plan {
        let (t, out) = (self.cfg.read_at, self.cfg.read_at + self.cfg.sram_latency);
        // each MRF row becomes one column of the weight tile
        vec![
            Access::read(mreg(vs1), self.stream(t)).via(port),
            Access::write(self.wbuf(vd), self.stream(out)).via(Resource::WeightWrite(self.which)),
        ]
    }

    fn matmul(&self, vd: Acc, vs1: MReg, vs2: WBuf, accumulate: bool) -> Plan {
        let c = &self.cfg;
        let t = c.read_at;
        let feed = t + c.sram_latency;
        let out = feed + c.pipeline_depth;
        // column j is in use from feed + j*skew until the last row has passed every PE row
        let weights = Window {
            at: feed,
            rows: c.rows,
            stride: c.weight_skew,
            hold: (c.rows - 1) * (c.stride + c.weight_skew),
        };
        let mut plan = vec![
            Access::read(mreg(vs1), self.stream(t)).via(self.read_port(0)),
            Access::read(self.wbuf(vs2), weights),
            Access::write(self.acc(vd), self.stream(out)).via(self.acc_write_port(vd)),
        ];
        if accumulate {
            // partial sums enter the array alongside their activation rows
            plan.push(Access::read(self.acc(vd), self.stream(t)).via(self.acc_read_port(vd)));
        }
        plan
    }

    fn push_acc(&self, vd: Acc, src: &[RegRef], ports: &[Resource]) -> Plan {
        let c = &self.cfg;
        let (t, out) = (c.read_at, c.read_at + c.sram_latency);
        let mut plan: Plan = (src.iter().zip(ports))
            .map(|(&reg, &port)| Access::read(reg, self.stream(t)).via(port))
            .collect();
        plan.push(
            Access::write(self.acc(vd), self.stream(out))
                .via(Resource::AccLoadPath(self.which))
                .via(self.acc_write_port(vd)),
        );
        plan
    }

    fn pop(&self, vs2: Acc, dst: &[RegRef], scale: Option<EReg>) -> Plan {
        let c = &self.cfg;
        let (t, out) = (c.read_at, c.read_at + c.sram_latency);
        let mut plan = vec![
            Access::read(self.acc(vs2), self.stream(t)).via(self.acc_read_port(vs2)),
        ];
        plan.extend(scale.map(|e| Access::read(ereg(e), Window::once(0))));
        // write port i carries register i of the destination
        plan.extend(
            (0..)
                .zip(dst)
                .map(|(i, &reg)| Access::write(reg, self.stream(out)).via(self.write_port(i))),
        );
        plan
    }

    // a single-register push streams on read port 1 if it's free, else port 0 
    fn single_push(&self, plan: impl Fn(Resource) -> Plan) -> Vec<Plan> {
        vec![plan(self.read_port(1)), plan(self.read_port(0))]
    }

    fn read_port(&self, i: u8) -> Resource {
        Resource::MrfReadPort(self.exu(), i)
    }

    fn write_port(&self, i: u8) -> Resource {
        Resource::MrfWritePort(self.exu(), i)
    }

    fn acc_read_port(&self, a: Acc) -> Resource {
        Resource::AccRead(self.which, a.idx())
    }

    fn acc_write_port(&self, a: Acc) -> Resource {
        Resource::AccWrite(self.which, a.idx())
    }

    fn stream(&self, at: u32) -> Window {
        Window::stream(at, self.cfg.rows, self.cfg.stride)
    }

    fn acc(&self, a: Acc) -> RegRef {
        RegRef { file: RegFile::Acc(self.which), idx: a.idx() }
    }

    fn wbuf(&self, w: WBuf) -> RegRef {
        RegRef { file: RegFile::Wb(self.which), idx: w.idx() }
    }
}

impl ExecutionUnit for MatrixUnit {
    fn plan(&self, insn: &Instruction) -> Vec<Plan> {
        assert_eq!(insn.spec().exu, self.exu(), "{insn} does not issue to {:?}", self.which);
        use Instruction as I;
        match *insn {
            I::VmatpushWeightMxu0 { vd, vs1 } | I::VmatpushWeightMxu1 { vd, vs1 } => {
                self.single_push(|port| self.push_weight(vd, vs1, port))
            }
            I::VmatmulMxu0 { vd, vs1, vs2 } | I::VmatmulMxu1 { vd, vs1, vs2 } => {
                vec![self.matmul(vd, vs1, vs2, false)]
            }
            I::VmatmulAccMxu0 { vd, vs1, vs2 } | I::VmatmulAccMxu1 { vd, vs1, vs2 } => {
                vec![self.matmul(vd, vs1, vs2, true)]
            }
            I::VmatpushAccFp8Mxu0 { vd, vs1 } | I::VmatpushAccFp8Mxu1 { vd, vs1 } => {
                self.single_push(|port| self.push_acc(vd, &[mreg(vs1)], &[port]))
            }
            I::VmatpushAccBf16Mxu0 { vd, vs1 } | I::VmatpushAccBf16Mxu1 { vd, vs1 } => {
                // a bf16 pair streams on both read ports at once
                vec![self.push_acc(vd, &pair(vs1), &[self.read_port(0), self.read_port(1)])]
            }
            I::VmatpopFp8AccMxu0 { vd, vs2, es1 } | I::VmatpopFp8AccMxu1 { vd, vs2, es1 } => {
                vec![self.pop(vs2, &[mreg(vd)], Some(es1))]
            }
            I::VmatpopBf16AccMxu0 { vd, vs2 } | I::VmatpopBf16AccMxu1 { vd, vs2 } => {
                vec![self.pop(vs2, &pair(vd), None)]
            }
            _ => unimplemented!("no MXU timing for {insn}"),
        }
    }
}

fn mreg(r: MReg) -> RegRef {
    RegRef { file: RegFile::Mrf, idx: r.idx() }
}

fn pair(p: MRegPair) -> [RegRef; 2] {
    [mreg(p.lo()), mreg(p.hi())]
}

fn ereg(e: EReg) -> RegRef {
    RegRef { file: RegFile::Erf, idx: e.idx() }
}
