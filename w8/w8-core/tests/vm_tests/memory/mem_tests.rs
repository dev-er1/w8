use w8_core::vm::ExecuteVariant;
// Test for VM memory.
use w8_core::{
    isa::{instruction::Instruction, opcode::OperationCode, register::Register},
    vm::{VMCallDecision, WVM},
};

use crate::vm_tests::helpers::{imm, reg, run_on_with_dispatch};

#[test]
fn nothing_after_vmcall_exit() {
    let vm = run_on_with_dispatch(
        WVM::new(0, ExecuteVariant::default()),
        vec![
            Instruction {
                opcode: OperationCode::MOVE,
                operand1: Some(reg(0)),
                operand2: Some(imm(67)),
                operand3: None,
            },
            Instruction {
                opcode: OperationCode::VMCALL,
                operand1: Some(imm(0)),
                operand2: Some(imm(0)),
                operand3: Some(imm(0)),
            },
            Instruction {
                opcode: OperationCode::IADD,
                operand1: Some(reg(0)),
                operand2: Some(reg(0)),
                operand3: Some(imm(1)),
            },
        ],
        |_, service, _, _| {
            if service == 0 {
                VMCallDecision::Exit { code: 0 }
            } else {
                VMCallDecision::Continue
            }
        },
    );

    assert_eq!(vm.registers[Register(0)], 67);
    assert_eq!(vm.exit_code, Some(0));
}
