// Tests of the `IsInTerminal` service.
use std::io::{self, IsTerminal};

use w8_core::vm::WVM;
use w8_core::{isa::register::Register, vm::ExecuteVariant};
use wdh::{
    HostDecision, WDH,
    error::HostError,
    service::{
        SERVICE_IS_IN_TERMINAL, is_in_terminal,
        is_in_terminal::ARGS_SIZE,
        stream::{STREAM_STDERR, STREAM_STDOUT, Stream},
    },
};

#[test]
fn parse_args_parses_register_and_stream() {
    let (register, stream) = is_in_terminal::parse_args(&[7, STREAM_STDERR]).unwrap();
    assert_eq!(register.0, 7);
    assert_eq!(stream, Stream::Stderr);
}

#[test]
fn parse_args_rejects_a_wrong_size() {
    for got in [0usize, 1, 3] {
        assert_eq!(
            is_in_terminal::parse_args(&[0, 0, 0, 0][..got]).unwrap_err(),
            HostError::InvalidArgsSize {
                service: SERVICE_IS_IN_TERMINAL,
                expected: ARGS_SIZE,
                got: got as u64,
            }
        );
    }
}

#[test]
fn parse_args_rejects_an_invalid_register() {
    assert_eq!(
        is_in_terminal::parse_args(&[255, STREAM_STDOUT]).unwrap_err(),
        HostError::InvalidRegister { got: 255 }
    );
}

#[test]
fn parse_args_rejects_an_unknown_stream() {
    assert_eq!(
        is_in_terminal::parse_args(&[0, 3]).unwrap_err(),
        HostError::UnknownStream { got: 3 }
    );
}

#[test]
fn stream_is_terminal_matches_std() {
    assert_eq!(Stream::Stdin.is_terminal(), io::stdin().is_terminal());
    assert_eq!(Stream::Stdout.is_terminal(), io::stdout().is_terminal());
    assert_eq!(Stream::Stderr.is_terminal(), io::stderr().is_terminal());
}

#[test]
fn dispatch_writes_the_result_into_the_register() {
    let mut vm = WVM::new(64 * 1024, ExecuteVariant::default());
    let mut host = WDH::new();

    let args = [9, STREAM_STDOUT];
    let decision = host
        .dispatch(&mut vm, SERVICE_IS_IN_TERMINAL, &args)
        .unwrap();

    assert_eq!(decision, HostDecision::Continue);
    assert_eq!(
        vm.registers[Register(9)],
        u64::from(io::stdout().is_terminal())
    );
}

#[test]
fn dispatch_leaves_the_register_untouched_on_an_unknown_stream() {
    let mut vm = WVM::new(64 * 1024, ExecuteVariant::default());
    let mut host = WDH::new();

    let err = host
        .dispatch(&mut vm, SERVICE_IS_IN_TERMINAL, &[9, 3])
        .unwrap_err();
    assert_eq!(err, HostError::UnknownStream { got: 3 });
    assert_eq!(vm.registers[Register(9)], 0);
}
