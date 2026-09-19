//! This module contains the executors of [`Command`].
pub mod check;
pub mod compile;
pub mod help;
pub mod run;

use crate::{
    ansiprint,
    cmd::Command,
    command::{
        check::CheckArguments, compile::CompileArguments, help::HelpArguments, run::RunArguments,
    },
};

use libw8::W8_VERSION;

pub fn route(cmd: Command) -> i32 {
    match cmd {
        Command::Help { cmd } => help::help(HelpArguments { cmd }),
        Command::Run {
            file,
            time,
            memory,
            execute,
            is_assembly,
        } => run::run(RunArguments {
            file,
            time,
            memory,
            executeby: execute,
            is_assembly,
        }),
        Command::Compile { file, output, time } => {
            compile::compile(CompileArguments { file, output, time })
        }
        Command::Check { file, time } => check::check(CheckArguments { file, time }),
        Command::Version => {
            ansiprint!("\x1b[1mv{W8_VERSION}\x1b[0m");
            0
        }
    }
}
