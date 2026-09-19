// Tests for integer arithmetic.
pub mod iadd_tests;
pub mod imul_tests;
pub mod isub_tests;
pub mod sdiv_tests;
pub mod sequence_tests;
pub mod srem_tests;
pub mod udiv_tests;
pub mod urem_tests;

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
pub fn get_result(opcode: OperationCode, a: u64, b: u64) -> u64 {
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
    w8.interpretate().expect("d");
    w8.registers[Register(0)]
}
