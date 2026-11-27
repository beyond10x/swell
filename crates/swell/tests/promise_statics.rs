//! The statics of `Promise` that take an iterable (`#sec-promise.all`, `#sec-promise.allsettled`,
//! `#sec-promise.any`, `#sec-promise.race`), `Promise.withResolvers` (`#sec-promise.withResolvers`),
//! `Promise.try` (`#sec-promise.try`), `Promise.prototype.finally`
//! (`#sec-promise.prototype.finally`) and `AggregateError` (`#sec-aggregate-error-objects`), with
//! the bounds a combined promise reaches (`docs/design/engine.md`, decision 3, "Depth bounds" and
//! "The microtask checkpoint is one entry", and decision 4, "A built-in function with slots") and
//! HOST-201 and HOST-202 (`docs/design/host-interface.md`, "The kind of an AggregateError, and
//! kinds to come"). Written by `story:promise-statics-finally-and-aggregate-error`. Run also with
//! the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::RefCell;

use support::host::{TestHost, agent_with, global, is_error_of, limits, object, realm};
use support::script::{Outcome, define_function, eval, eval_text};
use support::stack::stack_agent;
use swell::{
    Agent, Brand, CheckpointReport, Completion, Cx, ErrorKind, ErrorSummary, HostCall,
    HostObjectSpec, InterruptHandle, Limits, Outcome as HostOutcome, PromiseState, PropertyKey,
    Prototype, Realm, Termination, Value,
};

/// The bytes of the slots of an element function of `Promise.all`, `allSettled` and `any`: the
/// box of its slots (32), its [[AlreadyCalled]] flag (an `Rc<Cell<bool>>`, 24) and the record it
/// shares with the other element functions of its call (an `Rc` of the list of values, the
/// capability and [[RemainingElements]], 88), each counted in the box of each function
/// (`object/builtin_function.rs`, the library's test
/// `the_slots_of_an_element_function_are_charged_their_boxes`).
const ELEMENT_BOX_BYTES: u64 = 144;

/// An agent of the test host with `limits` and a realm.
fn fresh(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// The script `source`, which must complete normally, as `show` writes its value.
fn answer(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> String {
    match eval_text(agent, realm, source) {
        Outcome::Normal(text) => text,
        other => panic!("{source} answered {other:?}"),
    }
}

/// A checkpoint that must answer `Normal`: its report.
fn checkpoint(agent: &mut Agent<TestHost>) -> CheckpointReport {
    match agent.perform_microtask_checkpoint().unwrap() {
        HostOutcome::Normal(report) => report,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A collection that must answer `Normal`, then `heap_bytes()`.
fn collected(agent: &mut Agent<TestHost>) -> u64 {
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    agent.heap_bytes().unwrap()
}

/// What `promise_state` reads of the promise the script `source` answers, in `realm`: `Pending`,
/// or the settlement with what `show_settled` writes of its result.
fn settled(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> String {
    let HostOutcome::Normal(promise) = eval(agent, realm, source) else {
        panic!("{source} did not complete normally");
    };
    let state = agent.promise_state(&promise).unwrap();
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let (word, result) = match state {
                PromiseState::Pending => return "Pending".to_string(),
                PromiseState::Fulfilled(result) => ("Fulfilled", cx.get(&result).unwrap()),
                PromiseState::Rejected(result) => ("Rejected", cx.get(&result).unwrap()),
            };
            let shown = if is_error_of(cx, global, result, "RangeError") {
                "RangeError".to_string()
            } else {
                support::script::show(cx, result)
            };
            format!("{word}({shown})")
        })
        .unwrap()
}

/// `#sec-performpromiseall`: the combined promise is fulfilled with an Array of the values in the
/// order of the iterable, whatever order the elements settle in, a value that is no promise
/// included; with no element it is fulfilled with an empty Array.
#[test]
fn promise_all_resolves_with_the_values_in_order_whatever_the_order_of_settlement() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var out = 'pending', empty = 'pending', r = [];\n\
         function make(i) { return new Promise(function (res) { r[i] = res; }); }\n\
         Promise.all([make(0), make(1), 'plain', make(2)]).then(function (v) {\n\
           out = Array.isArray(v) + ':' + v.length + ':' + v.join();\n\
         });\n\
         Promise.all([]).then(function (v) { empty = Array.isArray(v) + ':' + v.length; });\n\
         r[2]('c'); r[0]('a');",
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out"), "\"pending\"");
    assert_eq!(answer(&mut agent, &realm, "empty"), "\"true:0\"");
    answer(&mut agent, &realm, "r[1]('b');");
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out"), "\"true:4:a,b,plain,c\"");
}

/// `#sec-performpromiseallsettled`: the combined promise waits for every element, a rejection
/// included, and is fulfilled with one object per element, `{ status, value }` or `{ status,
/// reason }`, in the order of the iterable.
#[test]
fn promise_all_settled_waits_for_every_element_and_records_each_outcome() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var out = 'pending', r, j;\n\
         var a = new Promise(function (res) { r = res; });\n\
         var b = new Promise(function (_, rej) { j = rej; });\n\
         Promise.allSettled([a, b, 3]).then(function (v) {\n\
           out = v.map(function (o) {\n\
             return o.status + '=' + ('value' in o ? o.value : o.reason) + '/' + Object.keys(o).join('+');\n\
           }).join();\n\
         });\n\
         j('no');",
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out"), "\"pending\"");
    answer(&mut agent, &realm, "r('yes');");
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out"),
        "\"fulfilled=yes/status+value,rejected=no/status+reason,fulfilled=3/status+value\""
    );
}

