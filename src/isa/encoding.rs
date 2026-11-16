// Instruction encoding: bit slots, per-format layouts, per-instruction spec record, and a
// [define_isa!] macro that turns the ISA table into the Instruction enum
//
// Every instruction is identified by **match/mask**

use core::fmt;

use crate::isa::operands::{OperandError, OperandKind};

const fn low_mask(bits: u32) -> u32 {
    if bits >= 32 {
        u32::MAX
    } else {
        (1 << bits) - 1
    }
}

// A field of the instruction word, possibly scattered. Each piece (src_lo, width, dst_lo) moves
// width bits starting at bit src_lo of the value to bit dst_lo of the word
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub pieces: &'static [(u32, u32, u32)],
}

impl Slot {
    // OR the slot into the instruction word
    pub const fn place(&self, value: u32) -> u32 {
        let mut word = 0;
        let mut i = 0;
        while i < self.pieces.len() {
            let (src, width, dst) = self.pieces[i];
            word |= ((value >> src) & low_mask(width)) << dst;
            i += 1;
        }
        word
    }

    // extract a given "slot" from the instruction word
    pub const fn extract(&self, word: u32) -> u32 {
        let mut value = 0;
        let mut i = 0;
        while i < self.pieces.len() {
            let (src, width, dst) = self.pieces[i];
            value |= ((word >> dst) & low_mask(width)) << src;
            i += 1;
        }
        value
    }

    // mask for the slot
    pub const fn mask(&self) -> u32 {
        self.place(u32::MAX)
    }

    // lowest portion of the field
    pub const fn operand_mask(&self, bits: u32) -> u32 {
        self.place(low_mask(bits))
    }

    // One past the highest value bit the slot carries.
    pub const fn value_bits(&self) -> u32 {
        let mut top = 0;
        let mut i = 0;
        while i < self.pieces.len() {
            let (src, width, _) = self.pieces[i];
            if src + width > top {
                top = src + width;
            }
            i += 1;
        }
        top
    }

    // Whether value survives a round trip through the slot.
    pub const fn fits(&self, value: u32) -> bool {
        self.extract(self.place(value)) == value
    }
}

// layouts of slots in the ISA
#[allow(non_snake_case, non_upper_case_globals)]
pub mod format {
    use super::Slot;

    macro_rules! field {
        ($lo:literal, $width:literal) => {
            Slot {
                pieces: &[(0, $width, $lo)],
            }
        };
    }

    pub mod R {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const rd: Slot = field!(7, 5);
        pub const funct3: Slot = field!(12, 3);
        pub const rs1: Slot = field!(15, 5);
        pub const rs2: Slot = field!(20, 5);
        pub const funct7: Slot = field!(25, 7);
    }

    pub mod I {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const rd: Slot = field!(7, 5);
        pub const funct3: Slot = field!(12, 3);
        pub const rs1: Slot = field!(15, 5);
        pub const imm: Slot = field!(20, 12);
        // Shift-immediate split of imm: imm[11:5] selects SRLI/SRAI, imm[4:0] is the shamt.
        pub const imm_hi: Slot = field!(25, 7);
        pub const shamt: Slot = field!(20, 5);
    }

    pub mod S {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const funct3: Slot = field!(12, 3);
        pub const rs1: Slot = field!(15, 5);
        pub const rs2: Slot = field!(20, 5);
        // imm[11:5] at 31:25, imm[4:0] at 11:7.
        pub const imm: Slot = Slot {
            pieces: &[(0, 5, 7), (5, 7, 25)],
        };
    }

    pub mod SB {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const funct3: Slot = field!(12, 3);
        pub const rs1: Slot = field!(15, 5);
        pub const rs2: Slot = field!(20, 5);
        // imm[12|10:5] at 31:25, imm[4:1|11] at 11:7; imm[0] is not encoded.
        pub const imm: Slot = Slot {
            pieces: &[(1, 4, 8), (5, 6, 25), (11, 1, 7), (12, 1, 31)],
        };
    }

