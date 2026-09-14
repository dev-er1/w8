// Tests for `IADD`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn iadd_2_2() {
    assert_eq!(get_result(OperationCode::IADD, 2, 2), 4)
}

#[test]
fn iadd_wrap_max_plus_one() {
    assert_eq!(get_result(OperationCode::IADD, u64::MAX, 1), 0);
}

#[test]
fn iadd_wrap_max_plus_max() {
    assert_eq!(
        get_result(OperationCode::IADD, u64::MAX, u64::MAX),
        u64::MAX.wrapping_add(u64::MAX)
    );
}

#[test]
fn iadd_wrap_half_overflow() {
    assert_eq!(
        get_result(OperationCode::IADD, u64::MAX - 5, 10),
        (u64::MAX - 5).wrapping_add(10)
    );
}
