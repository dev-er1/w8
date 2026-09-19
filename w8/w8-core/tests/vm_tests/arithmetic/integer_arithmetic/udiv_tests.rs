// Tests for `UDIV`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn udiv_8_2() {
    assert_eq!(get_result(OperationCode::UDIV, 8, 2), 4);
}

#[test]
fn udiv_by_one() {
    assert_eq!(get_result(OperationCode::UDIV, 12345, 1), 12345);
}

#[test]
fn udiv_truncate() {
    assert_eq!(get_result(OperationCode::UDIV, 7, 2), 3);
}

#[test]
fn udiv_max() {
    assert_eq!(get_result(OperationCode::UDIV, u64::MAX, 2), u64::MAX / 2);
}

#[test]
#[should_panic]
fn udiv_by_zero() {
    get_result(OperationCode::UDIV, 5, 0);
}
