// Tests for `FDIV`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::float_arithmetic::get_result;

#[test]
fn fdiv_9_by_3() {
    assert_eq!(get_result(OperationCode::FDIV, 9.0, 3.0), 3.0);
}

#[test]
fn fdiv_zero_division_is_infinite() {
    assert_eq!(get_result(OperationCode::FDIV, 1.0, 0.0), f64::INFINITY);
}

#[test]
fn fdiv_half() {
    assert_eq!(get_result(OperationCode::FDIV, 1.0, 2.0), 0.5);
}

#[test]
fn fdiv_nan_from_zero_div_zero() {
    assert!(get_result(OperationCode::FDIV, 0.0, 0.0).is_nan());
}

#[test]
fn fdiv_negative() {
    assert_eq!(get_result(OperationCode::FDIV, -6.0, 2.0), -3.0);
}
