// Tests of the `Write` service and its underlying write helper.
use std::io::{self, Write};

use w8_core::vm::{ExecuteVariant, WVM};
use wdh::{
    HostDecision, WDH,
    error::HostError,
    service::{
        SERVICE_WRITE,
        encoding::{ENCODING_ASCII, ENCODING_UTF8, ENCODING_UTF16, Encoding},
        stream::{STREAM_STDERR, STREAM_STDOUT},
        write,
        write::ARGS_HEADER_SIZE,
    },
};

struct VecWriter(Vec<u8>);

impl Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed pipe"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Builds a block: `[stream][encoding][data]`.
fn block(stream: u8, encoding: u8, data: &[u8]) -> Vec<u8> {
    let mut args = vec![stream, encoding];
    args.extend_from_slice(data);
    args
}

#[test]
fn write_to_writes_an_ascii_block() {
    let mut sink = VecWriter(Vec::new());
    write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_ASCII, b"Hello, WVM!"),
    )
    .unwrap();
    assert_eq!(sink.0, b"Hello, WVM!");
}

#[test]
fn write_to_writes_a_utf8_block() {
    let mut sink = VecWriter(Vec::new());
    write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_UTF8, "Привет, W8!".as_bytes()),
    )
    .unwrap();
    assert_eq!(sink.0, "Привет, W8!".as_bytes());
}

#[test]
fn write_to_converts_a_utf16_block_to_utf8() {
    let data: Vec<u8> = "Привет, W8!"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();

    let mut sink = VecWriter(Vec::new());
    write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_UTF16, &data)).unwrap();
    assert_eq!(sink.0, "Привет, W8!".as_bytes());
}

#[test]
fn write_to_writes_nothing_for_an_empty_string() {
    let mut sink = VecWriter(Vec::new());
    write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_ASCII, b"")).unwrap();
    assert!(sink.0.is_empty());
}

#[test]
fn write_to_rejects_a_block_shorter_than_the_header() {
    let mut sink = VecWriter(Vec::new());
    for got in [0usize, 1] {
        assert_eq!(
            write::write_to(&mut sink, &[0u8; 4][..got]).unwrap_err(),
            HostError::InvalidArgsSize {
                service: SERVICE_WRITE,
                expected: ARGS_HEADER_SIZE as u64,
                got: got as u64,
            }
        );
    }
}

#[test]
fn write_to_rejects_an_unknown_stream() {
    let mut sink = VecWriter(Vec::new());
    let err = write::write_to(&mut sink, &block(7, ENCODING_ASCII, b"data")).unwrap_err();
    assert_eq!(err, HostError::UnknownStream { got: 7 });
    assert!(sink.0.is_empty());
}

#[test]
fn write_rejects_writing_to_stdin() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    let mut host = WDH::new();

    let err = host
        .dispatch(&mut vm, SERVICE_WRITE, &block(0, ENCODING_ASCII, b"data"))
        .unwrap_err();
    assert_eq!(err, HostError::UnwritableStream { got: 0 });
}

#[test]
fn write_to_rejects_an_unknown_encoding() {
    let mut sink = VecWriter(Vec::new());
    let err = write::write_to(&mut sink, &block(STREAM_STDOUT, 7, b"data")).unwrap_err();
    assert_eq!(err, HostError::UnknownEncoding { got: 7 });
    assert!(sink.0.is_empty());
}

#[test]
fn write_to_rejects_invalid_ascii() {
    let mut sink = VecWriter(Vec::new());

    let err =
        write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_ASCII, b"ab\x80")).unwrap_err();
    assert_eq!(err, HostError::InvalidASCII { offset: 2 });

    let err = write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_ASCII, b"\xC3\xA9"),
    )
    .unwrap_err();
    assert_eq!(err, HostError::InvalidASCII { offset: 0 });
}

#[test]
fn write_to_rejects_invalid_utf8() {
    let mut sink = VecWriter(Vec::new());

    let err =
        write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_UTF8, b"ab\xFF")).unwrap_err();
    assert_eq!(err, HostError::InvalidUTF8 { offset: 2 });

    let err =
        write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_UTF8, b"\xC3")).unwrap_err();
    assert_eq!(err, HostError::InvalidUTF8 { offset: 0 });
}

