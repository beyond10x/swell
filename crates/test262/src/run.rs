//! One run of one case on the engine (design, decision 11; test262 `INTERPRETING.md`): a fresh
//! agent and realm per run with `print` and `$262` (`host.rs`), `assert.js` and `sta.js` (not for
//! `raw`), the `includes:` in order, then the case, with `"use strict";` and a line end in front
//! for a strict run, judged by its `negative`. Every run the runner makes has the limits of
//! [`LIMITS`], so a case that does not end is ended by a budget. Runs go to worker threads whose
//! stack is `min_stack_bytes()` of the limits, the stack the engine asks for them (HOST-17);
//! each worker reads a harness file once and parses it again for every run, since a parsed
//! script belongs to one agent.
//!
//! Each harness file and the case are separate scripts of one realm, evaluated in that order;
//! every script is parsed before the first one runs, so a case that does not parse runs nothing.
//! `async` cases are run like any other until slice 10 brings `doneprintHandle.js` and the job
//! queue: `$DONE` is not defined, so they fail. A module run fails without running until slice
//! 11.
//!
//! A parse error is a `SyntaxError` whatever its cause, and the engine's parser gives it for three
//! causes: an error ECMA-262 asks for, a construct of a later slice it refuses, and its nesting
//! limit (`swell_syntax::MessageKind`). Only the first is what a `negative: parse` case asks for.
//! A run whose harness file or case ends in a refusal or the nesting limit fails, whatever the
//! case's `negative` says; the kind is read from the `message` of the error object the engine
//! gives, through [`swell_syntax::message_kind`], and the source is not parsed again.
//!
//! `$262.evalScript` keeps that rule for the source it parses (`host.rs`): it throws the first
//! parse error as the steps say, and when that is a refusal or the nesting limit it marks the
//! run. After the case, a marked run ends failed whatever the case and its `negative` answered,
//! read as a run that wants another realm is read, with the detail `evalScript: refused:
//! <message>` or `evalScript: nesting: <message>`. A run marked for a refusal is a refused run
//! ([`Ran::refused`]) and counts in `test262-refused`; one marked for the nesting limit is not.
//!
//! A refusal reached through `eval`, `Function`, `$262.evalScript` or at run time (a regular
//! expression literal) is a `SyntaxError` the case may catch. The engine counts every refusal
//! it raises (`Agent::refusals`, `docs/design/host-interface.md` section 13), and the runner
//! reads the count once every script of the run parsed and again when the run ends: a run in
//! which it moved fails whatever the case and its `negative` answered, with the detail
//! `counted refusal: <n> refusal(s)`, after the detail of `$262.evalScript` when that marked the
//! run too, and is a refused run (`Ran::counted_refusals`). A refusal of a script's own parse
//! ends the run before the first read, with its own cause.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use swell::{
    Agent, Cx, Limits, ObjectRef, Outcome as Evaluated, Parsed, PropertyKey, Realm, Root, Seeds,
    Source, Termination, Value,
};
use swell_syntax::{MessageKind, message_kind};

use crate::count::{Case, Outcome, Run};
use crate::host::{self, EvalScriptMark, RunnerHost, SetupError};

/// The limits of every run, in one table. The two budgets are the design's first values (decision
/// 11: "2^30 steps and 512 MiB, chosen, not derived"); the twenty most expensive runs are printed
/// so that they can be set from data. The depth limits are chosen as well; each of them is a
/// recorded omission of the section whose algorithm has no limit
/// (`conformance/test262/sections.txt`: `parser_nesting` in ParseScript, `frame_depth` in
/// [[Call]] of ECMAScript functions, `string_length` in string concatenation).
pub const LIMITS: Limits = Limits {
    steps_per_entry: 1 << 30,
    memory_bytes: 512 << 20,
    frame_depth: 10_000,
    reentry_depth: 1_000,
    parser_nesting: 1_000,
    string_length: 1 << 30,
    bigint_bits: 1 << 30,
};

/// The seeds of every run: fixed, so that every run of a case is the same run (decision 13).
pub const SEEDS: Seeds = Seeds {
    hash_key: [0x26; 16],
    random_seed: [0x62; 32],
};

