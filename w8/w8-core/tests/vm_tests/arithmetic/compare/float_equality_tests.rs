// Tests for `FEQ`, `FNE`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::compare::get_float;

#[allow(clippy::approx_constant)]
#[test]
fn feq_equal_values() {
    assert_eq!(get_float(OperationCode::FEQ, 3.14, 3.14), 1);
}

#[test]
fn feq_not_equal() {
    assert_eq!(get_float(OperationCode::FEQ, 1.0, 2.0), 0);
}

#[test]
fn feq_negative_zero_vs_positive_zero() {
    assert_eq!(get_float(OperationCode::FEQ, -0.0, 0.0), 1);
}

#[test]
fn feq_infinity_equal() {
    assert_eq!(
        get_float(OperationCode::FEQ, f64::INFINITY, f64::INFINITY),
        1
    );
}

#[test]
fn feq_nan_not_equal_to_self() {
    assert_eq!(get_float(OperationCode::FEQ, f64::NAN, f64::NAN), 0);
}

#[test]
fn fne_not_equal() {
    assert_eq!(get_float(OperationCode::FNE, 1.0, 2.0), 1);
}

#[test]
fn fne_equal() {
    assert_eq!(get_float(OperationCode::FNE, 42.0, 42.0), 0);
}

#[test]
fn fne_nan() {
    assert_eq!(get_float(OperationCode::FNE, f64::NAN, f64::NAN), 1);
}
