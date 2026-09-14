// Tests for `FLT`, `FLE`, `FGT`, `FGE`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::compare::get_float;

#[test]
fn flt_less() {
    assert_eq!(get_float(OperationCode::FLT, 1.5, 2.5), 1);
}

#[test]
fn flt_equal() {
    assert_eq!(get_float(OperationCode::FLT, 3.0, 3.0), 0);
}

#[test]
fn flt_greater() {
    assert_eq!(get_float(OperationCode::FLT, 10.0, 1.0), 0);
}

#[test]
fn flt_negative_vs_positive() {
    assert_eq!(get_float(OperationCode::FLT, -5.0, 1.0), 1);
}

#[test]
fn flt_nan_returns_false() {
    assert_eq!(get_float(OperationCode::FLT, f64::NAN, 1.0), 0);
    assert_eq!(get_float(OperationCode::FLT, 1.0, f64::NAN), 0);
}

#[test]
fn flt_infinity() {
    assert_eq!(
        get_float(OperationCode::FLT, f64::NEG_INFINITY, f64::INFINITY),
        1
    );
}

#[test]
fn fle_less() {
    assert_eq!(get_float(OperationCode::FLE, 1.0, 2.0), 1);
}

#[test]
fn fle_equal() {
    assert_eq!(get_float(OperationCode::FLE, 5.5, 5.5), 1);
}

#[test]
fn fle_greater() {
    assert_eq!(get_float(OperationCode::FLE, 10.0, 1.0), 0);
}

#[test]
fn fle_nan() {
    assert_eq!(get_float(OperationCode::FLE, f64::NAN, 1.0), 0);
}

#[test]
fn fgt_greater() {
    assert_eq!(get_float(OperationCode::FGT, 5.0, 1.0), 1);
}

#[test]
fn fgt_equal() {
    assert_eq!(get_float(OperationCode::FGT, 2.0, 2.0), 0);
}

#[test]
fn fgt_less() {
    assert_eq!(get_float(OperationCode::FGT, 0.5, 10.0), 0);
}

#[test]
fn fgt_negative() {
    assert_eq!(get_float(OperationCode::FGT, -1.0, -5.0), 1);
}

#[test]
fn fgt_nan() {
    assert_eq!(get_float(OperationCode::FGT, f64::NAN, 1.0), 0);
}

#[test]
fn fge_greater() {
    assert_eq!(get_float(OperationCode::FGE, 3.0, 1.0), 1);
}

#[test]
fn fge_equal() {
    assert_eq!(get_float(OperationCode::FGE, 4.0, 4.0), 1);
}

#[test]
fn fge_less() {
    assert_eq!(get_float(OperationCode::FGE, 0.0, 1.0), 0);
}

#[test]
fn fge_nan() {
    assert_eq!(get_float(OperationCode::FGE, f64::NAN, 1.0), 0);
}
