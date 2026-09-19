// Tests for directive recognition: a well-formed directive becomes a
// single `Directive` token, a malformed or unknown one is split into
// a dot and ordinary tokens.
use w8_core::isa::register::Register;
use wdasm::lexer::directive::{DefineValue, Directive};
use wdasm::lexer::err::LexerErrorKind;

use super::*;

#[test]
fn align_directive_becomes_a_single_token() {
    let (tokens, errors) = tokenize(".align 8");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Align { bytes: 8 })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn directive_names_are_case_insensitive() {
    let (tokens, errors) = tokenize(".ALIGN 4 .MinVersion 0.2.0 .DEFINE X, 5");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Align { bytes: 4 })
    ));
    assert!(matches!(
        tokens[1].kind,
        TokenKind::Directive(Directive::MinVersion {
            major: 0,
            minor: 2,
            patch: 0
        })
    ));
    assert!(matches!(
        tokens[2].kind,
        TokenKind::Directive(Directive::Define { .. })
    ));
    assert!(matches!(tokens[3].kind, TokenKind::End));
}

#[test]
fn align_accepts_a_plus_sign() {
    let (tokens, errors) = tokenize(".align +8");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Align { bytes: 8 })
    ));
}

#[test]
fn zero_align_reaches_the_parser_as_a_directive_token() {
    let (tokens, errors) = tokenize(".align 0");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Align { bytes: 0 })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn negative_align_is_not_a_directive_token() {
    let (tokens, errors) = tokenize(".align -4");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Integer(-4)));
    assert!(matches!(tokens[3].kind, TokenKind::End));
}

#[test]
fn minversion_operand_is_a_version_literal() {
    let (tokens, errors) = tokenize(".minversion 1.2.3");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::MinVersion {
            major: 1,
            minor: 2,
            patch: 3
        })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn minversion_with_part_out_of_u16_range_is_not_a_directive_token() {
    let (tokens, errors) = tokenize(".minversion 0.0.65536");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(
        tokens[2].kind,
        TokenKind::Version {
            major: 0,
            minor: 0,
            patch: 65536
        }
    ));
    assert!(matches!(tokens[3].kind, TokenKind::End));
}

#[test]
fn minversion_with_an_overflowing_part_is_reported() {
    let (tokens, errors) = tokenize(".minversion 0.4.99999999999999999999");

    assert!(!errors.is_empty());
    assert!(matches!(errors[0].kind, LexerErrorKind::InvalidVersion(_)));
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
}

#[test]
fn define_with_integer_value() {
    let (tokens, errors) = tokenize(".define FOO, 42");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Define {
            value: DefineValue::Number(42),
            ..
        })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn define_with_float_value_keeps_the_bit_pattern() {
    let (tokens, errors) = tokenize(".define HALF, 0.5");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Define {
            value: DefineValue::Number(v),
            ..
        }) if v == 0.5f64.to_bits()
    ));
}

#[test]
fn define_with_negative_integer_wraps() {
    let (tokens, errors) = tokenize(".define NEG, -1");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Define {
            value: DefineValue::Number(u64::MAX),
            ..
        })
    ));
}

#[test]
fn define_with_a_register_value() {
    let (tokens, errors) = tokenize(".define RAX, R0");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Define {
            value: DefineValue::Register(Register(0)),
            ..
        })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn define_register_value_is_case_insensitive() {
    let (tokens, errors) = tokenize(".define SP, r1");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::Define {
            value: DefineValue::Register(Register(1)),
            ..
        })
    ));
}

#[test]
fn ldefine_with_a_register_value() {
    let (tokens, errors) = tokenize(".ldefine TMP, R5");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::LDefine {
            name: _,
            value: DefineValue::Register(Register(5)),
        })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn ldefine_with_an_integer_value() {
    let (tokens, errors) = tokenize(".ldefine WIDTH, 800");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::LDefine {
            value: DefineValue::Number(800),
            ..
        })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn ldefine_name_is_case_insensitive() {
    let (tokens, errors) = tokenize(".LDefine TMP, R5");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Directive(Directive::LDefine { .. })
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn directive_position_covers_the_whole_directive() {
    let source = SourceCode::new(".align 8".to_string());
    let mut str_pool = StrPool::from_source(&source);
    let mut lexer = Lexer::new(source, &mut str_pool);
    lexer.tokenize();

    assert!(lexer.errors.is_empty());
    assert_eq!(lexer.tokens[0].position.start, 0);
    assert_eq!(lexer.tokens[0].position.end, 8);
}

#[test]
fn define_name_is_interned_in_str_pool() {
    let source = SourceCode::new(".define FOO, 42".to_string());
    let mut str_pool = StrPool::from_source(&source);
    let mut lexer = Lexer::new(source, &mut str_pool);
    lexer.tokenize();

    assert!(lexer.errors.is_empty());
    match &lexer.tokens[0].kind {
        TokenKind::Directive(Directive::Define { name, value }) => {
            assert!(matches!(*value, DefineValue::Number(42)));
            assert_eq!(str_pool.get(*name), "FOO");
        }
        _ => panic!("expected a Define directive"),
    }
}

#[test]
fn unknown_directive_is_dot_and_ident() {
    let (tokens, errors) = tokenize(".section text");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[3].kind, TokenKind::End));
}

#[test]
fn known_directive_with_missing_operand_is_dot_and_ident() {
    for src in [".align", ".minversion", ".define"] {
        let (tokens, errors) = tokenize(src);

        assert!(errors.is_empty(), "`{src}` must not produce a lexer error");
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
        assert!(matches!(tokens[2].kind, TokenKind::End));
    }
}

#[test]
fn known_directive_with_wrong_operand_is_dot_and_ident() {
    let (tokens, errors) = tokenize(".align foo .minversion 5 .define X, foo");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[3].kind, TokenKind::Dot));
    assert!(matches!(tokens[4].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[5].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[6].kind, TokenKind::Dot));
    assert!(matches!(tokens[7].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[8].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[9].kind, TokenKind::Comma));
    assert!(matches!(tokens[10].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[11].kind, TokenKind::End));
}

#[test]
fn dot_without_a_name_is_just_a_dot() {
    let (tokens, errors) = tokenize(". .align8");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Dot));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[3].kind, TokenKind::End));
}

#[test]
fn define_name_must_be_an_identifier() {
    let (tokens, errors) = tokenize(".define R0, 5 .define MOVE, 5");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Register(_)));
    assert!(matches!(tokens[3].kind, TokenKind::Comma));
    assert!(matches!(tokens[4].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[5].kind, TokenKind::Dot));
    assert!(matches!(tokens[6].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[7].kind, TokenKind::Mnemonic(_)));
    assert!(matches!(tokens[8].kind, TokenKind::Comma));
    assert!(matches!(tokens[9].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[10].kind, TokenKind::End));
}
