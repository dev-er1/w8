// Tests for a floating-point arithmetic sequence.
use w8_core::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode::*,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{ExecuteVariant, WVM},
};

fn reg(r: u8) -> Operand {
    Operand {
        kind: OperandKind::Register(Register(r)),
    }
}

fn imm_f(value: f64) -> Operand {
    Operand {
        kind: OperandKind::Immediate(value.to_bits()),
    }
}

#[test]
fn float_arithmetic_sequence() {
    let mut vm = WVM::new(0, ExecuteVariant::default());

    vm.program = vec![
        Instruction {
            opcode: FADD,
            operand1: Some(reg(0)),
            operand2: Some(imm_f(1.5)),
            operand3: Some(imm_f(2.5)),
        },
        Instruction {
            opcode: FMUL,
            operand1: Some(reg(1)),
            operand2: Some(reg(0)),
            operand3: Some(imm_f(2.0)),
        },
        Instruction {
            opcode: FSUB,
            operand1: Some(reg(2)),
            operand2: Some(reg(1)),
            operand3: Some(imm_f(1.0)),
        },
        Instruction {
            opcode: FDIV,
            operand1: Some(reg(3)),
            operand2: Some(reg(2)),
            operand3: Some(imm_f(2.0)),
        },
        Instruction {
            opcode: FREM,
            operand1: Some(reg(4)),
            operand2: Some(reg(2)),
            operand3: Some(imm_f(2.5)),
        },
    ];

    vm.interpretate().expect("execution failed");

    assert_eq!(f64::from_bits(vm.registers[Register(0)]), 4.0);
    assert_eq!(f64::from_bits(vm.registers[Register(1)]), 8.0);
    assert_eq!(f64::from_bits(vm.registers[Register(2)]), 7.0);
    assert_eq!(f64::from_bits(vm.registers[Register(3)]), 3.5);
    assert_eq!(f64::from_bits(vm.registers[Register(4)]), 2.0);
}
