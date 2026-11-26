#![forbid(unsafe_code)]
//! `swell`, the shell of the engine (`docs/design/quality-and-comparison.md` § 7): runs one
//! script, from a file or from `-e`, and exits with a status per outcome. It is a host, not the
//! engine: it reads the file, writes standard output and standard error, answers the clock and
//! interrupts a run after `--timeout`; the engine keeps no ambient authority.
//!
//! `swell [flags] <file.js>` or `swell [flags] -e <source>`. The flags and the exit statuses are
//! [`args::USAGE`].

mod args;
mod shell;

use std::process::ExitCode;
use std::thread;

use args::{Command, Script};

fn main() -> ExitCode {
    let command = match args::parse(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(message) => return usage_error(&message),
    };
    let options = match command {
        Command::Help => {
            print!("{}", args::USAGE);
            return ExitCode::SUCCESS;
        }
        Command::Version => {
            println!("swell {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Command::Run(options) => options,
    };
    let text = match &options.script {
        Script::Source(text) => text.clone(),
        Script::File(path) => match std::fs::read(path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(_) => return usage_error(&format!("{} is not UTF-8", path.display())),
            },
            Err(error) => return usage_error(&format!("cannot read {}: {error}", path.display())),
        },
    };
    let stack = options.limits.min_stack_bytes();
    let spawned = thread::Builder::new()
        .name("swell".to_string())
        .stack_size(stack)
        .spawn(move || shell::run(&text, options.limits, options.timeout));
    let code = match spawned {
        Ok(handle) => match handle.join() {
            Ok(code) => code,
            Err(_) => {
                eprintln!("swell: the run panicked");
                shell::FAULT
            }
        },
        Err(error) => {
            return usage_error(&format!(
                "cannot start a thread with {stack} bytes of stack for these limits: {error}"
            ));
        }
    };
    ExitCode::from(code)
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("swell: {message}");
    eprintln!(
        "usage: swell [flags] <file.js> | swell [flags] -e <source>; swell --help names the flags"
    );
    ExitCode::from(args::USAGE_ERROR)
}
