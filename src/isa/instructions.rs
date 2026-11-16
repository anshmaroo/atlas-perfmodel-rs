// Atlas ISA table
//

use crate::isa::encoding::define_isa;
use crate::isa::operands::{
    Acc, BImm, Channel, EReg, Imm12, Imm16, Imm20, JImm, MReg, MRegPair, UImm5, WBuf, XReg,
};

define_isa! {
    // ---- LOAD (0x03): scalar loads read VMEM through the LSU ----------------------------------
    Lb    = "lb",   Lsu,    I { opcode: 0b0000011, funct3: 0b000 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Lh    = "lh",   Lsu,    I { opcode: 0b0000011, funct3: 0b001 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Lw    = "lw",   Lsu,    I { opcode: 0b0000011, funct3: 0b010 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Lbu   = "lbu",  Lsu,    I { opcode: 0b0000011, funct3: 0b100 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Lhu   = "lhu",  Lsu,    I { opcode: 0b0000011, funct3: 0b101 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Seld  = "seld", Lsu,    I { opcode: 0b0000011, funct3: 0b110 } (rd: EReg [W], imm: Imm12, rs1: XReg [R]);
    Seli  = "seli", Scalar, I { opcode: 0b0000011, funct3: 0b111 } (rd: EReg [W], imm: Imm12);

    // ---- Tensor load/store (0x07): immediate counts 32-word VMEM lines --------------------
    Vload  = "vload",  Lsu, VLS { opcode: 0b0000111, funct2: 0b00 } (vd: MReg [W], imm: Imm12, rs1: XReg [R]);
    Vstore = "vstore", Lsu, VLS { opcode: 0b0000111, funct2: 0b01 } (vd: MReg [R], imm: Imm12, rs1: XReg [R]);

    // ---- MISC-MEM (0x0F) ----------------------------------------------------------------------
    Fence = "fence", Scalar, I { opcode: 0b0001111, funct3: 0b000, imm: 0 } ();

    // ---- OP-IMM (0x13) ------------------------------------------------------------------------
    Addi  = "addi",  Scalar, I { opcode: 0b0010011, funct3: 0b000 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Slli  = "slli",  Scalar, I { opcode: 0b0010011, funct3: 0b001, imm_hi: 0b0000000 } (rd: XReg [W], rs1: XReg [R], imm: UImm5 => shamt);
    Slti  = "slti",  Scalar, I { opcode: 0b0010011, funct3: 0b010 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Sltiu = "sltiu", Scalar, I { opcode: 0b0010011, funct3: 0b011 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Xori  = "xori",  Scalar, I { opcode: 0b0010011, funct3: 0b100 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Srli  = "srli",  Scalar, I { opcode: 0b0010011, funct3: 0b101, imm_hi: 0b0000000 } (rd: XReg [W], rs1: XReg [R], imm: UImm5 => shamt);
    Srai  = "srai",  Scalar, I { opcode: 0b0010011, funct3: 0b101, imm_hi: 0b0100000 } (rd: XReg [W], rs1: XReg [R], imm: UImm5 => shamt);
    Ori   = "ori",   Scalar, I { opcode: 0b0010011, funct3: 0b110 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Andi  = "andi",  Scalar, I { opcode: 0b0010011, funct3: 0b111 }                    (rd: XReg [W], rs1: XReg [R], imm: Imm12);

    // ---- AUIPC (0x17) -------------------------------------------------------------------------
    Auipc = "auipc", Scalar, U { opcode: 0b0010111 } (rd: XReg [W], imm: Imm20);

    // ---- STORE (0x23) -------------------------------------------------------------------------
    Sb = "sb", Lsu, S { opcode: 0b0100011, funct3: 0b000 } (rs2: XReg [R], imm: Imm12, rs1: XReg [R]);
    Sh = "sh", Lsu, S { opcode: 0b0100011, funct3: 0b001 } (rs2: XReg [R], imm: Imm12, rs1: XReg [R]);
    Sw = "sw", Lsu, S { opcode: 0b0100011, funct3: 0b010 } (rs2: XReg [R], imm: Imm12, rs1: XReg [R]);

    // ---- OP (0x33) ----------------------------------------------------------------------------
    Add  = "add",  Scalar, R { opcode: 0b0110011, funct3: 0b000, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Sub  = "sub",  Scalar, R { opcode: 0b0110011, funct3: 0b000, funct7: 0b0100000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Sll  = "sll",  Scalar, R { opcode: 0b0110011, funct3: 0b001, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Slt  = "slt",  Scalar, R { opcode: 0b0110011, funct3: 0b010, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Sltu = "sltu", Scalar, R { opcode: 0b0110011, funct3: 0b011, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Xor  = "xor",  Scalar, R { opcode: 0b0110011, funct3: 0b100, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Srl  = "srl",  Scalar, R { opcode: 0b0110011, funct3: 0b101, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Sra  = "sra",  Scalar, R { opcode: 0b0110011, funct3: 0b101, funct7: 0b0100000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    Or   = "or",   Scalar, R { opcode: 0b0110011, funct3: 0b110, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);
    And  = "and",  Scalar, R { opcode: 0b0110011, funct3: 0b111, funct7: 0b0000000 } (rd: XReg [W], rs1: XReg [R], rs2: XReg [R]);

    // ---- LUI (0x37) ---------------------------------------------------------------------------
    Lui = "lui", Scalar, U { opcode: 0b0110111 } (rd: XReg [W], imm: Imm20);

    // ---- Vector (0x57) ----------------------
    VaddBf16       = "vadd.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b0000000 } (vd: MRegPair [W], vs1: MRegPair [R], vs2: MRegPair [R]);
    VredsumBf16    = "vredsum.bf16",     Vector, VR { opcode: 0b1010111, funct7: 0b0000001 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VsubBf16       = "vsub.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b0000010 } (vd: MRegPair [W], vs1: MRegPair [R], vs2: MRegPair [R]);
    VmulBf16       = "vmul.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b0000011 } (vd: MRegPair [W], vs1: MRegPair [R], vs2: MRegPair [R]);
    VminimumBf16   = "vminimum.bf16",    Vector, VR { opcode: 0b1010111, funct7: 0b0000100 } (vd: MRegPair [W], vs1: MRegPair [R], vs2: MRegPair [R]);
    VredminBf16    = "vredmin.bf16",     Vector, VR { opcode: 0b1010111, funct7: 0b0000101 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VmaximumBf16   = "vmaximum.bf16",    Vector, VR { opcode: 0b1010111, funct7: 0b0000110 } (vd: MRegPair [W], vs1: MRegPair [R], vs2: MRegPair [R]);
    VredmaxBf16    = "vredmax.bf16",     Vector, VR { opcode: 0b1010111, funct7: 0b0000111 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VredsumRowBf16 = "vredsum.row.bf16", Vector, VR { opcode: 0b1010111, funct7: 0b0100001 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VredminRowBf16 = "vredmin.row.bf16", Vector, VR { opcode: 0b1010111, funct7: 0b0100100 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VredmaxRowBf16 = "vredmax.row.bf16", Vector, VR { opcode: 0b1010111, funct7: 0b0100110 } (vd: MRegPair [W], vs1: MRegPair [R]);
    Vmov           = "vmov",             Vector, VR { opcode: 0b1010111, funct7: 0b1000000 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VrecipBf16     = "vrecip.bf16",      Vector, VR { opcode: 0b1010111, funct7: 0b1000001 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VexpBf16       = "vexp.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b1000010 } (vd: MRegPair [W], vs1: MRegPair [R]);
    Vexp2Bf16      = "vexp2.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1000011 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VpackBf16Fp8   = "vpack.bf16.fp8",   Vector, VR { opcode: 0b1010111, funct7: 0b1000100 } (vd: MReg [W], vs2: MRegPair [R], es1: EReg [R] => vs1);
    VunpackFp8Bf16 = "vunpack.fp8.bf16", Vector, VR { opcode: 0b1010111, funct7: 0b1000101 } (vd: MRegPair [W], vs2: MReg [R], es1: EReg [R] => vs1);
    VsquareBf16    = "vsquare.bf16",     Vector, VR { opcode: 0b1010111, funct7: 0b1000110 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VcubeBf16      = "vcube.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1000111 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VreluBf16      = "vrelu.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1001000 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VsinBf16       = "vsin.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b1001001 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VcosBf16       = "vcos.bf16",        Vector, VR { opcode: 0b1010111, funct7: 0b1001010 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VtanhBf16      = "vtanh.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1001011 } (vd: MRegPair [W], vs1: MRegPair [R]);
    Vlog2Bf16      = "vlog2.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1001100 } (vd: MRegPair [W], vs1: MRegPair [R]);
    VsqrtBf16      = "vsqrt.bf16",       Vector, VR { opcode: 0b1010111, funct7: 0b1001101 } (vd: MRegPair [W], vs1: MRegPair [R]);

    // ---- Vector load immediate (0x5F): immediate is raw BF16 bits -------------------------
    VliAll = "vli.all", Vector, VI { opcode: 0b1011111, funct3: 0b000 } (vd: MRegPair [W], imm: Imm16);
    VliRow = "vli.row", Vector, VI { opcode: 0b1011111, funct3: 0b001 } (vd: MRegPair [W], imm: Imm16);
    VliCol = "vli.col", Vector, VI { opcode: 0b1011111, funct3: 0b010 } (vd: MReg [W], imm: Imm16);
    VliOne = "vli.one", Vector, VI { opcode: 0b1011111, funct3: 0b011 } (vd: MReg [W], imm: Imm16);

    // ---- BRANCH (0x63): one delay slot --------------------------------------------------------
    Beq  = "beq",  Scalar, SB { opcode: 0b1100011, funct3: 0b000 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);
    Bne  = "bne",  Scalar, SB { opcode: 0b1100011, funct3: 0b001 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);
    Blt  = "blt",  Scalar, SB { opcode: 0b1100011, funct3: 0b100 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);
    Bge  = "bge",  Scalar, SB { opcode: 0b1100011, funct3: 0b101 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);
    Bltu = "bltu", Scalar, SB { opcode: 0b1100011, funct3: 0b110 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);
    Bgeu = "bgeu", Scalar, SB { opcode: 0b1100011, funct3: 0b111 } (rs1: XReg [R], rs2: XReg [R], imm: BImm);

    // ---- JALR (0x67) --------------------------------------------------------------------------
    Jalr  = "jalr",              Scalar, I { opcode: 0b1100111, funct3: 0b000 } (rd: XReg [W], rs1: XReg [R], imm: Imm12);
    Delay = "delay" !functional, Scalar, I { opcode: 0b1100111, funct3: 0b001 } (imm: Imm12);

    // ---- XLU (0x6B) --------------------------------------------------------------------
    VtrposeXlu = "vtrpose.xlu", Xlu, VR { opcode: 0b1101011, funct7: 0b0000000 } (vd: MReg [W], vs1: MReg [R]);

    // ---- JAL (0x6F) ---------------------------------------------------------------------------
    Jal = "jal", Scalar, UJ { opcode: 0b1101111 } (rd: XReg [W], imm: JImm);

    // ---- SYSTEM (0x73) ------------------------------------------------------------------------
    Csrrw  = "csrrw",  Scalar, CSR { opcode: 0b1110011, funct3: 0b001 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Csrrs  = "csrrs",  Scalar, CSR { opcode: 0b1110011, funct3: 0b010 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Csrrc  = "csrrc",  Scalar, CSR { opcode: 0b1110011, funct3: 0b011 } (rd: XReg [W], imm: Imm12, rs1: XReg [R]);
    Csrrwi = "csrrwi", Scalar, CSR { opcode: 0b1110011, funct3: 0b101 } (rd: XReg [W], imm: Imm12, rs1: UImm5);
    Csrrsi = "csrrsi", Scalar, CSR { opcode: 0b1110011, funct3: 0b110 } (rd: XReg [W], imm: Imm12, rs1: UImm5);
    Csrrci = "csrrci", Scalar, CSR { opcode: 0b1110011, funct3: 0b111 } (rd: XReg [W], imm: Imm12, rs1: UImm5);
    Ecall  = "ecall"  !functional, Scalar, I { opcode: 0b1110011, funct3: 0b000, imm: 0 } ();
    Ebreak = "ebreak" !functional, Scalar, I { opcode: 0b1110011, funct3: 0b000, imm: 1 } ();

    // ---- MXU (0x77): mxu0 = systolic array, mxu1 = inner-product tree ------------------------
    VmatpushWeightMxu0  = "vmatpush.weight.mxu0",  MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0000000 } (vd: WBuf [W], vs1: MReg [R]);
    VmatpushWeightMxu1  = "vmatpush.weight.mxu1",  MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0000001 } (vd: WBuf [W], vs1: MReg [R]);
    VmatpushAccFp8Mxu0  = "vmatpush.acc.fp8.mxu0",  MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0000010 } (vd: Acc [W], vs1: MReg [R]);
    VmatpushAccFp8Mxu1  = "vmatpush.acc.fp8.mxu1",  MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0000011 } (vd: Acc [W], vs1: MReg [R]);
    VmatpushAccBf16Mxu0 = "vmatpush.acc.bf16.mxu0", MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0000100 } (vd: Acc [W], vs1: MRegPair [R]);
    VmatpushAccBf16Mxu1 = "vmatpush.acc.bf16.mxu1", MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0000101 } (vd: Acc [W], vs1: MRegPair [R]);
    VmatpopFp8AccMxu0   = "vmatpop.fp8.acc.mxu0",   MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0000110 } (vd: MReg [W], vs2: Acc [R], es1: EReg [R] => vs1);
    VmatpopFp8AccMxu1   = "vmatpop.fp8.acc.mxu1",   MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0000111 } (vd: MReg [W], vs2: Acc [R], es1: EReg [R] => vs1);
    VmatpopBf16AccMxu0  = "vmatpop.bf16.acc.mxu0",  MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0001000 } (vd: MRegPair [W], vs2: Acc [R]);
    VmatpopBf16AccMxu1  = "vmatpop.bf16.acc.mxu1",  MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0001001 } (vd: MRegPair [W], vs2: Acc [R]);
    VmatmulMxu0         = "vmatmul.mxu0",           MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0001010 } (vd: Acc [W], vs1: MReg [R], vs2: WBuf [R]);
    VmatmulMxu1         = "vmatmul.mxu1",           MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0001011 } (vd: Acc [W], vs1: MReg [R], vs2: WBuf [R]);
    VmatmulAccMxu0      = "vmatmul.acc.mxu0",       MatrixSystolic, VR { opcode: 0b1110111, funct7: 0b0001100 } (vd: Acc [RW], vs1: MReg [R], vs2: WBuf [R]);
    VmatmulAccMxu1      = "vmatmul.acc.mxu1",       MatrixInner,    VR { opcode: 0b1110111, funct7: 0b0001101 } (vd: Acc [RW], vs1: MReg [R], vs2: WBuf [R]);

    // ---- DMA (0x7B, 0x7F): the channel is carried in "funct3", but is really an immediate -----------------------------------
    DmaLoad   = "dma.load",              Dma, R { opcode: 0b1111011, funct7: 0b0000000 } (rd: XReg [R], rs1: XReg [R], rs2: XReg [R], ch: Channel => funct3);
    DmaStore  = "dma.store",             Dma, R { opcode: 0b1111011, funct7: 0b0000001 } (rd: XReg [R], rs1: XReg [R], rs2: XReg [R], ch: Channel => funct3);
    DmaConfig = "dma.config",            Dma, R { opcode: 0b1111111, funct7: 0b0000000 } (rs1: XReg [R], ch: Channel => funct3);
    DmaWait   = "dma.wait" !functional,  Dma, R { opcode: 0b1111111, funct7: 0b0000001 } (ch: Channel => funct3);
}
