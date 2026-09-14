use w8_core::vm::ExecuteVariant;
// Tests for `SAR`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn sar_positive() {
    assert_eq!(get_result(OperationCode::SAR, 64, 3), 8);
}

#[test]
fn sar_by_zero() {
    assert_eq!(get_result(OperationCode::SAR, 0x1234, 0), 0x1234);
}

#[test]
fn sar_zero() {
    assert_eq!(get_result(OperationCode::SAR, 0, 10), 0);
}

#[test]
fn sar_negative_sign_extend() {
    assert_eq!(
        get_result(OperationCode::SAR, 0x8000_0000_0000_0000, 63),
        u64::MAX
    );
}

#[test]
fn sar_negative_shift_one() {
    assert_eq!(
        get_result(OperationCode::SAR, 0x8000_0000_0000_0000, 1),
        0xC000_0000_0000_0000
    );
}

#[test]
fn sar_negative_shift_all() {
    assert_eq!(
        get_result(OperationCode::SAR, 0xFFFF_FFFF_FFFF_FFFF, 4),
        0xFFFF_FFFF_FFFF_FFFF
    );
}

#[test]
fn sar_positive_does_not_extend() {
    assert_eq!(get_result(OperationCode::SAR, 0x4000_0000_0000_0000, 62), 1);
}

#[test]
fn sar_wrap_around() {
    assert_eq!(get_result(OperationCode::SAR, 1, 64), 1);
}

#[test]
fn sar_negative_small_shift() {
    assert_eq!(
        get_result(OperationCode::SAR, 0xFFFF_FFFF_FFFF_FF00, 8),
        0xFFFF_FFFF_FFFF_FFFF
    );
}

#[test]
fn sar_mixed_register_immediate() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 0x8000_0000_0000_0000;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::SAR,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::imm(63)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], u64::MAX);
}