    pub mod U {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const rd: Slot = field!(7, 5);
        pub const imm: Slot = field!(12, 20);
    }

    pub mod UJ {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const rd: Slot = field!(7, 5);
        // imm[20|10:1|11|19:12] at 31:12; imm[0] is not encoded.
        pub const imm: Slot = Slot {
            pieces: &[(1, 10, 21), (11, 1, 20), (12, 8, 12), (20, 1, 31)],
        };
    }

    pub mod VLS {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const vd: Slot = field!(7, 6);
        pub const funct2: Slot = field!(13, 2);
        pub const rs1: Slot = field!(15, 5);
        pub const imm: Slot = field!(20, 12);
    }

    pub mod VR {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const vd: Slot = field!(7, 6);
        // also carries es1 for instructions that take an exponent register.
        pub const vs1: Slot = field!(13, 6);
        pub const vs2: Slot = field!(19, 6);
        pub const funct7: Slot = field!(25, 7);
    }

    pub mod VI {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const vd: Slot = field!(7, 6);
        // 3 bits (15:13) per the spec;
        // every vli.* funct3 is below 4.
        pub const funct3: Slot = field!(13, 3);
        pub const imm: Slot = field!(16, 16);
    }

    pub mod CSR {
        use super::Slot;
        pub const opcode: Slot = field!(0, 7);
        pub const rd: Slot = field!(7, 5);
        pub const funct3: Slot = field!(12, 3);
        // A register for csrrw/s/c, 5-bit uimm for csrrwi/si/ci.
        pub const rs1: Slot = field!(15, 5);
        // CSR number
        pub const imm: Slot = field!(20, 12);
    }
}

// instruction format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    R,
    I,
    S,
    SB,
    U,
    UJ,
    VLS,
    VR,
    VI,
    CSR,
}

// execution unit an instruction issues to
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Exu {
    Scalar,
    Vector,
    Xlu,
    // MXU0, the systolic array
    MatrixSystolic,
    // MXU1, the inner-product tree
    MatrixInner,
    Dma,
    Lsu,
}

// how an instruction uses a register operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

impl Access {
    pub const fn reads(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    pub const fn writes(self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }
}

// One operand of an instruction, in assembly order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandSpec {
    pub name: &'static str,
    pub kind: OperandKind,
    // None for immediates and DMA channels.
    pub access: Option<Access>,
}

// Static description of one instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsnSpec {
    pub mnemonic: &'static str,
    pub format: Format,
    pub exu: Exu,
    // False for instructions with no functional effect (delay, ecall, ebreak, dma.wait).
    pub functional: bool,
    pub match_bits: u32,
    pub mask: u32,
    pub operands: &'static [OperandSpec],
}

impl InsnSpec {
    // Whether two specs could both match some word.
    pub const fn overlaps(&self, other: &InsnSpec) -> bool {
        (self.match_bits ^ other.match_bits) & self.mask & other.mask == 0
    }
}

// Index of the first pair of overlapping specs, if any.
pub const fn first_overlap(specs: &[InsnSpec]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < specs.len() {
        let mut j = i + 1;
        while j < specs.len() {
            if specs[i].overlaps(&specs[j]) {
                return Some((i, j));
            }
            j += 1;
        }
        i += 1;
    }
    None
}

// "a and b have overlapping encodings", built at compile time (const panics can't format).
#[doc(hidden)]
pub const fn overlap_message(a: &str, b: &str) -> ([u8; 128], usize) {
    let parts = [
        b"".as_slice(),
        a.as_bytes(),
        b" and ",
        b.as_bytes(),
        b" have overlapping encodings",
    ];
    let mut buf = [0u8; 128];
    let mut len = 0;
    let mut p = 0;
    while p < parts.len() {
        let mut i = 0;
        while i < parts[p].len() && len < buf.len() {
            buf[len] = parts[p][i];
            len += 1;
            i += 1;
        }
        p += 1;
    }
    (buf, len)
}

// most operands any instruction takes (dma.load/dma.store).
pub const MAX_OPERANDS: usize = 4;

