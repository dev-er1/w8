// N-body interaction (geometric inverse square via Newton's method):
// heavy floating-point arithmetic + the memory of the bodies.
//
// Body: px +0, py +8, pz +16, vx +24, vy +32, vz +40, mass +48 (a 64-byte row).
use criterion::Criterion;

use w8_core::isa::{instruction::Instruction, opcode::OperationCode as Op};

use super::*;

const BODY_STRIDE: u64 = 64;
const PX: u64 = 0;
const PY: u64 = 8;
const PZ: u64 = 16;
const VX: u64 = 24;
const VY: u64 = 32;
const VZ: u64 = 40;

/// The square root of `d2` into `dist` (input/output — registers r0..r7,
/// the caller must not use them on top).
fn emit_sqrt(asm: &mut Asm, d2: u8, dist: u8, t1: u8) {
    asm.push(i3(Op::FADD, reg(dist), reg(d2), fimm(1.0)));
    for _ in 0..9 {
        asm.push(i3(Op::FDIV, reg(t1), reg(d2), reg(dist)));
        asm.push(i3(Op::FADD, reg(t1), reg(dist), reg(t1)));
        asm.push(i3(Op::FMUL, reg(dist), reg(t1), fimm(0.5)));
    }
}

fn emit_pair(asm: &mut Asm, i: u64, j: u64) {
    let pi = i * BODY_STRIDE;
    let pj = j * BODY_STRIDE;
    for (off, reg_delta) in [(PX, 0u8), (PY, 1u8), (PZ, 2u8)] {
        asm.push(i2(Op::LOAD64, reg(6), imm(pj + off)));
        asm.push(i2(Op::LOAD64, reg(7), imm(pi + off)));
        asm.push(i3(Op::FSUB, reg(reg_delta), reg(6), reg(7)));
    }
    asm.push(i3(Op::FMUL, reg(4), reg(0), reg(0)));
    asm.push(i3(Op::FMUL, reg(3), reg(1), reg(1)));
    asm.push(i3(Op::FADD, reg(4), reg(4), reg(3)));
    asm.push(i3(Op::FMUL, reg(3), reg(2), reg(2)));
    asm.push(i3(Op::FADD, reg(4), reg(4), reg(3)));
    emit_sqrt(asm, 4, 5, 3);
    asm.push(i3(Op::FMUL, reg(3), reg(5), reg(5)));
    asm.push(i3(Op::FMUL, reg(3), reg(3), reg(5)));
    asm.push(i2(Op::MOVE, reg(6), fimm(1.0)));
    asm.push(i3(Op::FDIV, reg(5), reg(6), reg(3)));
    for (off, reg_delta) in [(VX, 0u8), (VY, 1u8), (VZ, 2u8)] {
        asm.push(i3(Op::FMUL, reg(6), reg(reg_delta), reg(5)));
        asm.push(i3(Op::FMUL, reg(6), reg(6), fimm(1.0)));
        asm.push(i2(Op::LOAD64, reg(7), imm(pj + off)));
        asm.push(i3(Op::FSUB, reg(7), reg(7), reg(6)));
        asm.push(i2(Op::STORE64, imm(pj + off), reg(7)));
        asm.push(i3(Op::FMUL, reg(6), reg(reg_delta), reg(5)));
        asm.push(i2(Op::LOAD64, reg(7), imm(pi + off)));
        asm.push(i3(Op::FADD, reg(7), reg(7), reg(6)));
        asm.push(i2(Op::STORE64, imm(pi + off), reg(7)));
    }
}

fn emit_advance(asm: &mut Asm, bodies: u64) {
    for b in 0..bodies {
        let p = b * BODY_STRIDE;
        for (off_p, off_v) in [(PX, VX), (PY, VY), (PZ, VZ)] {
            asm.push(i2(Op::LOAD64, reg(6), imm(p + off_p)));
            asm.push(i2(Op::LOAD64, reg(7), imm(p + off_v)));
            asm.push(i3(Op::FADD, reg(6), reg(6), reg(7)));
            asm.push(i2(Op::STORE64, imm(p + off_p), reg(6)));
        }
    }
}

fn program(bodies: u64, steps: u64) -> Vec<Instruction> {
    let mut asm = Asm::new();
    asm.push(i2(Op::MOVE, reg(8), imm(steps)));
    asm.label("step_loop");
    for i in 0..bodies {
        for j in (i + 1)..bodies {
            emit_pair(&mut asm, i, j);
        }
    }
    emit_advance(&mut asm, bodies);
    asm.push(i3(Op::ISUB, reg(8), reg(8), imm(1)));
    asm.jnz(reg(8), "step_loop");
    asm.finish()
}

pub fn nbody(c: &mut Criterion) {
    let bytes = encode_to_nb(&program(4, 2_500));
    c.bench_function("w8/nbody", |b| b.iter(|| load_and_run(&bytes, MEMORY)));
}