/// What one run gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    /// Passed, failed, or ended by a budget.
    pub outcome: Outcome,
    /// The steps every script of the run spent, harness files included.
    pub steps: u64,
    /// The budget that ended it, when `outcome` is `Budget`.
    pub budget: Option<Termination>,
    /// What ended it: the parse error, the thrown error or the termination; for a negative case
    /// that passed, the error it passed with; empty for any other pass.
    pub detail: String,
    /// The kind of the parse error that ended it, of a harness file, of the case or of the
    /// source of a `$262.evalScript` that marked the run.
    pub parse_error: Option<MessageKind>,
    /// How far the engine's count of refusals (`Agent::refusals`) moved while the run's scripts
    /// were evaluated: the refusals `eval`, `Function`, `$262.evalScript` or an instruction
    /// raised, caught or not. More than 0 fails the run.
    pub counted_refusals: u64,
    /// The run panicked; it is counted failed.
    pub panicked: bool,
    /// The time it took, with the runs made again for `createRealm`.
    pub elapsed: Duration,
}

impl Ran {
    fn ended(end: End, steps: u64) -> Ran {
        Ran {
            outcome: end.outcome,
            steps,
            budget: end.budget,
            detail: end.detail,
            parse_error: end.parse_error,
            counted_refusals: end.counted_refusals,
            panicked: false,
            elapsed: Duration::ZERO,
        }
    }

    /// The run ended in a refusal of the parser, of its own sources or inside
    /// `$262.evalScript`, or the engine counted a refusal while its scripts ran.
    pub fn refused(&self) -> bool {
        matches!(self.parse_error, Some(MessageKind::Refusal { .. })) || self.counted_refusals > 0
    }
}

/// The name of a run's mode as the runner prints it.
pub fn mode(run: Run) -> &'static str {
    match run {
        Run::NonStrict | Run::Raw => "non-strict",
        Run::Strict => "strict",
        Run::Module => "module",
    }
}

/// Runs every job, each a case and one of its runs, on `workers` threads with `limits` (the
/// runner gives [`LIMITS`]), and answers what each gave, in the order of `jobs`. Which worker runs
/// a job does not change what it gives.
pub fn run_all(
    checkout: &Path,
    jobs: &[(&Case, Run)],
    workers: NonZeroUsize,
    limits: &Limits,
) -> Result<Vec<Ran>, String> {
    let next = AtomicUsize::new(0);
    let done = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers.get());
        for n in 0..workers.get() {
            let next = &next;
            let handle = thread::Builder::new()
                .name(format!("test262-worker-{n}"))
                .stack_size(limits.min_stack_bytes())
                .spawn_scoped(scope, move || {
                    let mut worker = Worker::new(checkout, *limits);
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(case, run)) = jobs.get(index) else {
                            break;
                        };
                        done.push((index, worker.run_caught(case, run)));
                    }
                    done
                })
                .map_err(|e| format!("a worker thread cannot be started: {e}"))?;
            handles.push(handle);
        }
        let mut done = Vec::with_capacity(jobs.len());
        for handle in handles {
            done.extend(
                handle
                    .join()
                    .map_err(|_| "a worker thread panicked outside a run".to_string())?,
            );
        }
        Ok::<_, String>(done)
    })?;
    let mut slots: Vec<Option<Ran>> = vec![None; jobs.len()];
    for (index, ran) in done {
        if let Some(slot) = slots.get_mut(index) {
            *slot = Some(ran);
        }
    }
    slots
        .into_iter()
        .enumerate()
        .map(|(index, ran)| ran.ok_or_else(|| format!("run {index} gave no result")))
        .collect()
}

/// The harness files of a run, by name and text, and the text of its case.
type Sources = (Vec<(String, Rc<str>)>, String);

/// A worker: the harness files it has read, by name.
struct Worker<'p> {
    checkout: &'p Path,
    limits: Limits,
    harness: BTreeMap<String, Result<Rc<str>, String>>,
}

