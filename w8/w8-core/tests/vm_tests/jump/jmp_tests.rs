use w8_core::vm::ExecuteVariant;
// Tests for `JMP`.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::WVM,
};

use crate::vm_tests::helpers::*;

#[test]
fn jmp_immediate_forward_skips_instructions() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(1)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::JMP,
            operand1: Some(imm(3)),
            operand2: None,
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
            operand2: Some(imm(3)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 1);
    assert_eq!(vm.registers[Register(1)], 3);
}

#[test]
fn jmp_register_address() {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.registers[Register(0)] = 3;

    let vm = run_on(
        vm,
        vec![
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(1)),
                operand2: Some(imm(10)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::JMP,
                operand1: Some(reg(0)),
                operand2: None,
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
                operand1: Some(reg(1)),
                operand2: Some(imm(30)),
                operand3: None,
            },
        ],
    );
    assert_eq!(vm.registers[Register(1)], 30);
}

#[test]
fn jmp_to_last_instruction() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(5)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::JMP,
            operand1: Some(imm(3)),
            operand2: None,
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
            operand1: Some(reg(0)),
            operand2: Some(imm(99)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 99);
    assert_eq!(vm.registers[Register(1)], 0);
}

#[test]
fn jmp_to_program_end_terminates() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(7)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::JMP,
            operand1: Some(imm(3)),
            operand2: None,
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(0)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 7);
}

#[test]
fn jmp_forward_past_several_instructions() {
    let vm = run(vec![
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(0)),
            operand2: Some(imm(1)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::JMP,
            operand1: Some(imm(5)),
            operand2: None,
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
            operand2: Some(imm(3)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(3)),
            operand2: Some(imm(4)),
            operand3: None,
        },
        Instruction {
            opcode: OperationCode::MOVE,
            operand1: Some(reg(4)),
            operand2: Some(imm(5)),
            operand3: None,
        },
    ]);
    assert_eq!(vm.registers[Register(0)], 1);
    assert_eq!(vm.registers[Register(1)], 0);
    assert_eq!(vm.registers[Register(2)], 0);
    assert_eq!(vm.registers[Register(3)], 0);
    assert_eq!(vm.registers[Register(4)], 5);
}
