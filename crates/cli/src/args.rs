//! The command line of `swell`: one flag per field of `Limits`, `--timeout`, `-e` and `--help`.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use swell::Limits;

/// The exit status of a usage error (`sysexits.h`, `EX_USAGE`).
pub const USAGE_ERROR: u8 = 64;

/// The limits of a run whose flags name none: the limits the test262 runner runs every case
/// with (`crates/test262/src/run.rs`, `LIMITS`; design decision 11). The engine has no default
/// of its own (`Limits` has no `Default`), so the shell states one.
pub const DEFAULT_LIMITS: Limits = Limits {
    steps_per_entry: 1 << 30,
    memory_bytes: 512 << 20,
    frame_depth: 10_000,
    reentry_depth: 1_000,
    parser_nesting: 1_000,
    string_length: 1 << 30,
    bigint_bits: 1 << 30,
};

pub const USAGE: &str = "\
usage: swell [flags] <file.js>
       swell [flags] -e <source>

Runs one script in a new agent and realm, then its promise jobs. print(...) writes its
arguments, each converted to a string, joined by a space and ended by a newline, to standard
output; console.log is the same function. Date.now() reads the system clock.

flags (a number is decimal, or max for the largest value of its type; 0 is a limit of zero):
  --steps <n>          steps_per_entry, steps of the script and of the promise jobs (default 1073741824)
  --memory <bytes>     memory_bytes, bytes charged to the heap (default 536870912)
  --depth <n>          frame_depth, ECMAScript execution contexts on the stack (default 10000)
  --reentry <n>        reentry_depth, calls from Rust back into script (default 1000)
  --nesting <n>        parser_nesting, nesting levels of the parser (default 1000)
  --string-length <n>  string_length, UTF-16 code units of a string made at run time (default 1073741824)
  --bigint-bits <n>    bigint_bits, bits of a BigInt made at run time (default 1073741824)
  --timeout <ms>       interrupt the run after <ms> milliseconds of wall time (default none)
  -e <source>          run <source> instead of a file
  --version            print swell and the version of the package
  -h, --help           print this text

exit status:
  0   the script and its jobs ended normally
  1   an uncaught throw (its message on standard error)
  2   a syntax error
  3   the step budget
  4   the memory budget
  5   a depth limit: frame_depth, reentry_depth or parser_nesting
  6   interrupted (--timeout)
  64  a usage error, or a file that cannot be read
  70  an engine fault
";

/// What the command line asks for.
pub enum Command {
    Help,
    Version,
    Run(Options),
}

/// A run's script, limits and timeout.
pub struct Options {
    pub script: Script,
    pub limits: Limits,
    pub timeout: Option<Duration>,
}

pub enum Script {
    File(PathBuf),
    Source(String),
}

/// The command line `args`, the program name left out; `Err` holds the usage error's message.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let mut limits = DEFAULT_LIMITS;
    let mut timeout = None;
    let mut script = None;
    while let Some(arg) = args.next() {
        let Some(flag) = arg.to_str().filter(|a| a.starts_with('-')) else {
            set(&mut script, Script::File(PathBuf::from(arg)))?;
            continue;
        };
        if flag == "-h" || flag == "--help" {
            return Ok(Command::Help);
        }
        if flag == "--version" {
            return Ok(Command::Version);
        }
        let flag = flag.to_string();
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?
            .into_string()
            .map_err(|_| format!("the value of {flag} is not UTF-8"))?;
        match flag.as_str() {
            "-e" => set(&mut script, Script::Source(value))?,
            "--steps" => limits.steps_per_entry = number(&flag, &value)?,
            "--memory" => limits.memory_bytes = number(&flag, &value)?,
            "--depth" => limits.frame_depth = number(&flag, &value)?,
            "--reentry" => limits.reentry_depth = number(&flag, &value)?,
            "--nesting" => limits.parser_nesting = number(&flag, &value)?,
            "--string-length" => limits.string_length = number(&flag, &value)?,
            "--bigint-bits" => limits.bigint_bits = number(&flag, &value)?,
            "--timeout" => timeout = Some(Duration::from_millis(number(&flag, &value)?)),
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    let script = script.ok_or_else(|| "no script: name a file or give -e <source>".to_string())?;
    Ok(Command::Run(Options {
        script,
        limits,
        timeout,
    }))
}

fn set(script: &mut Option<Script>, new: Script) -> Result<(), String> {
    if script.is_some() {
        return Err("more than one script: name one file or give one -e <source>".to_string());
    }
    *script = Some(new);
    Ok(())
}

/// A limit's value: decimal digits, or `max` for the type's largest value.
fn number<T: std::str::FromStr + Max>(flag: &str, value: &str) -> Result<T, String> {
    if value == "max" {
        return Ok(T::MAX);
    }
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "{flag} takes a decimal number or max, not {value:?}"
        ));
    }
    value
        .parse()
        .map_err(|_| format!("{flag} {value} is too large"))
}

/// The largest value of a limit's type.
trait Max {
    const MAX: Self;
}

impl Max for u32 {
    const MAX: Self = u32::MAX;
}

impl Max for u64 {
    const MAX: Self = u64::MAX;
}
