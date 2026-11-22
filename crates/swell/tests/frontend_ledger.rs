//! The frontend on the memory ledger and the interrupt (`story:frontend-scratch-on-the-memory-ledger`):
//! the parser's tree and the compile step's scratch of `eval` and `Function` are charged to
//! `memory_bytes` before they grow, so a flat source whose tree does not fit ends the run in
//! `Terminated(MemoryBudget)` with everything credited; an interrupt set by
//! HostEnsureCanCompileStrings ends the run before the source is copied (HOST-168); and `eval`
//! charges a step per code unit of its source before it parses. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{limits, seeds};
use swell::{
    Agent, CompileStrings, Completion, Cx, HookCx, Host, InterruptHandle, Job, Limits, LoadRequest,
    ObjectRef, Outcome, Parsed, PropertyKey, Realm, Script, Source, Value,
};

/// A host whose required hooks do nothing; with an interrupt handle, its
/// `host_ensure_can_compile_strings` sets the interrupt and answers `Ok(())`.
#[derive(Default)]
struct LedgerHost {
    interrupt: Option<InterruptHandle>,
}

impl Host for LedgerHost {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        if let Some(handle) = &cx.host_mut().interrupt {
            handle.interrupt();
        }
        Ok(())
    }

    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// The lines of the flat source: 5,000, as the gate's flat source of `src/script.rs`. The
/// budget of [`eval_and_function_of_a_flat_source_end_in_memory_budget`] is the reading before
/// the call plus R, what a successful call keeps, plus 4 KiB. R grows with the lines (836,363
/// bytes at 5,000, 3,339,227 at 20,000, measured on 2026-11-22), and so does the tree the
/// compile step holds above it, while the room stays 4 KiB: at 5,000 lines the call still ends
/// in `Terminated(MemoryBudget)`, and a tree that was not charged would let it end normally.
/// With 20,000 the test took 1.03 CPU seconds alone, and in the gate it was terminated at 60 s
/// once at a load of 110 to 166 on 18 cores.
const FLAT_LINES: usize = 5_000;

/// `function unused() {`, [`FLAT_LINES`] lines `x = 1;`, then `}`.
fn flat_source() -> String {
    format!("function unused() {{\n{}}}", "x = 1;\n".repeat(FLAT_LINES))
}

/// An agent with [`LedgerHost`] and `limits`, and a realm.
fn agent(limits: Limits) -> (Agent<LedgerHost>, Realm) {
    let mut agent = Agent::new(LedgerHost::default(), limits, seeds()).unwrap();
    let Outcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    (agent, realm)
}

/// Define the global Strings `names` of `realm`.
fn define(agent: &mut Agent<LedgerHost>, realm: &Realm, names: &[(&str, &str)]) {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let Value::Object(object) = cx.get(&global).unwrap() else {
                panic!("the global is no object");
            };
            for &(name, text) in names {
                let key = PropertyKey::String(cx.new_string(name).unwrap());
                let value = Value::String(cx.new_string(text).unwrap());
                assert!(cx.set(object, key, value, true).unwrap().unwrap());
            }
        })
        .unwrap();
}

fn parsed(agent: &mut Agent<LedgerHost>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        _ => panic!("{source:?} did not parse"),
    }
}

/// What an evaluation ended in: `normal`, `throw` or the termination.
fn ended(outcome: Outcome<swell::Root>) -> String {
    match outcome {
        Outcome::Normal(_) => "normal".to_string(),
        Outcome::Throw(_) => "throw".to_string(),
        Outcome::Terminated(termination) => format!("{termination:?}"),
    }
}

/// An agent of `limits` with the flat source as `src` and its lines as `body`, the script
/// `call` parsed and a collection run: the agent, the script and `heap_bytes()` then.
fn prepared(limits: Limits, call: &str) -> (Agent<LedgerHost>, Script, u64) {
    let source = flat_source();
    let body = "x = 1;\n".repeat(FLAT_LINES);
    let (mut agent, realm) = agent(limits);
    define(&mut agent, &realm, &[("src", &source), ("body", &body)]);
    let script = parsed(&mut agent, &realm, call);
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    (agent, script, before)
}

