// Tests for all arithmetic: floating-point and integer.
use w8_core::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode::*,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{ExecuteVariant, WVM},
};

fn reg(r: u8) -> Operand {
    Operand {
        kind: OperandKind::Register(Register(r)),
    }
}

fn imm_i(value: u64) -> Operand {
    Operand {
        kind: OperandKind::Immediate(value),
    }
}

fn imm_f(value: f64) -> Operand {
    Operand {
        kind: OperandKind::Immediate(value.to_bits()),
    }
}

fn execute(program: Vec<Instruction>) -> WVM {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.program = program;
    vm.interpretate().expect("execution failed");
    vm
}

#[test]
fn mixed_integer_and_float_sequence() {
    let vm = execute(vec![
        Instruction {
            opcode: IADD,
            operand1: Some(reg(0)),
            operand2: Some(imm_i(12)),
            operand3: Some(imm_i(5)),
        },
        Instruction {
            opcode: FADD,
            operand1: Some(reg(1)),
            operand2: Some(imm_f(3.5)),
            operand3: Some(imm_f(2.5)),
        },
        Instruction {
            opcode: IMUL,
            operand1: Some(reg(2)),
            operand2: Some(reg(0)),
            operand3: Some(imm_i(2)),
        },
        Instruction {
            opcode: FDIV,
            operand1: Some(reg(3)),
            operand2: Some(reg(1)),
            operand3: Some(imm_f(2.0)),
        },
        Instruction {
            opcode: UDIV,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(imm_i(5)),
        },
        Instruction {
            opcode: UREM,
            operand1: Some(reg(5)),
            operand2: Some(reg(2)),
            operand3: Some(imm_i(5)),
        },
        Instruction {
            opcode: FSUB,
            operand1: Some(reg(6)),
            operand2: Some(reg(3)),
            operand3: Some(imm_f(1.5)),
        },
        Instruction {
            opcode: FREM,
            operand1: Some(reg(7)),
            operand2: Some(reg(6)),
            operand3: Some(imm_f(1.0)),
        },
    ]);

    assert_eq!(vm.registers[Register(0)], 17);
    assert_eq!(f64::from_bits(vm.registers[Register(1)]), 6.0);
    assert_eq!(vm.registers[Register(2)], 34);
    assert_eq!(f64::from_bits(vm.registers[Register(3)]), 3.0);
    assert_eq!(vm.registers[Register(4)], 6);
    assert_eq!(vm.registers[Register(5)], 4);
    assert_eq!(f64::from_bits(vm.registers[Register(6)]), 1.5);
    assert_eq!(f64::from_bits(vm.registers[Register(7)]), 0.5);
}

#[test]
fn signed_and_wrapping_integer_arithmetic_with_float_infinity() {
    let vm = execute(vec![
        Instruction {
            opcode: IADD,
            operand1: Some(reg(0)),
            operand2: Some(imm_i(u64::MAX)),
            operand3: Some(imm_i(1)),
        },
        Instruction {
            opcode: ISUB,
            operand1: Some(reg(1)),
            operand2: Some(imm_i(0)),
            operand3: Some(imm_i(1)),
        },
        Instruction {
            opcode: SDIV,
            operand1: Some(reg(2)),
            operand2: Some(imm_i((-9i64) as u64)),
            operand3: Some(imm_i(2)),
        },
        Instruction {
            opcode: SREM,
            operand1: Some(reg(3)),
            operand2: Some(imm_i((-9i64) as u64)),
            operand3: Some(imm_i(4)),
        },
        Instruction {
            opcode: FADD,
            operand1: Some(reg(4)),
            operand2: Some(imm_f(f64::INFINITY)),
            operand3: Some(imm_f(1.0)),
        },
        Instruction {
            opcode: FMUL,
            operand1: Some(reg(5)),
            operand2: Some(reg(4)),
            operand3: Some(imm_f(0.0)),
        },
        Instruction {
            opcode: FSUB,
            operand1: Some(reg(6)),
            operand2: Some(reg(4)),
            operand3: Some(imm_f(f64::INFINITY)),
        },
    ]);

    assert_eq!(vm.registers[Register(0)], 0);
    assert_eq!(vm.registers[Register(1)], u64::MAX);
    assert_eq!(vm.registers[Register(2)], (-4i64) as u64);
    assert_eq!(vm.registers[Register(3)], (-1i64) as u64);
    assert_eq!(f64::from_bits(vm.registers[Register(4)]), f64::INFINITY);
    assert!(f64::from_bits(vm.registers[Register(5)]).is_nan());
    assert!(f64::from_bits(vm.registers[Register(6)]).is_nan());
}

