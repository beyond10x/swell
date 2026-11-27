//! Tests of the operand stack the instruction loop pushes and cuts
//! (`story:property-escape-cases-spend-less-in-the-interpreter-loop`): the charge of an operand
//! stack that grows across a push of a value, read with `heap_bytes()` on the base of that story,
//! and [`Agent::replace`] of 0, 1 and several entries with an Iterator Record and an enumerator
//! among the removed ones, whose bytes leave the frame's `charged` and are credited at once.
//! Run also with the feature `collect-every-safe-point`.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use super::for_of::record_bytes;
use super::frame::{Entry, Enumerator};
use crate::builtins::string::transform::tests::Answer;
use crate::heap::accounting::ChargedVec;
use crate::host::hooks::test_host::NoHooks;
use crate::host::{Completion, Cx, HostCall};
use crate::object::string::across::Across;
use crate::ops::iteration::IteratorRecord;
use crate::value::Value;

// The module is test code (`#[cfg(test)] mod loop_tests` in `interp/mod.rs`); the attribute
// says so to the scan of `source_scan::host_2_no_ambient_authority_in_engine_crates`.
#[cfg(test)]
std::thread_local! {
    /// `heap_bytes()` at each call of `probe`.
    static READINGS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    /// What `stage` read after each of its steps.
    static STAGED: RefCell<Vec<Staged>> = const { RefCell::new(Vec::new()) };
}

/// The host function `probe`: notes `heap_bytes()`.
fn probe(cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
    let bytes = cx.agent.heap_bytes().unwrap();
    READINGS.with(|readings| readings.borrow_mut().push(bytes));
    Ok(Value::Undefined)
}

/// The rise of `heap_bytes()` from the first to the second `probe()` of
/// [`an_operand_stack_that_grows_is_charged_as_on_the_base`], read on the base of the story: the
/// operand stack of the script's frame grows from 8 entries to 64 while the 44 values of the
/// call are pushed, each added entry charged `entry_bytes()`.
const GROWN: u64 = 896;

/// `docs/design/engine.md`, "Memory budget": an operand stack that grows is charged before the
/// entry is pushed, by the entries it adds, and stays charged until its frame is popped. The
/// values of a call of 41 arguments (constants pushed one by one, then the call of `probe`) grow
/// the script frame's stack; `heap_bytes()` read inside the call rises by [`GROWN`] over the
/// reading before the statement, and the reading after the call is the reading inside it. The
/// defects it holds: a push past the capacity that charges nothing, or charges for a growth
/// other than the one made.
#[test]
fn an_operand_stack_that_grows_is_charged_as_on_the_base() {
    let numbers = (1..=40)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut across = Across::with_steps(1_000_000);
    across.function("probe", probe, false);
    let source = format!("function f() {{}} probe(); f({numbers}, probe()); probe(); 'done'");
    READINGS.with(|readings| readings.borrow_mut().clear());
    assert_eq!(across.run(&source), Answer::Normal("done".to_string()));
    let readings = READINGS.with(|readings| readings.borrow().clone());
    let [before, inside, after] = readings[..] else {
        panic!("probe was called {} times", readings.len());
    };
    assert_eq!(
        (inside - before, after),
        (GROWN, inside),
        "heap_bytes: {before} before, {inside} inside the call, {after} after it"
    );
}

/// What `stage` read after one of its steps: the bytes charged, the running frame's `charged`
/// and the length of its operand stack, each less what they were when it was entered, the
/// capacity of the stack, and the top entry's value (a Number) or what the top entry is.
#[derive(Clone, Debug, PartialEq)]
struct Staged {
    step: &'static str,
    used: u64,
    charged: u64,
    entries: usize,
    capacity: usize,
    top: String,
}

/// The bytes the enumerator `stage` puts on the stack holds charged.
const ENUMERATOR_BYTES: u64 = 1_000;

