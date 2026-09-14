use w8_core::vm::ExecuteVariant;
// Tests for `STORE64`.
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
fn store64_register_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xDEAD_BEEF_CAFE_BABE;
    vm.registers[Register(1)] = 10;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(reg(1)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(10), Some(0xDEAD_BEEF_CAFE_BABE));
}

#[test]
fn store64_immediate_value_at_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 20;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(reg(1)),
            operand2: Some(imm(0x1234_5678_9ABC_DEF0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(20), Some(0x1234_5678_9ABC_DEF0));
}

#[test]
fn store64_register_value_at_immediate_address() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0x0F0E_0D0C_0B0A_0908;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(30)),
            operand2: Some(reg(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(30), Some(0x0F0E_0D0C_0B0A_0908));
}

#[test]
fn store64_immediate_value_at_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(8)),
            operand2: Some(imm(0xFFFF_FFFF_FFFF_FFFF)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(8), Some(0xFFFF_FFFF_FFFF_FFFF));
}

#[test]
fn store64_little_endian() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x0102_0304_0506_0708)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0x08));
    assert_eq!(vm.memory.load_u8(1), Some(0x07));
    assert_eq!(vm.memory.load_u8(2), Some(0x06));
    assert_eq!(vm.memory.load_u8(3), Some(0x05));
    assert_eq!(vm.memory.load_u8(4), Some(0x04));
    assert_eq!(vm.memory.load_u8(5), Some(0x03));
    assert_eq!(vm.memory.load_u8(6), Some(0x02));
    assert_eq!(vm.memory.load_u8(7), Some(0x01));
}

#[test]
fn store64_overwrites_existing_value() {
    let mut vm = vm_with_memory();
    vm.memory.store_u64(0, 0xFFFF_FFFF_FFFF_FFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x0000_0000_0000_0000)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(0), Some(0x0000_0000_0000_0000));
}

#[test]
fn store64_at_address_zero() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(0)),
            operand2: Some(imm(0xDEAD_BEEF_CAFE_BABE)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(0), Some(0xDEAD_BEEF_CAFE_BABE));
}

#[test]
fn store64_at_last_valid_address() {
    let vm = WVM::new(8, ExecuteVariant::default());
    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x0123_4567_89AB_CDEF)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(0), Some(0x0123_4567_89AB_CDEF));
}

#[test]
fn store64_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE64,
        operand1: Some(imm(0)),
        operand2: Some(imm(0xFFFF_FFFF_FFFF_FFFF)),
        operand3: None,
    }]) {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(err.kind, VMErrorKind::InvalidAddress { .. }));
}

#[test]
fn store64_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 9;
    vm.program = vec![Instruction {
        opcode: OperationCode::STORE64,
        operand1: Some(reg(1)),
        operand2: Some(imm(0xFFFF_FFFF_FFFF_FFFF)),
        operand3: None,
    }];

    let err = match vm.interpretate() {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::InvalidAddress {
            got: 9,
            memory_length: 16
        }
    ));
}

#[test]
fn store64_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE64,
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
fn store64_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE64,
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
fn store64_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::STORE64,
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
fn store64_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 10;
    vm.registers[Register(2)] = 0xDEAD_BEEF_CAFE_BABE;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(reg(1)),
            operand2: Some(reg(2)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 10);
    assert_eq!(vm.registers[Register(2)], 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn store64_multiple_stores_sequence() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE64,
                operand1: Some(imm(0)),
                operand2: Some(imm(0x1111_1111_1111_1111)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE64,
                operand1: Some(imm(8)),
                operand2: Some(imm(0x2222_2222_2222_2222)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::STORE64,
                operand1: Some(imm(16)),
                operand2: Some(imm(0x3333_3333_3333_3333)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.memory.load_u64(0), Some(0x1111_1111_1111_1111));
    assert_eq!(vm.memory.load_u64(8), Some(0x2222_2222_2222_2222));
    assert_eq!(vm.memory.load_u64(16), Some(0x3333_3333_3333_3333));
}

#[test]
fn store64_store_then_load_verify() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::STORE64,
                operand1: Some(imm(8)),
                operand2: Some(imm(0xDEAD_BEEF_CAFE_BABE)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD64,
                operand1: Some(reg(0)),
                operand2: Some(imm(8)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn store64_at_unaligned_address() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(1)),
            operand2: Some(imm(0x0102_0304_0506_0708)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u64(1), Some(0x0102_0304_0506_0708));
}

#[test]
fn store64_one_byte_short_returns_error() {
    let mut vm = WVM::new(8, ExecuteVariant::default());
    vm.registers[Register(1)] = 1;
    vm.program = vec![Instruction {
        opcode: OperationCode::STORE64,
        operand1: Some(reg(1)),
        operand2: Some(imm(0xFFFF_FFFF_FFFF_FFFF)),
        operand3: None,
    }];

    let err = match vm.interpretate() {
        Err(err) => err,
        Ok(_) => panic!("expected execution error"),
    };

    assert!(matches!(
        err.kind,
        VMErrorKind::InvalidAddress {
            got: 1,
            memory_length: 8
        }
    ));
}

#[test]
fn store64_all_bytes_written_correctly() {
    let vm = vm_with_memory();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::STORE64,
            operand1: Some(imm(0)),
            operand2: Some(imm(0x0011_2233_4455_6677)),
            operand3: None,
        }],
    );

    assert_eq!(vm.memory.load_u8(0), Some(0x77));
    assert_eq!(vm.memory.load_u8(1), Some(0x66));
    assert_eq!(vm.memory.load_u8(2), Some(0x55));
    assert_eq!(vm.memory.load_u8(3), Some(0x44));
    assert_eq!(vm.memory.load_u8(4), Some(0x33));
    assert_eq!(vm.memory.load_u8(5), Some(0x22));
    assert_eq!(vm.memory.load_u8(6), Some(0x11));
    assert_eq!(vm.memory.load_u8(7), Some(0x00));
}
