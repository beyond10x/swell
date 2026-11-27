//! `swell-bench-exec`: runs one engine process for the benchmark harness of `bench/` and prints
//! one line, `wall_us=<n> maxrss_kib=<n> exit=<code|-> signal=<n|-> timed_out=<0|1>`.
//!
//! The wall time is taken from just before the process is spawned to just after it is reaped
//! (process start to exit, K3). The peak resident set is `ru_maxrss` of the reaped children
//! (`getrusage(RUSAGE_CHILDREN)`, what GNU `time` prints as `%M`, K4); since this tool reaps
//! exactly one child, it is that child's peak. One measurement per invocation: the harness
//! starts this tool once per run.
//!
//! Usage: `swell-bench-exec [--stdout FILE] [--stderr FILE] [--timeout SECONDS] -- PROGRAM ARGS...`
#![forbid(unsafe_code)]

use std::fs::File;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use nix::sys::resource::{UsageWho, getrusage};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

struct Options {
    stdout: Option<String>,
    stderr: Option<String>,
    timeout: Option<u64>,
    program: String,
    args: Vec<String>,
}

fn parse(mut argv: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut stdout = None;
    let mut stderr = None;
    let mut timeout = None;
    loop {
        let Some(arg) = argv.next() else {
            return Err("no -- before the program".to_string());
        };
        match arg.as_str() {
            "--stdout" => stdout = Some(argv.next().ok_or("--stdout needs a file")?),
            "--stderr" => stderr = Some(argv.next().ok_or("--stderr needs a file")?),
            "--timeout" => {
                let value = argv.next().ok_or("--timeout needs seconds")?;
                let seconds = value
                    .parse::<u64>()
                    .map_err(|_| format!("--timeout {value} is not a number of seconds"))?;
                timeout = Some(seconds);
            }
            "--" => break,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let program = argv.next().ok_or("no program after --")?;
    Ok(Options {
        stdout,
        stderr,
        timeout,
        program,
        args: argv.collect(),
    })
}

fn sink(path: Option<&String>) -> Result<Stdio, String> {
    match path {
        None => Ok(Stdio::null()),
        Some(path) => File::create(path)
            .map(Stdio::from)
            .map_err(|error| format!("cannot create {path}: {error}")),
    }
}

fn run(options: &Options) -> Result<String, String> {
    let stdout = sink(options.stdout.as_ref())?;
    let stderr = sink(options.stderr.as_ref())?;
    let timed_out = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let mut child = Command::new(&options.program)
        .args(&options.args)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", options.program))?;
    let (done, watch) = mpsc::channel::<()>();
    let watchdog = options.timeout.and_then(|seconds| {
        let pid = i32::try_from(child.id()).ok()?;
        let flag = Arc::clone(&timed_out);
        Some(std::thread::spawn(move || {
            if watch.recv_timeout(Duration::from_secs(seconds)).is_err() {
                flag.store(true, Ordering::SeqCst);
                let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
            }
        }))
    });
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for {}: {error}", options.program))?;
    let wall = start.elapsed();
    let _ = done.send(());
    if let Some(handle) = watchdog {
        let _ = handle.join();
    }
    let usage = getrusage(UsageWho::RUSAGE_CHILDREN)
        .map_err(|error| format!("getrusage failed: {error}"))?;
    let exit = status
        .code()
        .map_or_else(|| "-".to_string(), |code| code.to_string());
    let signal = status
        .signal()
        .map_or_else(|| "-".to_string(), |signal| signal.to_string());
    Ok(format!(
        "wall_us={} maxrss_kib={} exit={exit} signal={signal} timed_out={}",
        wall.as_micros(),
        usage.max_rss(),
        u8::from(timed_out.load(Ordering::SeqCst)),
    ))
}

fn main() -> ExitCode {
    let options = match parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("swell-bench-exec: {message}");
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("swell-bench-exec: {message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    fn words(line: &str) -> impl Iterator<Item = String> {
        line.split(' ')
            .map(str::to_string)
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn parses_files_timeout_and_program() {
        let options = parse(words("--stdout o --stderr e --timeout 7 -- prog a b"));
        let Ok(options) = options else {
            unreachable!("refused a valid line");
        };
        assert_eq!(options.stdout.as_deref(), Some("o"));
        assert_eq!(options.stderr.as_deref(), Some("e"));
        assert_eq!(options.timeout, Some(7));
        assert_eq!(options.program, "prog");
        assert_eq!(options.args, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn refuses_a_line_without_a_program() {
        assert!(parse(words("--timeout 3")).is_err());
        assert!(parse(words("--timeout x -- prog")).is_err());
        assert!(parse(words("--")).is_err());
    }
}