// an instruction's operand field bits, in assembly order
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandBits {
    bits: [u32; MAX_OPERANDS],
    len: usize,
}

impl OperandBits {
    pub fn new(values: &[u32]) -> Self {
        let mut bits = [0; MAX_OPERANDS];
        bits[..values.len()].copy_from_slice(values);
        Self {
            bits,
            len: values.len(),
        }
    }
}

impl core::ops::Deref for OperandBits {
    type Target = [u32];

    fn deref(&self) -> &[u32] {
        &self.bits[..self.len]
    }
}

// a word that is not a valid instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    // no instruction's match/mask accepts the word.
    Illegal(u32),
    // the word matches mnemonic, but an operand value is forbidden (e.g. register pair m63).
    BadOperand { word: u32, mnemonic: &'static str },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Illegal(word) => write!(f, "illegal instruction 0x{word:08x}"),
            Self::BadOperand { word, mnemonic } => {
                write!(f, "0x{word:08x} is {mnemonic} with an invalid operand")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// failure to build an instruction from named operand values
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildError {
    Missing {
        mnemonic: &'static str,
        name: &'static str,
    },
    Unknown {
        mnemonic: &'static str,
        name: String,
    },
    Operand {
        name: &'static str,
        error: OperandError,
    },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { mnemonic, name } => write!(f, "{mnemonic} needs operand {name}"),
            Self::Unknown { mnemonic, name } => write!(f, "{mnemonic} has no operand {name}"),
            Self::Operand { name, error } => write!(f, "operand {name}: {error}"),
        }
    }
}

impl std::error::Error for BuildError {}

// a string that is not a mnemonic
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownMnemonic(pub String);

impl fmt::Display for UnknownMnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown mnemonic {}", self.0)
    }
}

impl std::error::Error for UnknownMnemonic {}

#[doc(hidden)]
macro_rules! __isa_slot {
    ($fmt:ident, $op:ident) => {
        $crate::isa::encoding::format::$fmt::$op
    };
    ($fmt:ident, $op:ident, $slot:ident) => {
        $crate::isa::encoding::format::$fmt::$slot
    };
}

#[doc(hidden)]
macro_rules! __isa_access {
    () => {
        None::<$crate::isa::encoding::Access>
    };
    (R) => {
        Some($crate::isa::encoding::Access::Read)
    };
    (W) => {
        Some($crate::isa::encoding::Access::Write)
    };
    (RW) => {
        Some($crate::isa::encoding::Access::ReadWrite)
    };
}

#[doc(hidden)]
macro_rules! __isa_functional {
    () => {
        true
    };
    (functional) => {
        false
    };
}

