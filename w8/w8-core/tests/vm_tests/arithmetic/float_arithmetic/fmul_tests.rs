// Tests for `FMUL`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::float_arithmetic::get_result;

#[test]
fn fmul_2_5_by_4() {
    assert_eq!(get_result(OperationCode::FMUL, 2.5, 4.0), 10.0);
}

#[test]
fn fmul_infinity_times_zero_is_nan() {
    assert!(get_result(OperationCode::FMUL, f64::INFINITY, 0.0).is_nan());
}

#[test]
fn fmul_by_zero() {
    assert_eq!(get_result(OperationCode::FMUL, 123.0, 0.0), 0.0);
    assert_eq!(get_result(OperationCode::FMUL, 0.0, 456.0), 0.0);
}

#[test]
fn fmul_negative() {
    assert_eq!(get_result(OperationCode::FMUL, -2.0, 3.5), -7.0);
}
