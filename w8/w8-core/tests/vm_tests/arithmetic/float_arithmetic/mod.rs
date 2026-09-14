// Tests for floating-point arithmetic.
pub mod fadd_tests;
pub mod fdiv_tests;
pub mod fmul_tests;
pub mod frem_tests;
pub mod fsub_tests;
pub mod sequence_tests;

use w8_core::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{ExecuteVariant, WVM},
};

// Helper function for testing.
pub fn get_result(opcode: OperationCode, a: f64, b: f64) -> f64 {
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
    f64::from_bits(w8.registers[Register(0)])
}
