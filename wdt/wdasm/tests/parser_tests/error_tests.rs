// Tests for parser errors and recovery after them.
use wdasm::parser::ast::Statement;
use wdasm::parser::err::ParserErrorKind;

use super::*;

#[test]
fn label_without_colon_is_an_error() {
    let (ast, errors) = parse("foo\nMOVE R0, 1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedLabelColon
    ));
    // The line is skipped, but parsing continues: the instruction below was parsed.
    assert_eq!(ast.program.len(), 1);
}

#[test]
fn local_label_without_colon_is_an_error() {
    let (ast, errors) = parse("main:\n*loop\nMOVE R0, 1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedLabelColon
    ));
    assert_eq!(ast.program.len(), 2);
}

#[test]
fn missing_comma_between_operands() {
    let (ast, errors) = parse("MOVE R0 R1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0].kind, ParserErrorKind::ExpectedComma));

    // The operands were parsed despite the error.
    assert_eq!(ast.program.len(), 1);
    assert!(matches!(ast.program[0], Statement::Instruction { .. }));
}

#[test]
fn too_few_operands() {
    let (ast, errors) = parse("MOVE R0");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::IncorrectNumberOfOperands {
            expected: 2,
            got: 1,
        }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn too_many_operands() {
    let (ast, errors) = parse("NOP R0");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::IncorrectNumberOfOperands {
            expected: 0,
            got: 1,
        }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn destination_must_be_a_register() {
    let (ast, errors) = parse("MOVE 42, R1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedRegisterOperand { .. }
    ));

    assert_eq!(ast.program.len(), 1);
}

#[test]
fn destination_label_is_rejected() {
    let (ast, errors) = parse("MOVE loop, R1\nloop:");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedRegisterOperand { .. }
    ));
    assert_eq!(ast.program.len(), 2);
}

#[test]
fn trailing_tokens_after_instruction() {
    let (ast, errors) = parse("MOVE R0, R1 extra");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::IncorrectNumberOfOperands {
            expected: 2,
            got: 3,
        }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn unexpected_token_at_statement_start() {
    let (ast, errors) = parse(": MOVE R0, R1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::UnexpectedToken {
            expected: "a label, an instruction, or a directive",
            ..
        }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn error_positions_point_to_the_culprit() {
    let (_, errors) = parse("MOVE R0\n");

    assert_eq!(errors.len(), 1);

    assert_eq!(errors[0].position.start, 0);
    assert_eq!(errors[0].position.end, 4);
}

#[test]
fn recovery_continues_after_an_error() {
    let (ast, errors) = parse("MOVE R0\nNOP\nRET");

    assert_eq!(errors.len(), 1);

    assert_eq!(ast.program.len(), 2);
    assert!(matches!(ast.program[0], Statement::Instruction { .. }));
    assert!(matches!(ast.program[1], Statement::Instruction { .. }));
}

#[test]
fn several_errors_are_collected() {
    let (ast, errors) = parse("MOVE R0\nMOVE\nMOVE R1, 2");

    assert_eq!(errors.len(), 2);
    assert_eq!(ast.program.len(), 1);
}

#[test]
fn align_without_operand_is_an_error() {
    let (ast, errors) = parse(".align\nNOP");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedDirectiveOperand
    ));

    assert_eq!(ast.program.len(), 1);
}

#[test]
fn align_with_non_positive_value_is_an_error() {
    let (ast, errors) = parse(".align 0");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::InvalidAlignBytes { value: 0 }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn align_with_negative_value_is_an_error() {
    let (ast, errors) = parse(".align -4");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::InvalidAlignBytes { value: -4 }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn align_with_non_integer_operand_is_an_error() {
    let (ast, errors) = parse(".align foo");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::UnexpectedDirectiveOperand { .. }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn unknown_directive_is_an_error() {
    let (ast, errors) = parse(".section text");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::UnknownDirective { .. }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn dot_without_directive_name_is_an_error() {
    let (ast, errors) = parse(". MOVE R0, 1");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedDirectiveName
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn trailing_tokens_after_directive() {
    let (ast, errors) = parse(".align 4 8");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::UnexpectedToken {
            expected: "end of statement",
            ..
        }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn minversion_without_version_is_an_error() {
    let (ast, errors) = parse(".minversion\nNOP");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::ExpectedDirectiveOperand
    ));

    assert_eq!(ast.program.len(), 1);
}

#[test]
fn minversion_with_non_version_operand_is_an_error() {
    let (ast, errors) = parse(".minversion 5");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::UnexpectedDirectiveOperand { .. }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn minversion_with_part_out_of_u16_range_is_an_error() {
    let (ast, errors) = parse(".minversion 0.0.65536");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::MinVersionOutOfRange { .. }
    ));
    assert!(ast.program.is_empty());
}

#[test]
fn minversion_newer_than_current_w8_is_an_error() {
    let (ast, errors) = parse(".minversion 99.0.0");

    assert_eq!(errors.len(), 1);
    assert!(matches!(
        errors[0].kind,
        ParserErrorKind::MinVersionTooNew { .. }
    ));
    assert!(ast.program.is_empty());
}
