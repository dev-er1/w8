use w8_core::vm::ExecuteVariant;
// Tests for `XOR`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn xor_basic() {
    assert_eq!(get_result(OperationCode::XOR, 0xFF00, 0x0FF0), 0xF0F0);
}

#[test]
fn xor_with_zero() {
    assert_eq!(get_result(OperationCode::XOR, 0xABCD, 0), 0xABCD);
}

#[test]
fn xor_with_self() {
    assert_eq!(get_result(OperationCode::XOR, 0x1234_5678, 0x1234_5678), 0);
}

#[test]
fn xor_with_max() {
    assert_eq!(
        get_result(OperationCode::XOR, 0xFFFF_0000_FFFF_0000, u64::MAX),
        0x0000_FFFF_0000_FFFF
    );
}

#[test]
fn xor_large_values() {
    assert_eq!(
        get_result(
            OperationCode::XOR,
            0xDEAD_BEEF_CAFE_BABE,
            0xFFFF_FFFF_FFFF_FFFF
        ),
        0xDEAD_BEEF_CAFE_BABE ^ u64::MAX
    );
}

#[test]
fn xor_toggle_bits() {
    let v = 0x0F0F_0F0F_0F0F_0F0F;
    let m = 0xFF00_FF00_FF00_FF00;
    assert_eq!(get_result(OperationCode::XOR, v, m), 0xF00F_F00F_F00F_F00F);
}

#[test]
fn xor_register_sources() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 0xFF00;
    w8.registers[w8_core::isa::register::Register(2)] = 0x0FF0;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::XOR,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 0xF0F0);
}