/// `#sec-performpromiseany`: when every element rejects, the combined promise is rejected with an
/// AggregateError of the current realm whose own `errors`, not enumerable, holds the reasons in
/// the order of the iterable, and which has no own `message`; the first element to be fulfilled
/// fulfills it.
#[test]
fn promise_any_rejects_with_an_aggregate_error_of_every_reason() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var out = 'pending', first = 'pending', j = [];\n\
         function make(i) { return new Promise(function (_, rej) { j[i] = rej; }); }\n\
         Promise.any([make(0), make(1)]).then(function () { out = 'fulfilled'; }, function (e) {\n\
           out = [e instanceof AggregateError, Object.getPrototypeOf(e) === AggregateError.prototype,\n\
             Array.isArray(e.errors), e.errors.join('+'), Object.prototype.hasOwnProperty.call(e, 'message'),\n\
             Object.getOwnPropertyDescriptor(e, 'errors').enumerable, Error.isError(e)].join();\n\
         });\n\
         Promise.any([Promise.reject(1), Promise.resolve(2), 3]).then(function (v) { first = v; });\n\
         j[1]('second');",
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out"), "\"pending\"");
    assert_eq!(answer(&mut agent, &realm, "first"), "2");
    answer(&mut agent, &realm, "j[0]('first');");
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out"),
        "\"true,true,true,first+second,false,false,true\""
    );
}

/// `#sec-performpromiserace`: the combined promise settles as the first element to settle, by
/// the order of settlement, not of the iterable; a rejection settles it too.
#[test]
fn promise_race_settles_with_the_first_to_settle() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var out = [], r = [];\n\
         function make(i) { return new Promise(function (res) { r[i] = res; }); }\n\
         Promise.race([make(0), make(1)]).then(function (v) { out.push('fulfilled ' + v); });\n\
         r[1]('b'); r[0]('a');\n\
         Promise.race([new Promise(function () {}), Promise.reject('no')])\n\
           .then(null, function (e) { out.push('rejected ' + e); });",
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out.join()"),
        "\"fulfilled b,rejected no\""
    );
}

/// The element functions (`#sec-promise.all-resolve-element-functions`,
/// `#sec-promise.allsettled-resolve-element-functions`,
/// `#sec-promise.allsettled-reject-element-functions`, `#sec-promise.any-reject-element-functions`):
/// a function of `length` 1 and the empty `name`, one per element, which records its element
/// once and answers undefined; a second call changes nothing, and the two functions of one
/// element of `allSettled` share their [[AlreadyCalled]]. A constructor whose `resolve` answers
/// its argument hands them to a thenable's `then`.
#[test]
fn an_element_function_resolves_once() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "var fns = [], pairs = [], out = [];\n\
             var C = function (executor) { return new Promise(executor); };\n\
             C.resolve = function (v) { return v; };\n\
             var thenable = { then: function (f, r) { fns.push(f); pairs.push([f, r]); } };\n\
             Promise.all.call(C, [thenable, thenable]).then(function (v) { out.push('all ' + v.join()); });\n\
             var first = fns[0](1), again = fns[0](9);\n\
             fns[1](2); fns[1](8); fns[0](7);\n\
             Promise.allSettled.call(C, [thenable]).then(function (v) { out.push('settled ' + v[0].status + ' ' + v[0].reason); });\n\
             pairs[2][1]('x'); pairs[2][0]('y'); pairs[2][1]('z');\n\
             Promise.any.call(C, [thenable]).then(null, function (e) { out.push('any ' + e.errors.join()); });\n\
             pairs[3][1]('p'); pairs[3][1]('q');\n\
             [fns.length, fns[0] !== fns[1], fns[0].length, fns[0].name === '', String(first), String(again),\n\
              Object.getPrototypeOf(fns[0]) === Function.prototype, pairs[2][0] !== pairs[2][1]].join();",
        ),
        "\"4,true,1,true,undefined,undefined,true,true\""
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out.join('|')"),
        "\"all 1,2|settled rejected x|any p\""
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new fns[0]();"),
        Outcome::Throw("TypeError".to_string())
    );
}

/// `#sec-promise.all` and the three statics after it, steps 7 and 8: a throw of the iterator's
/// `next` sets [[Done]] (`#sec-iteratorstepvalue`), so `return` is not called, and the combined
/// promise is rejected with that throw; a throw of the constructor's `resolve` leaves the
/// iterator not done, so IteratorClose calls `return` once, and the combined promise is rejected
/// with the throw of `resolve`. The same for each of the four.
#[test]
fn a_throw_of_the_iterator_closes_nothing_and_a_throw_of_resolve_closes_it() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var returns = 0, out = [], counts = [];\n\
         function iterable(next) {\n\
           var it = { next: next, 'return': function () { returns++; return {}; } };\n\
           var o = {}; o[Symbol.iterator] = function () { return it; }; return o;\n\
         }\n\
         var fromNext = new Error('next'), fromResolve = new Error('resolve');\n\
         var C = function (executor) { return new Promise(executor); };\n\
         C.resolve = function () { throw fromResolve; };\n\
         ['all', 'allSettled', 'any', 'race'].forEach(function (name) {\n\
           Promise[name](iterable(function () { throw fromNext; }))\n\
             .then(null, function (e) { out.push(name + ' next ' + (e === fromNext)); });\n\
           counts.push(returns);\n\
           Promise[name].call(C, iterable(function () { return { done: false, value: 1 }; }))\n\
             .then(null, function (e) { out.push(name + ' resolve ' + (e === fromResolve)); });\n\
           counts.push(returns);\n\
         });",
    );
    assert_eq!(
        answer(&mut agent, &realm, "counts.join()"),
        "\"0,1,1,2,2,3,3,4\""
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out.join()"),
        "\"all next true,all resolve true,allSettled next true,allSettled resolve true,\
         any next true,any resolve true,race next true,race resolve true\""
    );
}

