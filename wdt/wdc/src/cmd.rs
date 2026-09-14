// wdc/src/cmd.rs
//
//! Definition of the commands.
use libw8::ExecuteVariant;

pub enum Command {
    // `w8c help [--info <command>] [--dont-show-banner]`
    Help {
        /// The command to show information about.
        cmd: Option<String>,
    },

    /// `w8c run <file> [--time] [--memory <bytes]`
    Run {
        /// Path to the file to execute.
        file: String,

        /// If `true` — print the execution time.
        time: bool,

        /// How much memory to allocate for program execution.
        memory: Option<usize>,

        execute: ExecuteVariant,
    },

    /// `w8c compile <file> [--output <path>] [--time]`
    Compile {
        /// Path to the W8 Assembly (`.wa`) file.
        file: String,

        /// Where to write the resulting `.wb` file.
        ///
        /// If `None` — next to the source file.
        output: Option<String>,

        /// If `true` — print the compilation time.
        time: bool,
    },

    /// `w8c check <file> [--time]`
    Check {
        /// Path to the W8 Assembly (`.wa`) file.
        file: String,

        /// If `true` — print the check time.
        time: bool,
    },

    /// `w8c version`
    Version,
}

// `*Info` and `const COMMAND` are needed only to display information about
// any command.

#[derive(Debug, Clone, Copy)]
pub struct FlagInfo {
    pub usage: &'static str,
    pub description: &'static str,
}

pub struct CommandInfo {
    pub name: &'static str,

    pub usage: &'static str,
    pub description: &'static str,
    pub flags: &'static [FlagInfo],
}

pub const COMMAND: &[CommandInfo] = &[
    CommandInfo {
        name: "help",
        usage: "help",
        description: "Display help.",
        flags: &[FlagInfo {
            usage: "--info <command>",
            description: "Show information about <command>.",
        }],
    },
    CommandInfo {
        name: "run",
        usage: "run <file>",
        description: "Execute W8 Bytecode.",
        flags: &[
            FlagInfo {
                usage: "--time",
                description: "Show execution time.",
            },
            FlagInfo {
                usage: "--memory <bytes>",
                description: "Allocate the specified amount of memory for program execution.",
            },
            FlagInfo {
                usage: "--execute-by <variant>",
                description: "Execute the program using the specified option (`inter` -- interpreter, `jit` -- JIT compiler).",
            },
        ],
    },
    CommandInfo {
        name: "compile",
        usage: "compile <file>",
        description: "Compile W8 Assembly to W8 Bytecode.",
        flags: &[
            FlagInfo {
                usage: "--output <path>",
                description: "Write the output to <path> instead of the default .wb file.",
            },
            FlagInfo {
                usage: "--time",
                description: "Show compilation time.",
            },
        ],
    },
    CommandInfo {
        name: "check",
        usage: "check <file>",
        description: "Check an W8 Assembly file for errors.",
        flags: &[FlagInfo {
            usage: "--time",
            description: "Show check time.",
        }],
    },
    CommandInfo {
        name: "version",
        usage: "version",
        description: "Print W8 version.",
        flags: &[],
    },
];
