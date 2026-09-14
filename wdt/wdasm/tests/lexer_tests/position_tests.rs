// Tests for token positions in the source code.
use wdasm::lexer::err::LexerErrorKind;
use wdasm::position::Position;

use super::*;

#[test]
fn positions_on_the_first_line() {
    let (tokens, _) = tokenize("MOVE R0, 42");

    assert_eq!(tokens[0].position, Position::new(0, 4));
    assert_eq!(tokens[1].position, Position::new(5, 7));
    assert_eq!(tokens[2].position, Position::new(7, 8));
    assert_eq!(tokens[3].position, Position::new(9, 11));
    assert_eq!(tokens[4].position, Position::new(11, 11));
}

#[test]
fn positions_after_newline_are_byte_offsets() {
    let (tokens, _) = tokenize("MOVE R0, R1\nCALL foo");

    assert_eq!(tokens[4].position, Position::new(11, 12));
    assert_eq!(tokens[5].position, Position::new(12, 16));
    assert_eq!(tokens[6].position, Position::new(17, 20));
}

#[test]
fn newline_token_has_single_byte_span() {
    let (tokens, _) = tokenize("MOVE R0, R1\nMOVE R0, R2");

    assert_eq!(tokens[4].position, Position::new(11, 12));
}

#[test]
fn crlf_is_a_single_newline_token() {
    let (tokens, errors) = tokenize("MOVE R0, R1\r\nMOVE R0, R2");

    assert!(errors.is_empty());

    let newlines = tokens
        .iter()
        .filter(|t| matches!(t.kind, TokenKind::Newline))
        .count();
    assert_eq!(newlines, 1);
    assert!(matches!(tokens[5].kind, TokenKind::Mnemonic(_)));
}

#[test]
fn comment_does_not_affect_following_positions() {
    let (tokens, errors) = tokenize("MOVE R0, R1 ; note\nCALL foo");

    assert!(errors.is_empty());
    assert_eq!(tokens[4].position, Position::new(18, 19));
    assert_eq!(tokens[5].position, Position::new(19, 23));
}

#[test]
fn end_token_is_at_source_end() {
    let (tokens, _) = tokenize("NOP\n");

    assert_eq!(tokens[2].position, Position::new(4, 4));
    assert!(matches!(tokens[2].kind, TokenKind::End));
}

#[test]
fn error_position_covers_whole_register_word() {
    let (_, errors) = tokenize("R256");

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].pos.start, 0);
    assert_eq!(errors[0].pos.end, 4);
}

#[test]
fn numbers_cover_their_full_span() {
    let (tokens, _) = tokenize("1234 1.5 -2");

    assert_eq!(tokens[0].position, Position::new(0, 4));
    assert_eq!(tokens[1].position, Position::new(5, 8));
    assert_eq!(tokens[2].position, Position::new(9, 11));
}

#[test]
fn error_position_covers_single_character() {
    let (_, errors) = tokenize("!");

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].pos, Position::new(0, 1));
}

#[test]
fn positions_after_an_error_continue() {
    let (tokens, errors) = tokenize("MOVE ! R0");

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].pos, Position::new(5, 6));
    assert_eq!(tokens[1].position, Position::new(7, 9));
    assert_eq!(tokens[2].position, Position::new(9, 9));
}

#[test]
fn non_ascii_bytes_are_reported_per_byte() {
    // The lexer works with ASCII only.
    let (tokens, errors) = tokenize("я");

    assert_eq!(errors.len(), 2);
    assert!(matches!(
        errors[0].kind,
        LexerErrorKind::UnexpectedCharacter(_)
    ));
    assert_eq!(errors[0].pos, Position::new(0, 1));
    assert_eq!(errors[1].pos, Position::new(1, 2));
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn end_token_after_last_token() {
    let (tokens, _) = tokenize("42");

    assert_eq!(tokens[0].position, Position::new(0, 2));
    assert_eq!(tokens[1].position, Position::new(2, 2));
}

#[test]
fn positions_across_multiple_lines() {
    let (tokens, _) = tokenize("A\nB\nC");

    assert_eq!(tokens[0].position, Position::new(0, 1));
    assert_eq!(tokens[1].position, Position::new(1, 2));
    assert_eq!(tokens[2].position, Position::new(2, 3));
    assert_eq!(tokens[3].position, Position::new(3, 4));
    assert_eq!(tokens[4].position, Position::new(4, 5));
    assert_eq!(tokens[5].position, Position::new(5, 5));
}
