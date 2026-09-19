mod ansi;
mod cmd;
mod command;
mod error;
mod parser;

use crate::{command::route, parser::parse};

fn main() {
    let parsed = parse();
    let exit_code = match parsed {
        Ok(cmd) => route(cmd),
        Err(err) => {
            err.report();
            1
        }
    };
    std::process::exit(exit_code);
}
