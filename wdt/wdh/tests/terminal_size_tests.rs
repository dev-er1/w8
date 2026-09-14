// Tests of the `TerminalSize` service.
use std::io;

use w8_core::vm::WVM;
use w8_core::{isa::register::Register, vm::ExecuteVariant};
use wdh::{
    HostDecision, WDH,
    error::HostError,
    service::{
        SERVICE_TERMINAL_SIZE,
        stream::{STREAM_STDOUT, Stream},
        terminal_size,
    },
};

#[test]
fn parse_args_parses_stream_and_registers() {
    let (stream, cols, rows) = terminal_size::parse_args(&[1, 7, 8]).unwrap();
    assert_eq!(stream, Stream::Stdout);
    assert_eq!(cols.0, 7);
    assert_eq!(rows.0, 8);
}

#[test]
fn parse_args_rejects_a_wrong_size() {
    for got in [0usize, 1, 2, 4] {
        assert_eq!(
            terminal_size::parse_args(&[0, 0, 0, 0][..got]).unwrap_err(),
            HostError::InvalidArgsSize {
                service: SERVICE_TERMINAL_SIZE,
                expected: terminal_size::ARGS_SIZE,
                got: got as u64,
            }
        );
    }
}

#[test]
fn parse_args_rejects_an_unknown_stream() {
    assert_eq!(
        terminal_size::parse_args(&[7, 0, 0]).unwrap_err(),
        HostError::UnknownStream { got: 7 }
    );
}

#[test]
fn parse_args_rejects_an_invalid_register() {
    assert_eq!(
        terminal_size::parse_args(&[1, 255, 0]).unwrap_err(),
        HostError::InvalidRegister { got: 255 }
    );
    assert_eq!(
        terminal_size::parse_args(&[1, 0, 255]).unwrap_err(),
        HostError::InvalidRegister { got: 255 }
    );
}

#[test]
fn terminal_size_matches_the_expected_value() {
    let (width, height) = terminal_size::terminal_size(Stream::Stdout);
    let expected = ::terminal_size::terminal_size_of(io::stdout());
    match expected {
        Some((::terminal_size::Width(w), ::terminal_size::Height(h))) => {
            assert_eq!(width, u64::from(w));
            assert_eq!(height, u64::from(h));
        }
        None => assert_eq!((width, height), (0, 0)),
    }
}

#[test]
fn dispatch_writes_the_size_into_the_registers() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    let mut host = WDH::new();

    let args = [STREAM_STDOUT, 7, 8];
    let decision = host
        .dispatch(&mut vm, SERVICE_TERMINAL_SIZE, &args)
        .unwrap();

    let (width, height) = terminal_size::terminal_size(Stream::Stdout);
    assert_eq!(decision, HostDecision::Continue);
    assert_eq!(vm.registers[Register(7)], width);
    assert_eq!(vm.registers[Register(8)], height);
}

#[test]
fn dispatch_leaves_the_registers_untouched_on_an_error() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    let mut host = WDH::new();

    let err = host
        .dispatch(&mut vm, SERVICE_TERMINAL_SIZE, &[7, 0, 0])
        .unwrap_err();
    assert_eq!(err, HostError::UnknownStream { got: 7 });
    assert_eq!(vm.registers[Register(0)], 0);
    assert_eq!(vm.registers[Register(1)], 0);
}
