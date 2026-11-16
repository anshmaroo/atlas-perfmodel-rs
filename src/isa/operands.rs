// Operand types: typed register indices and immediates

// Immediates are stored as bit patterns. a constructor accepts the signed-or-unsigned
// range and keeps only the field bits, so Imm12::new(-1) == Imm12::new(4095)

use core::fmt;

// The type of an operand field
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperandKind {
    XReg,
    EReg,
    MReg,
    MRegPair,
    Acc,
    WBuf,
    Channel,
    Imm12,
    UImm5,
    Imm16,
    Imm20,
    BImm,
    JImm,
}

impl OperandKind {
    // accepted constructor range [lo, hi)
    pub const fn range(self) -> (i64, i64) {
        match self {
            Self::XReg | Self::EReg => (0, 32),
            Self::MReg => (0, 64),
            Self::MRegPair => (0, 63),
            Self::Acc | Self::WBuf => (0, 2),
            Self::Channel => (0, 8),
            Self::Imm12 => (-(1 << 11), 1 << 12),
            Self::UImm5 => (0, 1 << 5),
            Self::Imm16 => (-(1 << 15), 1 << 16),
            Self::Imm20 => (-(1 << 19), 1 << 20),
            Self::BImm => (-(1 << 12), 1 << 13),
            Self::JImm => (-(1 << 20), 1 << 21),
        }
    }

    // Number of value bits the operand occupies in its slot
    pub const fn bits(self) -> u32 {
        match self {
            Self::XReg | Self::EReg | Self::UImm5 => 5,
            Self::MReg | Self::MRegPair => 6,
            Self::Acc | Self::WBuf => 1,
            Self::Channel => 3,
            Self::Imm12 => 12,
            Self::Imm16 => 16,
            Self::Imm20 => 20,
            Self::BImm => 13,
            Self::JImm => 21,
        }
    }

    /// True for operands that name architectural state
    /// DMA channels are encoded like registers but select a queue, not state
    pub const fn is_register(self) -> bool {
        matches!(
            self,
            Self::XReg | Self::EReg | Self::MReg | Self::MRegPair | Self::Acc | Self::WBuf
        )
    }
}

// A value that cannot be represented by an operand type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperandError {
    OutOfRange { kind: OperandKind, value: i64 },
    // branch and jump immediates carry an implicit zero low bit.
    Odd { kind: OperandKind, value: i64 },
}

impl fmt::Display for OperandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OutOfRange { kind, value } => {
                let (lo, hi) = kind.range();
                write!(f, "{kind:?} value {value} is out of range [{lo}, {hi})")
            }
            Self::Odd { kind, value } => write!(f, "{kind:?} value {value} must be even"),
        }
    }
}

impl std::error::Error for OperandError {}

// implemented by every operand type the ISA table can place in an instruction slot
pub trait Operand: Copy + fmt::Display {
    const KIND: OperandKind;
    // Number of value bits. a slot wider than this has its upper bits fixed to zero.
    const BITS: u32;
    // The value's field bits (two's complement for signed immediates)
    fn to_bits(self) -> u32;
    // inverse of [Operand::to_bits]; None for bit patterns the type forbids
    // (e.g. 63 for a register pair).
    fn from_bits(bits: u32) -> Option<Self>;
    /// checked construction from an integer
    fn from_i64(value: i64) -> Result<Self, OperandError>;
}

const fn check_range(kind: OperandKind, value: i64) -> Result<(), OperandError> {
    let (lo, hi) = kind.range();
    if lo <= value && value < hi {
        Ok(())
    } else {
        Err(OperandError::OutOfRange { kind, value })
    }
}

