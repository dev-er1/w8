// Tests for `LSL`.
use w8_core::{
    isa::{opcode::OperationCode, register::Register},
    vm::{ExecuteVariant, WVM},
};

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn lsl_basic() {
    assert_eq!(get_result(OperationCode::LSL, 1, 4), 16);
}

#[test]
fn lsl_by_zero() {
    assert_eq!(get_result(OperationCode::LSL, 0x1234, 0), 0x1234);
}

#[test]
fn lsl_zero() {
    assert_eq!(get_result(OperationCode::LSL, 0, 10), 0);
}

#[test]
fn lsl_to_msb() {
    assert_eq!(get_result(OperationCode::LSL, 1, 63), 1u64 << 63);
}

#[test]
fn lsl_wrap_around() {
    assert_eq!(get_result(OperationCode::LSL, 1, 64), 1);
}

#[test]
fn lsl_wrap_large() {
    assert_eq!(get_result(OperationCode::LSL, 1, 128), 1);
}

#[test]
fn lsl_lose_bits() {
    assert_eq!(
        get_result(OperationCode::LSL, 0xFFFF_FFFF_FFFF_FFFF, 4),
        0xFFFF_FFFF_FFFF_FFF0
    );
}

#[test]
fn lsl_max_shift_255() {
    assert_eq!(get_result(OperationCode::LSL, 1, 255), 1 << 63);
}

#[test]
fn lsl_registers() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.registers[Register(1)] = 3;
    w8.registers[Register(2)] = 5;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::LSL,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(0)], 96);
}

#[test]
fn lsl_shift_by_register() {
    let mut w8 = WVM::new(0, ExecuteVariant::default());
    w8.registers[Register(0)] = 1;
    w8.registers[Register(1)] = 63;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::LSL,
        operand1: Some(crate::vm_tests::helpers::reg(2)),
        operand2: Some(crate::vm_tests::helpers::reg(0)),
        operand3: Some(crate::vm_tests::helpers::reg(1)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[Register(2)], 1u64 << 63);
}
