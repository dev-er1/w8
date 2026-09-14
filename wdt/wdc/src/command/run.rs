// wdc/src/command/run.rs
//
//! Execution of the `run` command.
use std::{cell::RefCell, io::Read, path::Path, rc::Rc, time::Instant};

use libw8::{
    BytecodeSource, ExecuteVariant, VMCallDecision, W8, W8Assembler, WVMError, WVMErrorKind,
};
use wdh::{HostDecision, WDH};

use crate::{ansi::ansi_supported, ansiprint};

/// The shared host state: the `wdh` host itself and the first error
/// that a service returned, if any.
struct SharedHost {
    wdh: WDH,
    error: Option<wdh::error::HostError>,
}

pub struct RunArguments {
    pub file: String,
    pub time: bool,
    pub memory: Option<usize>,
    pub executeby: ExecuteVariant,
}

pub fn run(args: RunArguments) -> i32 {
    let start = Instant::now();

    let source = if args.file == "-" {
        // Read the bytecode from stdin.
        let mut bytes = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut bytes) {
            report_error(WVMError::new(
                WVMErrorKind::IoError(e),
                None,
                ansi_supported(),
            ));
            return 1;
        }
        BytecodeSource::Bytes(bytes)
    } else if is_assembly(&args.file) {
        // An W8 Assembly file: compile it into instructions and execute.
        let instructions = match W8Assembler::assemble_from_path(&args.file) {
            Ok(instructions) => instructions,
            Err(e) => {
                e.report();
                return 1;
            }
        };

        BytecodeSource::Instructions(instructions)
    } else {
        BytecodeSource::File(args.file.into())
    };

    let w8 = if let Some(memory) = args.memory {
        W8::with_memory_size(memory)
    } else {
        W8::new()
    };

    // The host of the VM. It owns the default services (their exit codes,
    // `VMCALL`s, etc.) and answers every `VMCALL` request of the program.
    //
    // The host is shared with the dispatcher of `run_with` (which requires
    // a `'static` closure) through `Rc`; it is taken back when the VM run
    // is finished.
    let host = Rc::new(RefCell::new(SharedHost {
        wdh: WDH::new(),
        error: None,
    }));
    let host_for_closure = host.clone();

    let exit_code = match w8.run_with(
        source,
        move |vm, service, address, size| {
            // The VM has already checked the bounds of the arguments block, so
            // the slice below is always valid. It is copied because some
            // services (for example, `IsInTerminal`) write the result into
            // the VM and therefore need a mutable access to it.
            let args = {
                let memory = vm.memory.as_slice();
                let start = address as usize;
                &memory[start..start + size as usize]
            }
            .to_vec();

            let mut host = host_for_closure.borrow_mut();
            match host.wdh.dispatch(vm, service, &args) {
                Ok(HostDecision::Continue) => VMCallDecision::Continue,
                Ok(HostDecision::Exit { code }) => VMCallDecision::Exit { code },
                Err(err) => {
                    host.error = Some(err);
                    VMCallDecision::Exit { code: 1 }
                }
            }
        },
        args.executeby,
    ) {
        Ok(exit_code) => exit_code,
        Err(e) => {
            report_error(e);
            return 1;
        }
    };

    // The VM run is finished and the dispatcher is dropped, so this `Rc`
    // is unique again and the host can be taken back.
    let host = Rc::into_inner(host).expect("the host is shared only with the VM run");
    let host = host.into_inner();

    if let Some(err) = host.error {
        ansiprint!("\x1b[1;31mError\x1b[0m: \x1b[1m{err}\x1b[0m.");
        return 1;
    }

    if args.time {
        ansiprint!(
            "\n\x1b[1;36mFinished\x1b[0m in \x1b[1m{:?}\x1b[0m",
            start.elapsed()
        );
    }

    // The `run` command terminates with the exit code of the program.
    exit_code.map_or(0, |code| i32::try_from(code).unwrap_or(1))
}

/// Whether the file is an W8 Assembly source (`.wa`).
fn is_assembly(file: &str) -> bool {
    Path::new(file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wa"))
}

/// Prints an execution error in the same style as the other CLI errors.
fn report_error(e: WVMError) {
    let e = WVMError::new(e.kind, e.instruction, ansi_supported());
    e.report();
}
