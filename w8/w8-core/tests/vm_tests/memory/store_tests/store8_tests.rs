use w8_core::vm::ExecuteVariant;
// Tests for `STORE8`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

const MEM_SIZE: usize = 64;

fn vm_with_memory() -> WVM {
    WVM::new(MEM_SIZE, ExecuteVariant::default())
}

#[test]
fn store8_register_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xAB;
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(reg(1)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(10), Some(0xAB));
}

#[test]
fn store8_immediate_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 20;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(reg(1)),
            operand2: Some(imm(0xCD)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(20), Some(0xCD));
}

#[test]
fn store8_register_value_at_immediate_address() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xEF;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(30)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(30), Some(0xEF));
}

#[test]
fn store8_immediate_value_at_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(5)),
            operand2: Some(imm(0x42)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(5), Some(0x42));
}

#[test]
fn store8_overwrites_existing_value() {
    let mut vm = vm_with_memory();
    vm.memory.store_u8(15, 0xFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(15)),
            operand2: Some(imm(0x00)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(15), Some(0x00));
}

#[test]
fn store8_at_address_zero() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(0)),
            operand2: Some(imm(0xAA)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0xAA));
}

#[test]
fn store8_at_last_valid_address() {
    let vm = WVM::new(1, ExecuteVariant::default());
    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x7F)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0x7F));
}

#[test]
fn store8_value_truncated_to_u8() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xDEAD_BEEF_CAFE_BABE;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(0)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0xBE));
}

#[test]
fn store8_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE8,
        operand1: Some(imm(0)),
        operand2: Some(imm(0xFF)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(err.kind, VMErrorKind::InvalidAddress { .. }));
}

#[test]
fn store8_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 100;
    vm.program = vec![Instruction {
        opcode: OperationCode::STORE8,
        operand1: Some(reg(1)),
        operand2: Some(imm(0xFF)),
        operand3: None,
    }];

    let err = match vm.interpretate() {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::InvalidAddress {
            got: 100,
            memory_length: 16
        }
    ));
}

#[test]
fn store8_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE8,
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
fn store8_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE8,
        operand1: Some(reg(0)),
        operand2: None,
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectNumberOfOperands {
            expected: 2,
            got: 1
        }
    ));
}

#[test]
fn store8_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE8,
        operand1: None,
        operand2: None,
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::IncorrectNumberOfOperands {
            expected: 2,
            got: 0
        }
    ));
}

#[test]
fn store8_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;
    vm.registers[Register(2)] = 0xAB;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(reg(1)),
            operand2: Some(reg(2)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
    assert_eq!(vm.registers[Register(2)], 0xAB);
}

#[test]
fn store8_multiple_stores_sequence() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE8,
                operand1: Some(imm(0)),
                operand2: Some(imm(0x11)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE8,
                operand1: Some(imm(1)),
                operand2: Some(imm(0x22)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE8,
                operand1: Some(imm(2)),
                operand2: Some(imm(0x33)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0x11));
    assert_eq!(vm.memory.load_u8(1), Some(0x22));
    assert_eq!(vm.memory.load_u8(2), Some(0x33));
}

#[test]
fn store8_store_then_load_verify() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE8,
                operand1: Some(imm(10)),
                operand2: Some(imm(0x77)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x77);
}

#[test]
fn store8_immediate_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(5)] = 40;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(reg(5)),
            operand2: Some(imm(0x99)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(40), Some(0x99));
}

#[test]
fn store8_only_affects_target_byte() {
    let mut vm = vm_with_memory();
    vm.memory.store_u16(0, 0xFFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE8,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x00)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0x00));
    assert_eq!(vm.memory.load_u8(1), Some(0xFF));
}
