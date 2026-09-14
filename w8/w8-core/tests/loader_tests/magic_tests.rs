// Tests on the magic section of `.wb`-files.
use w8_core::loader::err::LoaderErrorKind;

use super::*;

#[test]
fn valid_magic_accepts() {
    let result = run_loader(make_nb(&nop_bytes()));
    assert!(result.is_ok());
}

#[test]
fn empty_file_rejected() {
    let err = match run_loader(vec![]) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
    ));
}

#[test]
fn too_short_for_magic_rejected() {
    let err = match run_loader(b"NV".to_vec()) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
    ));
}

#[test]
fn wrong_magic_rejected() {
    let mut data = b"XXXXX".to_vec();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

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
fn lowercase_magic_rejected() {
    let mut data = b"nvmbc".to_vec();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

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
fn partial_magic_rejected() {
    let mut data = b"W8".to_vec();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

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
fn magic_with_trailing_bytes_after_magic_ok() {
    let result = run_loader(make_nb(&[]));
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn magic_with_extra_bytes_before_instruction_stream_ok() {
    let result = run_loader(make_nb(&nop_bytes()));
    assert!(result.is_ok());
}

#[test]
fn magic_exactly_five_bytes_with_version_rejected() {
    let mut data = b"NVMBC".to_vec();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.push(0x00);

    let err = match run_loader(data) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::FileIsNotInW8BytecodeFormat { .. }
    ));
}
