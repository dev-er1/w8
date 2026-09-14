// Tests for `IEQ` & `INE`.
use w8_core::{isa::opcode::OperationCode, vm::ExecuteVariant};

use crate::vm_tests::arithmetic::compare::get_int;

#[test]
fn ieq_equal_values() {
    assert_eq!(get_int(OperationCode::IEQ, 42, 42), 1);
}

#[test]
fn ieq_not_equal() {
    assert_eq!(get_int(OperationCode::IEQ, 42, 43), 0);
}

#[test]
fn ieq_zero_and_zero() {
    assert_eq!(get_int(OperationCode::IEQ, 0, 0), 1);
}

#[test]
fn ieq_large_equal() {
    assert_eq!(get_int(OperationCode::IEQ, u64::MAX, u64::MAX), 1);
}

#[test]
fn ieq_register_sources() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = 100;
    w8.registers[w8_core::isa::register::Register(2)] = 100;
    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::IEQ,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];
    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 1);
}

#[test]
fn ine_not_equal() {
    assert_eq!(get_int(OperationCode::INE, 10, 20), 1);
}

#[test]
fn ine_equal() {
    assert_eq!(get_int(OperationCode::INE, 10, 10), 0);
}

#[test]
fn ine_zero_and_max() {
    assert_eq!(get_int(OperationCode::INE, 0, u64::MAX), 1);
}
