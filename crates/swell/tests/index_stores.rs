//! A store at an array index, as scripts see it: OrdinarySet (`#sec-ordinaryset`) and the Array
//! [[DefineOwnProperty]] (`#sec-array-exotic-objects-defineownproperty-p-desc`) answer alike
//! whether the engine takes its path for an Array that is its own receiver or the general one.
//! Each test states a condition under which that path must not decide alone, and the defect
//! that would turn it red. Written by `story:an-index-store-does-not-go-through-text`; every
//! test here passed on the base of that story, before its path for an Array existed. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval, eval_text, run_both};
use swell::Outcome as HostOutcome;

/// A String answer as [`support::script::show`] writes it.
fn normal(text: &str) -> Outcome {
    Outcome::Normal(format!("\"{text}\""))
}

/// `put(o, i, v)` stores `o[i] = v` and counts in `errors` the TypeError a refused store throws
/// in strict code, so that one script runs to its end in both modes.
const PUT: &str = "var errors = 0; \
    function put(o, i, v) { \
      try { o[i] = v; } catch (e) { if (e instanceof TypeError) errors++; else throw e; } \
    }\n";

/// A setter for an index on `Array.prototype` (one hop) and on `Object.prototype` (two hops)
/// is called by a store at the end of the dense part and at a hole, and the Array gets no
/// element and keeps its length. The defect it holds: a store at the end or at a hole that
/// does not look at the prototypes for the index.
#[test]
fn a_setter_on_a_prototype_is_called_for_an_index_the_array_lacks() {
    // The log is a String: an Array log would store at the very index the setter is for.
    let source = "var log = ''; \
        Object.defineProperty(Array.prototype, '1', \
          { set: function (v) { log += ' A' + v; }, configurable: true }); \
        Object.defineProperty(Object.prototype, '4', \
          { set: function (v) { log += ' O' + v; }, configurable: true }); \
        var a = [0]; a[1] = 5; \
        var b = [0, , 2]; b[1] = 6; \
        var c = [0, 1, 2, 3]; c[4] = 7; \
        delete Array.prototype[1]; delete Object.prototype[4]; \
        [log, a.length, 1 in a, b.length, 1 in b, c.length, 4 in c].join();";
    let answer = normal(" A5 A6 O7,1,false,3,false,4,false");
    assert_eq!(run_both(source), (answer.clone(), answer));
}

/// A non-writable data property for an index on a prototype refuses the store: false in
/// sloppy code, a TypeError in strict code, and the Array keeps its length and gets no
/// element. The defect it holds: a store that writes the receiver without reading the
/// attributes the chain holds for the index.
#[test]
fn a_read_only_index_on_a_prototype_refuses_the_store() {
    let source = format!(
        "{PUT}Object.defineProperty(Object.prototype, '0', \
          {{ value: 'p', writable: false, configurable: true }}); \
        Object.defineProperty(Array.prototype, '2', \
          {{ value: 'q', writable: false, configurable: true }}); \
        var a = []; put(a, 0, 1); \
        var b = [0, 1]; put(b, 2, 1); \
        var r = [a.length, a[0], Object.keys(a).length, b.length, b[2], errors].join(); \
        delete Object.prototype[0]; delete Array.prototype[2]; r;"
    );
    assert_eq!(
        run_both(&source),
        (normal("0,p,0,2,q,0"), normal("0,p,0,2,q,2"))
    );
}

/// A frozen, a sealed and a non-extensible Array, one whose `length` is not writable and a
/// non-extensible one with a hole: a store at an element each holds changes it where the
/// element is writable, and a store at the end or at the hole is refused. The defects it
/// holds: a store at the end that does not read [[Extensible]] or the length's [[Writable]],
/// and a store at an element that does not read its [[Writable]].
#[test]
fn integrity_levels_and_a_read_only_length_refuse_as_before() {
    let source = format!(
        "{PUT}var f = Object.freeze([1, 2]); put(f, 0, 9); put(f, 2, 3); \
        var s = Object.seal([1, 2]); put(s, 0, 9); put(s, 2, 3); \
        var n = Object.preventExtensions([1, 2]); put(n, 1, 8); put(n, 2, 3); \
        var l = [1, 2]; Object.defineProperty(l, 'length', {{ writable: false }}); \
        put(l, 0, 7); put(l, 2, 3); \
        var h = [1, , 3]; Object.preventExtensions(h); put(h, 1, 2); \
        [f.join(), f.length, s.join(), s.length, n.join(), n.length, l.join(), l.length, \
         h.length, 1 in h, errors].join(';');"
    );
    assert_eq!(
        run_both(&source),
        (
            normal("1,2;2;9,2;2;1,8;2;7,2;2;3;false;0"),
            normal("1,2;2;9,2;2;1,8;2;7,2;2;3;false;6")
        )
    );
}

