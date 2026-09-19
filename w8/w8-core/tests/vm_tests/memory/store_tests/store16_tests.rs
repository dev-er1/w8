use w8_core::vm::ExecuteVariant;
// Tests for `STORE16`.
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
fn store16_register_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xAABB;
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(reg(1)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(10), Some(0xAABB));
}

#[test]
fn store16_immediate_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 20;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(reg(1)),
            operand2: Some(imm(0xCCDD)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(20), Some(0xCCDD));
}

#[test]
fn store16_register_value_at_immediate_address() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xEEFF;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(30)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(30), Some(0xEEFF));
}

#[test]
fn store16_immediate_value_at_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(5)),
            operand2: Some(imm(0x1234)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(5), Some(0x1234));
}

#[test]
fn store16_little_endian() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(0)),
            operand2: Some(imm(0xABCD)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0xCD));
    assert_eq!(vm.memory.load_u8(1), Some(0xAB));
}

#[test]
fn store16_overwrites_existing_value() {
    let mut vm = vm_with_memory();
    vm.memory.store_u16(15, 0xFFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(15)),
            operand2: Some(imm(0x0000)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(15), Some(0x0000));
}

#[test]
fn store16_at_address_zero() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(0)),
            operand2: Some(imm(0xBEEF)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(0), Some(0xBEEF));
}

#[test]
fn store16_at_last_valid_address() {
    let vm = WVM::new(2, ExecuteVariant::default());
    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(0)),
            operand2: Some(imm(0xCAFE)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(0), Some(0xCAFE));
}

#[test]
fn store16_value_truncated_to_u16() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xDEAD_BEEF_CAFE_BABE;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(0)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(0), Some(0xBABE));
}

#[test]
fn store16_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE16,
        operand1: Some(imm(0)),
        operand2: Some(imm(0xFFFF)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(err.kind, VMErrorKind::InvalidAddress { .. }));
}

#[test]
fn store16_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 15;
    vm.program = vec![Instruction {
        opcode: OperationCode::STORE16,
        operand1: Some(reg(1)),
        operand2: Some(imm(0xFFFF)),
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
fn store16_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE16,
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
fn store16_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE16,
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
fn store16_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE16,
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
fn store16_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;
    vm.registers[Register(2)] = 0xAABB;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(reg(1)),
            operand2: Some(reg(2)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
    assert_eq!(vm.registers[Register(2)], 0xAABB);
}

#[test]
fn store16_multiple_stores_sequence() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE16,
                operand1: Some(imm(0)),
                operand2: Some(imm(0x1111)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE16,
                operand1: Some(imm(2)),
                operand2: Some(imm(0x2222)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE16,
                operand1: Some(imm(4)),
                operand2: Some(imm(0x3333)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.memory.load_u16(0), Some(0x1111));
    assert_eq!(vm.memory.load_u16(2), Some(0x2222));
    assert_eq!(vm.memory.load_u16(4), Some(0x3333));
}

#[test]
fn store16_store_then_load_verify() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE16,
                operand1: Some(imm(10)),
                operand2: Some(imm(0x7788)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD16,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x7788);
}

#[test]
fn store16_at_unaligned_address() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(3)),
            operand2: Some(imm(0x1234)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(3), Some(0x1234));
}

#[test]
fn store16_only_affects_two_bytes() {
    let mut vm = vm_with_memory();
    vm.memory.store_u32(0, 0xFFFF_FFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE16,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x0000)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u16(0), Some(0x0000));
    assert_eq!(vm.memory.load_u16(2), Some(0xFFFF));
}
