// Dense arithmetic: a mix of integer, bitwise and floating-point
// operations in one loop (no memory operations).
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

fn program(n: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.push(i2(Op::MOVE, reg(0), imm(0)));
    asm.push(i2(Op::MOVE, reg(2), imm(n)));
    asm.push(i2(Op::MOVE, reg(3), fimm(1.0)));
    asm.push(i2(Op::MOVE, reg(4), fimm(0.5)));
    asm.label("loop");
    asm.push(i3(Op::IADD, reg(0), reg(0), imm(1)));
    asm.push(i3(Op::IMUL, reg(1), reg(0), reg(0)));
    asm.push(i3(Op::ISUB, reg(1), reg(1), reg(0)));
    asm.push(i3(Op::AND, reg(1), reg(1), imm(0xFFFF)));
    asm.push(i3(Op::XOR, reg(1), reg(1), reg(0)));
    asm.push(i3(Op::LSL, reg(5), reg(1), imm(3)));
    asm.push(i3(Op::LSR, reg(5), reg(5), imm(2)));
    asm.push(i3(Op::SREM, reg(5), reg(5), imm(7)));
    asm.push(i3(Op::UREM, reg(5), reg(5), imm(7)));
    asm.push(i3(Op::FADD, reg(3), reg(3), reg(4)));
    asm.push(i3(Op::FMUL, reg(4), reg(4), fimm(0.9999)));
    asm.push(i3(Op::FSUB, reg(6), reg(3), reg(4)));
    asm.push(i3(Op::FDIV, reg(6), reg(6), fimm(1.25)));
    asm.push(i3(Op::IADD, reg(1), reg(1), reg(6)));
    asm.push(i3(Op::ISUB, reg(2), reg(2), imm(1)));
    asm.jnz(reg(2), "loop");
    asm.finish()
}

pub fn dense_arithmetic_10k(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(10_000));
    c.bench_function("w8/dense_arithmetic_10k", |b| {
        b.iter(|| load_and_run(&bytes, MEMORY))
    });
}
