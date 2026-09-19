// Tests for code generation errors.
use wdasm::codegen::err::CodegenErrorKind;
use wdasm::position::Position;

use super::*;

#[test]
fn duplicate_label_is_reported() {
    let err = codegen("a:\nNOP\na:").expect_err("duplicate label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::DuplicateLabel { ref name } if name == "a"
    ));
    // The error points to the second label declaration.
    assert_eq!(err.position, Position::new(7, 9));
}

#[test]
fn undefined_label_is_reported() {
    let err = codegen("JMP nowhere").expect_err("undefined label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "nowhere"
    ));
    // The error points to the instruction with the reference.
    assert_eq!(err.position, Position::new(0, 3));
}

#[test]
fn first_undefined_label_wins() {
    let err = codegen("JMP a\nJMP b").expect_err("undefined labels must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "a"
    ));
}

#[test]
fn error_message_contains_label_name() {
    let err = codegen("JMP nowhere").expect_err("undefined label must fail");

    assert!(err.to_string().contains("nowhere"));
    assert!(err.to_string().contains("undefined label"));
}

#[test]
fn duplicate_label_message_contains_label_name() {
    let err = codegen("x:\nx:").expect_err("duplicate label must fail");

    assert!(err.to_string().contains("x"));
    assert!(err.to_string().contains("duplicate label"));
}

#[test]
fn duplicate_label_is_checked_before_undefined_reference() {
    // The duplicate is detected in the first pass and overrides
    // the undefined reference in the second.
    let err = codegen("a:\nJMP b\na:").expect_err("duplicate label must fail");

    assert!(matches!(err.kind, CodegenErrorKind::DuplicateLabel { .. }));
}

#[test]
fn labels_are_case_sensitive() {
    // “Loop” and “loop” are different identifiers.
    let err = codegen("JMP Loop\nloop:\nNOP").expect_err("case mismatch must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "Loop"
    ));
}

#[test]
fn undefined_label_in_second_operand() {
    let err = codegen("JZ R0, nowhere").expect_err("undefined label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "nowhere"
    ));
    // The error points to the instruction, not the operand.
    assert_eq!(err.position, Position::new(0, 2));
}

#[test]
fn undefined_label_in_third_operand() {
    let err = codegen("IADD R0, R1, nowhere").expect_err("undefined label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "nowhere"
    ));
    assert_eq!(err.position, Position::new(0, 4));
}

#[test]
fn undefined_reference_when_program_has_no_instructions() {
    let err = codegen("JMP nowhere\nend:").expect_err("undefined label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "nowhere"
    ));
}

#[test]
fn duplicate_label_on_adjacent_lines_is_reported() {
    let err = codegen("x:\nx:\nMOVE R0, 1").expect_err("duplicate label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::DuplicateLabel { ref name } if name == "x"
    ));
    // The position is the second declaration (after the first and the newline).
    assert_eq!(err.position, Position::new(3, 5));
}

#[test]
fn error_on_duplicate_label_then_reference_to_undefined_keeps_first_position() {
    // The duplicate-label error in the first pass has the position of the
    // second declaration, even if an undefined reference follows.
    let err = codegen("a:\nJMP b\na:\nJMP c").expect_err("duplicate label must fail");

    assert!(matches!(err.kind, CodegenErrorKind::DuplicateLabel { .. }));
    assert_eq!(err.position, Position::new(9, 11));
}

#[test]
fn local_label_outside_any_scope_is_an_error() {
    let err = codegen("*loop:\nNOP").expect_err("a scoped label needs a scope");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::LocalLabelWithoutScope { ref name } if name == "*loop"
    ));
    // The error points to the local label declaration.
    assert_eq!(err.position, Position::new(0, 6));
}

#[test]
fn duplicate_local_label_in_the_same_scope_is_an_error() {
    let err = codegen("main:\n*x:\nNOP\n*x:").expect_err("duplicate local label must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::DuplicateLabel { ref name } if name == "*x"
    ));
}

#[test]
fn local_label_error_message_uses_the_star_prefix() {
    let err = codegen("main:\nJMP *nowhere").expect_err("undefined local label must fail");

    assert!(err.to_string().contains("*nowhere"));
}
