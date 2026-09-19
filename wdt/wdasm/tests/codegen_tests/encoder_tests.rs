// Tests for `.wb` file generation (the `codegen::encoder` submodule).
use w8_core::{
    W8_VERSION,
    isa::{instruction::Instruction, opcode::OperationCode},
    loader::W8Loader,
};
use wdasm::codegen::encoder;

use super::*;

// Builds an instruction from an opcode and up to three operands.
fn instr(opcode: OperationCode, operands: [Option<Operand>; 3]) -> Instruction {
    Instruction {
        opcode,
        operand1: operands[0],
        operand2: operands[1],
        operand3: operands[2],
    }
}

// Runs bytes through the `.wb` file loader.
fn load(bytes: &[u8]) -> Vec<Instruction> {
    W8Loader::new(bytes.to_vec())
        .transpile()
        .expect("valid bytecode")
}

// Compares programs via Display (the types do not implement PartialEq).
fn assert_programs_eq(actual: &[Instruction], expected: &[Instruction]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.to_string(), expected.to_string());
    }
}

#[test]
fn empty_program_is_just_the_header() {
    let bytes = encoder::encode(&[], None);

    assert_eq!(bytes.len(), 11);
    assert_eq!(&bytes[..5], b"NVMBC");
}

#[test]
fn version_bytes_match_core_version() {
    let bytes = encoder::encode(&[], None);

    let parts: Vec<u16> = W8_VERSION
        .split('.')
        .take(3)
        .map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect();

    assert_eq!(&bytes[5..7], parts[0].to_le_bytes().as_slice());
    assert_eq!(&bytes[7..9], parts[1].to_le_bytes().as_slice());
    assert_eq!(&bytes[9..11], parts[2].to_le_bytes().as_slice());
}

#[test]
fn declared_min_version_overrides_the_header() {
    let bytes = encoder::encode(&[], Some((0, 1, 1)));

    assert_eq!(&bytes[5..7], 0u16.to_le_bytes().as_slice());
    assert_eq!(&bytes[7..9], 1u16.to_le_bytes().as_slice());
    assert_eq!(&bytes[9..11], 1u16.to_le_bytes().as_slice());
}

#[test]
fn declared_min_version_with_high_parts() {
    let bytes = encoder::encode(&[], Some((65535, 12, 65535)));

    assert_eq!(&bytes[5..7], 65535u16.to_le_bytes().as_slice());
    assert_eq!(&bytes[7..9], 12u16.to_le_bytes().as_slice());
    assert_eq!(&bytes[9..11], 65535u16.to_le_bytes().as_slice());
}

#[test]
fn declared_min_version_does_not_affect_instructions() {
    let program = [instr(OperationCode::NOP, [None, None, None])];
    let bytes = encoder::encode(&program, Some((0, 1, 0)));

    assert_eq!(&bytes[11..], &[0x00, 0x00]);
}

#[test]
fn bytecode_with_a_foreign_version_is_not_loaded() {
    // W8 versions are incompatible with each other: a file compiled
    // with another version must not load on the current VM.
    let bytes = encoder::encode(&[], Some((0, 0, 0)));

    assert!(W8Loader::new(bytes).transpile().is_err());
}

#[test]
fn instructions_follow_the_header() {
    let program = [instr(OperationCode::NOP, [None, None, None])];
    let bytes = encoder::encode(&program, None);

    assert_eq!(&bytes[11..], &[0x00, 0x00]);
}

#[test]
fn zero_operand_instructions_encode_opcode_and_zero_count() {
    let program = [
        instr(OperationCode::NOP, [None, None, None]),
        instr(OperationCode::NOP, [None, None, None]),
        instr(OperationCode::RET, [None, None, None]),
    ];
    let bytes = encoder::encode(&program, None);

    assert_eq!(&bytes[11..13], &[0x00, 0x00]); // NOP
    assert_eq!(&bytes[13..15], &[0x00, 0x00]); // NOP
    assert_eq!(&bytes[15..17], &[OperationCode::RET as u8, 0x00]);
}

#[test]
fn move_with_register_and_immediate() {
    let program = [instr(
        OperationCode::MOVE,
        [Some(reg(0)), Some(imm(42)), None],
    )];
    let bytes = encoder::encode(&program, None);

    assert_eq!(
        &bytes[11..],
        &[
            0x01, 0x02, 0x00, 0x00, 0x01, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
        ]
    );
}

#[test]
fn move_with_two_registers() {
    let program = [instr(
        OperationCode::MOVE,
        [Some(reg(0)), Some(reg(1)), None],
    )];
    let bytes = encoder::encode(&program, None);

    assert_eq!(&bytes[11..], &[0x01, 0x02, 0x00, 0x00, 0x00, 0x01]);
}

