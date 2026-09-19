use std::{collections::HashMap, hint::black_box};

use w8_core::{
    W8_VERSION,
    isa::{
        instruction::Instruction,
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    loader::W8Loader,
    vm::{ExecuteVariant, WVM, memory::WVMMemory},
};

// ====== Constants shared by all benchmarks ======

/// The register number of the value-stack pointer (used by the recursive benchmarks).
pub const SP: u8 = 14;

/// The VM memory size for the benchmarks (4 MiB — enough for the sieve up to 1 000 000).
pub const MEMORY: usize = 1 << 22;

// ====== Benchmarks (a separate file for each) ======

pub mod ackermann;
pub mod binary_trees;
pub mod dense_arithmetic_10k;
pub mod fib_loop_100k;
pub mod fib_loop_10k;
pub mod fib_recursive;
pub mod mandelbrot;
pub mod nbody;
pub mod sieve;
pub mod spectral_norm;
pub mod tak;

// ====== Operands and instructions ======

/// A register operand.
pub fn reg(n: u8) -> Operand {
    Operand {
        kind: OperandKind::Register(Register(n)),
    }
}

/// An immediate operand.
pub fn imm(v: u64) -> Operand {
    Operand {
        kind: OperandKind::Immediate(v),
    }
}

/// An immediate from the bits of an `f64` (floating-point constants).
pub fn fimm(v: f64) -> Operand {
    imm(v.to_bits())
}

pub fn i0(op: OperationCode) -> Instruction {
    Instruction {
        opcode: op,
        operand1: None,
        operand2: None,
        operand3: None,
    }
}

pub fn i1(op: OperationCode, o1: Operand) -> Instruction {
    Instruction {
        opcode: op,
        operand1: Some(o1),
        operand2: None,
        operand3: None,
    }
}

pub fn i2(op: OperationCode, o1: Operand, o2: Operand) -> Instruction {
    Instruction {
        opcode: op,
        operand1: Some(o1),
        operand2: Some(o2),
        operand3: None,
    }
}

pub fn i3(op: OperationCode, o1: Operand, o2: Operand, o3: Operand) -> Instruction {
    Instruction {
        opcode: op,
        operand1: Some(o1),
        operand2: Some(o2),
        operand3: Some(o3),
    }
}

// ====== Assembling programs with labels ======

/// A program assembler with named labels for jumps.
pub struct Asm {
    code: Vec<Instruction>,
    labels: HashMap<String, usize>,
    fixups: Vec<(usize, String, u8)>,
}

impl Asm {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            labels: HashMap::new(),
            fixups: Vec::new(),
        }
    }

    /// Marks the current position with a name.
    pub fn label(&mut self, name: impl Into<String>) {
        self.labels.insert(name.into(), self.code.len());
    }

    /// Adds an instruction and returns its index.
    pub fn push(&mut self, instr: Instruction) -> usize {
        self.code.push(instr);
        self.code.len() - 1
    }

    pub fn jump(&mut self, label: &str) {
        let idx = self.push(i1(OperationCode::JMP, imm(0)));
        self.fixups.push((idx, label.to_string(), 1));
    }

    pub fn call(&mut self, label: &str) {
        let idx = self.push(i1(OperationCode::CALL, imm(0)));
        self.fixups.push((idx, label.to_string(), 1));
    }

    pub fn jz(&mut self, cond: Operand, label: &str) {
        let idx = self.push(i2(OperationCode::JZ, cond, imm(0)));
        self.fixups.push((idx, label.to_string(), 2));
    }

    pub fn jnz(&mut self, cond: Operand, label: &str) {
        let idx = self.push(i2(OperationCode::JNZ, cond, imm(0)));
        self.fixups.push((idx, label.to_string(), 2));
    }

    /// Resolves the jumps and returns the program.
    pub fn finish(mut self) -> Vec<Instruction> {
        for (idx, label, slot) in self.fixups {
            let target = *self
                .labels
                .get(&label)
                .unwrap_or_else(|| panic!("unknown label `{label}`"))
                as u64;
            match slot {
                1 => self.code[idx].operand1 = Some(imm(target)),
                2 => self.code[idx].operand2 = Some(imm(target)),
                _ => unreachable!(),
            }
        }
        self.code
    }
}

// ====== Encoding and running ======

/// Encodes the program into W8 Bytecode format bytes (`Vec<u8>`).
pub fn encode_to_nb(program: &[Instruction]) -> Vec<u8> {
    let mut bytes = Vec::new();

    // Magic + the minimum W8 version.
    bytes.extend_from_slice(b"NVMBC");
    for part in W8_VERSION.split('.').map(|p| p.parse::<u16>().unwrap()) {
        bytes.extend_from_slice(&part.to_le_bytes());
    }

    for instr in program {
        bytes.push(instr.opcode as u8);
        bytes.push(instr.operand_count() as u8);
        for operand in [instr.operand1, instr.operand2, instr.operand3]
            .into_iter()
            .flatten()
        {
            match operand.kind {
                OperandKind::Register(Register(r)) => {
                    bytes.push(0x00);
                    bytes.push(r);
                }
                OperandKind::Immediate(v) => {
                    bytes.push(0x01);
                    bytes.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
    }

    bytes
}

/// Loads bytes in the `.wb` format (transpilation into instructions) and executes the program.
/// Returns the VM with the result.
pub fn load_and_run_vm(bytes: &[u8], memory_size: usize) -> WVM {
    let instructions = W8Loader::new(bytes.to_vec())
        .transpile()
        .expect("failed to load benchmark bytecode");
    let mut vm = WVM::from_program_and_memory(
        instructions,
        WVMMemory::new(memory_size),
        ExecuteVariant::default(),
    );
    vm.interpretate().expect("benchmark program failed");
    vm
}

/// Loads bytes in the `.wb` format and executes the program (the measured part).
pub fn load_and_run(bytes: &[u8], memory_size: usize) {
    let vm = load_and_run_vm(bytes, memory_size);
    black_box(vm);
}
