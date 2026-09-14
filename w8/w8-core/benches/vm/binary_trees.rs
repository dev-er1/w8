// Binary trees: counting the number of nodes of a full tree of depth 15.
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

/// `make_tree(depth in r0) -> r0` — the node count of a full tree of that depth.
fn program(depth: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.jump("main");
    asm.label("make_tree");
    asm.push(i3(Op::IEQ, reg(2), reg(0), imm(0)));
    asm.jnz(reg(2), "leaf");
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(0), reg(0), imm(1)));
    asm.call("make_tree");
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i2(Op::STORE64, reg(SP), reg(0)));
    asm.push(i3(Op::IADD, reg(SP), reg(SP), imm(8)));
    asm.push(i3(Op::ISUB, reg(0), reg(1), imm(1)));
    asm.call("make_tree");
    asm.push(i3(Op::ISUB, reg(SP), reg(SP), imm(8)));
    asm.push(i2(Op::LOAD64, reg(1), reg(SP)));
    asm.push(i3(Op::IADD, reg(0), reg(0), reg(1)));
    asm.push(i3(Op::IADD, reg(0), reg(0), imm(1)));
    asm.push(i0(Op::RET));
    asm.label("leaf");
    asm.push(i2(Op::MOVE, reg(0), imm(1)));
    asm.push(i0(Op::RET));
    asm.label("main");
    asm.push(i2(Op::MOVE, reg(SP), imm(0)));
    asm.push(i2(Op::MOVE, reg(0), imm(depth)));
    asm.call("make_tree");
    asm.finish()
}

pub fn binary_trees(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(15));
    c.bench_function("w8/binary_trees", |b| {
        b.iter(|| load_and_run(&bytes, MEMORY))
    });
}
