//! The interrupt handle (`docs/design/host-interface.md`, decision 12). The checks that read the
//! flag during a run are `story:object-model-and-realms` and
//! `story:interpreter-and-script-entry-points`.
#![cfg(test)]

mod support;

use std::thread;
use std::time::Duration;

use std::cell::{Cell, RefCell};

use support::host::TestHost;
use support::script::{
    Outcome as Shown, define_function, describe, eval, eval_text, messages, parse,
};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, HostCall, HostData, HostError,
    HostObjectSpec, InterruptHandle, ObjectRef, Outcome, Parsed, Prototype, Realm, RealmRef,
    Source, Termination, Tracer, Value,
};
use swell_syntax::{MessageKind, message_kind};

fn send_and_sync<T: Send + Sync + 'static>(_: &T) {}

/// HOST-94: `InterruptHandle` is `Send` and `Sync`: a clone moves into a spawned thread, and
/// what it sets there the agent's own handle reads.
#[test]
fn host_94_interrupt_handle_moves_into_a_thread() {
    let agent = support::host::agent().unwrap();
    let handle: InterruptHandle = agent.interrupt_handle();
    send_and_sync(&handle);
    assert!(!handle.is_set());

    let clone = handle.clone();
    thread::spawn(move || clone.interrupt()).join().unwrap();
    assert!(handle.is_set());
    assert!(
        agent.interrupt_handle().is_set(),
        "every handle of one agent shares the flag"
    );

    let clone = handle.clone();
    thread::spawn(move || clone.clear()).join().unwrap();
    assert!(!handle.is_set());

    // A handle of another agent is another flag.
    let other = support::host::agent().unwrap();
    handle.interrupt();
    assert!(!other.interrupt_handle().is_set());
}

struct Stale(ObjectRef);

impl HostData for Stale {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.object(self.0);
    }
}

/// HOST-95: `interrupt()` on the handle of a faulted agent returns, and the agent's calls still
/// answer `Err(HostError::Faulted)`.
#[test]
fn host_95_interrupt_on_a_faulted_agent_returns() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let handle = agent.interrupt_handle();

    // Fault the agent: a host object whose trace lists a freed object.
    let freed = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()
            .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let _root = agent
        .with_cx(&realm, |cx| {
            let holder = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(2),
                    data: Some(Box::new(Stale(freed))),
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            cx.root(Value::Object(holder)).unwrap()
        })
        .unwrap();
    assert!(matches!(
        agent.collect(),
        Ok(Outcome::Terminated(Termination::EngineFault))
    ));
    assert_eq!(agent.state(), AgentState::Faulted);

    handle.interrupt();
    let from_thread = handle.clone();
    thread::spawn(move || from_thread.interrupt())
        .join()
        .unwrap();
    assert!(handle.is_set());

    assert_eq!(agent.state(), AgentState::Faulted);
    assert!(matches!(
        agent.with_cx(&realm, |_| ()),
        Err(HostError::Faulted)
    ));
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));
    handle.clear();
    assert!(matches!(
        agent.with_cx(&realm, |_| ()),
        Err(HostError::Faulted)
    ));
}

/// HOST-90: `interrupt()` called from another thread while the script `while (true) {}` runs
/// with a step budget of 2^62 makes `evaluate_script` answer `Terminated(Interrupted)`.
#[test]
fn host_90_interrupt_from_another_thread_ends_the_loop() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 1 << 62;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "while (true) {}");
    let handle = agent.interrupt_handle();
    let interrupter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        handle.interrupt();
    });
    let outcome = agent.evaluate_script(&script).unwrap();
    interrupter.join().unwrap();
    assert!(matches!(
        outcome,
        Outcome::Terminated(Termination::Interrupted)
    ));
    assert!(agent.last_entry_steps().unwrap() > 0);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-91: after an entry answered `Terminated(Interrupted)`, `is_set()` reads false and the
/// next script `1` answers `Normal(1)`.
#[test]
fn host_91_interrupted_entry_clears_the_flag() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let handle = agent.interrupt_handle();
    let script = parse(&mut agent, &realm, "while (true) {}");
    handle.interrupt();
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Terminated(Termination::Interrupted))
    ));
    assert!(!handle.is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Shown::Normal("1".to_string())
    );
}

