// wdc/src/command/help.rs
//
//! Execution of the `help` command.
use crate::{
    ansi::unicode_supported,
    ansiprint,
    cmd::{COMMAND, CommandInfo},
    error::{CLIError, CLIErrorKind},
};

pub struct HelpArguments {
    pub cmd: Option<String>,
}

pub fn help(args: HelpArguments) -> i32 {
    if let Some(cmd) = args.cmd {
        if let Some(index) = COMMAND.iter().position(|arg| arg.name == cmd) {
            print_command_info(index);
            return 0;
        } else {
            let err = CLIError::new(CLIErrorKind::UnknownCommand(cmd), None, None);
            err.report();
            return 1;
        }
    }

    // An em dash is a Unicode character, so we check Unicode support.
    if unicode_supported() {
        ansiprint!("\x1b[1;33mUsage\x1b[0m:");
        println!("  w8c <command> [flags]");
        println!();
        ansiprint!("\x1b[1;33mCommands\x1b[0m:");
        for cmd in COMMAND {
            print_command(cmd);
        }
    } else {
        ansiprint!("\x1b[1;33mUsage\x1b[0m:");
        println!("    w8c <command> [flags]");
        println!();
        ansiprint!("\x1b[1;33mCommands\x1b[0m:");
        for cmd in COMMAND {
            print_command(cmd);
        }
    }

    0
}

fn print_command(cmd: &CommandInfo) {
    ansiprint!("    \x1b[1m{:<10}\x1b[0m  {}", cmd.name, cmd.description);
}

fn print_command_info(cmd_index: usize) {
    let cmd = &COMMAND[cmd_index];
    ansiprint!("\x1b[1m{}\x1b[0m", cmd.name);
    println!();
    ansiprint!("\x1b[1;33mUsage\x1b[0m:");
    print!("    w8c {} ", cmd.usage);
    let mut flags = String::new();
    for flag in cmd.flags {
        flags.push_str(format!("[{}] ", flag.usage).as_str());
    }
    println!("{flags}");
    println!();
    ansiprint!("\x1b[1;33mDescription\x1b[0m:");
    ansiprint!("    {}", cmd.description);
    println!();
    ansiprint!("\x1b[1;33mFlags\x1b[0m:");
    for flag in cmd.flags {
        println!("  \x1b[1m{:<18}\x1b[0m  {}", flag.usage, flag.description);
    }
}
