use w8_core::vm::ExecuteVariant;
// Tests for `MOVE`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

#[test]
fn move_register_to_register_copies_value() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(1)] = 42;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 42);
    assert_eq!(vm.registers[Register(1)], 42);
}

#[test]
fn move_immediate_to_register_sets_value() {
    let vm = run(vec![Instruction {
        opcode: OperationCode::MOVE,
        operand1: Some(reg(2)),
        operand2: Some(imm(123)),
        operand3: None,
    }]);

    assert_eq!(vm.registers[Register(2)], 123);
}

#[test]
fn move_overwrites_existing_register_value() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(3)] = 7;
    vm.registers[Register(4)] = 99;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(3)),
            operand2: Some(reg(4)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(3)], 99);
    assert_eq!(vm.registers[Register(4)], 99);
}

#[test]
fn move_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::MOVE,
        operand1: Some(reg(0)),
        operand2: Some(reg(1)),
        operand3: Some(imm(1)),
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
fn move_with_immediate_destination_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::MOVE,
        operand1: Some(imm(0)),
        operand2: Some(reg(0)),
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
fn move_with_out_of_range_register_returns_error() {
    // W8 has 255 registers (0-254): 255 must be rejected at encoding.
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::MOVE,
        operand1: Some(reg(255)),
        operand2: Some(imm(1)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::InvalidRegister { got: 255 }
    ));
}
