use w8_core::vm::ExecuteVariant;
// Tests for `NOT`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::{arithmetic::bitwise::get_not_result, helpers::*};

#[test]
fn not_basic() {
    assert_eq!(get_not_result(0xFF00), 0xFFFF_FFFF_FFFF_00FF);
}

#[test]
fn not_zero() {
    assert_eq!(get_not_result(0), u64::MAX);
}

#[test]
fn not_max() {
    assert_eq!(get_not_result(u64::MAX), 0);
}

#[test]
fn not_double_not_original() {
    assert_eq!(get_not_result(get_not_result(0xDEAD_BEEF)), 0xDEAD_BEEF);
}

#[test]
fn not_all_patterns() {
    assert_eq!(get_not_result(0xAAAA_AAAA_AAAA_AAAA), 0x5555_5555_5555_5555);
    assert_eq!(get_not_result(0x5555_5555_5555_5555), 0xAAAA_AAAA_AAAA_AAAA);
}

#[test]
fn not_large_value() {
    assert_eq!(
        get_not_result(0xDEAD_BEEF_CAFE_BABE),
        0xDEAD_BEEF_CAFE_BABE ^ u64::MAX
    );
}

#[test]
fn not_with_wrong_operand_count_returns_error() {
    let err = match crate::vm_tests::helpers::interpretate_with_result(vec![Instruction {
        opcode: OperationCode::NOT,
        operand1: Some(reg(0)),
        operand2: Some(reg(1)),
        operand3: Some(imm(2)),
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectNumberOfOperands {
            expected: 2,
            got: 3
        }
    ));
}

#[test]
fn not_with_immediate_destination_returns_error() {
    let err = match crate::vm_tests::helpers::interpretate_with_result(vec![Instruction {
        opcode: OperationCode::NOT,
        operand1: Some(imm(0)),
        operand2: Some(imm(0xFF)),
        operand3: None,
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
fn not_register_sources() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.registers[Register(1)] = 0xFF00;

    w8.program = vec![Instruction {
        opcode: OperationCode::NOT,
        operand1: Some(reg(0)),
        operand2: Some(reg(1)),
        operand3: None,
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(0)], 0xFFFF_FFFF_FFFF_00FF);
    assert_eq!(w8.registers[Register(1)], 0xFF00);
}