macro_rules! register {
    ($(#[$doc:meta])* $name:ident, $helper:ident, $prefix:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u8);

        impl $name {
            pub const fn new(idx: i64) -> Result<Self, OperandError> {
                match check_range(OperandKind::$name, idx) {
                    Ok(()) => Ok(Self(idx as u8)),
                    Err(e) => Err(e),
                }
            }

            pub const fn idx(self) -> u8 {
                self.0
            }
        }

        #[doc = concat!("Shorthand for ", stringify!($name), "::new(idx).unwrap(); ")]
        #[doc = "an out-of-range literal fails at compile time in const contexts."]
        pub const fn $helper(idx: i64) -> $name {
            match $name::new(idx) {
                Ok(r) => r,
                Err(_) => panic!(concat!(stringify!($name), " index out of range")),
            }
        }

        impl Operand for $name {
            const KIND: OperandKind = OperandKind::$name;
            const BITS: u32 = OperandKind::$name.bits();

            fn to_bits(self) -> u32 {
                u32::from(self.0)
            }

            fn from_bits(bits: u32) -> Option<Self> {
                Self::new(i64::from(bits)).ok()
            }

            fn from_i64(value: i64) -> Result<Self, OperandError> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($prefix, "{}"), self.0)
            }
        }
    };
}

register!(
    // Scalar register x0-x31
    XReg, x, "x"
);
register!(
    // Exponent (scale) register e0–e31
    EReg, e, "e"
);
register!(
    // Matrix register m0–m63
    MReg, m, "m"
);
register!(
    // A BF16 register pair {m[r], m[r+1]}, named by its low register r in 0..=62
    //
    // VPU additionally requires r to be even but the MXU does not, so alignment is
    // left to the VPU issue model rather than made an ISA-level rule
    MRegPair, mp, "m"
);
register!(
    // MXU accumulator buffer acc0/acc1
    Acc, acc, "acc"
);
register!(
    /// MXU weight buffer slot w0/w1
    WBuf,
    w,
    "w"
);
register!(
    /// DMA channel ch0–ch7, encoded in (immediate) funct3
    Channel,
    ch,
    "ch"
);

impl MRegPair {
    // low register of the pair
    pub const fn lo(self) -> MReg {
        MReg(self.0)
    }

    // high register of the pair
    pub const fn hi(self) -> MReg {
        MReg(self.0 + 1)
    }
}

macro_rules! immediate {
    ($(#[$doc:meta])* $name:ident, repr: $repr:ty, even: $even:literal, signed_display: $signed:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name($repr);

        impl $name {
            const MASK: u32 = (1 << OperandKind::$name.bits()) - 1;

            // accepts the signed or unsigned range and stores the field bits
            pub const fn new(value: i64) -> Result<Self, OperandError> {
                match check_range(OperandKind::$name, value) {
                    Err(e) => Err(e),
                    Ok(()) if $even && value & 1 != 0 => {
                        Err(OperandError::Odd { kind: OperandKind::$name, value })
                    }
                    Ok(()) => Ok(Self((value as u32 & Self::MASK) as $repr)),
                }
            }

            // raw field in bits
            pub const fn bits(self) -> u32 {
                self.0 as u32
            }

            // field read as unsigned
            pub const fn zext(self) -> u32 {
                self.0 as u32
            }

            // the field read as two's complement
            pub const fn sext(self) -> i32 {
                let shift = 32 - OperandKind::$name.bits();
                (((self.0 as u32) << shift) as i32) >> shift
            }
        }

        impl Operand for $name {
            const KIND: OperandKind = OperandKind::$name;
            const BITS: u32 = OperandKind::$name.bits();

            fn to_bits(self) -> u32 {
                self.bits()
            }

            fn from_bits(bits: u32) -> Option<Self> {
                (bits & !Self::MASK == 0 && !($even && bits & 1 != 0)).then_some(Self(bits as $repr))
            }

            fn from_i64(value: i64) -> Result<Self, OperandError> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if $signed {
                    write!(f, "{}", self.sext())
                } else {
                    write!(f, "{}", self.zext())
                }
            }
        }
    };
}

