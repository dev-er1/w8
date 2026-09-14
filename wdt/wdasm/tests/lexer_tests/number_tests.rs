use wdasm::lexer::err::LexerErrorKind;

use super::*;

#[test]
fn integer_literals() {
    let (tokens, errors) = tokenize("0 42 -7 +7 007");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(0)));
    assert!(matches!(tokens[1].kind, TokenKind::Integer(42)));
    assert!(matches!(tokens[2].kind, TokenKind::Integer(-7)));
    assert!(matches!(tokens[3].kind, TokenKind::Integer(7)));

    assert!(matches!(tokens[4].kind, TokenKind::Integer(7)));
}

#[test]
fn integer_literal_extremes() {
    let (tokens, errors) = tokenize(&format!("{} {}", i64::MAX, i64::MIN));

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(v) if v == i64::MAX));
    assert!(matches!(tokens[1].kind, TokenKind::Integer(v) if v == i64::MIN));
}

#[test]
fn float_literals() {
    let (tokens, errors) = tokenize("1.5 -2.25 .5 2.25e3 1E-2");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Float(v) if v == 1.5));
    assert!(matches!(tokens[1].kind, TokenKind::Float(v) if v == -2.25));
    assert!(matches!(tokens[2].kind, TokenKind::Float(v) if v == 0.5));
    assert!(matches!(tokens[3].kind, TokenKind::Float(v) if v == 2250.0));
    assert!(matches!(tokens[4].kind, TokenKind::Float(v) if v == 0.01));
}

#[test]
fn dot_without_following_digit_ends_the_number() {
    let (tokens, errors) = tokenize("5. 5.5");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[1].kind, TokenKind::Dot));
    assert!(matches!(tokens[2].kind, TokenKind::Float(v) if v == 5.5));
}

#[test]
fn dot_after_float_starts_a_version() {
    let (tokens, errors) = tokenize("5.5.5");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Version {
            major: 5,
            minor: 5,
            patch: 5
        }
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn version_literals() {
    let (tokens, errors) = tokenize("0.2.0 1.0.0 10.99.3");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Version {
            major: 0,
            minor: 2,
            patch: 0
        }
    ));
    assert!(matches!(
        tokens[1].kind,
        TokenKind::Version {
            major: 1,
            minor: 0,
            patch: 0
        }
    ));
    assert!(matches!(
        tokens[2].kind,
        TokenKind::Version {
            major: 10,
            minor: 99,
            patch: 3
        }
    ));
}

#[test]
fn overflowing_version_part_is_reported() {
    let (tokens, errors) = tokenize("0.4.99999999999999999999");

    assert!(!errors.is_empty());
    assert!(matches!(errors[0].kind, LexerErrorKind::InvalidVersion(_)));
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn version_with_an_empty_part_is_reported() {
    let (tokens, errors) = tokenize(".5.5.5");

    assert!(!errors.is_empty());
    assert!(matches!(errors[0].kind, LexerErrorKind::InvalidVersion(_)));
    assert_eq!(tokens.len(), 1);
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn two_dot_version_keeps_float_for_two_parts() {
    let (tokens, errors) = tokenize("2.5 0.2 .5");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Float(v) if v == 2.5));
    assert!(matches!(tokens[1].kind, TokenKind::Float(v) if v == 0.2));
    assert!(matches!(tokens[2].kind, TokenKind::Float(v) if v == 0.5));
}

#[test]
fn exponent_without_digits_is_not_an_exponent() {
    let (tokens, errors) = tokenize("5e 5e+");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Integer(5)));
    assert!(matches!(tokens[3].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[4].kind, TokenKind::Plus));
}

#[test]
fn sign_followed_by_non_digit_is_punctuation() {
    let (tokens, errors) = tokenize("- +");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Minus));
    assert!(matches!(tokens[1].kind, TokenKind::Plus));
}

#[test]
fn number_adjacent_to_ident_is_split() {
    let (tokens, errors) = tokenize("42x");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(42)));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
}

#[test]
fn signed_zero_is_an_integer() {
    let (tokens, errors) = tokenize("0 -0 +0");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(0)));
    assert!(matches!(tokens[1].kind, TokenKind::Integer(0)));
    assert!(matches!(tokens[2].kind, TokenKind::Integer(0)));
}

#[test]
fn exponent_with_sign() {
    let (tokens, errors) = tokenize("1e+5 1E-3");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Float(v) if v == 100_000.0));
    assert!(matches!(tokens[1].kind, TokenKind::Float(v) if v == 0.001));
}

#[test]
fn leading_zeros_in_floats() {
    let (tokens, errors) = tokenize("007.5 00.25");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Float(v) if v == 7.5));
    assert!(matches!(tokens[1].kind, TokenKind::Float(v) if v == 0.25));
}

#[test]
fn dot_separated_numbers() {
    let (tokens, errors) = tokenize("1..5");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Integer(1)));
    assert!(matches!(tokens[1].kind, TokenKind::Dot));
    assert!(matches!(tokens[2].kind, TokenKind::Float(v) if v == 0.5));
}

#[test]
fn float_with_unfinished_exponent_keeps_integer_part() {
    let (tokens, errors) = tokenize("5.5e 2E");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Float(v) if v == 5.5));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Integer(2)));
    assert!(matches!(tokens[3].kind, TokenKind::Ident(_)));
}

#[test]
fn overflowing_integer_is_reported() {
    let (tokens, errors) = tokenize("9223372036854775808");

    assert_eq!(errors.len(), 1, "the integer literal must overflow `i64`");
    assert!(matches!(errors[0].kind, LexerErrorKind::InvalidInteger(_)));
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn overflowing_float_is_reported() {
    let (tokens, errors) = tokenize("1e400");

    assert_eq!(errors.len(), 1, "the literal must not be finite `f64`");
    assert!(matches!(errors[0].kind, LexerErrorKind::InvalidFloat(_)));
    assert!(matches!(tokens[0].kind, TokenKind::End));
}
