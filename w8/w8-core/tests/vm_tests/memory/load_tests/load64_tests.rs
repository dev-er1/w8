use w8_core::vm::ExecuteVariant;
// Tests for `LOAD64`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{WVM, err::VMErrorKind},
};

use crate::vm_tests::helpers::*;

const MEM_SIZE: usize = 256;

fn vm_with_memory() -> WVM {
    let mut vm = WVM::new(MEM_SIZE, ExecuteVariant::default());
    vm.memory.store_u64(8, 0xDEAD_BEEF_CAFE_BABE).unwrap();
    vm.memory.store_u64(16, 0x1234_5678_9ABC_DEF0).unwrap();
    vm.memory.store_u64(248, 0xFFFF_FFFF_FFFF_FFFF).unwrap();
    vm
}

#[test]
fn load64_loads_qword_from_address_in_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 8;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn load64_loads_qword_from_immediate_address() {
    let vm = run_on(
        vm_with_memory(),
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(2)),
            operand2: Some(imm(8)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(2)], 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn load64_little_endian() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u8(0, 0xBE).unwrap();
    vm.memory.store_u8(1, 0xBA).unwrap();
    vm.memory.store_u8(2, 0xFE).unwrap();
    vm.memory.store_u8(3, 0xCA).unwrap();
    vm.memory.store_u8(4, 0xEF).unwrap();
    vm.memory.store_u8(5, 0xBE).unwrap();
    vm.memory.store_u8(6, 0xAD).unwrap();
    vm.memory.store_u8(7, 0xDE).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn load64_from_address_zero() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u64(0, 0x0123_4567_89AB_CDEF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0123_4567_89AB_CDEF);
}

#[test]
fn load64_from_last_valid_address() {
    let mut vm = WVM::new(8, ExecuteVariant::default());
    vm.memory.store_u64(0, 0x0F0E_0D0C_0B0A_0908).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0F0E_0D0C_0B0A_0908);
}

#[test]
fn load64_out_of_bounds_zero_memory_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD64,
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
fn load64_out_of_bounds_address_returns_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.registers[Register(1)] = 9;

    vm.program = vec![Instruction {
        opcode: OperationCode::LOAD64,
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
            got: 9,
            memory_length: 16
        }
    ));
}

#[test]
fn load64_with_wrong_operand_count_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD64,
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
fn load64_with_immediate_destination_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD64,
        operand1: Some(imm(0)),
        operand2: Some(imm(8)),
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
fn load64_overwrites_destination_register() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0x0000_0000_0000_0000;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(16)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x1234_5678_9ABC_DEF0);
}

#[test]
fn load64_max_qword_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u64(0, 0xFFFF_FFFF_FFFF_FFFF).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xFFFF_FFFF_FFFF_FFFF);
}

#[test]
fn load64_min_qword_value() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u64(0, 0x0000_0000_0000_0000).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0000_0000_0000_0000);
}

#[test]
fn load64_address_register_unchanged() {
    let mut vm = vm_with_memory();
    vm.registers[Register(1)] = 8;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(reg(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(1)], 8);
}

#[test]
fn load64_only_one_operand_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD64,
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
fn load64_no_operands_returns_error() {
    let err = match interpretate_with_result(vec![Instruction {
        opcode: OperationCode::LOAD64,
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
fn load64_multiple_loads_into_different_registers() {
    let mut vm = WVM::new(64, ExecuteVariant::default());
    vm.memory.store_u64(8, 0x1111_1111_1111_1111).unwrap();
    vm.memory.store_u64(24, 0x2222_2222_2222_2222).unwrap();
    vm.memory.store_u64(40, 0x3333_3333_3333_3333).unwrap();

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::LOAD64,
                operand1: Some(reg(0)),
                operand2: Some(imm(8)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD64,
                operand1: Some(reg(1)),
                operand2: Some(imm(24)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::LOAD64,
                operand1: Some(reg(2)),
                operand2: Some(imm(40)),
                operand3: None,
            },
        ],
    );

    assert_eq!(vm.registers[Register(0)], 0x1111_1111_1111_1111);
    assert_eq!(vm.registers[Register(1)], 0x2222_2222_2222_2222);
    assert_eq!(vm.registers[Register(2)], 0x3333_3333_3333_3333);
}

#[test]
fn load64_from_unaligned_address() {
    let mut vm = WVM::new(32, ExecuteVariant::default());
    vm.memory.store_u8(1, 0xEF).unwrap();
    vm.memory.store_u8(2, 0xCD).unwrap();
    vm.memory.store_u8(3, 0xAB).unwrap();
    vm.memory.store_u8(4, 0x89).unwrap();
    vm.memory.store_u8(5, 0x67).unwrap();
    vm.memory.store_u8(6, 0x45).unwrap();
    vm.memory.store_u8(7, 0x23).unwrap();
    vm.memory.store_u8(8, 0x01).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(1)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0123_4567_89AB_CDEF);
}

#[test]
fn load64_preserves_other_registers() {
    let mut vm = vm_with_memory();
    vm.registers[Register(0)] = 0xAAAA_BBBB_CCCC_DDDD;
    vm.registers[Register(2)] = 0xEEEE_FFFF_0000_1111;

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(8)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(vm.registers[Register(2)], 0xEEEE_FFFF_0000_1111);
}

#[test]
fn load64_boundary_at_end_of_memory() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    vm.memory.store_u64(8, 0x0102_0304_0506_0708).unwrap();

    let vm = run_on(
        vm,
        vec![Instruction {
            opcode: OperationCode::LOAD64,
            operand1: Some(reg(0)),
            operand2: Some(imm(8)),
            operand3: None,
        }],
    );

    assert_eq!(vm.registers[Register(0)], 0x0102_0304_0506_0708);
}

#[test]
fn load64_one_byte_short_returns_error() {
    let mut vm = WVM::new(8, ExecuteVariant::default());
    vm.registers[Register(1)] = 1;

    vm.program = vec![Instruction {
        opcode: OperationCode::LOAD64,
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
            got: 1,
            memory_length: 8
        }
    ));
}
