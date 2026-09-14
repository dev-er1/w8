use w8_core::vm::ExecuteVariant;
// Tests for `LOAD32`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

const MEM_SIZE: usize = 256;

fn vm_with_memory() -> WVM {
    let mut vm = WVM::new(MEM_SIZE, ExecuteVariant::default());
    vm.memory.store_u32(10, 0xDEAD_BEEF).unwrap();
    vm.memory.store_u32(20, 0xCAFE_BABE).unwrap();
    vm.memory.store_u32(252, 0x1234_5678).unwrap();
    vm
}

#[test]
fn load32_loads_dword_from_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF);
}

#[test]
fn load32_loads_dword_from_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(2)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(2)], 0xDEAD_BEEF);
}

#[test]
fn load32_zero_extends_to_u64() {
    let mut vm = WVM::new(32, ExecuteVariant::default());
    vm.memory.store_u32(0, 0x8000_0000).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0000_0000_8000_0000);
}

#[test]
fn load32_little_endian() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(10, 0xEF).unwrap();
    vm.memory.store_u8(11, 0xBE).unwrap();
    vm.memory.store_u8(12, 0xAD).unwrap();
    vm.memory.store_u8(13, 0xDE).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(10)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF);
}

#[test]
fn load32_from_address_zero() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u32(0, 0x1234_5678).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x1234_5678);
}

#[test]
fn load32_from_last_valid_address() {
    let mut vm = WVM::new(4, ExecuteVariant::default());
    vm.memory.store_u32(0, 0xCAFE_BABE).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xCAFE_BABE);
}

#[test]
fn load32_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD32,
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
fn load32_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 13;

    vm.program = vec![Instruction {
        opcode: OperationCode::LOAD32,
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
            got: 13,
            memory_length: 16
        }
    ));
}

#[test]
fn load32_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD32,
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
fn load32_with_immediate_destination_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD32,
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
fn load32_overwrites_destination_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xFFFF_FFFF_FFFF_FFFF;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(20)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xCAFE_BABE);
}

#[test]
fn load32_max_dword_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u32(0, 0xFFFF_FFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xFFFF_FFFF);
}

#[test]
fn load32_min_dword_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u32(0, 0x0000_0000).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0000_0000);
}

#[test]
fn load32_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
}

#[test]
fn load32_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD32,
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
fn load32_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD32,
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
fn load32_multiple_loads_into_different_registers() {
    let mut vm = WVM::new(64, ExecuteVariant::default());
    vm.memory.store_u32(10, 0x1111_1111).unwrap();
    vm.memory.store_u32(20, 0x2222_2222).unwrap();
    vm.memory.store_u32(30, 0x3333_3333).unwrap();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD32,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD32,
                operand1: Some(reg(1)),
                operand2: Some(imm(20)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD32,
                operand1: Some(reg(2)),
                operand2: Some(imm(30)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x1111_1111);
    assert_eq!(vm.registers[Register(1)], 0x2222_2222);
    assert_eq!(vm.registers[Register(2)], 0x3333_3333);
}

#[test]
fn load32_from_unaligned_address() {
    let mut vm = WVM::new(32, ExecuteVariant::default());
    vm.memory.store_u8(3, 0x78).unwrap();
    vm.memory.store_u8(4, 0x56).unwrap();
    vm.memory.store_u8(5, 0x34).unwrap();
    vm.memory.store_u8(6, 0x12).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(3)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x1234_5678);
}

#[test]
fn load32_high_bits_cleared() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u32(0, 0xFFFF_FFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD32,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0000_0000_FFFF_FFFF);
}