#[test]
fn write_to_rejects_an_odd_number_of_utf16_bytes() {
    let mut sink = VecWriter(Vec::new());
    let err = write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_UTF16, &[0x41, 0x00, 0x42]),
    )
    .unwrap_err();
    assert_eq!(err, HostError::InvalidUTF16 { offset: 3 });
    assert!(sink.0.is_empty());
}

#[test]
fn write_to_rejects_a_lone_surrogate() {
    let mut sink = VecWriter(Vec::new());

    let err = write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_UTF16, &0xD800u16.to_le_bytes()),
    )
    .unwrap_err();
    assert_eq!(err, HostError::InvalidUTF16 { offset: 0 });

    let err = write::write_to(
        &mut sink,
        &block(STREAM_STDOUT, ENCODING_UTF16, &0xDC00u16.to_le_bytes()),
    )
    .unwrap_err();
    assert_eq!(err, HostError::InvalidUTF16 { offset: 0 });
}

#[test]
fn write_to_rejects_a_high_surrogate_not_followed_by_a_low_one() {
    let mut sink = VecWriter(Vec::new());
    let data: Vec<u8> = [0xD800, 0x0041]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let err = write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_UTF16, &data)).unwrap_err();
    assert_eq!(err, HostError::InvalidUTF16 { offset: 0 });
    assert!(sink.0.is_empty());
}

#[test]
fn parse_args_parses_stream_encoding_and_data() {
    let args = block(STREAM_STDERR, ENCODING_UTF16, b"data");
    let (stream, encoding, data) = write::parse_args(&args).unwrap();
    assert_eq!(stream, wdh::service::stream::Stream::Stderr);
    assert_eq!(encoding, Encoding::UTF16);
    assert_eq!(data, b"data");
}

#[test]
fn write_to_reports_the_failed_write() {
    let mut sink = FailingWriter;
    let err =
        write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_ASCII, b"data")).unwrap_err();
    assert_eq!(
        err,
        HostError::WriteFailed {
            kind: io::ErrorKind::BrokenPipe
        }
    );
}

#[test]
fn write_to_reports_the_failed_flush() {
    struct FlushFailingWriter;

    impl Write for FlushFailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }

    let mut sink = FlushFailingWriter;
    let err =
        write::write_to(&mut sink, &block(STREAM_STDOUT, ENCODING_ASCII, b"data")).unwrap_err();
    assert_eq!(
        err,
        HostError::WriteFailed {
            kind: io::ErrorKind::Other
        }
    );
}

#[test]
fn dispatch_continues_on_write() {
    let mut vm = WVM::new(16, ExecuteVariant::default());
    let mut host = WDH::new();

    let decision = host
        .dispatch(
            &mut vm,
            SERVICE_WRITE,
            &block(STREAM_STDOUT, ENCODING_ASCII, b""),
        )
        .expect("an empty string writes nothing and succeeds");
    assert_eq!(decision, HostDecision::Continue);
}

#[test]
fn dispatch_notifies_the_observer_on_write() {
    use std::{cell::RefCell, rc::Rc};

    let calls = Rc::new(RefCell::new(Vec::new()));
    let calls_for_observer = calls.clone();

    struct RecordingObserver(Rc<RefCell<Vec<(u64, usize)>>>);

    impl wdh::observer::Observer for RecordingObserver {
        fn on_vmcall(&mut self, service: u64, args: &[u8]) {
            self.0.borrow_mut().push((service, args.len()));
        }
    }

    let mut vm = WVM::new(16, ExecuteVariant::default());
    let mut host = WDH::new().with_observer(RecordingObserver(calls_for_observer));

    host.dispatch(
        &mut vm,
        SERVICE_WRITE,
        &block(STREAM_STDOUT, ENCODING_ASCII, b""),
    )
    .unwrap();

    assert_eq!(*calls.borrow(), vec![(SERVICE_WRITE, 2)]);
}
