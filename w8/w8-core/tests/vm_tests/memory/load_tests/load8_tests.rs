use w8_core::vm::ExecuteVariant;
// Tests for `LOAD8`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

const MEM_SIZE: usize = 256;

fn vm_with_memory() -> WVM {
    let mut vm = WVM::new(MEM_SIZE, ExecuteVariant::default());
    vm.memory.store_u8(10, 0xAB).unwrap();
    vm.memory.store_u8(11, 0xCD).unwrap();
    vm.memory.store_u8(255, 0x42).unwrap();
    vm
}

#[test]
fn load8_loads_byte_from_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xAB);
}

#[test]
fn load8_loads_byte_from_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(2)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(2)], 0xAB);
}

#[test]
fn load8_zero_extends_to_u64() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(255)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x42);
    assert_eq!(vm.registers[Register(0)], 0x0000_0000_0000_0042);
}

#[test]
fn load8_different_bytes_in_memory() {
    let vm = vm_with_memory();
    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(1)),
                operand2: Some(imm(11)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0xAB);
    assert_eq!(vm.registers[Register(1)], 0xCD);
}

#[test]
fn load8_from_address_zero() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(0, 0xFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xFF);
}

#[test]
fn load8_from_last_valid_address() {
    let mut vm = WVM::new(1, ExecuteVariant::default());
    vm.memory.store_u8(0, 0x7F).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x7F);
}

#[test]
fn load8_out_of_bounds_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD8,
        operand1: Some(reg(0)),
        operand2: Some(imm(0)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::InvalidAddress {
            got: 0,
            memory_length: 0
        }
    ));
}

#[test]
fn load8_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 100;
    vm.program = vec![Instruction {
        opcode: OperationCode::LOAD8,
        operand1: Some(reg(0)),
        operand2: Some(reg(1)),
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
fn load8_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD8,
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
fn load8_with_immediate_destination_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD8,
        operand1: Some(imm(0)),
        operand2: Some(imm(10)),
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
fn load8_zero_address_register_source() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 0;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0);
}

#[test]
fn load8_overwrites_destination_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xDEAD_BEEF_CAFE_BABE;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xAB);
}

#[test]
fn load8_max_byte_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(5, 0xFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(5)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xFF);
}

#[test]
fn load8_min_byte_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(3, 0x00).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(imm(3)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x00);
}

#[test]
fn load8_address_in_register_does_not_change() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD8,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
}

#[test]
fn load8_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD8,
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
fn load8_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD8,
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
fn load8_multiple_loads_accumulate_in_different_registers() {
    let mut vm = WVM::new(64, ExecuteVariant::default());
    vm.memory.store_u8(10, 0x11).unwrap();
    vm.memory.store_u8(20, 0x22).unwrap();
    vm.memory.store_u8(30, 0x33).unwrap();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(1)),
                operand2: Some(imm(20)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD8,
                operand1: Some(reg(2)),
                operand2: Some(imm(30)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x11);
    assert_eq!(vm.registers[Register(1)], 0x22);
    assert_eq!(vm.registers[Register(2)], 0x33);
}
