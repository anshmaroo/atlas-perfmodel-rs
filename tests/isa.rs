//! Properties of the ISA table and its generated API.

use std::collections::BTreeSet;

use npu_model_rs::isa::{
    BImm, BuildError, DecodeError, Exu, Imm12, Instruction, Mnemonic, Mxu, OperandKind, RegFile,
    RegRef, SPECS, acc, ch, e, m, mp, w, x,
};

/// xorshift64: deterministic, dependency-free randomness.
fn rng(mut state: u64) -> impl FnMut() -> u32 {
    move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 32) as u32
    }
}

#[test]
fn table_shape() {
    assert_eq!(Mnemonic::COUNT, 99);
    assert_eq!(SPECS.len(), Mnemonic::COUNT);
    for (i, &mnemonic) in Mnemonic::ALL.iter().enumerate() {
        assert_eq!(mnemonic as usize, i);
        assert_eq!(mnemonic.as_str().parse::<Mnemonic>(), Ok(mnemonic));
    }
    assert!("vadd".parse::<Mnemonic>().is_err());
    // A decoded instruction is a small `Copy` value, not a heap object.
    assert!(
        size_of::<Instruction>() <= 8,
        "Instruction is {} bytes",
        size_of::<Instruction>()
    );
}

#[test]
fn decode_inverts_encode_on_random_words() {
    let mut next = rng(0x9E37_79B9_7F4A_7C15);
    // Uniformly random words: strict masks make most of them illegal.
    for _ in 0..1 << 20 {
        let word = next();
        if let Ok(insn) = Instruction::decode(word) {
            assert_eq!(insn.encode(), word, "{insn}");
        }
    }
    // Every instruction's fixed bits with random operand bits.
    for spec in &SPECS {
        for _ in 0..2000 {
            let word = spec.match_bits | (next() & !spec.mask);
            match Instruction::decode(word) {
                Ok(insn) => {
                    assert_eq!(insn.spec().mnemonic, spec.mnemonic);
                    assert_eq!(insn.encode(), word, "{insn}");
                }
                Err(DecodeError::BadOperand { mnemonic, .. }) => {
                    assert_eq!(mnemonic, spec.mnemonic);
                    let has_pair = spec
                        .operands
                        .iter()
                        .any(|o| o.kind == OperandKind::MRegPair);
                    assert!(
                        has_pair,
                        "{mnemonic}: only register pairs have forbidden values"
                    );
                }
                Err(err) => panic!("{}: {err}", spec.mnemonic),
            }
        }
    }
    assert_eq!(Instruction::decode(0), Err(DecodeError::Illegal(0)));
}

#[test]
fn quirky_encodings() {
    // SRAI: imm[11:5] = 0b0100000 distinguishes it from SRLI.
    let srai = Instruction::Srai {
        rd: x(1),
        rs1: x(2),
        imm: npu_model_rs::isa::UImm5::new(3).unwrap(),
    };
    assert_eq!(srai.encode(), 0x4031_5093);
    // ECALL and EBREAK differ only in the immediate.
    assert_eq!(Instruction::Ecall {}.encode(), 0x0000_0073);
    assert_eq!(Instruction::Ebreak {}.encode(), 0x0010_0073);
    // The DMA channel is funct3.
    assert_eq!(Instruction::DmaWait { ch: ch(5) }.encode(), 0x0200_507f);
    // `es1` sits in the vs1 field.
    let vpack = Instruction::VpackBf16Fp8 {
        vd: m(1),
        vs2: mp(4),
        es1: e(3),
    };
    assert_eq!(vpack.encode(), 0x8820_60d7);
    // Signed and unsigned spellings of an immediate are the same instruction.
    let a = Instruction::Addi {
        rd: x(1),
        rs1: x(0),
        imm: Imm12::new(-1).unwrap(),
    };
    let b = Instruction::Addi {
        rd: x(1),
        rs1: x(0),
        imm: Imm12::new(4095).unwrap(),
    };
    assert_eq!(a, b);
    // Branch offsets: raw immediate = 2 x word offset.
    let beq = Instruction::Beq {
        rs1: x(1),
        rs2: x(2),
        imm: BImm::from_words(-4).unwrap(),
    };
    assert_eq!(Instruction::decode(beq.encode()), Ok(beq));
}

#[test]
fn display() {
    let cases = [
        (
            Instruction::Add {
                rd: x(1),
                rs1: x(2),
                rs2: x(3),
            },
            "add x1, x2, x3",
        ),
        (
            Instruction::VpackBf16Fp8 {
                vd: m(3),
                vs2: mp(4),
                es1: e(5),
            },
            "vpack.bf16.fp8 m3, m4, e5",
        ),
        (
            Instruction::VmatmulMxu0 {
                vd: acc(1),
                vs1: m(2),
                vs2: w(0),
            },
            "vmatmul.mxu0 acc1, m2, w0",
        ),
        (
            Instruction::Beq {
                rs1: x(1),
                rs2: x(2),
                imm: BImm::from_raw(-8).unwrap(),
            },
            "beq x1, x2, -8",
        ),
        (Instruction::DmaWait { ch: ch(3) }, "dma.wait ch3"),
        (Instruction::Ecall {}, "ecall"),
    ];
    for (insn, text) in cases {
        assert_eq!(insn.to_string(), text);
    }
}

