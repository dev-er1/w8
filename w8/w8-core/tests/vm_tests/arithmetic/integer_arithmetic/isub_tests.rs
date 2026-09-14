// Tests for `ISUB`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn isub_2_2() {
    assert_eq!(get_result(OperationCode::ISUB, 2, 2), 0);
}

#[test]
fn isub_underflow() {
    assert_eq!(get_result(OperationCode::ISUB, 0, 1), 0u64.wrapping_sub(1));
}

#[test]
fn isub_wrap_max() {
    assert_eq!(get_result(OperationCode::ISUB, u64::MAX, u64::MAX), 0);
}