#[test]
fn iadd_with_three_registers() {
    let program = [instr(
        OperationCode::IADD,
        [Some(reg(0)), Some(reg(1)), Some(reg(2))],
    )];
    let bytes = encoder::encode(&program, None);

    assert_eq!(
        &bytes[11..],
        &[
            OperationCode::IADD as u8,
            0x03,
            0x00,
            0x00,
            0x00,
            0x01,
            0x00,
            0x02
        ]
    );
}

#[test]
fn immediate_zero_writes_eight_zero_bytes() {
    let program = [instr(
        OperationCode::MOVE,
        [Some(reg(0)), Some(imm(0)), None],
    )];
    let bytes = encoder::encode(&program, None);

    // 11 header + 2 (opcode, count) + 2 (tag, register) = 15: the immediate tag.
    assert_eq!(bytes[15], 0x01);
    assert_eq!(&bytes[16..], &[0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn immediate_max_u64_writes_all_ff() {
    let program = [instr(
        OperationCode::MOVE,
        [Some(reg(0)), Some(imm(u64::MAX)), None],
    )];
    let bytes = encoder::encode(&program, None);

    let mut expected = vec![0x01, 0x02, 0x00, 0x00, 0x01];
    expected.extend_from_slice(&u64::MAX.to_le_bytes());

    assert_eq!(&bytes[11..], expected.as_slice());
}

#[test]
fn max_register_254_writes_fe() {
    // 254 — the maximum register number (W8 has 255 registers, 0-254).
    let program = [instr(
        OperationCode::MOVE,
        [Some(reg(254)), Some(imm(1)), None],
    )];
    let bytes = encoder::encode(&program, None);

    assert_eq!(&bytes[13..15], &[0x00, 0xFE]);
}

#[test]
fn all_opcodes_encode_in_enum_order() {
    for value in 0..=OperationCode::VMCALL as u8 {
        let opcode = OperationCode::try_from(value).expect("valid opcode");
        let program = [instr(opcode, [None, None, None])];
        let bytes = encoder::encode(&program, None);

        assert_eq!(bytes[11], value, "opcode byte for {opcode:?}");
        assert_eq!(bytes[12], 0);
    }
}

#[test]
fn mixed_program_writes_instructions_in_order() {
    let program = [
        instr(OperationCode::NOP, [None, None, None]),
        instr(OperationCode::MOVE, [Some(reg(0)), Some(imm(1)), None]),
        instr(OperationCode::NOP, [None, None, None]),
    ];
    let bytes = encoder::encode(&program, None);

    // NOP (2 bytes) + MOVE reg,imm (13 bytes) + NOP (2 bytes).
    assert_eq!(bytes.len(), 11 + 2 + 13 + 2);
    assert_eq!(bytes[11], 0x00); // NOP
    assert_eq!(bytes[13], OperationCode::MOVE as u8); // MOVE
    assert_eq!(bytes[26], 0x00); // NOP
}

#[test]
fn roundtrip_empty_program() {
    let bytes = encoder::encode(&[], None);

    assert_programs_eq(&load(&bytes), &[]);
}

#[test]
fn roundtrip_simple_program() {
    let program = [
        instr(OperationCode::NOP, [None, None, None]),
        instr(OperationCode::NOP, [None, None, None]),
        instr(OperationCode::MOVE, [Some(reg(0)), Some(imm(42)), None]),
        instr(
            OperationCode::IADD,
            [Some(reg(0)), Some(reg(1)), Some(reg(2))],
        ),
    ];

    let bytes = encoder::encode(&program, None);

    assert_programs_eq(&load(&bytes), &program);
}

#[test]
fn roundtrip_extreme_values() {
    let program = [
        instr(
            OperationCode::MOVE,
            [Some(reg(254)), Some(imm(u64::MAX)), None],
        ),
        instr(OperationCode::MOVE, [Some(reg(0)), Some(imm(0)), None]),
    ];

    let bytes = encoder::encode(&program, None);

    assert_programs_eq(&load(&bytes), &program);
}

#[test]
fn full_pipeline_roundtrip() {
    let program = codegen("MOVE R0, 42\nIADD R0, R1, 2\nNOP").expect("valid program");

    let bytes = encoder::encode(&program, None);

    assert_programs_eq(&load(&bytes), &program);
}

#[test]
fn pipeline_with_labels_roundtrip() {
    let program = codegen("loop:\nMOVE R0, 5\nJNZ R0, loop\nCALL sub\nNOP\nsub:\nRET")
        .expect("valid program");

    let bytes = encoder::encode(&program, None);

    assert_programs_eq(&load(&bytes), &program);
}

#[test]
fn pipeline_float_bit_pattern_roundtrip() {
    let program = codegen("MOVE R0, 1.5\nNOP").expect("valid program");

    let bytes = encoder::encode(&program, None);

    assert_programs_eq(&load(&bytes), &program);
}
