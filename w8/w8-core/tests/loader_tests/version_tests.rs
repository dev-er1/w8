use w8_core::{W8_VERSION, loader::err::LoaderErrorKind};

use super::*;

fn current_version() -> [u16; 3] {
    let mut parts = current_version_parts();

    [
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    ]
}

fn expect_unsupported_version(data: Vec<u8>) {
    let err = match run_loader(data) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    assert!(matches!(
        err.kind,
        LoaderErrorKind::UnsupportedVersion { .. }
    ));
}

#[test]
fn the_current_version_parses_ok() {
    let [major, minor, patch] = current_version();
    let result = run_loader(make_nb_with_version(major, minor, patch, &nop_bytes()));
    assert!(result.is_ok());
}

#[test]
fn an_older_version_fails() {
    expect_unsupported_version(make_nb_with_version(0, 0, 0, &nop_bytes()));
}

#[test]
fn a_newer_patch_version_fails() {
    let [major, minor, patch] = current_version();
    expect_unsupported_version(make_nb_with_version(major, minor, patch + 1, &nop_bytes()));
}

#[test]
fn a_newer_minor_version_fails() {
    let [major, minor, patch] = current_version();
    expect_unsupported_version(make_nb_with_version(major, minor + 1, patch, &nop_bytes()));
}

#[test]
fn version_with_large_numbers_in_file_fails() {
    expect_unsupported_version(make_nb_with_version(99, 99, 99, &nop_bytes()));
}

#[test]
fn the_empty_file_with_the_current_version_parses_ok() {
    let [major, minor, patch] = current_version();
    let result = run_loader(make_nb_with_version(major, minor, patch, &[]));
    assert!(result.is_ok());
}

#[test]
fn version_parsed_correctly_from_bytes() {
    let data = make_nb_with_version(0, 0, 0, &[]);
    assert_eq!(&data[5..7], &[0x00, 0x00]);
    assert_eq!(&data[7..9], &[0x00, 0x00]);
    assert_eq!(&data[9..11], &[0x00, 0x00]);
}

#[test]
fn the_version_error_reports_both_versions() {
    let err = match run_loader(make_nb_with_version(0, 0, 0, &nop_bytes())) {
        Err(e) => e,
        Ok(_) => panic!("expected loader error"),
    };

    let message = err.to_string();
    assert!(message.contains("0.0.0"), "got: {message}");
    assert!(message.contains(W8_VERSION), "got: {message}");
}