/// The number of pending promises of
/// [`the_values_of_a_pending_promise_all_are_charged_until_it_settles`].
/// 1,000 in the default mode; with the feature `collect-every-safe-point`, where every
/// instruction of the loops that make and settle the promises collects the heap, 20 (50 took
/// 2.5 s of nextest time on the build node).
const PENDING: u32 = if cfg!(feature = "collect-every-safe-point") {
    20
} else {
    1_000
};

/// "A built-in function with slots" and the list of values (`#sec-performpromiseall`, step 4.d,
/// "Append undefined to values"): `Promise.all` over [`PENDING`] promises that stay pending
/// makes `heap_bytes()`, after a collection that keeps them, rise above the reading before by
/// at least [`PENDING`] times [`ELEMENT_BOX_BYTES`] more than `Promise.race` over as many does
/// (race makes the same reactions and derived promises, and no element function); once every
/// promise is resolved and the checkpoint has run, one `collect()` leaves it above the reading
/// before by no more than race leaves it, with an Array of as many values pushed after its
/// settle, plus the room of the Array of values (a Value per element, at most twice its
/// length).
///
/// It holds at any size of the realm (`story:tests-hold-at-any-size-of-the-realm`): a first
/// round of `Promise.all` over twice as many promises grows the arenas, and each round's `make`
/// drops every object of the round before it (`ps`, `resolvers`, `combined` and `values` are
/// assigned again), so no arena grows between the readings of a round. Before, the first round
/// was the measured one's size and its `combined` with its Array of values lived on into the
/// measured round; with the seven set methods of `story:set-methods` in the realm and no
/// padding, in the collect mode, the arena of objects grew from 512 to 1,024 slots between the
/// reading before and the pending one (98,576 bytes above before after the settle, against 640).
/// And the reading after was compared with before alone: the settle frees more than the boxes
/// (with the larger first round alone, after was 5,136 bytes below before in the collect mode
/// and 267,776 in the default one), so boxes that stayed charged after the settle passed. The
/// control round of race takes that out. The test prints the six readings.
#[test]
fn the_values_of_a_pending_promise_all_are_charged_until_it_settles() {
    let (mut agent, realm) = fresh(limits());
    let make = |count: u32| {
        format!(
            "var resolvers = [], ps = [], combined = undefined, values = undefined;\n\
             for (var i = 0; i < {count}; i++) ps.push(new Promise(function (r) {{ resolvers.push(r); }}));"
        )
    };
    let settle = "for (var i = 0; i < resolvers.length; i++) resolvers[i](i); resolvers = [];";
    let pushed = format!("values = []; for (var i = 0; i < {PENDING}; i++) values.push(i); 0");
    let round = |agent: &mut Agent<TestHost>, count: u32, combinator: &str, then: &str| {
        answer(agent, &realm, &make(count));
        let before = collected(agent);
        answer(
            agent,
            &realm,
            &format!("combined = Promise.{combinator}(ps); 0"),
        );
        let pending = collected(agent);
        answer(agent, &realm, settle);
        checkpoint(agent);
        answer(agent, &realm, then);
        let after = collected(agent);
        (before, pending, after)
    };
    round(&mut agent, 2 * PENDING, "all", "0");
    let race = round(&mut agent, PENDING, "race", &pushed);
    let (before, pending, after) = round(&mut agent, PENDING, "all", "0");
    println!(
        "heap_bytes: race before {}, pending {}, after {}; \
         Promise.all of {PENDING} before {before}, pending {pending}, after {after}",
        race.0, race.1, race.2
    );
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "combined.then(function (v) { ps = v.length; }); 0"
        ),
        "0"
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "ps"), PENDING.to_string());
    // pending - before >= race pending - race before + the boxes, without a subtraction.
    assert!(
        pending + race.0 >= before + race.1 + u64::from(PENDING) * ELEMENT_BOX_BYTES,
        "{pending} - {before} against race {} - {}",
        race.1,
        race.0
    );
    // after - before <= race after - race before + the room, without a subtraction.
    let array = 2 * u64::from(PENDING) * size_of::<Value>() as u64;
    assert!(
        after + race.0 <= before + race.2 + array,
        "{after} - {before} against race {} - {}",
        race.2,
        race.0
    );
}

thread_local! {
    /// The interrupt handle of the agent of the test that runs, for the host function
    /// `interruptNow`.
    static INTERRUPT: RefCell<Option<InterruptHandle>> = const { RefCell::new(None) };
}

