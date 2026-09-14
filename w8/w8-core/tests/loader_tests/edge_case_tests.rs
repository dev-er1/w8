// Tests on edge cases of the `.wb`-file loader.
use w8_core::{isa::opcode::OperationCode, loader::err::LoaderErrorKind};

use super::*;

#[test]
fn exactly_11_bytes_no_instructions() {
    let data = make_nb(&[]);
    assert_eq!(data.len(), 11);

    let instructions = run_loader(data).expect("expected successful parse");
    assert!(instructions.is_empty());
}

#[test]
fn only_instruction_without_padding() {
    let data = make_nb(&nop_bytes());
    assert_eq!(data.len(), 11 + 2);

    let instructions = run_loader(data).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);
}

#[test]
fn three_zero_operand_instructions() {
    let mut bytes = vec![];
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&nop_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 3);
}

#[test]
fn nop_followed_by_vmcall() {
    let mut bytes = vec![];
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&vmcall_bytes(0, 0, 1));

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 2);

    assert!(matches!(instructions[0].opcode, OperationCode::NOP));
    assert!(matches!(instructions[1].opcode, OperationCode::VMCALL));
}

#[test]
fn three_register_operands_maximum() {
    let bytes = vec![0x0A, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02];

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);
    assert_eq!(instructions[0].operand_count(), 3);
}

#[test]
fn three_immediate_operands_maximum() {
    let mut bytes = vec![0x0A, 0x03];

    bytes.push(0x01);
    bytes.extend_from_slice(&1u64.to_le_bytes());

    bytes.push(0x01);
    bytes.extend_from_slice(&2u64.to_le_bytes());

    bytes.push(0x01);
    bytes.extend_from_slice(&3u64.to_le_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);

    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_immediate()
            .unwrap(),
        1
    );
    assert_eq!(
        instructions[0]
            .operand2
            .unwrap()
            .expect_immediate()
            .unwrap(),
        2
    );
    assert_eq!(
        instructions[0]
            .operand3
            .unwrap()
            .expect_immediate()
            .unwrap(),
        3
    );
}

#[test]
fn immediate_max_u64_value() {
    let bytes = move_reg_imm(0, u64::MAX);

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    let val = instructions[0]
        .operand2
        .unwrap()
        .expect_immediate()
        .unwrap();
    assert_eq!(val, u64::MAX);
}

#[test]
fn immediate_zero_value() {
    let bytes = move_reg_imm(0, 0);

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    let val = instructions[0]
        .operand2
        .unwrap()
        .expect_immediate()
        .unwrap();
    assert_eq!(val, 0);
}

#[test]
fn register_number_zero() {
    let bytes = move_reg_reg(0, 0);

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    let dst = instructions[0].operand1.unwrap().expect_register().unwrap();
    let src = instructions[0].operand2.unwrap().expect_register().unwrap();
    assert_eq!(dst.0, 0);
    assert_eq!(src.0, 0);
}

#[test]
fn register_number_255_is_rejected() {
    // W8 has 255 registers, numbered 0-254: the byte 0xFF is out of range.
    let bytes = move_reg_reg(255, 254);

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("register 255 must be rejected"),
    };
    assert!(matches!(
        err.kind,
        LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
    ));
}

#[test]
fn all_valid_opcodes_up_to_vmcall() {
    // Checks that all opcodes from 0 to 52 parse without errors.
    for opcode in 0..=52u8 {
        let bytes = vec![opcode, 0x00];
        let result = run_loader(make_nb(&bytes));
        assert!(result.is_ok(), "opcode {opcode} should be valid");
    }
}

#[test]
fn first_opcode_after_vmcall_fails() {
    let bytes = vec![53, 0x00];
    let result = run_loader(make_nb(&bytes));
    assert!(result.is_err());
}

#[test]
fn instruction_at_max_file_size_without_truncation() {
    let mut bytes = vec![];
    for _ in 0..256 {
        bytes.extend_from_slice(&nop_bytes());
    }

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 256);
}

#[test]
fn single_byte_operand_tag_missing_body() {
    let bytes = vec![0x02, 0x01, 0x00];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnexpectedEndOfFile { .. }
    ));
}

#[test]
fn mixed_register_and_immediate_operands() {
    let mut bytes = vec![0x06, 0x02, 0x00, 0x05, 0x01];
    bytes.extend_from_slice(&255u64.to_le_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::STORE8));

    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_register()
            .unwrap()
            .0,
        5
    );
    assert_eq!(
        instructions[0]
            .operand2
            .unwrap()
            .expect_immediate()
            .unwrap(),
        255
    );
}

#[test]
fn call_with_immediate_offset() {
    let mut bytes = vec![0x32, 0x01, 0x01];
    bytes.extend_from_slice(&10u64.to_le_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::CALL));
    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_immediate()
            .unwrap(),
        10
    );
}

#[test]
fn not_instruction_with_register() {
    let bytes = vec![0x1B, 0x01, 0x00, 0x07];

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::NOT));
    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_register()
            .unwrap()
            .0,
        7
    );
}
