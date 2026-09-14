// Tests for `SREM`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn srem_7_3() {
    assert_eq!(get_result(OperationCode::SREM, 7, 3), 1);
}

#[test]
fn srem_divisible() {
    assert_eq!(get_result(OperationCode::SREM, 12, 4), 0);
}

#[test]
fn srem_negative() {
    assert_eq!(
        get_result(OperationCode::SREM, (-7i64) as u64, 3),
        (-1i64) as u64
    );
}

#[test]
fn srem_negative_divisor() {
    assert_eq!(
        get_result(OperationCode::SREM, (-7i64) as u64, (-3i64) as u64),
        (-1i64) as u64
    );
}

#[test]
#[should_panic]
fn srem_by_zero() {
    get_result(OperationCode::SREM, 7, 0);
}
