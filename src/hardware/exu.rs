use crate::hardware::vpu::LaneBox;
use crate::isa::{Exu, Instruction, Mxu, RegFile, RegRef};


// window in which a location is accessed
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub at: u32,
    pub rows: u32,
    // cycles between consecutive rows (1 means 1 row per cycle)
    pub stride: u32,
    // extra cycles each row stays in use (stationary operands such as weights)
    pub hold: u32,
}

impl Window {
    // a single touch: scalar operands, or anything modeled as one instant
    pub const fn once(at: u32) -> Self {
        Self { at, rows: 1, stride: 0, hold: 0 }
    }

    // rows touched one after another, stride cycles apart
    pub const fn stream(at: u32, rows: u32, stride: u32) -> Self {
        assert!(rows > 0, "a window touches at least one row");
        Self { at, rows, stride, hold: 0 }
    }

    // first cycle row r is touched
    pub const fn first(&self, r: u32) -> u32 {
        self.at + r * self.stride
    }

    // last cycle row r is touched
    pub const fn last(&self, r: u32) -> u32 {
        self.first(r) + self.hold
    }

    // last cycle any row is touched
    pub const fn end(&self) -> u32 {
        self.last(self.rows - 1)
    }
}

// resources that locations are accessed through
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Resource {
    // unit's request ports into the MRF
    MrfReadPort(Exu, u8),
    MrfWritePort(Exu, u8),
    // MXU accumulator load path
    AccLoadPath(Mxu),
    // acc buffer's SRAM ports (1R1W per buffer)
    AccRead(Mxu, u8),
    AccWrite(Mxu, u8),
    // weight buffer write port
    WeightWrite(Mxu),
    // VPU lane box: one op streams through it at a time, from either slot
    LaneBox(LaneBox),
    // an MRF bank SRAM ports (1R1W per bank)
    MrfBankRead(u8),
    MrfBankWrite(u8),
}

impl Resource {
    pub fn mrf_bank_port(loc: Loc, write: bool, mrf_banks: u8) -> Option<Self> {
        match loc {
            Loc::Reg(RegRef { file: RegFile::Mrf, idx }) => {
                let bank = idx % mrf_banks;
                Some(if write { Self::MrfBankWrite(bank) } else { Self::MrfBankRead(bank) })
            }
            _ => None,
        }
    }
}

// a location (register file or memory). tracked by rows for data hazards
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Loc {
    Reg(RegRef),
    Mem { mem: Memory, start: u64, len: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Memory {
    Vmem,
    Dram,
}

impl From<RegRef> for Loc {
    fn from(reg: RegRef) -> Self {
        Self::Reg(reg)
    }
}

// one location read or written over a window and the hardware the stream holds while it runs
// scoreboard will check the location for data hazards and hardware for data hazards

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    pub loc: Loc,
    pub write: bool,
    pub win: Window,
    // hardware the stream holds, e.g. a unit's MRF port or a lane box
    pub via: Vec<Resource>,
}

impl Access {
    pub fn read(loc: impl Into<Loc>, win: Window) -> Self {
        Self { loc: loc.into(), write: false, win, via: Vec::new() }
    }

    pub fn write(loc: impl Into<Loc>, win: Window) -> Self {
        Self { loc: loc.into(), write: true, win, via: Vec::new() }
    }

    pub fn via(mut self, res: Resource) -> Self {
        self.via.push(res);
        self
    }
}

// sequence of accesses an instruction takes (micro-ops?)
pub type Plan = Vec<Access>;

pub trait ExecutionUnit {
    fn plan(&self, insn: &Instruction) -> Vec<Plan>;
}
