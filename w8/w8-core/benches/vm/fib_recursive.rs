// Recursive Fibonacci (fib(22)) with a value stack in memory and `CALL` and `RET`.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

/// `fib(n)` in `r0`; the base case is `n < 2`; temporarily saves `n` on the stack.
fn program(n: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.jump("main");
    asm.label("fib");
    asm.push(i3(Op::SLT, reg(1), reg(0), imm(2)));
    asm.jnz(reg(1), "base_fib");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(1), reg(0), imm(1)));
    asm.push(i2(Op::MOVE, reg(0), reg(1)));
    asm.call("fib");
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(1), reg(1), imm(1)));
    asm.push(i3(Op::ISUB, reg(1), reg(1), imm(1)));
    asm.push(i2(Op::MOVE, reg(0), reg(1)));
    asm.call("fib");
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i3(Op::IADD, reg(0), reg(0), reg(1)));
    asm.push(i0(Op::RET));
    asm.label("base_fib");
    asm.push(i0(Op::RET));
    asm.label("main");
    asm.push(i2(Op::MOVE, reg(SP), imm(0)));
    asm.push(i2(Op::MOVE, reg(0), imm(n)));
    asm.call("fib");
    asm.finish()
}

pub fn fib_recursive(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(22));
    c.bench_function("w8/fib_recursive", |b| {
        b.iter(|| load_and_run(&bytes, MEMORY))
    });
}