impl<'p> Worker<'p> {
    fn new(checkout: &'p Path, limits: Limits) -> Self {
        Worker {
            checkout,
            limits,
            harness: BTreeMap::new(),
        }
    }

    /// The run, timed; a panic is a failed run.
    fn run_caught(&mut self, case: &Case, run: Run) -> Ran {
        let start = Instant::now();
        let caught = panic::catch_unwind(AssertUnwindSafe(|| self.run(case, run)));
        let mut ran = caught.unwrap_or_else(|payload| {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            Ran {
                outcome: Outcome::Failed,
                steps: 0,
                budget: None,
                detail: format!("panicked: {message}"),
                parse_error: None,
                counted_refusals: 0,
                panicked: true,
                elapsed: Duration::ZERO,
            }
        });
        ran.elapsed = start.elapsed();
        ran
    }

    fn run(&mut self, case: &Case, run: Run) -> Ran {
        if run == Run::Module {
            return Ran::ended(End::failed("a module run is not made before slice 11"), 0);
        }
        let (harness, text) = match self.sources(case, run) {
            Ok(sources) => sources,
            Err(detail) => return Ran::ended(End::failed(detail), 0),
        };
        let negative = Negative::of(case);
        // `createRealm` takes realms made before the run; with too few, the run is made again
        // with twice as many. The memory budget bounds how many a run can make.
        let mut realms = 0usize;
        loop {
            match attempt(&harness, &text, &negative, realms, &self.limits) {
                Attempt::Done(ran) => return ran,
                Attempt::MoreRealms => realms = realms.saturating_mul(2).max(1),
            }
        }
    }

    /// The harness files of the run, by name and text, and the case's text as the run evaluates
    /// it.
    fn sources(&mut self, case: &Case, run: Run) -> Result<Sources, String> {
        let mut names: Vec<&str> = Vec::new();
        if run != Run::Raw {
            names.extend(["assert.js", "sta.js"]);
            names.extend(case.meta.include_list());
        }
        let mut harness = Vec::with_capacity(names.len());
        for name in names {
            harness.push((name.to_string(), self.harness_file(name)?));
        }
        let file = self.checkout.join(&case.path);
        let source =
            fs::read_to_string(&file).map_err(|e| format!("{}: cannot be read: {e}", case.path))?;
        let text = match run {
            Run::Strict => format!("\"use strict\";\n{source}"),
            Run::NonStrict | Run::Raw | Run::Module => source,
        };
        Ok((harness, text))
    }

    /// The text of `harness/<name>`, read once.
    fn harness_file(&mut self, name: &str) -> Result<Rc<str>, String> {
        let checkout = self.checkout;
        self.harness
            .entry(name.to_string())
            .or_insert_with(|| {
                fs::read_to_string(checkout.join("harness").join(name))
                    .map(Rc::from)
                    .map_err(|e| format!("harness/{name}: cannot be read: {e}"))
            })
            .clone()
    }
}

