// Tests for `RET`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{ExecuteVariant, WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

#[test]
fn ret_returns_to_saved_address() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.call_stack.push(1);

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::RET,
                operand1: None,
                operand2: None,
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(0)),
                operand2: Some(imm(42)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(0)], 42);
}

#[test]
fn ret_empty_call_stack_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::RET,
        operand1: None,
        operand2: None,
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected EmptyCallStack error"),
    };

    assert!(matches!(err.kind, VMErrorKind::EmptyCallStack));
}

#[test]
fn ret_with_operands_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::RET,
        operand1: Some(imm(0)),
        operand2: None,
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected IncorrectNumberOfOperands error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectNumberOfOperands {
            expected: 0,
            got: 1
        }
    ));
}

#[test]
fn ret_returns_to_correct_address_among_multiple() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.call_stack.push(2);
    vm.call_stack.push(3);

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::RET,
                operand1: None,
                operand2: None,
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(0)),
                operand2: Some(imm(1)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::RET,
                operand1: None,
                operand2: None,
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(0)),
                operand2: Some(imm(99)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(0)], 99);
}

#[test]
fn ret_clears_one_entry_from_call_stack() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.call_stack.push(1);

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::RET,
                operand1: None,
                operand2: None,
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::NOP,
                operand1: None,
                operand2: None,
                operand3: None,
            },
        ],
    );
    assert!(vm.call_stack.is_empty());
}