/// An exotic object on the chain: a String object answers its indices as read-only own
/// properties (`#sec-string-exotic-objects-getownproperty-p`), so a store at one of them is
/// refused and one past them is made; an arguments object with a read-only index refuses as
/// well. A generator object, whose methods are all ordinary
/// (`#sec-properties-of-generator-instances`), lets a store through when it holds no index and
/// refuses it with a read-only index of its own. The defects it holds: a check of the chain
/// that reads an object's property list and not its [[GetOwnProperty]], and one that does not
/// read the list of a generator object.
#[test]
fn an_exotic_object_on_the_chain_answers_for_its_indices() {
    let source = format!(
        "{PUT}var a = []; Object.setPrototypeOf(a, new String('xyz')); \
        put(a, 0, 'q'); put(a, 3, 'r'); \
        function g(x) {{ \
          var b = []; Object.setPrototypeOf(b, arguments); \
          Object.defineProperty(arguments, '0', {{ writable: false }}); \
          put(b, 0, 5); return [b.length, b[0], x].join(); \
        }} \
        function* gen() {{}} \
        var c = []; Object.setPrototypeOf(c, gen()); put(c, 0, 'g'); \
        var held = gen(); Object.defineProperty(held, '0', {{ value: 'i', writable: false }}); \
        var d = []; Object.setPrototypeOf(d, held); put(d, 0, 'h'); \
        [a.length, Object.prototype.hasOwnProperty.call(a, 0), a[0], a[3], g(1), \
         c.length, c[0], d.length, d[0], errors].join(';');"
    );
    assert_eq!(
        run_both(&source),
        (
            normal("4;false;x;r;0,1,1;1;g;0;i;0"),
            normal("4;false;x;r;0,1,1;1;g;0;i;3")
        )
    );
}

/// Holes and the end: a store fills a hole of the dense part, one at its end raises the
/// length, one below a length set by hand leaves it, one a little beyond the end opens holes,
/// and one at the end after a truncation raises the length again. The defect it holds: a
/// length raised by a store below it, or not raised by one at or past it.
#[test]
fn holes_and_the_end_keep_the_length_right() {
    let source = "var a = [0, , 2]; a[1] = 1; \
        var b = [0]; b[1] = 1; b[2] = 2; \
        var c = []; c.length = 10; c[0] = 'x'; \
        var d = [0]; d[5] = 5; \
        var e = [0, 1, 2]; e.length = 1; e[1] = 'y'; \
        [a.join(), a.length, b.join(), b.length, c.length, c[0], d.length, 3 in d, \
         e.join(), e.length].join(';');";
    let answer = normal("0,1,2;3;0,1,2;3;10;x;6;false;0,y;2");
    assert_eq!(run_both(source), (answer.clone(), answer));
}

/// The steps of a store (`docs/design/engine.md`, "The step budget"): OrdinarySet charges a
/// step per prototype it passes to, and nothing else a store at an index does charges one. The
/// totals of each script below, after a setup entry, are those recorded on the base of
/// `story:an-index-store-does-not-go-through-text` (`3d5afeb`): at an element the Array
/// holds, at its end (two prototypes), at a hole, at the end of an Array with three
/// prototypes and of one with none, and 20 stores at the end in a loop, before and after a
/// truncation (20, not 100, for the mode with a collection at every safe point).
#[test]
fn the_steps_of_a_store_are_unchanged() {
    const SETUP: &str = "var a = [0, 1, 2]; var h = [0, , 2]; \
        var p = Object.create(Array.prototype); var b = []; Object.setPrototypeOf(b, p); \
        var n = []; Object.setPrototypeOf(n, null); var c = []; 0";
    const RECORDED: [(&str, u64); 7] = [
        ("a[1] = 9", 8),
        ("a[3] = 3", 10),
        ("h[1] = 1", 10),
        ("b[0] = 1", 11),
        ("n[0] = 1", 8),
        ("for (var i = 0; i < 20; i++) c[i] = i", 436),
        ("c.length = 0; for (var i = 0; i < 20; i++) c[i] = i", 460),
    ];
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, SETUP),
        Outcome::Normal("0".to_string())
    );
    let seen: Vec<(&str, u64)> = RECORDED
        .iter()
        .map(|&(source, _)| {
            let outcome = eval(&mut agent, &realm, source);
            assert!(matches!(outcome, HostOutcome::Normal(_)), "{source}");
            (source, agent.last_entry_steps().unwrap())
        })
        .collect();
    assert_eq!(seen, RECORDED.to_vec());
}
