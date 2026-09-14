// Tests for `SDIV`.
use w8_core::isa::opcode::OperationCode;

use crate::vm_tests::arithmetic::integer_arithmetic::get_result;

#[test]
fn sdiv_8_2() {
    assert_eq!(get_result(OperationCode::SDIV, 8, 2), 4);
}

#[test]
fn sdiv_by_one() {
    assert_eq!(get_result(OperationCode::SDIV, 12345, 1), 12345);
}

#[test]
fn sdiv_negative() {
    assert_eq!(
        get_result(OperationCode::SDIV, (-8i64) as u64, 2),
        (-4i64) as u64
    );
}

#[test]
fn sdiv_negative_negative() {
    assert_eq!(
        get_result(OperationCode::SDIV, (-8i64) as u64, (-2i64) as u64),
        4
    );
}

#[test]
#[should_panic]
fn sdiv_by_zero() {
    get_result(OperationCode::SDIV, 5, 0);
}
