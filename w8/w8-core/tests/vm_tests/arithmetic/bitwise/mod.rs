use w8_core::vm::ExecuteVariant;
// Tests for bitwise operations.
pub mod and_tests;
pub mod lsl_tests;
pub mod lsr_tests;
pub mod not_tests;
pub mod or_tests;
pub mod sar_tests;
pub mod sequence_tests;
pub mod xor_tests;

use w8_core::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::WVM,
};

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
    w8.interpretate().expect("execution failed");
    w8.registers[Register(0)]
}

pub fn get_not_result(a: u64) -> u64 {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.program = vec![Instruction {
        opcode: OperationCode::NOT,
        operand1: Some(Operand {
            kind: OperandKind::Register(Register(0)),
        }),
        operand2: Some(Operand {
            kind: OperandKind::Immediate(a),
        }),
        operand3: None,
    }];
    w8.interpretate().expect("execution failed");
    w8.registers[Register(0)]
}