immediate!(
    // 12-bit immediate (I/S/VLS/CSR formats)
    Imm12, repr: u16, even: false, signed_display: true
);
immediate!(
    // 5-bit unsigned immediate: shift amounts and the CSR uimm
    UImm5, repr: u8, even: false, signed_display: false
);
immediate!(
    // 16-bit immediate (VI format; vli.* takes raw BF16 bits)
    Imm16, repr: u16, even: false, signed_display: true
);
immediate!(
    // 20-bit upper immediate (U format)
    Imm20, repr: u32, even: false, signed_display: true
);
immediate!(
    // Branch immediate (SBImm12): the raw 13-bit B-type value, whose low bit is an
    // implicit zero
    BImm, repr: u16, even: true, signed_display: true
);
immediate!(
    // Jump immediate (Imm21): the raw 21-bit J-type value, whose low bit is an
    // implicit zero
    JImm, repr: u32, even: true, signed_display: true
);

macro_rules! word_offset_immediate {
    ($name:ident) => {
        impl $name {
            // from the raw encoded immediate
            pub const fn from_raw(raw: i64) -> Result<Self, OperandError> {
                Self::new(raw)
            }

            // from a displacement in instruction words
            pub const fn from_words(words: i64) -> Result<Self, OperandError> {
                match words.checked_mul(2) {
                    Some(raw) => Self::new(raw),
                    None => Err(OperandError::OutOfRange {
                        kind: OperandKind::$name,
                        value: words,
                    }),
                }
            }

            //s igned displacement in instruction words
            pub const fn word_offset(self) -> i32 {
                self.sext() >> 1
            }
        }
    };
}

word_offset_immediate!(BImm);
word_offset_immediate!(JImm);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_ranges() {
        assert_eq!(XReg::new(31).map(XReg::idx), Ok(31));
        assert!(XReg::new(32).is_err());
        assert!(XReg::new(-1).is_err());
        assert!(MReg::new(63).is_ok());
        assert!(Acc::new(2).is_err());
        assert!(Channel::new(7).is_ok());
        assert!(Channel::new(8).is_err());
    }

    #[test]
    fn register_pairs_exclude_63_but_not_odd() {
        assert_eq!(mp(62).hi(), m(63));
        assert!(MRegPair::new(63).is_err());
        assert!(MRegPair::new(61).is_ok());
        assert_eq!(MRegPair::from_bits(63), None);
        assert_eq!(MRegPair::from_bits(5), Some(mp(5)));
    }

    #[test]
    fn immediates_are_bit_patterns() {
        assert_eq!(Imm12::new(-1), Imm12::new(4095));
        assert_eq!(Imm12::new(-2048).map(Imm12::bits), Ok(0x800));
        assert!(Imm12::new(-2049).is_err());
        assert!(Imm12::new(4096).is_err());
        let imm = Imm12::new(-5).unwrap();
        assert_eq!((imm.sext(), imm.zext()), (-5, 4091));
        assert_eq!(Imm16::new(-1).map(Imm16::bits), Ok(0xFFFF));
        assert_eq!(Imm20::new(-524288).map(Imm20::sext), Ok(-524288));
        assert!(UImm5::new(32).is_err());
        assert_eq!(UImm5::new(31).unwrap().to_string(), "31");
    }

    #[test]
    fn branch_and_jump_immediates() {
        assert!(matches!(BImm::new(3), Err(OperandError::Odd { .. })));
        assert_eq!(BImm::from_words(-2048).map(BImm::bits), Ok(0x1000));
        assert!(BImm::from_words(-2049).is_err());
        assert_eq!(BImm::from_raw(-8).unwrap().word_offset(), -4);
        assert_eq!(BImm::from_bits(0x1001), None);
        assert_eq!(JImm::from_words(-524288).map(JImm::sext), Ok(-1048576));
        assert_eq!(JImm::from_raw(2097150).unwrap().word_offset(), -1);
        assert!(JImm::from_words(i64::MAX).is_err());
    }

    #[test]
    fn display() {
        assert_eq!(x(3).to_string(), "x3");
        assert_eq!(acc(1).to_string(), "acc1");
        assert_eq!(ch(7).to_string(), "ch7");
        assert_eq!(Imm12::new(4095).unwrap().to_string(), "-1");
    }
}