#[test]
fn float_nan_and_infinite_values_with_integer_operations() {
    let vm = execute(vec![
        Instruction {
            opcode: FDIV,
            operand1: Some(reg(0)),
            operand2: Some(imm_f(0.0)),
            operand3: Some(imm_f(0.0)),
        },
        Instruction {
            opcode: FREM,
            operand1: Some(reg(1)),
            operand2: Some(imm_f(4.5)),
            operand3: Some(imm_f(0.0)),
        },
        Instruction {
            opcode: FADD,
            operand1: Some(reg(2)),
            operand2: Some(reg(0)),
            operand3: Some(imm_f(1.0)),
        },
        Instruction {
            opcode: IADD,
            operand1: Some(reg(3)),
            operand2: Some(imm_i(5)),
            operand3: Some(imm_i(5)),
        },
        Instruction {
            opcode: IMUL,
            operand1: Some(reg(4)),
            operand2: Some(reg(3)),
            operand3: Some(imm_i(2)),
        },
    ]);

    assert!(f64::from_bits(vm.registers[Register(0)]).is_nan());
    assert!(f64::from_bits(vm.registers[Register(1)]).is_nan());
    assert!(f64::from_bits(vm.registers[Register(2)]).is_nan());
    assert_eq!(vm.registers[Register(3)], 10);
    assert_eq!(vm.registers[Register(4)], 20);
}

#[test]
fn full_integer_and_float_pipeline() {
    let vm = execute(vec![
        Instruction {
            opcode: IADD,
            operand1: Some(reg(0)),
            operand2: Some(imm_i(8)),
            operand3: Some(imm_i(7)),
        },
        Instruction {
            opcode: ISUB,
            operand1: Some(reg(1)),
            operand2: Some(imm_i(15)),
            operand3: Some(imm_i(20)),
        },
        Instruction {
            opcode: IMUL,
            operand1: Some(reg(2)),
            operand2: Some(imm_i(6)),
            operand3: Some(imm_i(3)),
        },
        Instruction {
            opcode: SDIV,
            operand1: Some(reg(3)),
            operand2: Some(imm_i((-3i64) as u64)),
            operand3: Some(imm_i(2)),
        },
        Instruction {
            opcode: UDIV,
            operand1: Some(reg(4)),
            operand2: Some(imm_i(18)),
            operand3: Some(imm_i(4)),
        },
        Instruction {
            opcode: UREM,
            operand1: Some(reg(5)),
            operand2: Some(imm_i(18)),
            operand3: Some(imm_i(4)),
        },
        Instruction {
            opcode: SREM,
            operand1: Some(reg(6)),
            operand2: Some(imm_i((-7i64) as u64)),
            operand3: Some(imm_i(4)),
        },
        Instruction {
            opcode: FADD,
            operand1: Some(reg(7)),
            operand2: Some(imm_f(5.5)),
            operand3: Some(imm_f(1.5)),
        },
        Instruction {
            opcode: FMUL,
            operand1: Some(reg(8)),
            operand2: Some(reg(7)),
            operand3: Some(imm_f(2.0)),
        },
        Instruction {
            opcode: FDIV,
            operand1: Some(reg(9)),
            operand2: Some(reg(8)),
            operand3: Some(imm_f(4.0)),
        },
        Instruction {
            opcode: FSUB,
            operand1: Some(reg(10)),
            operand2: Some(reg(9)),
            operand3: Some(imm_f(1.0)),
        },
        Instruction {
            opcode: FREM,
            operand1: Some(reg(11)),
            operand2: Some(reg(10)),
            operand3: Some(imm_f(1.2)),
        },
    ]);

    assert_eq!(vm.registers[Register(0)], 15);
    assert_eq!(vm.registers[Register(1)], u64::MAX - 4); // 15-20 = -5 as unsigned wrap
    assert_eq!(vm.registers[Register(2)], 18);
    assert_eq!(vm.registers[Register(3)], (-1i64) as u64);
    assert_eq!(vm.registers[Register(4)], 4);
    assert_eq!(vm.registers[Register(5)], 2);
    assert_eq!(vm.registers[Register(6)], (-3i64) as u64);
    assert_eq!(f64::from_bits(vm.registers[Register(7)]), 7.0);
    assert_eq!(f64::from_bits(vm.registers[Register(8)]), 14.0);
    assert_eq!(f64::from_bits(vm.registers[Register(9)]), 3.5);
    assert_eq!(f64::from_bits(vm.registers[Register(10)]), 2.5);
    assert!((f64::from_bits(vm.registers[Register(11)]) - 0.1).abs() < 1e-12);
}

