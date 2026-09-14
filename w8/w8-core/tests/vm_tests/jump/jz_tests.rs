use w8_core::vm::ExecuteVariant;
// Tests for `JZ`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::WVM,
};

use crate::vm_tests::helpers::*;

#[test]
fn jz_immediate_zero_jumps() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::JZ,
            operand1: Some(imm(0)),
            operand2: Some(imm(3)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(1)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(2)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(1)),
            operand2: Some(imm(99)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 0);
    assert_eq!(vm.registers[Register(1)], 99);
}

#[test]
fn jz_immediate_nonzero_does_not_jump() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::JZ,
            operand1: Some(imm(5)),
            operand2: Some(imm(4)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(10)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(1)),
            operand2: Some(imm(20)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(2)),
            operand2: Some(imm(30)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 10);
    assert_eq!(vm.registers[Register(1)], 20);
    assert_eq!(vm.registers[Register(2)], 30);
}

#[test]
fn jz_register_zero_jumps() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(0)] = 0;

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::JZ,
                operand1: Some(reg(0)),
                operand2: Some(imm(3)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(1)),
                operand2: Some(imm(1)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(1)),
                operand2: Some(imm(2)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(42)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(1)], 0);
    assert_eq!(vm.registers[Register(2)], 42);
}

#[test]
fn jz_register_nonzero_does_not_jump() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(0)] = 1;

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::JZ,
                operand1: Some(reg(0)),
                operand2: Some(imm(4)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(0)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(1)),
                operand2: Some(imm(20)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(30)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(0)], 10);
    assert_eq!(vm.registers[Register(1)], 20);
    assert_eq!(vm.registers[Register(2)], 30);
}

#[test]
fn jz_chain_skip_when_zero() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(0)] = 0;
    vm.registers[Register(1)] = 5;

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::JZ,
                operand1: Some(reg(0)),
                operand2: Some(imm(4)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(1)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::JNZ,
                operand1: Some(reg(1)),
                operand2: Some(imm(6)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(2)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(3)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(2)),
                operand2: Some(imm(4)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(2)], 4);
}
