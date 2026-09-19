// Takeuchi's function tak(x, y, z) — triple recursion with a value stack.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

/// `tak(x in r0, y in r1, z in r2) -> r0`; uses r5..r7 and the stack
/// to save the arguments between nested calls.
fn program(x: u64, y: u64, z: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.jump("main");
    asm.label("tak");
    asm.push(i3(Op::SGT, reg(3), reg(0), reg(1)));
    asm.jz(reg(3), "base_tak");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::STORE64, reg(SP), reg(1)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::STORE64, reg(SP), reg(2)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(0), reg(0), imm(1)));
    asm.call("tak");
    asm.push(i2(Op::MOVE, reg(4), reg(0)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(7), reg(SP)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(6), reg(SP)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(5), reg(SP)));
    asm.push(i3(Op::ISUB, reg(0), reg(6), imm(1)));
    asm.push(i2(Op::MOVE, reg(1), reg(7)));
    asm.push(i2(Op::MOVE, reg(2), reg(5)));
    asm.call("tak");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(0), reg(7), imm(1)));
    asm.push(i2(Op::MOVE, reg(1), reg(5)));
    asm.push(i2(Op::MOVE, reg(2), reg(6)));
    asm.call("tak");
    asm.push(i2(Op::MOVE, reg(2), reg(0)));
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i2(Op::MOVE, reg(0), reg(4)));
    asm.call("tak");
    asm.push(i0(Op::RET));
    asm.label("base_tak");
    asm.push(i2(Op::MOVE, reg(0), reg(2)));
    asm.push(i0(Op::RET));
    asm.label("main");
    asm.push(i2(Op::MOVE, reg(SP), imm(0)));
    asm.push(i2(Op::MOVE, reg(0), imm(x)));
    asm.push(i2(Op::MOVE, reg(1), imm(y)));
    asm.push(i2(Op::MOVE, reg(2), imm(z)));
    asm.call("tak");
    asm.finish()
}

pub fn tak(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(20, 15, 10));
    c.bench_function("w8/tak", |b| b.iter(|| load_and_run(&bytes, MEMORY)));
}