/// The host function `interruptNow`: it sets the agent's interrupt flag.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    INTERRUPT.with(|interrupt| {
        if let Some(interrupt) = interrupt.borrow().as_ref() {
            interrupt.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// The setup of [`a_termination_in_an_element_function_leaves_the_combined_promise_pending`]:
/// jobs a, the element function of `Promise.all([1])`, and c, in that order; the element
/// function resolves the combined promise with its Array, and the Array's `then`, a getter on
/// `Array.prototype`, runs `ends` inside the element function.
const ELEMENT_JOBS: &str = "var log = [], state = 'none';\n\
     function grow() { var a = []; for (;;) { a.push({}); } }\n\
     Promise.resolve().then(function () { log.push('a'); });\n\
     var combined = Promise.all([1]);\n\
     Promise.resolve().then(function () { log.push('c'); });\n\
     Object.defineProperty(Array.prototype, 'then', {\n\
       get: function () { log.push('b'); ends(); }, configurable: true\n\
     });";

/// Section 10 of `docs/design/host-interface.md`, "The microtask checkpoint": a termination in
/// an element function, by the step budget, the memory budget or the interrupt, ends the job
/// that runs it and the checkpoint; the combined promise stays pending and the job behind it
/// stays queued, which the next checkpoint runs.
#[test]
fn a_termination_in_an_element_function_leaves_the_combined_promise_pending() {
    let cases = [
        (Termination::StepBudget, "function ends() { for (;;) {} }"),
        (Termination::MemoryBudget, "function ends() { grow(); }"),
        (
            Termination::Interrupted,
            "function ends() { interruptNow(); for (;;) {} }",
        ),
    ];
    for (reason, ends) in cases {
        let setup = format!("{ends}\n{ELEMENT_JOBS}\n0");
        let mut limits = limits();
        if reason == Termination::MemoryBudget {
            let (mut probe, probe_realm) = fresh(limits);
            define_function(
                &mut probe,
                &probe_realm,
                "interruptNow",
                0,
                interrupt_now,
                false,
            );
            answer(&mut probe, &probe_realm, &setup);
            limits.memory_bytes = collected(&mut probe) + 32 * 1024;
        }
        if reason == Termination::StepBudget {
            limits.steps_per_entry = 1_000;
        }
        let (mut agent, realm) = fresh(limits);
        INTERRUPT.with(|interrupt| *interrupt.borrow_mut() = Some(agent.interrupt_handle()));
        define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
        answer(&mut agent, &realm, &setup);
        let outcome = agent.perform_microtask_checkpoint().unwrap();
        assert!(
            matches!(outcome, HostOutcome::Terminated(ended) if ended == reason),
            "{reason:?}: {outcome:?}"
        );
        assert_eq!(agent.pending_microtasks(), Ok(1), "{reason:?}");
        assert!(!agent.interrupt_handle().is_set());
        answer(&mut agent, &realm, "delete Array.prototype.then; 0");
        assert_eq!(
            checkpoint(&mut agent),
            CheckpointReport {
                promise_jobs: 1,
                host_microtasks: 0
            },
            "{reason:?}"
        );
        assert_eq!(
            answer(&mut agent, &realm, "log.join()"),
            "\"a,b,c\"",
            "{reason:?}"
        );
        assert_eq!(
            settled(&mut agent, &realm, "combined"),
            "Pending",
            "{reason:?}"
        );
    }
}

/// The re-entry limit of [`promise_all_at_reentry_depth_rejects_with_a_range_error`].
const REENTRY: u32 = 16;

/// The script of [`promise_all_at_reentry_depth_rejects_with_a_range_error`] at level `k`: `f`
/// recurses through `reenter` `k` times, then calls `Promise.all` as `C.all`, with the this value
/// `C`, whose capability is a promise of `%Promise%` and whose `resolve` is a script function
/// that re-enters twice through `reenter` before it answers `Promise.resolve(v)`, over an
/// iterable whose `next` is a script function that answers 1 and then done, and whose `return`
/// counts its calls.
fn at_level(k: u32) -> String {
    format!(
        "var returns = 0, i = 0, combined;\n\
         var it = {{ next: function () {{ i++; return i > 1 ? {{ done: true }} : {{ done: false, value: 1 }}; }},\n\
           'return': function () {{ returns++; return {{}}; }} }};\n\
         it[Symbol.iterator] = function () {{ return it; }};\n\
         function C(executor) {{ return new Promise(executor); }}\n\
         C.all = Promise.all;\n\
         C.resolve = function (v) {{\n\
           reenter(function () {{ reenter(function () {{}}); }});\n\
           return Promise.resolve(v);\n\
         }};\n\
         function f(k) {{\n\
           if (k === 0) {{ combined = C.all(it); return; }}\n\
           reenter(function () {{ f(k - 1); }});\n\
         }}\n\
         f({k}); 0"
    )
}

/// "Depth bounds" (`docs/design/engine.md`, decision 3): every call `Promise.all` makes into code
/// is one re-entry, and a refusal inside it is a throw of the loop. A script recurses through
/// `reenter` to the deepest level at which `Promise.all` still makes its capability (one level
/// deeper the executor of its NewPromiseCapability is refused, which that operation answers
/// with a `TypeError` thrown at once): there the call of `C.resolve` re-enters twice and passes
/// `reentry_depth`, the iterator is not done, so `return` is called once, and the combined
/// promise reads `Rejected` with the `RangeError`, read with `promise_state` with no checkpoint
/// between. One level shallower it is fulfilled with `[1]`. The refusal is two calls inside
/// `C.resolve`, not the static's own call: a call the static makes is refused at the depth at
/// which its capability's executor already is, one call deeper.
#[test]
fn promise_all_at_reentry_depth_rejects_with_a_range_error() {
    let mut deepest = None;
    for k in (REENTRY - 4..=REENTRY).rev() {
        let (mut agent, realm) = stack_agent(REENTRY);
        if let Outcome::Normal(_) = eval_text(&mut agent, &realm, &at_level(k)) {
            deepest = Some(k);
            assert_eq!(
                settled(&mut agent, &realm, "combined"),
                "Rejected(RangeError)",
                "at level {k}"
            );
            assert_eq!(answer(&mut agent, &realm, "returns"), "1", "at level {k}");
            break;
        }
    }
    let Some(deepest) = deepest else {
        panic!("Promise.all threw at every level from {} on", REENTRY - 4);
    };
    println!("Promise.all makes its capability down to level {deepest} of {REENTRY}");
    let (mut agent, realm) = stack_agent(REENTRY);
    answer(&mut agent, &realm, &at_level(deepest - 1));
    assert_eq!(answer(&mut agent, &realm, "returns"), "0");
    checkpoint(&mut agent);
    assert_eq!(settled(&mut agent, &realm, "combined"), "Fulfilled(object)");
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "var v; combined.then(function (x) { v = x; }); 0"
        ),
        "0"
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "v.join()"), "\"1\"");
}

