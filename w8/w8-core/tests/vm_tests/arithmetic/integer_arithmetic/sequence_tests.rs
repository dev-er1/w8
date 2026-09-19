// Test for integer arithmetic.
use w8_core::isa::{instruction::Instruction, opcode::OperationCode::*, register::Register};

use crate::vm_tests::helpers::*;

#[test]
fn integer_arithmetic_sequence() {
    let vm = run(vec![
        // R0 = 10 + 20 = 30
        Instruction {
            opcode: IADD,
            operand1: Some(reg(0)),
            operand2: Some(imm(10)),
            operand3: Some(imm(20)),
        },
        // R1 = R0 * 3 = 90
        Instruction {
            opcode: IMUL,
            operand1: Some(reg(1)),
            operand2: Some(reg(0)),
            operand3: Some(imm(3)),
        },
        // R2 = R1 - 40 = 50
        Instruction {
            opcode: ISUB,
            operand1: Some(reg(2)),
            operand2: Some(reg(1)),
            operand3: Some(imm(40)),
        },
        // R3 = R2 / 7 = 7
        Instruction {
            opcode: UDIV,
            operand1: Some(reg(3)),
            operand2: Some(reg(2)),
            operand3: Some(imm(7)),
        },
        // R4 = R2 % 7 = 1
        Instruction {
            opcode: UREM,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(imm(7)),
        },
        Instruction {
            opcode: NOP,
            operand1: None,
            operand2: None,
            operand3: None,
        },
    ]);

    assert_eq!(vm.registers[Register(0)], 30);
    assert_eq!(vm.registers[Register(1)], 90);
    assert_eq!(vm.registers[Register(2)], 50);
    assert_eq!(vm.registers[Register(3)], 7);
    assert_eq!(vm.registers[Register(4)], 1);
}
