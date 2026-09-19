// Tests for token recognition: mnemonics, registers,
// identifiers, and punctuation.
use w8_core::isa::opcode::OperationCode;
use wdasm::lexer::err::LexerErrorKind;

use super::*;

#[test]
fn all_mnemonics_are_recognized() {
    let mnemonics = [
        "nop", "move", "load8", "load16", "load32", "load64", "store8", "store16", "store32",
        "store64", "iadd", "isub", "imul", "sdiv", "udiv", "srem", "urem", "ineg", "fadd", "fsub",
        "fmul", "fdiv", "frem", "fneg", "and", "or", "xor", "not", "lsl", "lsr", "sar", "ieq",
        "ine", "slt", "sle", "sgt", "sge", "ult", "ule", "ugt", "uge", "feq", "fne", "flt", "fle",
        "fgt", "fge", "jmp", "jz", "jnz", "call", "ret", "vmcall",
    ];

    for mnemonic in mnemonics {
        let (tokens, errors) = tokenize(mnemonic);

        assert!(
            errors.is_empty(),
            "`{mnemonic}` must not produce any lexer error"
        );
        assert!(
            matches!(tokens[0].kind, TokenKind::Mnemonic(_)),
            "`{mnemonic}` must be recognized as a mnemonic"
        );
        assert!(
            matches!(tokens[1].kind, TokenKind::End),
            "an `End` token must follow the mnemonic"
        );
    }
}

#[test]
fn mnemonics_are_case_insensitive() {
    let (tokens, errors) = tokenize("MoVe jmp RET");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Mnemonic(OperationCode::MOVE)
    ));
    assert!(matches!(
        tokens[1].kind,
        TokenKind::Mnemonic(OperationCode::JMP)
    ));
    assert!(matches!(
        tokens[2].kind,
        TokenKind::Mnemonic(OperationCode::RET)
    ));
}

#[test]
fn registers_with_different_numbers() {
    let cases = [("R0", 0u8), ("r1", 1), ("R10", 10), ("r254", 254)];

    for (text, number) in cases {
        let (tokens, errors) = tokenize(text);

        assert!(
            errors.is_empty(),
            "`{text}` must not produce any lexer error"
        );
        assert!(
            matches!(tokens[0].kind, TokenKind::Register(r) if r.0 == number),
            "`{text}` must be the register R{number}"
        );
    }
}

#[test]
fn unknown_words_become_identifiers() {
    let (tokens, errors) = tokenize("foo _bar baz_1");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
}

#[test]
fn words_starting_with_r_are_not_always_registers() {
    // `result` starts with the letter r but not a digit, and a lone `r`
    // has no number at all — both are identifiers.
    for text in ["result", "r"] {
        let (tokens, errors) = tokenize(text);

        assert!(errors.is_empty());
        assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    }
}

#[test]
fn r0x_is_an_identifier() {
    // `r0x`` looks like a register only by its first character.
    let (tokens, errors) = tokenize("r0x");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
}

#[test]
fn punctuation_is_recognized() {
    let (tokens, errors) = tokenize(", : [ ] + - *");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Comma));
    assert!(matches!(tokens[1].kind, TokenKind::Colon));
    assert!(matches!(tokens[2].kind, TokenKind::OpeningSquareBracket));
    assert!(matches!(tokens[3].kind, TokenKind::EndingSquareBracket));
    assert!(matches!(tokens[4].kind, TokenKind::Plus));
    assert!(matches!(tokens[5].kind, TokenKind::Minus));
    assert!(matches!(tokens[6].kind, TokenKind::Asterisk));
    assert!(matches!(tokens[7].kind, TokenKind::End));
}

#[test]
fn star_with_a_word_becomes_a_local_label() {
    let (tokens, errors) = tokenize("*loop:");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::LocalLabel(_)));
    assert!(matches!(tokens[1].kind, TokenKind::Colon));
    assert!(matches!(tokens[2].kind, TokenKind::End));
}

#[test]
fn star_alone_stays_an_asterisk() {
    let (tokens, errors) = tokenize("*");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Asterisk));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn star_before_a_digit_stays_an_asterisk() {
    let (tokens, errors) = tokenize("*2");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Asterisk));
    assert!(matches!(tokens[1].kind, TokenKind::Integer(2)));
}

#[test]
fn local_label_names_allow_the_same_characters_as_identifiers() {
    let (tokens, errors) = tokenize("*_x1 *foo_2");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::LocalLabel(_)));
    assert!(matches!(tokens[1].kind, TokenKind::LocalLabel(_)));
}

#[test]
fn local_label_name_keeps_the_word_after_the_star() {
    let source = SourceCode::new("*loop: JMP *loop".to_string());
    let mut str_pool = StrPool::from_source(&source);
    let mut lexer = Lexer::new(source, &mut str_pool);
    lexer.tokenize();

    assert!(lexer.errors.is_empty());

    let declaration = match &lexer.tokens[0].kind {
        TokenKind::LocalLabel(id) => *id,
        _ => panic!("expected a local label declaration"),
    };
    let reference = match &lexer.tokens[3].kind {
        TokenKind::LocalLabel(id) => *id,
        _ => panic!("expected a local label reference"),
    };

    // The declaration and the reference share the interned name `loop`.
    assert_eq!(declaration, reference);
    assert_eq!(str_pool.get(declaration), "loop");
}