#[test]
fn mixed_pipeline_with_negative_and_zero() {
    let vm = execute(vec![
        Instruction {
            opcode: IADD,
            operand1: Some(reg(0)),
            operand2: Some(imm_i(100)),
            operand3: Some(imm_i(200)),
        },
        Instruction {
            opcode: FSUB,
            operand1: Some(reg(1)),
            operand2: Some(imm_f(10.5)),
            operand3: Some(imm_f(4.25)),
        },
        Instruction {
            opcode: IMUL,
            operand1: Some(reg(2)),
            operand2: Some(reg(0)),
            operand3: Some(imm_i(3)),
        },
        Instruction {
            opcode: FMUL,
            operand1: Some(reg(3)),
            operand2: Some(reg(1)),
            operand3: Some(imm_f(2.0)),
        },
        Instruction {
            opcode: ISUB,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(imm_i(100)),
        },
        Instruction {
            opcode: FDIV,
            operand1: Some(reg(5)),
            operand2: Some(reg(3)),
            operand3: Some(imm_f(2.5)),
        },
        Instruction {
            opcode: UDIV,
            operand1: Some(reg(6)),
            operand2: Some(reg(4)),
            operand3: Some(imm_i(16)),
        },
        Instruction {
            opcode: FREM,
            operand1: Some(reg(7)),
            operand2: Some(reg(3)),
            operand3: Some(imm_f(3.0)),
        },
        Instruction {
            opcode: UREM,
            operand1: Some(reg(8)),
            operand2: Some(reg(4)),
            operand3: Some(imm_i(128)),
        },
        Instruction {
            opcode: IADD,
            operand1: Some(reg(9)),
            operand2: Some(imm_i(u64::MAX)),
            operand3: Some(imm_i(1)),
        },
        Instruction {
            opcode: FADD,
            operand1: Some(reg(10)),
            operand2: Some(imm_f(-0.0)),
            operand3: Some(imm_f(0.0)),
        },
    ]);

    assert_eq!(vm.registers[Register(0)], 300);
    assert_eq!(f64::from_bits(vm.registers[Register(1)]), 6.25);
    assert_eq!(vm.registers[Register(2)], 900);
    assert_eq!(f64::from_bits(vm.registers[Register(3)]), 12.5);
    assert_eq!(vm.registers[Register(4)], 800);
    assert_eq!(f64::from_bits(vm.registers[Register(5)]), 5.0);
    assert_eq!(vm.registers[Register(6)], 50);
    assert_eq!(f64::from_bits(vm.registers[Register(7)]), 0.5);
    assert_eq!(vm.registers[Register(8)], 32);
    assert_eq!(vm.registers[Register(9)], 0);
    assert_eq!(f64::from_bits(vm.registers[Register(10)]), 0.0);
}
