// Tests for `AND`.
use w8_core::{isa::opcode::OperationCode, vm::ExecuteVariant};

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn and_basic() {
    assert_eq!(get_result(OperationCode::AND, 0xFF00, 0x0FF0), 0x0F00);
}

#[test]
fn and_with_zero() {
    assert_eq!(get_result(OperationCode::AND, 0xFFFF, 0), 0);
}

#[test]
fn and_with_max() {
    assert_eq!(get_result(OperationCode::AND, 0xABCD, u64::MAX), 0xABCD);
}

#[test]
fn and_no_common_bits() {
    assert_eq!(get_result(OperationCode::AND, 0xF0F0, 0x0F0F), 0);
}

#[test]
fn and_large_values() {
    assert_eq!(
        get_result(
            OperationCode::AND,
            0xDEAD_BEEF_CAFE_BABE,
            0xFFFF_FFFF_0000_0000
        ),
        0xDEAD_BEEF_0000_0000
    );
}

#[test]
fn and_self() {
    let v = 0x1234_5678_9ABC_DEF0;
    assert_eq!(get_result(OperationCode::AND, v, v), v);
}

#[test]
fn and_zero_and_max() {
    assert_eq!(get_result(OperationCode::AND, 0, u64::MAX), 0);
}

#[test]
fn and_all_bits_set() {
    assert_eq!(get_result(OperationCode::AND, u64::MAX, u64::MAX), u64::MAX);
}

#[test]
fn and_register_sources() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 0xFF00;
    w8.registers[w8_core::isa::register::Register(2)] = 0x0FF0;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::AND,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 0x0F00);
}