/// The `negative` of a case.
enum Negative<'c> {
    None,
    Parse(&'c str),
    Resolution(&'c str),
    Runtime(&'c str),
    Unknown(&'c str),
}

impl<'c> Negative<'c> {
    fn of(case: &'c Case) -> Self {
        let kind = case.meta.negative_type.as_str();
        match case.meta.negative_phase.as_str() {
            "" => Negative::None,
            "parse" => Negative::Parse(kind),
            "resolution" => Negative::Resolution(kind),
            "runtime" => Negative::Runtime(kind),
            other => Negative::Unknown(other),
        }
    }
}

/// How a run ended, before its steps are added.
struct End {
    outcome: Outcome,
    budget: Option<Termination>,
    detail: String,
    parse_error: Option<MessageKind>,
    counted_refusals: u64,
}

impl End {
    /// A pass; `detail` names the error a negative case passed with.
    fn passed(detail: impl Into<String>) -> End {
        End {
            outcome: Outcome::Passed,
            budget: None,
            detail: detail.into(),
            parse_error: None,
            counted_refusals: 0,
        }
    }

    fn failed(detail: impl Into<String>) -> End {
        End {
            outcome: Outcome::Failed,
            budget: None,
            detail: detail.into(),
            parse_error: None,
            counted_refusals: 0,
        }
    }

    /// A run the engine ended: by a budget, counted as such, or else failed.
    fn terminated(termination: Termination, what: &str) -> End {
        let detail = format!("{what}: Terminated({termination:?})");
        match termination {
            Termination::StepBudget | Termination::MemoryBudget => End {
                outcome: Outcome::Budget,
                budget: Some(termination),
                detail,
                parse_error: None,
                counted_refusals: 0,
            },
            Termination::Interrupted | Termination::EngineFault => End::failed(detail),
        }
    }

    /// A run that a parse error of kind `kind` ended: failed. `what` names the script.
    fn parse_failed(kind: MessageKind, what: &str, error: &Thrown) -> End {
        let cause = match kind {
            MessageKind::Grammar => "",
            MessageKind::Refusal { .. } => "refused: ",
            MessageKind::Nesting { .. } => "nesting limit: ",
        };
        End {
            parse_error: Some(kind),
            ..End::failed(format!("{what}parse: {cause}{error}"))
        }
    }

    /// A run `$262.evalScript` marked: failed, with the kind of the error, so that a refusal
    /// counts as one ([`Ran::refused`]) and the nesting limit does not.
    fn eval_script_marked(mark: EvalScriptMark) -> End {
        let cause = match mark.kind {
            MessageKind::Nesting { .. } => "nesting",
            MessageKind::Refusal { .. } => "refused",
            // `host.rs` marks no error of the grammar.
            MessageKind::Grammar => "grammar",
        };
        End {
            parse_error: Some(mark.kind),
            ..End::failed(format!("evalScript: {cause}: {}", mark.message))
        }
    }

    /// A run in which the engine's count of refusals moved by `count`: failed, whatever the case
    /// and its `negative` answered, with the detail `counted refusal: <count> refusal(s)`, after
    /// the detail of `$262.evalScript` when that marked the run (`marked`) too.
    fn counted_refusal(end: End, count: u64, marked: bool) -> End {
        let cause = format!("counted refusal: {count} refusal(s)");
        let detail = if marked {
            format!("{}; {cause}", end.detail)
        } else {
            cause
        };
        End {
            parse_error: end.parse_error,
            counted_refusals: count,
            ..End::failed(detail)
        }
    }
}

/// The kind of a parse error the engine gave, from its `message`. The engine writes the message
/// of a parse error as swell-syntax gives it with ` (<line>:<column>)` after it
/// (`crates/swell/src/script.rs`, `parse_script_in`); that position is taken off first.
fn parse_kind(error: &Thrown) -> MessageKind {
    message_kind(without_position(&error.text))
}

/// `text` without the ` (<line>:<column>)` the engine writes after the message of a parse error.
pub(crate) fn without_position(text: &str) -> &str {
    let Some((message, position)) = text.rsplit_once(" (") else {
        return text;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let is_position = position
        .strip_suffix(')')
        .and_then(|p| p.split_once(':'))
        .is_some_and(|(line, column)| digits(line) && digits(column));
    if is_position { message } else { text }
}

/// One attempt at a run, with `realms` realms made for `createRealm`.
enum Attempt {
    Done(Ran),
    /// `createRealm` found no realm left: the run is made again with more.
    MoreRealms,
}

fn attempt(
    harness: &[(String, Rc<str>)],
    text: &str,
    negative: &Negative<'_>,
    realms: usize,
    limits: &Limits,
) -> Attempt {
    let mut agent = match Agent::new(RunnerHost::default(), *limits, SEEDS) {
        Ok(agent) => agent,
        Err(error) => {
            return Attempt::Done(Ran::ended(End::failed(format!("Agent::new: {error:?}")), 0));
        }
    };
    let interrupt = agent.interrupt_handle();
    agent.host_mut().interrupt = Some(interrupt);
    let realm = match host::new_realm(&mut agent) {
        Ok((realm, _)) => realm,
        Err(error) => return Attempt::Done(Ran::ended(setup_failed(error), 0)),
    };
    for _ in 0..realms {
        match host::new_realm(&mut agent) {
            Ok((_, dollar)) => host::keep_spare(&mut agent, dollar),
            Err(error) => return Attempt::Done(Ran::ended(setup_failed(error), 0)),
        }
    }
    let mut steps = 0u64;
    let mut before = None;
    let end = run_scripts(
        &mut agent,
        &realm,
        (harness, text),
        negative,
        &mut steps,
        &mut before,
    );
    if agent.host().wanted_realm {
        return Attempt::MoreRealms;
    }
    // A refusal or the nesting limit `$262.evalScript` met fails the run, whatever the case and
    // its `negative` answered.
    let (end, marked) = match agent.host_mut().eval_script_mark.take() {
        Some(mark) => (End::eval_script_marked(mark), true),
        None => (end, false),
    };
    // So does every refusal the engine counted while the scripts ran (decision 11,
    // "Refusals"; `host-interface.md` section 13): `eval`, `Function`, `$262.evalScript` or a
    // regular expression literal, caught by the case or not.
    let counted = before.map_or(0, |before| agent.refusals().saturating_sub(before));
    let end = if counted > 0 {
        End::counted_refusal(end, counted, marked)
    } else {
        end
    };
    Attempt::Done(Ran::ended(end, steps))
}

fn setup_failed(error: SetupError) -> End {
    match error {
        SetupError::Terminated(termination) => End::terminated(termination, "realm"),
        SetupError::Other(detail) => End::failed(format!("realm: {detail}")),
    }
}

/// Parses every harness file and the case, then evaluates them in order. `before` is the
/// engine's count of refusals once every script parsed, read before the first one runs; it stays
/// `None` for a run that evaluates nothing.
fn run_scripts(
    agent: &mut Agent<RunnerHost>,
    realm: &Realm,
    (harness, text): (&[(String, Rc<str>)], &str),
    negative: &Negative<'_>,
    steps: &mut u64,
    before: &mut Option<u64>,
) -> End {
    let mut scripts = Vec::with_capacity(harness.len());
    for (name, source) in harness {
        match agent.parse_script(realm, Source::Utf8(source), None) {
            Ok(Parsed::Parsed(script)) => scripts.push((name, script)),
            Ok(Parsed::Errors(errors)) => {
                let error = first_error(agent, realm, &errors);
                return End::parse_failed(parse_kind(&error), &format!("{name}: "), &error);
            }
            Ok(Parsed::Terminated(termination)) => return End::terminated(termination, name),
            Err(error) => return End::failed(format!("{name}: parse: host error {error:?}")),
        }
    }
    let case = match agent.parse_script(realm, Source::Utf8(text), None) {
        Ok(Parsed::Parsed(script)) => script,
        Ok(Parsed::Errors(errors)) => {
            let error = first_error(agent, realm, &errors);
            // A refusal or the nesting limit is a SyntaxError too, but not the error a negative
            // case names: it fails the run whatever `negative` says.
            return match (parse_kind(&error), negative) {
                (MessageKind::Grammar, Negative::Parse(kind))
                    if error.name.as_deref() == Some(*kind) =>
                {
                    End::passed(format!("parse: {error}"))
                }
                (kind, _) => End::parse_failed(kind, "", &error),
            };
        }
        Ok(Parsed::Terminated(termination)) => return End::terminated(termination, "parse"),
        Err(error) => return End::failed(format!("parse: host error {error:?}")),
    };
    match negative {
        Negative::None | Negative::Runtime(_) => {}
        Negative::Parse(kind) => {
            return End::failed(format!("the case parsed; a {kind} was expected at parse"));
        }
        Negative::Resolution(kind) => {
            return End::failed(format!(
                "a {kind} at resolution needs a module; the case is a script"
            ));
        }
        Negative::Unknown(phase) => return End::failed(format!("unknown negative phase {phase}")),
    }
    // A refusal of a script's own parse ended the run above with its own cause; from here, a
    // refusal is one the scripts reached while they ran.
    *before = Some(agent.refusals());
    for (name, script) in &scripts {
        let evaluated = agent.evaluate_script(script);
        *steps = steps.saturating_add(agent.last_entry_steps().unwrap_or(0));
        match evaluated {
            Ok(Evaluated::Normal(_)) => {}
            Ok(Evaluated::Throw(thrown)) => {
                let error = describe(agent, realm, &thrown);
                return End::failed(format!("{name}: threw {error}"));
            }
            Ok(Evaluated::Terminated(termination)) => return End::terminated(termination, name),
            Err(error) => return End::failed(format!("{name}: host error {error:?}")),
        }
    }
    let evaluated = agent.evaluate_script(&case);
    *steps = steps.saturating_add(agent.last_entry_steps().unwrap_or(0));
    match (evaluated, negative) {
        (Ok(Evaluated::Normal(_)), Negative::Runtime(kind)) => End::failed(format!(
            "the case completed; a {kind} was expected at runtime"
        )),
        (Ok(Evaluated::Normal(_)), _) => End::passed(""),
        (Ok(Evaluated::Throw(thrown)), _) => {
            let error = describe(agent, realm, &thrown);
            match negative {
                Negative::Runtime(kind) if error.name.as_deref() == Some(*kind) => {
                    End::passed(format!("threw {error}"))
                }
                _ => End::failed(format!("threw {error}")),
            }
        }
        (Ok(Evaluated::Terminated(termination)), _) => End::terminated(termination, "case"),
        (Err(error), _) => End::failed(format!("host error {error:?}")),
    }
}

/// A thrown value as the runner reads it: the `name` of its constructor, when it is an object
/// whose `constructor` has a String `name` (design, decision 11: a `runtime` negative "must
/// throw an object whose constructor's `name` is the type"), and a text to print.
struct Thrown {
    name: Option<String>,
    text: String,
}

impl fmt::Display for Thrown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name}: {}", self.text),
            None => write!(f, "{}", self.text),
        }
    }
}