/// The `frame_depth` of [`promise_all_at_frame_depth_rejects_with_a_range_error`].
const FRAMES: u32 = 32;

/// "Depth bounds": a script function `Promise.all` calls runs in a frame of its own, checked
/// against `frame_depth`. A script measures how many frames its functions get (`room`, the
/// global code's frame below them), then recurses until one frame is left below `frame_depth`
/// and calls `Promise.all` with a constructor whose `resolve` calls a function: the constructor,
/// `next`, `resolve` and `return` each fit in the last frame, the function `resolve` calls does
/// not, so `resolve` throws the `RangeError`, `return` is called once, and the combined promise
/// reads `Rejected` with a `RangeError`, with no checkpoint between. One frame shallower it is
/// fulfilled. The iterator's `[Symbol.iterator]` is `%Iterator.prototype%`'s, a builtin.
#[test]
fn promise_all_at_frame_depth_rejects_with_a_range_error() {
    let mut limits = limits();
    limits.frame_depth = FRAMES;
    let (mut agent, realm) = fresh(limits);
    answer(
        &mut agent,
        &realm,
        "var room = 0;\n\
         function probe(k) { room = k; probe(k + 1); }\n\
         try { probe(1); } catch (e) {}\n\
         function attempt(depth) {\n\
           var state = { returns: 0, i: 0 };\n\
           var it = Object.create(Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())));\n\
           it.next = function () { state.i++; return state.i > 1 ? { done: true } : { done: false, value: 1 }; };\n\
           it['return'] = function () { state.returns++; return {}; };\n\
           function helper(v) { return Promise.resolve(v); }\n\
           var C = function (executor) { return new Promise(executor); };\n\
           C.resolve = function (v) { return helper(v); };\n\
           function g(k) { if (k === 1) { state.combined = Promise.all.call(C, it); return; } g(k - 1); }\n\
           g(depth);\n\
           return state;\n\
         }\n\
         var deep = attempt(room - 2), shallow = attempt(room - 3);\n\
         var deepCombined = deep.combined, shallowCombined = shallow.combined;\n\
         0",
    );
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "[room, deep.returns, shallow.returns].join()"
        ),
        format!("\"{},1,0\"", FRAMES - 1)
    );
    assert_eq!(
        settled(&mut agent, &realm, "deepCombined"),
        "Rejected(RangeError)"
    );
    checkpoint(&mut agent);
    assert_eq!(
        settled(&mut agent, &realm, "shallowCombined"),
        "Fulfilled(object)"
    );
}

/// The script of [`promise_all_over_an_iterator_without_end_ends_at_a_budget`]: `Promise.all`
/// over an iterator whose `next` answers `{ done: false, value: p }` for ever, `p` a promise that
/// stays pending, all of it held by nothing once the script ends.
const WITHOUT_END: &str = "(function () {\n\
     var p = new Promise(function () {});\n\
     var it = { next: function () { return { done: false, value: p }; } };\n\
     it[Symbol.iterator] = function () { return it; };\n\
     Promise.all(it);\n\
   })();";

/// The steps of the step budget case of
/// [`promise_all_over_an_iterator_without_end_ends_at_a_budget`].
const WITHOUT_END_STEPS: u64 = 1_000;

/// The room above the realm's reading that the memory budget case of
/// [`promise_all_over_an_iterator_without_end_ends_at_a_budget`] gives the script.
const WITHOUT_END_ROOM: u64 = 128 * 1024;

/// The bytes a collection after the script of
/// [`promise_all_over_an_iterator_without_end_ends_at_a_budget`] may leave above the reading
/// before it: room for what the realm may keep of a script, measured at 0 bytes in both cases.
const WITHOUT_END_KEPT: u64 = 8 * 1024;

