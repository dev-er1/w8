// Tests for `UREM`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn urem_7_3() {
    assert_eq!(get_result(OperationCode::UREM, 7, 3), 1);
}

#[test]
fn urem_divisible() {
    assert_eq!(get_result(OperationCode::UREM, 12, 4), 0);
}

#[test]
fn urem_less_than_divisor() {
    assert_eq!(get_result(OperationCode::UREM, 2, 5), 2);
}

#[test]
fn urem_max() {
    assert_eq!(get_result(OperationCode::UREM, u64::MAX, 2), u64::MAX % 2);
}

#[test]
#[should_panic]
fn urem_by_zero() {
    get_result(OperationCode::UREM, 7, 0);
}