#[test]
fn from_named_errors() {
    let missing = Instruction::from_named(Mnemonic::Add, &[("rd", 1), ("rs1", 2)]);
    assert_eq!(
        missing,
        Err(BuildError::Missing {
            mnemonic: "add",
            name: "rs2"
        })
    );
    let unknown = Instruction::from_named(Mnemonic::Ecall, &[("imm", 0)]);
    assert!(matches!(unknown, Err(BuildError::Unknown { .. })));
    let range = Instruction::from_named(Mnemonic::Addi, &[("rd", 32), ("rs1", 0), ("imm", 0)]);
    assert!(matches!(range, Err(BuildError::Operand { name: "rd", .. })));
}

fn mrf(idx: u8) -> RegRef {
    RegRef {
        file: RegFile::Mrf,
        idx,
    }
}

fn set(it: impl Iterator<Item = RegRef>) -> BTreeSet<RegRef> {
    it.collect()
}

#[test]
fn scalar_footprints() {
    let seld = Instruction::Seld {
        rd: e(4),
        imm: Imm12::new(0).unwrap(),
        rs1: x(2),
    };
    assert_eq!(
        set(seld.writes()),
        BTreeSet::from([RegRef {
            file: RegFile::Erf,
            idx: 4
        }])
    );
    assert_eq!(
        set(seld.reads()),
        BTreeSet::from([RegRef {
            file: RegFile::Xrf,
            idx: 2
        }])
    );
    // dma.load reads all three registers: rd is the VMEM address.
    let dma = Instruction::DmaLoad {
        rd: x(6),
        rs1: x(10),
        rs2: x(12),
        ch: ch(0),
    };
    assert_eq!(set(dma.reads()).len(), 3);
    assert_eq!(set(dma.writes()).len(), 0);
    let acc_mm = Instruction::VmatmulAccMxu1 {
        vd: acc(1),
        vs1: m(2),
        vs2: w(0),
    };
    let acc1 = RegRef {
        file: RegFile::Acc(Mxu::Mxu1),
        idx: 1,
    };
    assert!(set(acc_mm.reads()).contains(&acc1));
    assert_eq!(set(acc_mm.writes()), BTreeSet::from([acc1]));
}

/// The VPU and MXU models in Python (`hardware/vpu.py`, `hardware/mxu.py`) compute matrix-register
/// footprints from mnemonic strings. Restate those rules and check the table's types and roles agree.
#[test]
fn matrix_footprints_match_python_unit_models() {
    const VPU_TWO_INPUT: [&str; 5] = [
        "vadd.bf16",
        "vsub.bf16",
        "vmul.bf16",
        "vminimum.bf16",
        "vmaximum.bf16",
    ];
    let (vd, vs1, vs2) = (10u8, 20u8, 30u8);
    let pair = |r: u8| BTreeSet::from([mrf(r), mrf(r + 1)]);
    let one = |r: u8| BTreeSet::from([mrf(r)]);

    let mut checked = 0;
    for spec in &SPECS {
        let name = spec.mnemonic;
        let values: Vec<(&str, i64)> = spec
            .operands
            .iter()
            .map(|o| {
                let v = match (o.kind, o.name) {
                    (OperandKind::MReg | OperandKind::MRegPair, "vd") => vd,
                    (OperandKind::MReg | OperandKind::MRegPair, "vs1") => vs1,
                    (OperandKind::MReg | OperandKind::MRegPair, "vs2") => vs2,
                    (OperandKind::Acc | OperandKind::WBuf, _) => 1,
                    _ => 4, // valid (and even) for every other operand kind
                };
                (o.name, i64::from(v))
            })
            .collect();
        let insn = Instruction::from_named(name.parse().unwrap(), &values).unwrap();
        let reads: BTreeSet<_> = insn.reads().filter(|r| r.file == RegFile::Mrf).collect();
        let writes: BTreeSet<_> = insn.writes().filter(|r| r.file == RegFile::Mrf).collect();

        let (expected_reads, expected_writes) = match spec.exu {
            Exu::Vector => {
                // vpu.py: pair_write / source / VPU_TWO_INPUT logic.
                let writes = if ["vpack.bf16.fp8", "vli.col", "vli.one"].contains(&name) {
                    one(vd)
                } else {
                    pair(vd)
                };
                let mut reads = BTreeSet::new();
                if !name.starts_with("vli.") {
                    let source = if ["vpack.bf16.fp8", "vunpack.fp8.bf16"].contains(&name) {
                        vs2
                    } else {
                        vs1
                    };
                    reads.extend(if name == "vunpack.fp8.bf16" {
                        one(source)
                    } else {
                        pair(source)
                    });
                    if VPU_TWO_INPUT.contains(&name) {
                        reads.extend(pair(vs2));
                    }
                }
                (reads, writes)
            }
            Exu::MatrixSystolic | Exu::MatrixInner => {
                // mxu.py: banks = pair for push_bf16/pop_bf16; mreg = vd for pops, else vs1.
                let pop = name.starts_with("vmatpop");
                let mreg = if pop { vd } else { vs1 };
                let banks = if name.contains(".bf16.") {
                    pair(mreg)
                } else {
                    one(mreg)
                };
                if pop {
                    (BTreeSet::new(), banks)
                } else {
                    (banks, BTreeSet::new())
                }
            }
            _ => continue,
        };
        assert_eq!((reads, writes), (expected_reads, expected_writes), "{name}");
        checked += 1;
    }
    assert_eq!(checked, 29 + 14);
}