fn first_error(agent: &mut Agent<RunnerHost>, realm: &Realm, errors: &[Root]) -> Thrown {
    match errors.first() {
        Some(error) => describe(agent, realm, error),
        None => Thrown {
            name: None,
            text: "an empty list of parse errors".to_string(),
        },
    }
}

/// Reads the value `thrown` holds: its constructor's name and its `message`, or the value as a
/// string when it is not an object.
fn describe(agent: &mut Agent<RunnerHost>, realm: &Realm, thrown: &Root) -> Thrown {
    let read = agent.with_cx(realm, |cx| {
        let value = cx.get(thrown).ok()?;
        Some(match value {
            Value::Object(object) => Thrown {
                name: constructor_name(cx, object),
                text: property_text(cx, object, "message").unwrap_or_default(),
            },
            other => Thrown {
                name: None,
                text: text_of(cx, other).unwrap_or_default(),
            },
        })
    });
    read.ok().flatten().unwrap_or(Thrown {
        name: None,
        text: "a thrown value the runner cannot read".to_string(),
    })
}

fn get(cx: &mut Cx<'_, RunnerHost>, object: ObjectRef, name: &str) -> Option<Value> {
    let key = PropertyKey::String(cx.new_string(name).ok()?);
    cx.get_property(object, key).ok()?.ok()
}

fn constructor_name(cx: &mut Cx<'_, RunnerHost>, object: ObjectRef) -> Option<String> {
    let Value::Object(constructor) = get(cx, object, "constructor")? else {
        return None;
    };
    match get(cx, constructor, "name")? {
        Value::String(name) => cx.to_rust_string_lossy(name).ok(),
        _ => None,
    }
}

fn property_text(cx: &mut Cx<'_, RunnerHost>, object: ObjectRef, name: &str) -> Option<String> {
    let value = get(cx, object, name)?;
    text_of(cx, value)
}

fn text_of(cx: &mut Cx<'_, RunnerHost>, value: Value) -> Option<String> {
    let string = cx.to_string(value).ok()?.ok()?;
    cx.to_rust_string_lossy(string).ok()
}
