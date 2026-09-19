// Tests for `IMUL`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn imul_2_2() {
    assert_eq!(get_result(OperationCode::IMUL, 2, 2), 4);
}

#[test]
fn imul_by_zero() {
    assert_eq!(get_result(OperationCode::IMUL, 123, 0), 0);
    assert_eq!(get_result(OperationCode::IMUL, 0, 123), 0);
}

#[test]
fn imul_wrap() {
    let a = u64::MAX;
    let b = 2;

    assert_eq!(get_result(OperationCode::IMUL, a, b), a.wrapping_mul(b));
}

#[test]
fn imul_wrap_max() {
    assert_eq!(
        get_result(OperationCode::IMUL, u64::MAX, u64::MAX),
        u64::MAX.wrapping_mul(u64::MAX)
    );
}
