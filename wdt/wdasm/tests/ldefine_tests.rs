/// Integration tests for the `.ldefine` directive.
use std::{collections::HashMap, io, path::Path};

use w8_core::isa::{
    instruction::Instruction,
    operand::{Operand, OperandKind},
    register::Register,
};
use wdasm::{
    codegen, codegen::err::CodegenErrorKind, lexer::Lexer, parser::Parser, preprocess,
    src::SourceCode, str_pool::StrPool,
};

// Normalizes a path the same way on every platform: forward slashes
// and no leading `./`.
fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn codegen_with_files(
    main: &str,
    files: &[(&str, &str)],
) -> Result<codegen::CodegenResult, codegen::err::CodegenError> {
    let mut fs: HashMap<String, String> = files
        .iter()
        .map(|(path, src)| (normalize(path), src.to_string()))
        .collect();
    let mut resolver = |path: &str| {
        fs.remove(&normalize(path))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "file not found"))
    };

    let preprocessed = preprocess::preprocess(
        main,
        Some("main.asm"),
        Some(Path::new(".")),
        Some(&mut resolver),
        false,
    )
    .expect("preprocessing must succeed");

    let files_map: Vec<(u32, u32)> = preprocessed
        .segments
        .iter()
        .map(|segment| (segment.global_start, segment.file))
        .collect();
    let source = SourceCode::new(preprocessed.text);
    let mut str_pool = StrPool::from_source(&source);
    let mut lexer = Lexer::new(source, &mut str_pool);
    let tokens = lexer.tokenize().to_vec();
    assert!(lexer.errors.is_empty(), "lexer errors: {:?}", lexer.errors);

    let mut parser = Parser::new(tokens, &str_pool, &files_map);
    let ast = parser.parse().clone();
    assert!(
        parser.errors.is_empty(),
        "parser errors: {:?}",
        parser.errors
    );

    codegen::generate(&ast, &str_pool)
}

// The operands of the n-th instruction.
fn operands_of(result: &codegen::CodegenResult, n: usize) -> &Instruction {
    &result.instructions[n]
}

// Assertions on instruction operands (the types don not implement PartialEq).
fn assert_reg(operand: Option<Operand>, expected: u8) {
    assert!(matches!(
        operand,
        Some(Operand {
            kind: OperandKind::Register(Register(reg)),
        }) if reg == expected
    ));
}

fn assert_imm(operand: Option<Operand>, expected: u64) {
    assert!(matches!(
        operand,
        Some(Operand {
            kind: OperandKind::Immediate(value),
        }) if value == expected
    ));
}

#[test]
fn ldefine_substitutes_in_instruction_operands() {
    let result =
        codegen_with_files(".ldefine WIDTH, 800\nMOVE R0, WIDTH", &[]).expect("must compile");

    let instr = operands_of(&result, 0);
    assert_reg(instr.operand1, 0);
    assert_imm(instr.operand2, 800);
}

#[test]
fn ldefine_of_a_register_is_an_alias() {
    let result = codegen_with_files(".ldefine RAX, R0\nMOVE RAX, 5", &[]).expect("must compile");

    assert_reg(operands_of(&result, 0).operand1, 0);
}

#[test]
fn the_last_ldefine_of_a_name_wins() {
    let result =
        codegen_with_files(".ldefine X, 1\n.ldefine X, 2\nMOVE R0, X", &[]).expect("must compile");

    assert_imm(operands_of(&result, 0).operand2, 2);
}

#[test]
fn ldefine_does_not_leak_into_the_includer() {
    let result = codegen_with_files(
        ".define TMP, R2\n#include \"lib.asm\"\nMOVE R0, TMP",
        &[("lib.asm", ".ldefine TMP, R5\nMOVE R1, TMP")],
    )
    .expect("must compile");

    assert_eq!(result.instructions.len(), 2);
    assert_reg(operands_of(&result, 0).operand1, 1);
    assert_reg(operands_of(&result, 0).operand2, 5);
    assert_reg(operands_of(&result, 1).operand1, 0);
    assert_reg(operands_of(&result, 1).operand2, 2);
}

#[test]
fn ldefine_in_one_include_does_not_affect_another() {
    let result = codegen_with_files(
        ".define TMP, R2\n#include \"lib1.asm\"\n#include \"lib2.asm\"\nMOVE R0, TMP",
        &[
            ("lib1.asm", ".ldefine TMP, R5\nMOVE R1, TMP"),
            ("lib2.asm", ".ldefine TMP, R9\nMOVE R2, TMP"),
        ],
    )
    .expect("must compile");

    assert_eq!(result.instructions.len(), 3);
    assert_reg(operands_of(&result, 0).operand2, 5);
    assert_reg(operands_of(&result, 1).operand2, 9);
    assert_reg(operands_of(&result, 2).operand2, 2);
}

#[test]
fn ldefine_in_the_main_file_survives_an_include() {
    let result = codegen_with_files(
        ".ldefine X, 7\n#include \"lib.asm\"\nMOVE R0, X",
        &[("lib.asm", "NOP")],
    )
    .expect("must compile");

    assert_eq!(result.instructions.len(), 2);
    assert_imm(operands_of(&result, 1).operand2, 7);
}

#[test]
fn a_name_defined_locally_in_another_file_is_an_undefined_label() {
    let err = codegen_with_files(
        "#include \"lib.asm\"\nMOVE R0, TMP",
        &[("lib.asm", ".ldefine TMP, R5\nNOP")],
    )
    .expect_err("the main file must not see the library's local constant");

    assert!(matches!(err.kind, CodegenErrorKind::UndefinedLabel { .. }));
}

#[test]
fn define_still_leaks_into_included_files() {
    let result = codegen_with_files(
        "#include \"lib.asm\"\nMOVE R0, G",
        &[("lib.asm", ".define G, 42\nNOP")],
    )
    .expect("must compile");

    assert_eq!(result.instructions.len(), 2);
    assert_imm(operands_of(&result, 1).operand2, 42);
}
