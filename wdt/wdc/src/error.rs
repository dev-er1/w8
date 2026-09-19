// wdc/src/error.rs
//
//! CLI errors.
use std::{
    fmt::{self, Display, Formatter},
    iter::repeat_n,
};

use crate::ansiprint;

pub enum CLIErrorKind {
    /// Unknown flag.
    UnknownFlag(String),

    /// Unexpected value for a flag.
    ///
    /// The error occurs when a value is given to a flag
    /// that does not require one.
    ///
    /// ## Example
    /// ```text
    /// w8c help --show-banner 67
    ///                        ^^
    /// ```
    UnexpectedValue(
        // The flag.
        String,
        // The value.
        String,
    ),

    /// Unknown command.
    UnknownCommand(String),

    /// No value for a flag that requires one.
    MissingValueForFlag(
        // The flag.
        String,
    ),

    /// No value for a command that requires one.
    MissingValueForCommand(
        // The command.
        String,
    ),

    /// Unexpected argument.
    ///
    /// The error occurs when a command receives more arguments
    /// than it expects.
    ///
    /// ## Example
    /// ```text
    /// w8c run prog.wb extra
    ///              ^^^^^^^^
    /// ```
    UnexpectedArgument(
        // The argument.
        String,
    ),

    /// Invalid value.
    ///
    /// For example, the flag expects a [`u64`], but receives a negative number.
    InvalidValue(
        /// The flag name.
        String,
    ),
}

impl Display for CLIErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFlag(flag) => write!(f, "unknown flag: `{flag}`"),
            Self::UnexpectedValue(flag, value) => {
                write!(f, "unexpected value in flag `{flag}`: `{value}`")
            }
            Self::UnknownCommand(cmd) => write!(f, "unknown command: `{cmd}`"),
            Self::MissingValueForFlag(flag) => write!(f, "missing value for flag `{flag}`"),
            Self::MissingValueForCommand(cmd) => write!(f, "missing value for command `{cmd}`"),
            Self::UnexpectedArgument(arg) => write!(f, "unexpected argument: `{arg}`"),
            Self::InvalidValue(flag) => write!(f, "invalid value for flag `{flag}`"),
        }
    }
}

pub struct CLIError {
    pub kind: CLIErrorKind,

    /// CLI arguments for the pretty-print of the error.
    pub raw_args: Option<Vec<String>>,

    /// Which arguments in `raw_args` are wrong.
    /// Needed to show the error.
    pub which_args: Option<Vec<usize>>,
}

impl CLIError {
    pub fn new(
        kind: CLIErrorKind,
        raw_args: Option<Vec<String>>,
        which_args: Option<Vec<usize>>,
    ) -> Self {
        Self {
            kind,
            raw_args,
            which_args,
        }
    }

    pub fn report(&self) {
        ansiprint!("\x1b[1;31mError\x1b[0m: \x1b[1m{}\x1b[0m.", self.kind);
        println!();

        if let Some(raw_args) = &self.raw_args
            && let Some(which_args) = &self.which_args
        {
            // The original command line.
            let args = raw_args.join(" ");

            ansiprint!("\x1b[36m-->\x1b[0m  w8c {args}");

            // Pointers to the erroneous arguments.
            let mut pointers = String::new();

            for (i, arg) in raw_args.iter().enumerate() {
                if which_args.contains(&(i)) {
                    pointers.extend(repeat_n('^', arg.len()));
                } else {
                    pointers.extend(repeat_n(' ', arg.len()));
                }

                // A space between arguments.
                if i + 1 < raw_args.len() {
                    pointers.push(' ');
                }
            }

            ansiprint!("         \x1b[1;31m{pointers}\x1b[0m");
            println!();
        }
    }
}