/// The host function `stage`: on the operand stack of the frame that called it, an enumerator
/// holding [`ENUMERATOR_BYTES`], a value and an Iterator Record, each charged as `ForInStart` and
/// `GetIterator` charge them; then `replace(1, 3)` (the Iterator Record removed),
/// `replace(0, 4)`, `replace(4, 5)` (the enumerator among the four removed) and a drop of the
/// last entry, which leaves the stack as it was. It notes a [`Staged`] after each.
fn stage(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
    let agent = &mut *cx.agent;
    let used = agent.heap.accounting.used();
    let (charged, entries) = {
        let frame = agent.top_frame()?;
        (frame.charged, frame.stack.len())
    };
    let note = |agent: &mut crate::host::agent::Agent<NoHooks>, step: &'static str| {
        let rise = agent.heap.accounting.used() - used;
        let frame = agent.interp.frames.last().unwrap();
        let top = match frame.stack.last() {
            Some(Entry::Value(Value::Number(number))) => format!("{number}"),
            Some(Entry::Value(_)) => "value".to_string(),
            Some(Entry::Enumerator(_)) => "enumerator".to_string(),
            Some(Entry::Iterator(_)) => "iterator".to_string(),
            Some(_) => "other".to_string(),
            None => "none".to_string(),
        };
        let staged = Staged {
            step,
            used: rise,
            charged: frame.charged - charged,
            entries: frame.stack.len() - entries,
            capacity: frame.stack.capacity(),
            top,
        };
        STAGED.with(|staged_list| staged_list.borrow_mut().push(staged));
    };
    agent.heap.accounting.charge(ENUMERATOR_BYTES).unwrap();
    agent.top_frame()?.charged += ENUMERATOR_BYTES;
    agent.push_entry(Entry::Enumerator(Box::new(Enumerator {
        object: None,
        keys: Vec::new(),
        next: 0,
        bytes: ENUMERATOR_BYTES,
        chain: ChargedVec::new(),
    })))?;
    agent.push_value(Value::Number(1.0))?;
    agent.heap.accounting.charge(record_bytes()).unwrap();
    agent.top_frame()?.charged += record_bytes();
    agent.push_entry(Entry::Iterator(Box::new(IteratorRecord {
        iterator: call.callee,
        next_method: Value::Undefined,
        done: false,
    })))?;
    note(agent, "staged");
    agent.replace(1, Value::Number(3.0))?;
    note(agent, "replace 1");
    agent.replace(0, Value::Number(4.0))?;
    note(agent, "replace 0");
    agent.replace(4, Value::Number(5.0))?;
    note(agent, "replace 4");
    agent.drop_entries(1)?;
    note(agent, "dropped");
    Ok(Value::Undefined)
}

/// [`Agent::replace`] of 1 entry, an Iterator Record, of 0 entries and of 4 entries with an
/// enumerator among them, called on the operand stack of a running script: the value takes the
/// place of the removed entries; the bytes the removed Iterator Record and enumerator held leave
/// the frame's `charged` and are credited at the replace that removes them; a replace of 0
/// entries charges nothing; and the stack, whose capacity holds every entry, does not grow. The
/// defects it holds: a replace that writes over an entry without crediting what it held, and one
/// that credits or charges for the entries it keeps.
#[test]
fn a_replace_of_none_one_and_several_entries_credits_what_the_removed_ones_held() {
    let mut across = Across::with_steps(1_000_000);
    across.function("stage", stage, false);
    STAGED.with(|staged| staged.borrow_mut().clear());
    assert_eq!(
        across.run("stage(); 'done'"),
        Answer::Normal("done".to_string())
    );
    let staged = STAGED.with(|staged| staged.borrow().clone());
    let both = ENUMERATOR_BYTES + record_bytes();
    let capacity = staged.first().map_or(0, |first| first.capacity);
    let at = |step, used, charged, entries, top: &str| Staged {
        step,
        used,
        charged,
        entries,
        capacity,
        top: top.to_string(),
    };
    assert_eq!(
        staged,
        [
            at("staged", both, both, 3, "iterator"),
            at("replace 1", ENUMERATOR_BYTES, ENUMERATOR_BYTES, 3, "3"),
            at("replace 0", ENUMERATOR_BYTES, ENUMERATOR_BYTES, 4, "4"),
            at("replace 4", 0, 0, 1, "5"),
            at("dropped", 0, 0, 0, "value"),
        ]
    );
    assert!(
        capacity >= 6,
        "the stack had room for every entry: {capacity}"
    );
}