/// HOST-92: `interrupt()` called while no entry runs makes the next entry answer
/// `Terminated(Interrupted)` with `last_entry_steps()` 0.
#[test]
fn host_92_interrupt_before_an_entry_ends_it_at_zero_steps() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "var ran = 1;");
    agent.interrupt_handle().interrupt();
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Terminated(Termination::Interrupted))
    ));
    assert_eq!(agent.last_entry_steps(), Ok(0));
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof ran;"),
        Shown::Normal("\"undefined\"".to_string())
    );
}

/// HOST-93: `clear()` after `interrupt()` and before the next entry lets that entry run to
/// `Normal`.
#[test]
fn host_93_clear_before_the_entry_lets_it_run() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "var i = 0; while (i < 100) i++; i;");
    let handle = agent.interrupt_handle();
    handle.interrupt();
    handle.clear();
    let outcome = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        describe(&mut agent, &realm, outcome),
        Shown::Normal("100".to_string())
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: std::cell::Cell<Option<InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and returns `1`, counting its calls.
fn interrupt_now(
    cx: &mut Cx<'_, support::host::TestHost>,
    _call: HostCall<'_>,
) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// HOST-96: an interrupt set while a host function runs ends the run with
/// `Terminated(Interrupted)` at the first check after the host function returns to the engine:
/// the statement after the call does not run, and the host function's value is not used.
#[test]
fn host_96_interrupt_set_in_a_host_function_ends_the_run_after_it() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var after = 0; var got = interruptNow(); after = 1;"
        ),
        Shown::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.host().count, 1);
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "after + \",\" + got;"),
        Shown::Normal("\"0,undefined\"".to_string())
    );
}

/// A source the parser refuses (a BigInt literal, "... is not supported before slice <n>"):
/// a parse of it that runs moves `refusals()` by 1 (HOST-164).
const REFUSED: &str = "1n;";

/// A parse's answer as text: `Parsed`, `Errors(<n>)` or `Terminated(<termination>)`.
fn shown<T>(parsed: &Parsed<T>) -> String {
    match parsed {
        Parsed::Parsed(_) => "Parsed".to_string(),
        Parsed::Errors(errors) => format!("Errors({})", errors.len()),
        Parsed::Terminated(termination) => format!("Terminated({termination:?})"),
    }
}

/// `REFUSED` is refused: with no flag set, `parse_script` of it answers one `SyntaxError` whose
/// message, without the position ` (<line>:<column>)` the engine appends, reads as a refusal
/// to `swell_syntax::message_kind`, and `refusals()` moves by 1.
fn assert_refused(agent: &mut Agent<TestHost>, realm: &Realm) {
    let before = agent.refusals();
    let Parsed::Errors(errors) = agent
        .parse_script(realm, Source::Utf8(REFUSED), None)
        .unwrap()
    else {
        panic!("{REFUSED:?} was not refused");
    };
    let messages = messages(agent, realm, &errors);
    assert_eq!(messages.len(), 1, "{messages:?}");
    let (message, position) = messages[0].rsplit_once(" (").unwrap();
    assert_eq!(position, "1:2)", "{messages:?}");
    assert!(
        matches!(message_kind(message), MessageKind::Refusal { .. }),
        "{messages:?}"
    );
    assert_eq!(agent.refusals(), before + 1);
}

/// HOST-166: `interrupt()` called while no entry runs makes the next `parse_script` answer
/// `Parsed::Terminated(Interrupted)` before the parse, for a UTF-8 and a UTF-16 source alike:
/// `is_set()` then reads false, `refusals()` and `heap_bytes()` read what they read before the
/// call, and `last_entry_steps()` reads 0.
#[test]
fn host_166_a_pending_interrupt_ends_parse_script_before_the_parse() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let handle = agent.interrupt_handle();
    let units: Vec<u16> = REFUSED.encode_utf16().collect();
    for source in [Source::Utf8(REFUSED), Source::Utf16(&units)] {
        // An entry that spends steps first, so that the 0 below is the parse's own; its root is
        // held until the parse has answered.
        let ran = eval(&mut agent, &realm, "var i = 0; while (i < 10) i++; i;");
        assert!(matches!(ran, Outcome::Normal(_)));
        assert!(agent.last_entry_steps().unwrap() > 0);
        // A safe point releases the roots dropped so far, and a collection frees what they held,
        // so that the parse's own safe point finds nothing to free.
        agent.with_cx(&realm, |_| ()).unwrap();
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let refusals = agent.refusals();
        let bytes = agent.heap_bytes().unwrap();

        handle.interrupt();
        let parsed = agent.parse_script(&realm, source, None).unwrap();
        assert_eq!(shown(&parsed), "Terminated(Interrupted)");
        assert!(!handle.is_set());
        assert_eq!(agent.refusals(), refusals);
        assert_eq!(agent.heap_bytes(), Ok(bytes));
        assert_eq!(agent.last_entry_steps(), Ok(0));
        assert_eq!(agent.state(), AgentState::Alive);
        drop(ran);
    }
    assert_refused(&mut agent, &realm);
}

