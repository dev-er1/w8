use w8_core::vm::ExecuteVariant;
// Tests for `LSR`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn lsr_basic() {
    assert_eq!(get_result(OperationCode::LSR, 16, 4), 1);
}

#[test]
fn lsr_by_zero() {
    assert_eq!(get_result(OperationCode::LSR, 0x1234, 0), 0x1234);
}

#[test]
fn lsr_zero() {
    assert_eq!(get_result(OperationCode::LSR, 0, 10), 0);
}

#[test]
fn lsr_logical_not_arithmetic() {
    assert_eq!(get_result(OperationCode::LSR, 0x8000_0000_0000_0000, 63), 1);
}

#[test]
fn lsr_lsb_to_zero() {
    assert_eq!(get_result(OperationCode::LSR, 1, 1), 0);
}

#[test]
fn lsr_wrap_around() {
    assert_eq!(get_result(OperationCode::LSR, 1, 64), 1);
}

#[test]
fn lsr_wrap_large() {
    assert_eq!(get_result(OperationCode::LSR, 1, 128), 1);
}

#[test]
fn lsr_lose_low_bits() {
    assert_eq!(
        get_result(OperationCode::LSR, 0xFFFF_FFFF_FFFF_FFFF, 4),
        0x0FFF_FFFF_FFFF_FFFF
    );
}

#[test]
fn lsr_high_bit_becomes_zero() {
    assert_eq!(
        get_result(OperationCode::LSR, 0x8000_0000_0000_0000, 1),
        0x4000_0000_0000_0000
    );
}

#[test]
fn lsr_registers() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 256;
    w8.registers[w8_core::isa::register::Register(2)] = 8;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::LSR,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 1);
}
