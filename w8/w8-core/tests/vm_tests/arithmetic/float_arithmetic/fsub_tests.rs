// Tests for `FSUB`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::float_arithmetic::get_result;

#[test]
fn fsub_5_5_2_5() {
    assert_eq!(get_result(OperationCode::FSUB, 5.5, 2.5), 3.0);
}

#[test]
fn fsub_infinite_subtraction_is_nan() {
    assert!(get_result(OperationCode::FSUB, f64::INFINITY, f64::INFINITY).is_nan());
}

#[test]
fn fsub_negative() {
    assert_eq!(get_result(OperationCode::FSUB, -2.0, -3.5), 1.5);
}