/// A flag that `parse_script` consumed does not end the next entry: the parse after it answers
/// `Parsed`, and the evaluation of that script `Normal(1)`.
#[test]
fn a_flag_consumed_by_parse_script_does_not_end_the_next_entry() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    agent.interrupt_handle().interrupt();
    let parsed = agent
        .parse_script(&realm, Source::Utf8("1;"), None)
        .unwrap();
    assert_eq!(shown(&parsed), "Terminated(Interrupted)");
    assert!(!agent.interrupt_handle().is_set());

    let parsed = agent
        .parse_script(&realm, Source::Utf8("1;"), None)
        .unwrap();
    let Parsed::Parsed(script) = parsed else {
        panic!("the second parse answered {}", shown(&parsed));
    };
    let outcome = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        describe(&mut agent, &realm, outcome),
        Shown::Normal("1".to_string())
    );
}

thread_local! {
    /// The realm `interrupt_then_parse` parses in.
    static PARSE_REALM: Cell<Option<RealmRef>> = const { Cell::new(None) };
    /// What `cx.parse_script` answered `interrupt_then_parse`, as [`shown`] writes it.
    static PARSE_ANSWER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A host function that sets its agent's interrupt flag through the handle the host keeps,
/// then parses `REFUSED` through `cx.parse_script`, records the answer and returns `1`.
fn interrupt_then_parse(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    let realm = PARSE_REALM.with(Cell::get).unwrap();
    let parsed = cx.parse_script(realm, Source::Utf8(REFUSED), None).unwrap();
    PARSE_ANSWER.with(|cell| cell.replace(Some(shown(&parsed))));
    Ok(Value::Number(1.0))
}

/// HOST-167: a host function that calls `interrupt()` and then `cx.parse_script` gets
/// `Parsed::Terminated(Interrupted)` with `refusals()` unchanged, and the outermost entry answers
/// `Terminated(Interrupted)`. The same inside `with_cx`: the parse answers
/// `Parsed::Terminated(Interrupted)`, and the run stays ended until the closure returns.
#[test]
fn host_167_an_interrupt_set_in_a_host_function_ends_its_cx_parse_script() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    define_function(&mut agent, &realm, "f", 0, interrupt_then_parse, false);
    let realm_ref = agent
        .with_cx(&realm, |cx| cx.realm_ref(&realm).unwrap())
        .unwrap();
    PARSE_REALM.with(|cell| cell.set(Some(realm_ref)));
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));

    let refusals = agent.refusals();
    assert_eq!(
        eval_text(&mut agent, &realm, "f(); 1"),
        Shown::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.host().count, 1);
    assert_eq!(
        PARSE_ANSWER.with(|cell| cell.take()),
        Some("Terminated(Interrupted)".to_string())
    );
    assert_eq!(agent.refusals(), refusals);
    assert!(!agent.interrupt_handle().is_set());

    // Inside `with_cx`.
    let handle = agent.interrupt_handle();
    let (parsed, after) = agent
        .with_cx(&realm, |cx| {
            let own = cx.realm_ref(&realm).unwrap();
            handle.interrupt();
            let parsed = cx.parse_script(own, Source::Utf8(REFUSED), None).unwrap();
            let after = cx.new_string("after").map(|_| ());
            (shown(&parsed), after)
        })
        .unwrap();
    assert_eq!(parsed, "Terminated(Interrupted)");
    assert_eq!(after, Err(Abrupt::Terminated(Termination::Interrupted)));
    assert_eq!(agent.refusals(), refusals);
    assert!(!handle.is_set());

    // The next entry runs afresh, and `REFUSED` is refused when no flag is set.
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Shown::Normal("1".to_string())
    );
    let counted = agent
        .with_cx(&realm, |cx| {
            let own = cx.realm_ref(&realm).unwrap();
            shown(&cx.parse_script(own, Source::Utf8(REFUSED), None).unwrap())
        })
        .unwrap();
    assert_eq!(counted, "Errors(1)");
    assert_eq!(agent.refusals(), refusals + 1);
    assert_refused(&mut agent, &realm);
}
