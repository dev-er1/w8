// Tests for comparison operations.
pub mod equality_tests;
pub mod float_equality_tests;
pub mod float_ordered_tests;
pub mod sequence_tests;
pub mod signed_compare_tests;
pub mod unsigned_compare_tests;

use w8_core::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{ExecuteVariant, WVM},
};

pub fn get_int(opcode: OperationCode, a: u64, b: u64) -> u64 {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.program = vec![Instruction {
        opcode,
        operand1: Some(Operand {
            kind: OperandKind::Register(Register(0)),
        }),
        operand2: Some(Operand {
            kind: OperandKind::Immediate(a),
        }),
        operand3: Some(Operand {
            kind: OperandKind::Immediate(b),
        }),
    }];
    w8.interpretate().expect("execution failed");
    w8.registers[Register(0)]
}

pub fn get_float(opcode: OperationCode, a: f64, b: f64) -> u64 {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.program = vec![Instruction {
        opcode,
        operand1: Some(Operand {
            kind: OperandKind::Register(Register(0)),
        }),
        operand2: Some(Operand {
            kind: OperandKind::Immediate(a.to_bits()),
        }),
        operand3: Some(Operand {
            kind: OperandKind::Immediate(b.to_bits()),
        }),
    }];
    w8.interpretate().expect("execution failed");
    w8.registers[Register(0)]
}
