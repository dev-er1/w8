// The Ackermann function ack(m, n) — deep recursion with `CALL` and `RET`.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

/// `ack(m in r0, n in r1) -> r0`; the value stack for nested calls.
fn program(m: u64, n: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.jump("main");
    asm.label("ack");
    asm.push(i3(Op::IEQ, reg(2), reg(0), imm(0)));
    asm.jnz(reg(2), "m_zero");
    asm.push(i3(Op::IEQ, reg(2), reg(1), imm(0)));
    asm.jnz(reg(2), "n_zero");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::STORE64, reg(SP), reg(1)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(1), reg(1), imm(1)));
    asm.call("ack");
    asm.push(i2(Op::MOVE, reg(2), reg(0)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(0), reg(SP)));
    asm.push(i3(Op::ISUB, reg(0), reg(0), imm(1)));
    asm.push(i2(Op::MOVE, reg(1), reg(2)));
    asm.call("ack");
    asm.push(i0(Op::RET));
    asm.label("m_zero");
    asm.push(i3(Op::IADD, reg(0), reg(1), imm(1)));
    asm.push(i0(Op::RET));
    asm.label("n_zero");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::MOVE, reg(1), imm(1)));
    asm.push(i3(Op::ISUB, reg(0), reg(0), imm(1)));
    asm.call("ack");
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(2), reg(SP)));
    asm.push(i0(Op::RET));
    asm.label("main");
    asm.push(i2(Op::MOVE, reg(SP), imm(0)));
    asm.push(i2(Op::MOVE, reg(0), imm(m)));
    asm.push(i2(Op::MOVE, reg(1), imm(n)));
    asm.call("ack");
    asm.finish()
}

pub fn ackermann(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(3, 8));
    c.bench_function("w8/ackermann", |b| b.iter(|| load_and_run(&bytes, MEMORY)));
}
