// Tests for `FADD`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::float_arithmetic::get_result;

#[test]
fn fadd_1_5_and_2_25() {
    assert_eq!(get_result(OperationCode::FADD, 1.5, 2.25), 3.75);
}

#[test]
fn fadd_infinity() {
    assert_eq!(
        get_result(OperationCode::FADD, f64::INFINITY, 1.0),
        f64::INFINITY
    );
}

#[test]
fn fadd_negative() {
    assert_eq!(get_result(OperationCode::FADD, -1.5, 2.0), 0.5);
}
