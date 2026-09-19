// wdh/src/service/write.rs
//
//! The `Write` service: writes a string of VM memory to a standard
//! stream of the host process.
//!
//! ABI (see `wdh/VMCall-ABI/English/Service-ABI/Write-ABI.md`):
//!
//! ```text
//! VMCALL 1, <address>, <size>
//! ```
//!
//! The block of arguments is the string to write, prefixed with the
//! stream byte and the encoding byte.
//!
//! The host converts the string to UTF-8 and writes it to the stream.
//! stdin cannot be written to, so only stdout and stderr are valid
//! targets. The block has no fixed size; a block shorter than the
//! header is an error.
use std::io::{self, Write};

use crate::{
    error::HostError,
    service::{
        SERVICE_WRITE,
        encoding::Encoding,
        stream::{STREAM_STDIN, Stream},
    },
};

/// The size of the arguments header: the stream and the encoding bytes.
pub const ARGS_HEADER_SIZE: usize = 2;

pub fn parse_args(args: &[u8]) -> Result<(Stream, Encoding, &[u8]), HostError> {
    if args.len() < ARGS_HEADER_SIZE {
        return Err(HostError::InvalidArgsSize {
            service: SERVICE_WRITE,
            expected: ARGS_HEADER_SIZE as u64,
            got: args.len() as u64,
        });
    }

    let stream = Stream::from_byte(args[0])?;
    let encoding = Encoding::from_byte(args[1])?;
    Ok((stream, encoding, &args[ARGS_HEADER_SIZE..]))
}

pub fn write_to(sink: &mut dyn Write, args: &[u8]) -> Result<(), HostError> {
    let (_, encoding, data) = parse_args(args)?;
    flush_to(sink, &encoding.to_utf8(data)?)
}

/// Handles the `Write` service: writes the block to the standard stream
/// named in it.
pub fn write(args: &[u8]) -> Result<(), HostError> {
    let (stream, encoding, data) = parse_args(args)?;
    let bytes = encoding.to_utf8(data)?;
    match stream {
        Stream::Stdin => Err(HostError::UnwritableStream { got: STREAM_STDIN }),
        Stream::Stdout => flush_to(&mut io::stdout().lock(), &bytes),
        Stream::Stderr => flush_to(&mut io::stderr().lock(), &bytes),
    }
}

/// Writes the bytes to the sink and flushes it.
fn flush_to(sink: &mut dyn Write, bytes: &[u8]) -> Result<(), HostError> {
    sink.write_all(bytes)
        .and_then(|()| sink.flush())
        .map_err(|e| HostError::WriteFailed { kind: e.kind() })
}
