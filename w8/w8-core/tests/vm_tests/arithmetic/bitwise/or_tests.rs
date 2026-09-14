use w8_core::vm::ExecuteVariant;
// Tests for `OR`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::bitwise::get_result;

#[test]
fn or_basic() {
    assert_eq!(get_result(OperationCode::OR, 0xFF00, 0x0FF0), 0xFFF0);
}

#[test]
fn or_with_zero() {
    assert_eq!(get_result(OperationCode::OR, 0xABCD, 0), 0xABCD);
}

#[test]
fn or_with_max() {
    assert_eq!(get_result(OperationCode::OR, 0x1234, u64::MAX), u64::MAX);
}

#[test]
fn or_sets_all_bits() {
    assert_eq!(get_result(OperationCode::OR, 0xF0F0, 0x0F0F), 0xFFFF);
}

#[test]
fn or_large_values() {
    assert_eq!(
        get_result(
            OperationCode::OR,
            0xDEAD_0000_CAFE_0000,
            0x0000_BEEF_0000_BABE
        ),
        0xDEAD_BEEF_CAFE_BABE
    );
}

#[test]
fn or_self() {
    let v = 0x1234_5678_9ABC_DEF0;
    assert_eq!(get_result(OperationCode::OR, v, v), v);
}

#[test]
fn or_zero_and_max() {
    assert_eq!(get_result(OperationCode::OR, 0, u64::MAX), u64::MAX);
}

#[test]
fn or_register_sources() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 0xFF00;
    w8.registers[w8_core::isa::register::Register(2)] = 0x0FF0;

    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::OR,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];

    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 0xFFF0);
}