/// "Bounds": a loop over an iterator that never ends is ended by the step budget, with
/// `last_entry_steps()` at most `steps_per_entry`, or, as its list of values, its element
/// functions and the reactions on `p` grow, by the memory budget. After each, the script's
/// values are held by nothing, and one `collect()` brings `heap_bytes()` back to the reading
/// before the script within [`WITHOUT_END_KEPT`]: the list and the element functions are
/// credited. A first run grows the arenas to what the run needs, which a collection keeps; the
/// reading before is taken after it. The test prints the readings.
#[test]
fn promise_all_over_an_iterator_without_end_ends_at_a_budget() {
    for reason in [Termination::StepBudget, Termination::MemoryBudget] {
        let mut limits = limits();
        limits.steps_per_entry = match reason {
            Termination::StepBudget => WITHOUT_END_STEPS,
            _ => 1 << 40,
        };
        if reason == Termination::MemoryBudget {
            let (mut probe, _probe_realm) = fresh(limits);
            limits.memory_bytes = collected(&mut probe) + WITHOUT_END_ROOM;
        }
        let (mut agent, realm) = fresh(limits);
        let first = collected(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, WITHOUT_END),
            Outcome::Terminated(reason)
        );
        let before = collected(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, WITHOUT_END),
            Outcome::Terminated(reason)
        );
        let steps = agent.last_entry_steps().unwrap();
        let after = collected(&mut agent);
        println!(
            "{reason:?}: heap_bytes first {first}, before {before}, after {after}; {steps} steps"
        );
        if reason == Termination::StepBudget {
            assert!(steps <= WITHOUT_END_STEPS, "{steps} steps");
        }
        assert!(
            after <= before + WITHOUT_END_KEPT,
            "{after} against {before}"
        );
    }
}

/// `#sec-promise.prototype.finally`: the value of the promise passes through a `finally` whose
/// callback answers something else, and so does a rejection; the callback is called with no
/// argument. thenFinally and catchFinally are functions of `length` 1 and the empty `name`; an
/// onFinally that is not callable is handed to `then` as it is. `finally` has `length` 1.
#[test]
fn finally_passes_the_value_through_and_calls_its_callback_with_no_argument() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "var out = [], seen = [];\n\
             Promise.resolve(1).finally(function () { out.push('cb ' + arguments.length); return 'other'; })\n\
               .then(function (v) { out.push('value ' + v); });\n\
             var p = Promise.resolve(3);\n\
             p.then = function (a, b) { seen = [typeof a, a.length, a.name === '', typeof b, b.length, a !== b];\n\
               return Promise.prototype.then.call(this, a, b); };\n\
             p.finally(function () {});\n\
             var q = Promise.resolve(4);\n\
             q.then = function (a, b) { seen.push(a, b); return Promise.prototype.then.call(this, a, b); };\n\
             q.finally(5);\n\
             [seen.join(), Promise.prototype.finally.length, Promise.prototype.finally.name].join('|');"
        ),
        "\"function,1,true,function,1,true,5,5|1|finally\""
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out.join()"), "\"cb 0,value 1\"");
    answer(
        &mut agent,
        &realm,
        "out = [];\n\
         Promise.reject(2).finally(function () { out.push('cb ' + arguments.length); return 'other'; })\n\
           .then(null, function (e) { out.push('reason ' + e); });",
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out.join()"),
        "\"cb 0,reason 2\""
    );
}

/// `#sec-promise.prototype.finally`: a throw of the callback, or a rejection of the promise it
/// answers, rejects the promise `finally` answers, whatever the promise was settled with; a
/// `this` that is not an object throws a `TypeError` before anything else.
#[test]
fn finally_rejects_with_the_throw_of_its_callback() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var out = [], boom = new Error('cb');\n\
         Promise.resolve(1).finally(function () { throw boom; })\n\
           .then(null, function (e) { out.push('thrown ' + (e === boom)); });\n\
         Promise.reject(2).finally(function () { return Promise.reject('later'); })\n\
           .then(null, function (e) { out.push('rejected ' + e); });",
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "out.sort().join()"),
        "\"rejected later,thrown true\""
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "Promise.prototype.finally.call(1);"),
        Outcome::Throw("TypeError".to_string())
    );
}

/// `#sec-promise.try`: the callback is called at once, with the arguments after it and an
/// undefined `this`; its value fulfills the promise, its throw rejects it. A `this` that is not an
/// object throws a `TypeError`. `length` 1.
#[test]
fn promise_try_calls_its_callback_at_once_and_rejects_with_its_throw() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "'use strict'; var log = [], boom = new Error('x');\n\
             var p1 = Promise.try(function (a, b) { log.push('called ' + a + b + ' ' + arguments.length + ' ' + this); return 'v'; }, 1, 2);\n\
             log.push('after');\n\
             var p2 = Promise.try(function () { throw boom; });\n\
             p1.then(function (v) { log.push('p1 ' + v); });\n\
             p2.then(null, function (e) { log.push('p2 ' + (e === boom)); });\n\
             [log.join(), p1 instanceof Promise, Promise.try.length].join('|');"
        ),
        "\"called 12 2 undefined,after|true|1\""
    );
    checkpoint(&mut agent);
    assert_eq!(
        answer(&mut agent, &realm, "log.join()"),
        "\"called 12 2 undefined,after,p1 v,p2 true\""
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "Promise.try.call(1, function () {});"),
        Outcome::Throw("TypeError".to_string())
    );
}