// Defines the ISA from a table. Each row is
//
//
// Variant = "mnemonic" [!functional], Exu, Format { fixed_slot: value, .. } (operand: Type [R|W|RW] [=> slot], ..);
//
//
// operands are listed in assembly order. an operand goes in the slot
// of the same name unless => slot says otherwise. register operands carry an access role.
// !functional marks instructions with no functional effect (delay, dma wait, dma config)
//
// macro generates Instruction, Mnemonic, and SPECS, plus compile-time checks that each
// fixed value fits its slot, operands fit their slots, no two fields share bits, every register
// operand has a role, and no two instructions' encodings overlap.
macro_rules! define_isa {
    ($(
        $V:ident = $mn:literal $(! $nf:ident)?, $exu:ident, $fmt:ident { $($fs:ident : $fv:expr),* $(,)? }
            ( $($op:ident : $ty:ident $([$acc:ident])? $(=> $slot:ident)?),* $(,)? );
    )*) => {
        // an instruction with typed operands.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Instruction {
            $(
                #[doc = concat!("", $mn, "")]
                $V { $($op: $ty),* },
            )*
        }

        // instruction identity without operands; Mnemonic as usize indexes [SPECS]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Mnemonic {
            $(
                #[doc = concat!("", $mn, "")]
                $V,
            )*
        }

        #[allow(non_snake_case)]
        mod rows {
            use super::*;
            $(
                pub mod $V {
                    // rows without operands use neither OperandSpec nor Operand.
                    #![allow(unused_imports)]
                    use super::*;
                    use $crate::isa::encoding::{format, Exu, Format, InsnSpec, OperandSpec};
                    use $crate::isa::operands::Operand;

                    pub const FIXED: u32 = 0 $(| format::$fmt::$fs.mask())*;
                    pub const MATCH: u32 = 0 $(| format::$fmt::$fs.place($fv))*;
                    pub const OPERANDS: u32 = 0
                        $(| $crate::isa::encoding::__isa_slot!($fmt, $op $(, $slot)?)
                            .operand_mask(<$ty as Operand>::BITS))*;
                    pub const MASK: u32 = !OPERANDS;
                    pub const SPEC: InsnSpec = InsnSpec {
                        mnemonic: $mn,
                        format: Format::$fmt,
                        exu: Exu::$exu,
                        functional: $crate::isa::encoding::__isa_functional!($($nf)?),
                        match_bits: MATCH,
                        mask: MASK,
                        operands: &[$(OperandSpec {
                            name: stringify!($op),
                            kind: <$ty as Operand>::KIND,
                            access: $crate::isa::encoding::__isa_access!($($acc)?),
                        }),*],
                    };

                    // rows without operands make some of these checks trivially true
                    #[allow(clippy::bad_bit_mask)]
                    const _: () = {
                        $(assert!(
                            format::$fmt::$fs.fits($fv),
                            concat!("", $mn, ": fixed field ", stringify!($fs), " does not fit its slot")
                        );)*
                        $(assert!(
                            <$ty as Operand>::BITS
                                <= $crate::isa::encoding::__isa_slot!($fmt, $op $(, $slot)?).value_bits(),
                            concat!("", $mn, ": operand ", stringify!($op), " is wider than its slot")
                        );)*
                        $(assert!(
                            <$ty as Operand>::KIND.is_register()
                                == $crate::isa::encoding::__isa_access!($($acc)?).is_some(),
                            concat!("", $mn, ": operand ", stringify!($op),
                                " needs an access role iff it is a register")
                        );)*
                        assert!(
                            (0u32 $(+ $crate::isa::encoding::__isa_slot!($fmt, $op $(, $slot)?)
                                .operand_mask(<$ty as Operand>::BITS).count_ones())*)
                                == OPERANDS.count_ones(),
                            concat!("", $mn, ": two operands share bits")
                        );
                        assert!(FIXED & OPERANDS == 0, concat!("", $mn, ": a fixed field overlaps an operand"));
                        assert!(FIXED & 0x7f == 0x7f, concat!("", $mn, ": the opcode is not fixed"));
                        assert!(
                            SPEC.operands.len() <= $crate::isa::encoding::MAX_OPERANDS,
                            concat!("", $mn, ": too many operands")
                        );
                    };

                    #[inline]
                    #[allow(unused_variables)] // rows without operands ignore the word
                    pub fn decode(word: u32) -> Option<Instruction> {
                        Some(Instruction::$V {
                            $($op: <$ty as Operand>::from_bits(
                                $crate::isa::encoding::__isa_slot!($fmt, $op $(, $slot)?).extract(word),
                            )?,)*
                        })
                    }
                }
            )*
        }

        const TABLE: [$crate::isa::encoding::InsnSpec; Mnemonic::COUNT] = [$(rows::$V::SPEC,)*];

        // every instruction's spec indexed by Mnemonic as usize
        pub static SPECS: [$crate::isa::encoding::InsnSpec; Mnemonic::COUNT] = TABLE;

        const _: () = if let Some((a, b)) = $crate::isa::encoding::first_overlap(&TABLE) {
            let (buf, len) = $crate::isa::encoding::overlap_message(TABLE[a].mnemonic, TABLE[b].mnemonic);
            match ::core::str::from_utf8(buf.split_at(len).0) {
                Ok(msg) => panic!("{}", msg),
                Err(_) => panic!("two instructions have overlapping encodings"),
            }
        };

        impl Mnemonic {
            pub const ALL: &'static [Mnemonic] = &[$(Mnemonic::$V,)*];
            pub const COUNT: usize = Self::ALL.len();

            pub fn spec(self) -> &'static $crate::isa::encoding::InsnSpec {
                &SPECS[self as usize]
            }

            pub fn as_str(self) -> &'static str {
                self.spec().mnemonic
            }
        }

        impl ::core::str::FromStr for Mnemonic {
            type Err = $crate::isa::encoding::UnknownMnemonic;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($mn => Ok(Mnemonic::$V),)*
                    _ => Err($crate::isa::encoding::UnknownMnemonic(s.to_owned())),
                }
            }
        }

        impl ::core::fmt::Display for Mnemonic {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Instruction {
            pub const fn mnemonic(&self) -> Mnemonic {
                match self {
                    $(Self::$V { .. } => Mnemonic::$V,)*
                }
            }

            pub fn spec(&self) -> &'static $crate::isa::encoding::InsnSpec {
                self.mnemonic().spec()
            }

            // the 32-bit instruction word
            pub fn encode(&self) -> u32 {
                use $crate::isa::operands::Operand;
                match *self {
                    $(Self::$V { $($op),* } => {
                        rows::$V::MATCH
                            $(| $crate::isa::encoding::__isa_slot!($fmt, $op $(, $slot)?).place($op.to_bits()))*
                    })*
                }
            }

            // decodes a 32-bit instruction word
            pub fn decode(word: u32) -> Result<Self, $crate::isa::encoding::DecodeError> {
                $(
                    if word & rows::$V::MASK == rows::$V::MATCH {
                        return rows::$V::decode(word)
                            .ok_or($crate::isa::encoding::DecodeError::BadOperand { word, mnemonic: $mn });
                    }
                )*
                Err($crate::isa::encoding::DecodeError::Illegal(word))
            }

            // operand field bits in assembly order, parallel to spec().operands
            pub fn operand_bits(&self) -> $crate::isa::encoding::OperandBits {
                use $crate::isa::operands::Operand;
                match *self {
                    $(Self::$V { $($op),* } => $crate::isa::encoding::OperandBits::new(&[$($op.to_bits()),*]),)*
                }
            }

            // builds an instruction from operand values keyed by operand name
            pub fn from_named(
                mnemonic: Mnemonic,
                values: &[(&str, i64)],
            ) -> Result<Self, $crate::isa::encoding::BuildError> {
                use $crate::isa::encoding::BuildError;
                use $crate::isa::operands::Operand;
                let spec = mnemonic.spec();
                if let Some((name, _)) = values.iter().find(|(n, _)| spec.operands.iter().all(|o| o.name != *n)) {
                    return Err(BuildError::Unknown { mnemonic: spec.mnemonic, name: (*name).to_owned() });
                }
                let get = |name: &'static str| -> Result<i64, BuildError> {
                    values
                        .iter()
                        .find(|(n, _)| *n == name)
                        .map(|&(_, v)| v)
                        .ok_or(BuildError::Missing { mnemonic: spec.mnemonic, name })
                };
                Ok(match mnemonic {
                    $(Mnemonic::$V => Self::$V {
                        $($op: <$ty as Operand>::from_i64(get(stringify!($op))?)
                            .map_err(|error| BuildError::Operand { name: stringify!($op), error })?,)*
                    },)*
                })
            }
        }

        impl ::core::fmt::Display for Instruction {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(Self::$V { $($op),* } => {
                        f.write_str($mn)?;
                        let operands: &[&dyn ::core::fmt::Display] = &[$($op),*];
                        for (i, operand) in operands.iter().enumerate() {
                            f.write_str(if i == 0 { " " } else { ", " })?;
                            operand.fmt(f)?;
                        }
                        Ok(())
                    })*
                }
            }
        }
    };
}

pub(crate) use {__isa_access, __isa_functional, __isa_slot, define_isa};
