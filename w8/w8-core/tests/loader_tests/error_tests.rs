// Tests on loader errors of `.wb`-files.
use w8_core::loader::err::LoaderErrorKind;

use super::*;

#[test]
fn file_under_11_bytes_returns_format_error() {
    for size in 0..=10 {
        let data = vec![0u8; size];
        let err = match run_loader(data) {
            Err(e) => e,
            Ok(_) => panic!("expected loader error for size {size}"),
        };

        assert!(
            matches!(
                err.kind,
                LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
            ),
            "size {size}: expected FileIsNotInW8BytecodeFormat, got {:?}",
            err.kind,
        );
    }
}

#[test]
fn unknown_opcode_returns_error() {
    let bytes = vec![53, 0x00];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnknownOpcode { byte: 53 }
    ));
}

#[test]
fn opcode_255_returns_unknown_opcode() {
    let bytes = vec![0xFF, 0x00];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnknownOpcode { byte: 0xFF }
    ));
}

#[test]
fn unknown_operand_tag_returns_error() {
    let bytes = vec![0x01, 0x01, 0xFF, 0x00];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnknownOperandTag { byte: 0xFF }
    ));
}

#[test]
fn operand_tag_0x02_returns_error() {
    let bytes = vec![0x01, 0x01, 0x02, 0x00];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnknownOperandTag { byte: 0x02 }
    ));
}

#[test]
fn truncated_instruction_header_returns_unexpected_eof() {
    let bytes = vec![0x01];

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
fn truncated_register_operand_returns_unexpected_eof() {
    let bytes = vec![0x01, 0x02, 0x00, 0x00, 0x00];

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
fn truncated_immediate_operand_returns_unexpected_eof() {
    let bytes = vec![0x01, 0x02, 0x00, 0x00, 0x01, 0xAA, 0xBB, 0xCC];

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
fn truncated_after_opcode_with_operand_count_only() {
    let bytes = vec![0x01, 0x02];

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
fn operand_count_4_returns_unknown_opcode() {
    let bytes = vec![0x00, 0x04];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(err.kind, LoaderErrorKind::UnknownOpcode { .. }));
}

#[test]
fn operand_count_255_returns_unknown_opcode() {
    let bytes = vec![0x01, 0xFF];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(err.kind, LoaderErrorKind::UnknownOpcode { .. }));
}

#[test]
fn negative_error_reason_contains_description() {
    let err = match run_loader(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    let msg = err.to_string();
    assert!(msg.contains("not in W8 Bytecode format"));
}

#[test]
fn truncated_file_with_only_magic() {
    // 5 magic + 4 bytes of version (9 in total).
    let mut data = b"NVMBC".to_vec();
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    let err = match run_loader(data) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
    ));
}

#[test]
fn empty_instruction_with_operand_count_1_and_no_data() {
    let bytes = vec![0x00, 0x01];

    let err = match run_loader(make_nb(&bytes)) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnexpectedEndOfFile { .. }
    ));
}
