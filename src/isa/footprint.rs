// Register footprint: which architectural registers an instruction reads and writes.
//
// Derived from the instruction encoding table's operand types and access roles

use crate::isa::encoding::{Access, Exu};
use crate::isa::instructions::Instruction;
use crate::isa::operands::OperandKind;

// Matrix unit
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mxu {
    // systolic array
    Mxu0,
    // inner-product trees
    Mxu1,
}

// A register file
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RegFile {
    Xrf,
    Erf,
    Mrf,
    Acc(Mxu),
    Wb(Mxu),
}

// one architectural register
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegRef {
    pub file: RegFile,
    pub idx: u8,
}

impl Instruction {
    // registers this instruction reads
    pub fn reads(&self) -> impl Iterator<Item = RegRef> {
        self.footprint(Access::reads)
    }

    // registers this instruction writes
    pub fn writes(&self) -> impl Iterator<Item = RegRef> {
        self.footprint(Access::writes)
    }

    fn footprint(&self, wanted: fn(Access) -> bool) -> impl Iterator<Item = RegRef> {
        let spec = self.spec();
        let bits = self.operand_bits();
        let mxu = match spec.exu {
            Exu::MatrixSystolic => Some(Mxu::Mxu0),
            Exu::MatrixInner => Some(Mxu::Mxu1),
            _ => None,
        };
        let local = move |file: fn(Mxu) -> RegFile| {
            file(mxu.expect("MXU-local operand on an instruction that doesn't issue to an MXU"))
        };
        spec.operands
            .iter()
            .zip((0..bits.len()).map(move |i| bits[i]))
            .filter(move |(op, _)| op.access.is_some_and(wanted))
            .flat_map(move |(op, bits)| {
                let idx = bits as u8;
                let (file, banks) = match op.kind {
                    OperandKind::XReg => (RegFile::Xrf, 1),
                    OperandKind::EReg => (RegFile::Erf, 1),
                    OperandKind::MReg => (RegFile::Mrf, 1),
                    OperandKind::MRegPair => (RegFile::Mrf, 2),
                    OperandKind::Acc => (local(RegFile::Acc), 1),
                    OperandKind::WBuf => (local(RegFile::Wb), 1),
                    // table guarantees only registers have an access role
                    _ => unreachable!("non-register operand with an access role"),
                };
                [
                    Some(RegRef { file, idx }),
                    (banks == 2).then(|| RegRef { file, idx: idx + 1 }),
                ]
            })
            .flatten()
    }
}
