use tch::{Device, Kind, Tensor};
// define all architectural state here
// scalar registers, scaling registers, tensor registers, buffers (accmulation, weight), DMA channels


/* TENSOR REGISTER/FILE */
#[derive(Debug)]
pub struct TensorRegister<const WIDTH: usize, const HEIGHT: usize> {
    data: Tensor,
}

impl<const WIDTH: usize, const HEIGHT: usize> Clone for TensorRegister<WIDTH, HEIGHT> {
    fn clone(&self) -> Self {
        Self { data: self.data.copy() }
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> TensorRegister<WIDTH, HEIGHT> {
    pub fn new() -> Self {
        let data = Tensor::zeros([HEIGHT as i64, WIDTH as i64], (Kind::Float8e4m3fn, Device::Cpu)); // default to CPU, possibly use GPU
        Self { data }
    }

    pub fn read(&self) -> Tensor {
        self.data.shallow_clone()
    }

    pub fn write(&mut self, data: Tensor) {
        self.data = data
    }
}


pub struct TensorRegfile<const REGISTER_WIDTH: usize, const REGISTER_HEIGHT: usize> {
    registers: [TensorRegister<REGISTER_WIDTH, REGISTER_HEIGHT>; 64], // fixed size 64 entries, this is an ISA rule 
}

impl <const REGISTER_WIDTH: usize, const REGISTER_HEIGHT: usize> TensorRegfile<REGISTER_WIDTH, REGISTER_HEIGHT> {
    pub fn new() -> Self {
        let registers = std::array::from_fn(|_| TensorRegister::new());
        Self { registers }
    }

    // read a bf16 tile (register pair)
    pub fn read_tile(&self, lower_reg: usize) -> Tensor {
        assert!(lower_reg % 2 == 0 && lower_reg < 64);

        let a = self.registers[lower_reg].read();
        let b = self.registers[lower_reg + 1].read();
        let paired = Tensor::stack(&[a, b], -1);
        paired.view_dtype(Kind::BFloat16)
    }

    // write a bf16 tile (register pair)
    pub fn write_tile(&mut self, lower_reg: usize, tile: Tensor) {
        assert!(lower_reg % 2 == 0 && lower_reg < 64);
        let bytes = tile.view_dtype(Kind::Float8e4m3fn);

        let a = bytes.select(-1, 0);
        let b = bytes.select(-1, 1);

        self.registers[lower_reg].write(a);
        self.registers[lower_reg + 1].write(b);
    }
}

/* SCALAR/SCALING REGISTER/FILE */
type ScalarRegister = u32;

struct ScalarRegfile {
    registers: [ScalarRegister; 32]
}
impl ScalarRegfile {
    pub fn new() -> Self {
        let registers = [0 as u32; 32];
        Self {registers}
    }

    pub fn read(&self, index: usize) -> u32 {
        assert! (index < 32);
        self.registers[index]
    }

    pub fn write(&mut self, index: usize, data: u32) {
        assert! (index < 32);
        self.registers[index] = data;
    }

}

struct ScaleRegfile {
    registers: [ScalarRegister; 32]
}

impl ScaleRegfile {
    pub fn new() -> Self {
        let registers = [0 as u32; 32];
        Self {registers}
    }

    pub fn read(&self, index: usize) -> u32 {
        assert! (index < 32);
        self.registers[index]
    }

    pub fn write(&mut self, index: usize, data: u32) {
        assert! (index < 32);
        self.registers[index] = data;
    }

}


struct ArchState<const MACHINE_WIDTH: usize, const VMEM_SIZE: usize> {
    scalar_regfile: ScalarRegfile,
    scaling_regfile: ScaleRegfile,
    tensor_regfile: TensorRegfile<MACHINE_WIDTH, MACHINE_WIDTH>,
    vmem: [u8; VMEM_SIZE],
}
