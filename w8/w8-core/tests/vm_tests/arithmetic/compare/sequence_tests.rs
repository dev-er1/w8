// Tests for comparisons.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{ExecuteVariant, WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

#[test]
fn compare_wrong_operand_count() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::IEQ,
        operand1: Some(reg(0)),
        operand2: Some(reg(1)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectNumberOfOperands {
            expected: 3,
            got: 2
        }
    ));
}

#[test]
fn compare_immediate_destination() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::SLT,
        operand1: Some(imm(0)),
        operand2: Some(imm(1)),
        operand3: Some(imm(2)),
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectTypeOfOperand { .. }
    ));
}

#[test]
fn compare_chain_ieq_slt_uge() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.registers[Register(1)] = 10;
    w8.registers[Register(2)] = 20;

    w8.program = vec![
        Instruction {
            opcode: OperationCode::IEQ,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: Some(reg(2)),
        },
        Instruction {
            opcode: OperationCode::SLT,
            operand1: Some(reg(3)),
            operand2: Some(reg(1)),
            operand3: Some(reg(2)),
        },
        Instruction {
            opcode: OperationCode::UGE,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(reg(1)),
        },
    ];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(0)], 0);
    assert_eq!(w8.registers[Register(3)], 1);
    assert_eq!(w8.registers[Register(4)], 1);
}

#[test]
fn compare_float_chain() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    let a = 1.5f64;
    let b = 2.5f64;

    w8.registers[Register(1)] = a.to_bits();
    w8.registers[Register(2)] = b.to_bits();

    w8.program = vec![
        Instruction {
            opcode: OperationCode::FEQ,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: Some(reg(2)),
        },
        Instruction {
            opcode: OperationCode::FLT,
            operand1: Some(reg(3)),
            operand2: Some(reg(1)),
            operand3: Some(reg(2)),
        },
        Instruction {
            opcode: OperationCode::FGT,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(reg(1)),
        },
    ];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(0)], 0);
    assert_eq!(w8.registers[Register(3)], 1);
    assert_eq!(w8.registers[Register(4)], 1);
}

#[test]
fn compare_mixed_int_and_float() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());

    w8.program = vec![
        Instruction {
            opcode: OperationCode::IEQ,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: Some(imm(0)),
        },
        Instruction {
            opcode: OperationCode::FEQ,
            operand1: Some(reg(1)),
            operand2: Some(imm(f64::NAN.to_bits())),
            operand3: Some(imm(f64::NAN.to_bits())),
        },
    ];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(0)], 1);
    assert_eq!(w8.registers[Register(1)], 0);
}
