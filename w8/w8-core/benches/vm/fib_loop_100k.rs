// Iterative Fibonacci: 100 000 iterations in a loop.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

/// The same loop as in `fib_loop_10k`, but with more iterations.
fn program(n: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.push(i2(Op::MOVE, reg(0), imm(0)));
    asm.push(i2(Op::MOVE, reg(1), imm(1)));
    asm.push(i2(Op::MOVE, reg(2), imm(n)));
    asm.push(i2(Op::MOVE, reg(3), imm(0)));
    asm.label("loop");
    asm.push(i3(Op::ISUB, reg(2), reg(2), imm(1)));
    asm.jz(reg(2), "done");
    asm.push(i2(Op::MOVE, reg(3), reg(1)));
    asm.push(i3(Op::IADD, reg(1), reg(1), reg(0)));
    asm.push(i2(Op::MOVE, reg(0), reg(3)));
    asm.jump("loop");
    asm.label("done");
    asm.finish()
}

pub fn fib_loop_100k(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(100_000));
    c.bench_function("w8/fib_loop_100k", |b| {
        b.iter(|| load_and_run(&bytes, MEMORY))
    });
}