/// `#sec-promise.withResolvers`: an ordinary object with `promise`, `resolve` and `reject`, in
/// that order, the two functions those of the promise. A `this` that is not a constructor
/// throws a `TypeError`. `length` 0.
#[test]
fn with_resolvers_answers_a_promise_and_its_two_functions() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "var r = Promise.withResolvers(), s = Promise.withResolvers(), out = [];\n\
             r.resolve(5); r.reject(6); s.reject(7);\n\
             r.promise.then(function (v) { out.push('r ' + v); });\n\
             s.promise.then(null, function (e) { out.push('s ' + e); });\n\
             [Object.keys(r).join('+'), r.promise instanceof Promise, typeof r.resolve, typeof r.reject,\n\
              Object.getPrototypeOf(r) === Object.prototype, Promise.withResolvers.length].join();"
        ),
        "\"promise+resolve+reject,true,function,function,true,0\""
    );
    checkpoint(&mut agent);
    assert_eq!(answer(&mut agent, &realm, "out.join()"), "\"r 5,s 7\"");
    assert_eq!(
        eval_text(&mut agent, &realm, "Promise.withResolvers.call({});"),
        Outcome::Throw("TypeError".to_string())
    );
}

/// The properties `#sec-properties-of-the-promise-constructor` gives `%Promise%` for the statics:
/// each a method of the `length` its clause states and its name, { [[Writable]]: true,
/// [[Enumerable]]: false, [[Configurable]]: true }, not a constructor.
#[test]
fn the_statics_have_the_length_and_name_of_their_clauses() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "var out = [];\n\
             ['all', 'allSettled', 'any', 'race', 'try', 'withResolvers'].forEach(function (name) {\n\
               var d = Object.getOwnPropertyDescriptor(Promise, name), threw = false;\n\
               try { new d.value(); } catch (e) { threw = e instanceof TypeError; }\n\
               out.push([name, d.value.length, d.value.name === name, d.writable, d.enumerable, d.configurable, threw].join(' '));\n\
             });\n\
             out.join();"
        ),
        "\"all 1 true true false true true,allSettled 1 true true false true true,\
         any 1 true true false true true,race 1 true true false true true,\
         try 1 true true false true true,withResolvers 0 true true false true true\""
    );
}

/// `#sec-aggregate-error`: `AggregateError(errors, message, options)` with or without `new`
/// makes an object with [[ErrorData]] whose own `message`, `cause` and `errors` come in that
/// order, `errors` a new Array of what the iterable gave, writable, configurable and not
/// enumerable; `errors` that is not iterable throws a `TypeError`.
/// `#sec-properties-of-the-aggregate-error-constructors` and
/// `#sec-properties-of-the-aggregate-error-prototype-objects`: `length` 2, the [[Prototype]]
/// `%Error%`, and a prototype whose [[Prototype]] is `%Error.prototype%`, with `name` and the
/// empty `message`.
#[test]
fn an_aggregate_error_holds_its_errors_as_an_array() {
    let (mut agent, realm) = fresh(limits());
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "function* g() { yield 'a'; yield 'b'; }\n\
             var e = new AggregateError(g(), 'msg', { cause: 'c' });\n\
             var d = Object.getOwnPropertyDescriptor(e, 'errors'), f = AggregateError([1]);\n\
             [Array.isArray(e.errors), e.errors.join('+'), d.writable, d.enumerable, d.configurable, e.message,\n\
              e.cause, Object.getOwnPropertyNames(e).join('+'), f instanceof AggregateError, f.errors.length,\n\
              Object.prototype.hasOwnProperty.call(f, 'message'), AggregateError.length, AggregateError.name,\n\
              Object.getPrototypeOf(AggregateError) === Error,\n\
              Object.getPrototypeOf(AggregateError.prototype) === Error.prototype,\n\
              AggregateError.prototype.name, AggregateError.prototype.message === '',\n\
              AggregateError.prototype.constructor === AggregateError,\n\
              Object.prototype.toString.call(e), Error.isError(e), String(e)].join();"
        ),
        "\"true,a+b,true,false,true,msg,c,message+cause+errors,true,1,false,2,AggregateError,\
         true,true,AggregateError,true,true,[object Error],true,AggregateError: msg\""
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new AggregateError();"),
        Outcome::Throw("TypeError".to_string())
    );
}

