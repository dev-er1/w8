// Tests for `SLT`, `SLE`, `SGT`, `SGE`.
use w8_core::{isa::opcode::OperationCode, vm::ExecuteVariant};

use crate::vm_tests::arithmetic::compare::get_int;

#[test]
fn slt_negative_less_than_positive() {
    assert_eq!(get_int(OperationCode::SLT, (-5i64) as u64, 3), 1);
}

#[test]
fn slt_equal_values() {
    assert_eq!(get_int(OperationCode::SLT, 10, 10), 0);
}

#[test]
fn slt_positive_greater() {
    assert_eq!(get_int(OperationCode::SLT, 10, 5), 0);
}

#[test]
fn slt_negative_less_than_negative() {
    assert_eq!(
        get_int(OperationCode::SLT, (-10i64) as u64, (-5i64) as u64),
        1
    );
}

#[test]
fn slt_max_signed_is_negative() {
    assert_eq!(get_int(OperationCode::SLT, 0x8000_0000_0000_0000, 1), 1);
}

#[test]
fn sle_less() {
    assert_eq!(get_int(OperationCode::SLE, (-3i64) as u64, 0), 1);
}

#[test]
fn sle_equal() {
    assert_eq!(get_int(OperationCode::SLE, 7, 7), 1);
}

#[test]
fn sle_greater() {
    assert_eq!(get_int(OperationCode::SLE, 5, (-10i64) as u64), 0);
}

#[test]
fn sgt_positive_greater() {
    assert_eq!(get_int(OperationCode::SGT, 100, 1), 1);
}

#[test]
fn sgt_negative_less() {
    assert_eq!(
        get_int(OperationCode::SGT, (-5i64) as u64, (-10i64) as u64),
        1
    );
}

#[test]
fn sgt_equal() {
    assert_eq!(get_int(OperationCode::SGT, 5, 5), 0);
}

#[test]
fn sge_greater() {
    assert_eq!(get_int(OperationCode::SGE, 10, 5), 1);
}

#[test]
fn sge_equal() {
    assert_eq!(get_int(OperationCode::SGE, 10, 10), 1);
}

#[test]
fn sge_less() {
    assert_eq!(get_int(OperationCode::SGE, (-10i64) as u64, 0), 0);
}

#[test]
fn sge_negative_positive() {
    assert_eq!(get_int(OperationCode::SGE, (-1i64) as u64, 1), 0);
}

#[test]
fn signed_register_sources() {
    let mut w8 = w8_core::vm::WVM::new(0, ExecuteVariant::default());
    w8.registers[w8_core::isa::register::Register(1)] = (-5i64) as u64;
    w8.registers[w8_core::isa::register::Register(2)] = 3;
    w8.program = vec![w8_core::isa::instruction::Instruction {
        opcode: OperationCode::SLT,
        operand1: Some(crate::vm_tests::helpers::reg(0)),
        operand2: Some(crate::vm_tests::helpers::reg(1)),
        operand3: Some(crate::vm_tests::helpers::reg(2)),
    }];
    w8.interpretate().expect("execution failed");
    assert_eq!(w8.registers[w8_core::isa::register::Register(0)], 1);
}