/// `eval(src)` and `Function(body)` of the flat source, under a budget of what the heap reads
/// before the call, R and 4 KiB, where R is the rise of `live_bytes()` over a successful call
/// on a probe agent: the tree does not fit, the run ends in `Terminated(MemoryBudget)`, and
/// after a collection the heap reads as before the call.
#[test]
fn eval_and_function_of_a_flat_source_end_in_memory_budget() {
    for call in ["eval(src);", "Function(body);"] {
        let (mut probe, script, before) = prepared(limits(), call);
        let live = probe.live_bytes().unwrap();
        assert_eq!(ended(probe.evaluate_script(&script).unwrap()), "normal");
        let r = probe.live_bytes().unwrap() - live;
        let mut budget = limits();
        budget.memory_bytes = before + r + 4096;
        let (mut agent, script, at_start) = prepared(budget, call);
        assert_eq!(at_start, before, "{call}: the two agents read the same");
        println!("{call}: before {before} R {r}");
        assert_eq!(
            ended(agent.evaluate_script(&script).unwrap()),
            "MemoryBudget",
            "{call}"
        );
        agent.collect().unwrap();
        assert_eq!(agent.heap_bytes().unwrap(), before, "{call}");
    }
}

/// HOST-168: an interrupt set by HostEnsureCanCompileStrings, which then answers `Ok(())`, ends
/// `eval` and `Function` of a refused source in `Terminated(Interrupted)` before the source is
/// copied or parsed: the `try` catches nothing and `refusals()` does not move.
#[test]
fn host_168_an_interrupt_set_by_the_compile_strings_hook_ends_eval_before_its_parse() {
    for call in [
        "try { eval(r); } catch (e) {}",
        "try { Function(r); } catch (e) {}",
    ] {
        let (mut agent, realm) = agent(limits());
        define(&mut agent, &realm, &[("r", "<!-- x")]);
        let script = parsed(&mut agent, &realm, call);
        let handle = agent.interrupt_handle();
        agent.host_mut().interrupt = Some(handle);
        let refusals = agent.refusals();
        assert_eq!(
            ended(agent.evaluate_script(&script).unwrap()),
            "Interrupted",
            "{call}"
        );
        assert_eq!(agent.refusals(), refusals, "{call}");
    }
}

/// `eval` charges a step per code unit of its String before it copies it: a String of 100,000
/// code units that ends in a refused construct, under 10,000 steps per entry, ends the run in
/// `Terminated(StepBudget)` before the parse, so `refusals()` does not move.
#[test]
fn eval_charges_a_step_per_code_unit_before_its_parse() {
    let big = format!("{}<!-- x", "0;".repeat(49_997));
    assert_eq!(big.len(), 100_000);
    let mut limits = limits();
    limits.steps_per_entry = 10_000;
    let (mut agent, realm) = agent(limits);
    define(&mut agent, &realm, &[("big", &big)]);
    let script = parsed(&mut agent, &realm, "eval(big);");
    let refusals = agent.refusals();
    assert_eq!(ended(agent.evaluate_script(&script).unwrap()), "StepBudget");
    assert_eq!(agent.refusals(), refusals);
    assert!(agent.last_entry_steps().unwrap() <= 10_000);
}

/// A ledger with no bound that reads what it holds and its high-water mark.
#[derive(Default)]
struct HighWater {
    held: usize,
    high: usize,
}

impl swell_syntax::Ledger for HighWater {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.held += bytes;
        self.high = self.high.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The compile step of a script of a few code units charges near what it holds: for `1` and
/// `1 + 1`, the ledger holds at no time more than 1 KiB above the tree and the finished code.
/// The step charges each node's expansion as its lists grow (224 bytes for a list of up to
/// four tasks). Measured on `7a7bf0a`, when it charged a bound from the node's children before
/// the expansion (56 bytes a task for 8 tasks a child and 32 more: 1,792 bytes for a leaf,
/// 2,240 for one child, 2,688 for two): 2,411 bytes above the tree and the code for `1` (the
/// tree 881 bytes, the code 144, the peak 3,436) and 2,771 for `1 + 1` (882, 232, 3,885). The
/// peak is part of the room an entry of `1` needs after a memory-budget termination
/// (HOST-190). The test prints each peak.
#[test]
fn the_compile_step_of_a_short_script_charges_near_what_it_holds() {
    for text in ["1", "1 + 1"] {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut ledger = HighWater::default();
        let options = swell_syntax::ParseOptions { max_nesting: 100 };
        let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
            panic!("{text:?} did not parse");
        };
        ledger.high = ledger.held;
        let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
            panic!("{text:?} did not compile");
        };
        let held = tree.charged + code.charged;
        println!(
            "{text:?}: peak {} above the tree {} and the code {}",
            ledger.high.saturating_sub(held),
            tree.charged,
            code.charged
        );
        assert_eq!(ledger.held, held, "{text:?}: the step credited its scratch");
        assert!(
            ledger.high <= held + 1024,
            "{text:?}: peak {} above the tree {} and the code {}",
            ledger.high - held,
            tree.charged,
            code.charged
        );
    }
}
