// Atlas ISA: operand types, instruction encodings, and the instruction table
//

pub mod encoding;
pub mod footprint;
pub mod instructions;
pub mod operands;

pub use encoding::{Access, BuildError, DecodeError, Exu, Format, InsnSpec, OperandSpec};
pub use footprint::{Mxu, RegFile, RegRef};
pub use instructions::{Instruction, Mnemonic, SPECS};
pub use operands::{
    Acc, BImm, Channel, EReg, Imm12, Imm16, Imm20, JImm, MReg, MRegPair, Operand, OperandError,
    OperandKind, UImm5, WBuf, XReg, acc, ch, e, m, mp, w, x,
};