#[test]
fn a_star_does_not_change_how_identifiers_lex() {
    let (tokens, errors) = tokenize("JMP *loop");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Mnemonic(OperationCode::JMP)
    ));
    assert!(matches!(tokens[1].kind, TokenKind::LocalLabel(_)));
    assert!(matches!(tokens[2].kind, TokenKind::End));
}

#[test]
fn dot_is_a_token_when_not_part_of_a_number() {
    let (tokens, errors) = tokenize(".foo .5");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Dot));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Float(v) if v == 0.5));
}

#[test]
fn full_program_produces_expected_sequence() {
    let src = "main:\n    MOVE R0, 42\n    CALL foo\n    RET\n";
    let kinds = kinds(src);

    assert!(matches!(kinds[0], TokenKind::Ident(_)));
    assert!(matches!(kinds[1], TokenKind::Colon));
    assert!(matches!(kinds[2], TokenKind::Newline));
    assert!(matches!(kinds[3], TokenKind::Mnemonic(OperationCode::MOVE)));
    assert!(matches!(kinds[4], TokenKind::Register(_)));
    assert!(matches!(kinds[5], TokenKind::Comma));
    assert!(matches!(kinds[6], TokenKind::Integer(42)));
    assert!(matches!(kinds[7], TokenKind::Newline));
    assert!(matches!(kinds[8], TokenKind::Mnemonic(OperationCode::CALL)));
    assert!(matches!(kinds[9], TokenKind::Ident(_)));
    assert!(matches!(kinds[10], TokenKind::Newline));
    assert!(matches!(kinds[11], TokenKind::Mnemonic(OperationCode::RET)));
    assert!(matches!(kinds[12], TokenKind::Newline));
    assert!(matches!(kinds[13], TokenKind::End));
}

#[test]
fn comments_are_skipped() {
    let (tokens, errors) = tokenize("; только комментарий\nMOVE R0, R1 ; регистры\n");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Newline));
    assert!(matches!(
        tokens[1].kind,
        TokenKind::Mnemonic(OperationCode::MOVE)
    ));
    assert!(matches!(tokens[5].kind, TokenKind::Newline));
}

#[test]
fn identifiers_are_interned_in_str_pool() {
    let source = SourceCode::new("start: CALL start".to_string());
    let mut str_pool = StrPool::from_source(&source);
    let mut lexer = Lexer::new(source, &mut str_pool);
    lexer.tokenize();

    assert!(lexer.errors.is_empty());

    let first = match &lexer.tokens[0].kind {
        TokenKind::Ident(id) => *id,
        _ => panic!("expected an identifier"),
    };
    let second = match &lexer.tokens[3].kind {
        TokenKind::Ident(id) => *id,
        _ => panic!("expected an identifier"),
    };

    assert_eq!(first, second);
    assert_eq!(str_pool.get(first), "start");
}

#[test]
fn empty_input_has_only_the_end_token() {
    let (tokens, errors) = tokenize("");

    assert!(errors.is_empty());
    assert_eq!(
        tokens.len(),
        1,
        "empty input must produce exactly one token"
    );
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn whitespace_only_input_has_only_the_end_token() {
    let (tokens, errors) = tokenize(" \t\r ");

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 1, "whitespace must not produce tokens");
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn comment_only_input_has_only_the_end_token() {
    let (tokens, errors) = tokenize("; nothing to see here");

    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 1, "a comment must not produce tokens");
    assert!(matches!(tokens[0].kind, TokenKind::End));
}

#[test]
fn numbers_inside_identifiers_are_kept() {
    let (tokens, errors) = tokenize("label1 foo_2 _3abc");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
}

#[test]
fn underscores_start_an_identifier() {
    let (tokens, errors) = tokenize("_ _foo __");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Ident(_)));
}

#[test]
fn newlines_between_statements_resolve() {
    let (tokens, errors) = tokenize("A\n\nMOVE");

    assert!(errors.is_empty());
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[1].kind, TokenKind::Newline));
    assert!(matches!(tokens[2].kind, TokenKind::Newline));
    assert!(matches!(tokens[3].kind, TokenKind::Mnemonic(_)));
    assert!(matches!(tokens[4].kind, TokenKind::End));
}

#[test]
fn comment_at_end_of_input_has_no_trailing_newline() {
    let (tokens, errors) = tokenize("NOP ; trailing comment");

    assert!(errors.is_empty());
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Mnemonic(OperationCode::NOP)
    ));
    assert!(matches!(tokens[1].kind, TokenKind::End));
}

#[test]
fn unexpected_character_is_reported_and_skipped() {
    let (tokens, errors) = tokenize("MOVE ! R0");

    assert_eq!(errors.len(), 1, "`!` must be reported once");
    assert!(matches!(
        errors[0].kind,
        LexerErrorKind::UnexpectedCharacter('!')
    ));
    assert!(matches!(
        tokens[0].kind,
        TokenKind::Mnemonic(OperationCode::MOVE)
    ));
    assert!(matches!(tokens[1].kind, TokenKind::Register(r) if r.0 == 0));
    assert!(matches!(tokens[2].kind, TokenKind::End));
}

#[test]
fn registers_out_of_range_are_reported() {
    for text in ["R255", "R256", "r999", "R12345"] {
        let (tokens, errors) = tokenize(text);

        assert_eq!(errors.len(), 1, "`{text}` must be reported");
        assert!(matches!(errors[0].kind, LexerErrorKind::InvalidRegister(_)));
        assert!(matches!(tokens[0].kind, TokenKind::End));
    }
}
