// The Sieve of Eratosthenes up to 1 000 000: byte-memory work,
// nested loops and prime counting.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

fn program(n: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    // composite[0] = composite[1] = 1
    asm.push(i2(Op::STORE8, imm(0), imm(1)));
    asm.push(i2(Op::STORE8, imm(1), imm(1)));
    asm.push(i2(Op::MOVE, reg(1), imm(2)));
    asm.push(i2(Op::MOVE, reg(6), imm(0)));
    asm.label("outer");
    asm.push(i3(Op::IMUL, reg(2), reg(1), reg(1)));
    asm.push(i3(Op::ULT, reg(3), reg(2), imm(n)));
    asm.jz(reg(3), "done");
    asm.push(i2(Op::LOAD8, reg(4), reg(1)));
    asm.jnz(reg(4), "skip_i");
    asm.push(i3(Op::IADD, reg(6), reg(6), imm(1)));
    asm.push(i2(Op::MOVE, reg(5), reg(2)));
    asm.label("inner");
    asm.push(i3(Op::ULT, reg(3), reg(5), imm(n)));
    asm.jz(reg(3), "j_done");
    asm.push(i2(Op::STORE8, reg(5), imm(1)));
    asm.push(i3(Op::IADD, reg(5), reg(5), reg(1)));
    asm.jump("inner");
    asm.label("j_done");
    asm.label("skip_i");
    asm.push(i3(Op::IADD, reg(1), reg(1), imm(1)));
    asm.jump("outer");
    asm.label("done");
    asm.finish()
}

pub fn sieve(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(1_000_000));
    c.bench_function("w8/sieve", |b| b.iter(|| load_and_run(&bytes, MEMORY)));
}
