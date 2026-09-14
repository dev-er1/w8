use w8_core::vm::ExecuteVariant;
// Tests for `LOAD16`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

const MEM_SIZE: usize = 256;

fn vm_with_memory() -> WVM {
    let mut vm = WVM::new(MEM_SIZE, ExecuteVariant::default());
    vm.memory.store_u16(10, 0xAABB).unwrap();
    vm.memory.store_u16(20, 0xCCDD).unwrap();
    vm.memory.store_u16(254, 0x1234).unwrap();
    vm
}

#[test]
fn load16_loads_word_from_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xAABB);
}

#[test]
fn load16_loads_word_from_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(2)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(2)], 0xAABB);
}

#[test]
fn load16_zero_extends_to_u64() {
    let mut vm = WVM::new(32, ExecuteVariant::default());
    vm.memory.store_u16(0, 0x8000).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x8000);
    assert_eq!(vm.registers[Register(0)], 0x0000_0000_0000_8000);
}

#[test]
fn load16_little_endian() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(10, 0x34).unwrap();
    vm.memory.store_u8(11, 0x12).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x1234);
}

#[test]
fn load16_from_address_zero() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u16(0, 0xDEAD).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD);
}

#[test]
fn load16_from_last_valid_address() {
    let mut vm = WVM::new(2, ExecuteVariant::default());
    vm.memory.store_u16(0, 0xBEEF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xBEEF);
}

#[test]
fn load16_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD16,
        operand1: Some(reg(0)),
        operand2: Some(imm(0)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(err.kind, VMErrorKind::InvalidAddress { .. }));
}

#[test]
fn load16_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 15;

    vm.program = vec![Instruction {
        opcode: OperationCode::LOAD16,
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
            got: 15,
            memory_length: 16
        }
    ));
}

#[test]
fn load16_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD16,
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
fn load16_with_immediate_destination_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD16,
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
fn load16_overwrites_destination_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xFFFF_FFFF_FFFF_FFFF;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(20)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xCCDD);
}

#[test]
fn load16_max_word_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u16(0, 0xFFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xFFFF);
}

#[test]
fn load16_min_word_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u16(0, 0x0000).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0000);
}

#[test]
fn load16_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
}

#[test]
fn load16_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD16,
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
fn load16_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD16,
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
fn load16_multiple_loads_into_different_registers() {
    let mut vm = WVM::new(64, ExecuteVariant::default());
    vm.memory.store_u16(10, 0x1111).unwrap();
    vm.memory.store_u16(20, 0x2222).unwrap();
    vm.memory.store_u16(30, 0x3333).unwrap();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(1)),
                operand2: Some(imm(20)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(2)),
                operand2: Some(imm(30)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x1111);
    assert_eq!(vm.registers[Register(1)], 0x2222);
    assert_eq!(vm.registers[Register(2)], 0x3333);
}

#[test]
fn load16_from_unaligned_address() {
    let mut vm = WVM::new(32, ExecuteVariant::default());
    vm.memory.store_u8(3, 0x78).unwrap();
    vm.memory.store_u8(4, 0x56).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD16,
            operand1: Some(reg(0)),
            operand2: Some(imm(3)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x5678);
}

#[test]
fn load16_address_from_register_with_offset() {
    let mut vm = vm_with_memory();
    vm.registers[Register(5)] = 10;
    vm.registers[Register(6)] = 20;

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(0)),
                operand2: Some(reg(5)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(1)),
                operand2: Some(reg(6)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0xAABB);
    assert_eq!(vm.registers[Register(1)], 0xCCDD);
}
