// wdc/src/command/check.rs
//
//! Execution of the `check` command.
use std::time::Instant;

use libw8::W8Assembler;

use crate::ansiprint;

pub struct CheckArguments {
    pub file: String,
    pub time: bool,
}

pub fn check(args: CheckArguments) -> i32 {
    let start = Instant::now();

    if let Err(e) = W8Assembler::assemble_from_path(&args.file) {
        e.report();
        return 1;
    }

    ansiprint!("\x1b[1;32mChecked\x1b[0m \x1b[1m{}\x1b[0m", args.file);

    if args.time {
        ansiprint!(
            "\x1b[1;36mFinished\x1b[0m in \x1b[1m{:?}\x1b[0m",
            start.elapsed()
        );
    }

    0
}
