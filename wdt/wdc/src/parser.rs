// wdc/src/parser.rs
//
//! Conversion of a command line string into a [`Command`].
//!
//! We use `argparser` because it does not pull in other external
//! dependencies—making compilation faster—and also because it is MINE.
//! (Holy Rick And Morty reference🥀)
use argparser::{ArgumentParser, ParseError};
use libw8::ExecuteVariant;

use crate::{
    cmd::Command,
    error::{CLIError, CLIErrorKind},
};

pub fn parse() -> Result<Command, CLIError> {
    let raw: Vec<String> = std::env::args().skip(1).collect();

    if raw.is_empty() {
        return Ok(Command::Help { cmd: None });
    }

    let command_name = raw[0].as_str();

    match command_name {
        "help" => parse_help(&raw),
        "run" => parse_run(&raw),
        "compile" => parse_compile(&raw),
        "check" => parse_check(&raw),
        "version" => Ok(Command::Version),
        cmd => Err(CLIError::new(
            CLIErrorKind::UnknownCommand(cmd.to_string()),
            Some(raw),
            Some(vec![0]),
        )),
    }
}

fn parse_help(args: &[String]) -> Result<Command, CLIError> {
    let matches = ArgumentParser::new()
        .flag_with_value("info", &["--info"])
        .parse(argparser::str::Source::from_iter(args.iter().cloned()));

    if let Some(err) = matches.errors.clone().first() {
        return Err(CLIError::new(
            error_kind(err),
            Some(args.to_vec()),
            // For an erroneous flag or value, look up the argument index.
            // If not found — omit the pointer (`None`).
            find_arg_index(args, offending_token(err)).map(|index| vec![index]),
        ));
    }
    Ok(Command::Help {
        cmd: matches.value("info").map(str::to_owned),
    })
}

fn parse_run(args: &[String]) -> Result<Command, CLIError> {
    let matches = ArgumentParser::new()
        .typed_value::<usize>("memory", &["--memory"])
        .flag("time", &["--time"])
        .typed_value::<String>("execute-by", &["--execute-by"])
        .parse(argparser::str::Source::from_iter(args.iter().cloned()));

    if let Some(err) = matches.errors.clone().first() {
        return Err(CLIError::new(
            error_kind(err),
            Some(args.to_vec()),
            find_arg_index(args, offending_token(err)).map(|index| vec![index]),
        ));
    }

    // `positional()[0]` is always the command name (“run”), since we parse
    // the full arguments. The file, therefore, is at index `1`.
    let file = matches.get(1).ok_or_else(|| {
        CLIError::new(
            CLIErrorKind::MissingValueForCommand("run".to_string()),
            Some(args.to_vec()),
            Some(vec![0]),
        )
    })?;

    if let Some(extra) = matches.get(2) {
        return Err(CLIError::new(
            CLIErrorKind::UnexpectedArgument(extra.to_string()),
            Some(args.to_vec()),
            find_arg_index(args, extra).map(|index| vec![index]),
        ));
    }

    let execute = if let Some(s) = matches.get_one::<String>("execute-by") {
        match s.as_str() {
            "inter" => ExecuteVariant::ByInterpreter,
            "jit" => ExecuteVariant::ByJIT,
            _ => {
                return Err(CLIError::new(
                    CLIErrorKind::UnexpectedValue("--execute-by".to_string(), s.to_string()),
                    Some(args.to_vec()),
                    find_arg_index(args, s).map(|index| vec![index]),
                ));
            }
        }
    } else {
        ExecuteVariant::default()
    };

    Ok(Command::Run {
        file: file.to_string(),
        time: matches.flag("time"),
        memory: matches.get_one::<usize>("memory").copied(),
        execute,
    })
}