/// The summary `error_summary` reads of the thrown value of `source`.
fn summary_of_throw(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> ErrorSummary {
    let HostOutcome::Throw(thrown) = eval(agent, realm, source) else {
        panic!("{source} did not throw");
    };
    agent.error_summary(&thrown).unwrap().unwrap()
}

/// HOST-201 (ecma262:sec-properties-of-aggregate-error-instances) After the script `throw new
/// AggregateError([new TypeError("t")], "m")` answered `Throw`, `error_summary` of its root
/// answers kind `AggregateError` and message `[0x6D]`, also when the script set the error's
/// prototype to `null` before the throw; the rejection reason of the promise the script
/// `Promise.any([])` answers, read with `promise_state` with no checkpoint between
/// (PerformPromiseAny rejects it at once), answers kind `AggregateError` and message `None`;
/// `ErrorKind::AggregateError.name()` answers `"AggregateError"`, and `ErrorKind::ALL` holds
/// seven kinds and not `AggregateError`.
#[test]
fn host_201_error_summary_reads_the_kind_of_an_aggregate_error() {
    let (mut agent, realm) = fresh(limits());
    let expected = ErrorSummary {
        kind: ErrorKind::AggregateError,
        message: Some(vec![0x6D]),
    };
    assert_eq!(
        summary_of_throw(
            &mut agent,
            &realm,
            "throw new AggregateError([new TypeError(\"t\")], \"m\")"
        ),
        expected
    );
    assert_eq!(
        summary_of_throw(
            &mut agent,
            &realm,
            "var e = new AggregateError([new TypeError(\"t\")], \"m\");\n\
             Object.setPrototypeOf(e, null); throw e;"
        ),
        expected
    );
    let HostOutcome::Normal(promise) = eval(&mut agent, &realm, "Promise.any([])") else {
        panic!("Promise.any([]) did not complete normally");
    };
    let PromiseState::Rejected(reason) = agent.promise_state(&promise).unwrap() else {
        panic!("Promise.any([]) is not rejected");
    };
    assert_eq!(
        agent.error_summary(&reason).unwrap(),
        Some(ErrorSummary {
            kind: ErrorKind::AggregateError,
            message: None,
        })
    );
    assert_eq!(ErrorKind::AggregateError.name(), "AggregateError");
    assert_eq!(ErrorKind::ALL.len(), 7);
    assert!(!ErrorKind::ALL.contains(&ErrorKind::AggregateError));
}

/// The script of HOST-202, run with `e` a global holding the error `new_error` made.
const HOST_202: &str = "e instanceof AggregateError && Object.getPrototypeOf(e) === AggregateError.prototype && Array.isArray(e.errors) && e.errors.length === 0 && e.message === \"m\" && !Object.prototype.propertyIsEnumerable.call(e, \"errors\")";

/// HOST-202 (swell:D4) `new_error(realm, ErrorKind::AggregateError, "m")` runs no code and
/// answers an object for which the script `e instanceof AggregateError &&
/// Object.getPrototypeOf(e) === AggregateError.prototype && Array.isArray(e.errors) &&
/// e.errors.length === 0 && e.message === "m" &&
/// !Object.prototype.propertyIsEnumerable.call(e, "errors")`, run in that realm with `e` a global
/// holding it, answers `true`, and `error_summary` of it answers kind `AggregateError`. That no
/// code runs is read from setters of `errors` and `message` and a `[Symbol.iterator]` of
/// `Array.prototype` that the script installs, each of which would count a call.
#[test]
fn host_202_new_error_makes_an_aggregate_error_with_no_errors() {
    let (mut agent, realm) = fresh(limits());
    answer(
        &mut agent,
        &realm,
        "var calls = 0;\n\
         function count() { calls++; }\n\
         Object.defineProperty(Object.prototype, 'errors', { set: count, configurable: true });\n\
         Object.defineProperty(AggregateError.prototype, 'message', { set: count, configurable: true });\n\
         Array.prototype[Symbol.iterator] = function () { calls++; return [].values(); };\n\
         0",
    );
    let global_root = global(&mut agent, &realm);
    let (error, summary) = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let error = cx
                .new_error(realm_ref, ErrorKind::AggregateError, "m")
                .unwrap()
                .unwrap();
            let summary = cx.error_summary(Value::Object(error)).unwrap();
            let global = object(cx, &global_root);
            let key = PropertyKey::String(cx.new_string("e").unwrap());
            cx.set(global, key, Value::Object(error), true)
                .unwrap()
                .unwrap();
            (cx.root(Value::Object(error)).unwrap(), summary)
        })
        .unwrap();
    assert_eq!(answer(&mut agent, &realm, "calls"), "0");
    assert_eq!(
        summary.map(|summary| summary.kind),
        Some(ErrorKind::AggregateError)
    );
    answer(
        &mut agent,
        &realm,
        "delete Object.prototype.errors; delete AggregateError.prototype.message;\n\
         Object.defineProperty(AggregateError.prototype, 'message', { value: '', writable: true, configurable: true });\n\
         0",
    );
    assert_eq!(answer(&mut agent, &realm, HOST_202), "true");
    assert_eq!(
        agent.error_summary(&error).unwrap().map(|s| s.kind),
        Some(ErrorKind::AggregateError)
    );
}

/// `docs/design/host-interface.md`, "The kind of an AggregateError, and kinds to come": a host
/// object made with `error_kind` `AggregateError` gets [[ErrorData]] of that kind and no
/// `errors`: `error_summary` answers that kind, `Error.isError` of it is true, and it has no own
/// `errors`.
#[test]
fn a_host_object_with_the_aggregate_error_kind_has_no_errors() {
    let (mut agent, realm) = fresh(limits());
    let global_root = global(&mut agent, &realm);
    let summary = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let host_object = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::ObjectPrototypeOf(realm_ref),
                    brand: Brand(201),
                    data: None,
                    error_kind: Some(ErrorKind::AggregateError),
                })
                .unwrap()
                .unwrap();
            let global = object(cx, &global_root);
            let key = PropertyKey::String(cx.new_string("h").unwrap());
            cx.set(global, key, Value::Object(host_object), true)
                .unwrap()
                .unwrap();
            cx.error_summary(Value::Object(host_object)).unwrap()
        })
        .unwrap();
    assert_eq!(
        summary,
        Some(ErrorSummary {
            kind: ErrorKind::AggregateError,
            message: None,
        })
    );
    assert_eq!(
        answer(
            &mut agent,
            &realm,
            "[Error.isError(h), Object.prototype.hasOwnProperty.call(h, 'errors'), 'errors' in h].join()"
        ),
        "\"true,false,false\""
    );
}
