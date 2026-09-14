// Tests for `FREM`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::float_arithmetic::get_result;

#[test]
fn frem_5_5_mod_2() {
    assert_eq!(get_result(OperationCode::FREM, 5.5, 2.0), 1.5);
}

#[test]
fn frem_division_by_zero_is_nan() {
    assert!(get_result(OperationCode::FREM, 5.5, 0.0).is_nan());
}

#[test]
fn frem_negative() {
    assert_eq!(get_result(OperationCode::FREM, -5.5, 2.0), -1.5);
}

#[test]
fn frem_infinity_is_nan() {
    assert!(get_result(OperationCode::FREM, f64::INFINITY, 1.0).is_nan());
}