fn parse_compile(args: &[String]) -> Result<Command, CLIError> {
    let matches = ArgumentParser::new()
        .flag_with_value("output", &["--output"])
        .flag("time", &["--time"])
        .parse(argparser::str::Source::from_iter(args.iter().cloned()));

    if let Some(err) = matches.errors.clone().first() {
        return Err(CLIError::new(
            error_kind(err),
            Some(args.to_vec()),
            find_arg_index(args, offending_token(err)).map(|index| vec![index]),
        ));
    }

    // `positional()[0]` is always the command name (“compile”), since we parse
    // the full arguments. The file, therefore, is at index `1`.
    let file = matches.get(1).ok_or_else(|| {
        CLIError::new(
            CLIErrorKind::MissingValueForCommand("compile".to_string()),
            Some(args.to_vec()),
            Some(vec![0]),
        )
    })?;

    if let Some(extra) = matches.get(2) {
        return Err(CLIError::new(
            CLIErrorKind::UnexpectedArgument(extra.to_string()),
            Some(args.to_vec()),
            find_arg_index(args, extra).map(|index| vec![index]),
        ));
    }

    Ok(Command::Compile {
        file: file.to_string(),
        output: matches.value("output").map(str::to_owned),
        time: matches.flag("time"),
    })
}

fn parse_check(args: &[String]) -> Result<Command, CLIError> {
    let matches = ArgumentParser::new()
        .flag("time", &["--time"])
        .parse(argparser::str::Source::from_iter(args.iter().cloned()));

    if let Some(err) = matches.errors.clone().first() {
        return Err(CLIError::new(
            error_kind(err),
            Some(args.to_vec()),
            find_arg_index(args, offending_token(err)).map(|index| vec![index]),
        ));
    }

    // `positional()[0]` is always the command name (“check”), since we parse
    // the full arguments. The file, therefore, is at index `1`.
    let file = matches.get(1).ok_or_else(|| {
        CLIError::new(
            CLIErrorKind::MissingValueForCommand("check".to_string()),
            Some(args.to_vec()),
            Some(vec![0]),
        )
    })?;

    if let Some(extra) = matches.get(2) {
        return Err(CLIError::new(
            CLIErrorKind::UnexpectedArgument(extra.to_string()),
            Some(args.to_vec()),
            find_arg_index(args, extra).map(|index| vec![index]),
        ));
    }

    Ok(Command::Check {
        file: file.to_string(),
        time: matches.flag("time"),
    })
}

fn error_kind(err: &ParseError) -> CLIErrorKind {
    match err {
        ParseError::UnknownFlag(flag) => CLIErrorKind::UnknownFlag(flag.clone()),
        ParseError::UnexpectedValue { flag, value } => {
            CLIErrorKind::UnexpectedValue(flag.clone(), value.clone())
        }
        ParseError::MissingValue(flag) => CLIErrorKind::MissingValueForFlag(flag.clone()),
        ParseError::InvalidValue { flag, .. } => CLIErrorKind::InvalidValue(flag.clone()),
    }
}

/// Returns the “token” to point at in the original command line.
///
/// In `MissingValue` and `UnexpectedValue` errors `argparser` reports the flag
/// name without leading dashes, so we assemble the token in all possible forms.
fn offending_token(err: &ParseError) -> &str {
    match err {
        ParseError::UnknownFlag(flag) => flag,
        ParseError::UnexpectedValue { flag, .. } => flag,
        ParseError::MissingValue(flag) => flag,
        ParseError::InvalidValue { flag, .. } => flag,
    }
}

/// Searches for the index of the argument matching the flag or value from the error.
///
/// The comparison uses a “normalized” name: without leading dashes
/// and without the `=...` part (for flags of the form `--flag=value`).
///
/// For example, for the flag `info` the arguments `info`, `--info`,
/// `-info` and `--info=value` all match.
fn find_arg_index(args: &[String], needle: &str) -> Option<usize> {
    let needle = normalize(needle);

    args.iter().position(|arg| normalize(arg) == needle)
}

fn normalize(arg: &str) -> &str {
    let arg = arg.trim_start_matches('-');
    match arg.split_once('=') {
        Some((name, _)) => name,
        None => arg,
    }
}
