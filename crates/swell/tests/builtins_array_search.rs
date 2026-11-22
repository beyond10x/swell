//! The search and copy methods of `Array` (ECMA-262 23.1.2 and 23.1.3,
//! `story:array-search-and-copy-methods`): `Array.of` (`#sec-array.of`), `at`
//! (`#sec-array.prototype.at`), `concat` (`#sec-array.prototype.concat`), `includes`
//! (`#sec-array.prototype.includes`), `indexOf` (`#sec-array.prototype.indexof`), `lastIndexOf`
//! (`#sec-array.prototype.lastindexof`), `slice` (`#sec-array.prototype.slice`),
//! `toLocaleString` (`#sec-array.prototype.tolocalestring`), `toReversed`
//! (`#sec-array.prototype.toreversed`), `toSpliced` (`#sec-array.prototype.tospliced`) and
//! `with` (`#sec-array.prototype.with`), each with its property and its steps, in sloppy and in
//! strict code; the step budget over 2^32 - 1 indices, a step per index, and the re-entry limit
//! through a conversion and an element on the stack `Limits::min_stack_bytes` promises.
//!
//! The parts no script can reach before slice 6 (a constructor's @@species, an own
//! @@isConcatSpreadable, `Array.prototype[@@unscopables]` read as a property) are unit tests of
//! `crates/swell/src/builtins/array/search.rs`; the language scripts of `tests/language/` read
//! @@unscopables through `with`. No test here calls a builtin another story of the wave
//! delivers: a result is compared element by element or through `join`, and an element or a
//! `length` that is not writable is made with `Object.defineProperty`. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent, limits, realm, stack_with_room};
use support::script::{Outcome, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::Termination;

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// A script function that describes the property `name` of `holder`: the type of its value,
/// its "length" and "name", and its three attributes.
const DESCRIBE: &str = "function describe(holder, name) { \
       var p = Object.getOwnPropertyDescriptor(holder, name); \
       return typeof p.value + \" \" + p.value.length + \" \" + p.value.name + \" \" + \
         p.writable + \" \" + p.enumerable + \" \" + p.configurable; \
     }";

/// The method `name` of `holder` (a script expression) is a built-in function of `length`
/// named `name`, writable, not enumerable and configurable (clause 18), and not a constructor.
fn is_method(holder: &str, name: &str, length: u32) {
    both_answer(
        &format!(
            "{DESCRIBE} var made; \
             try {{ new ({holder}.{name})(); made = \"constructed\"; }} \
             catch (e) {{ made = e.name; }} \
             describe({holder}, \"{name}\") + \" \" + made;"
        ),
        &format!("\"function {length} {name} true false true TypeError\""),
    );
}

/// `#sec-array.prototype.indexof` and `#sec-array.prototype.includes`: `indexOf` compares by
/// IsStrictlyEqual and skips holes (HasProperty, so an index the prototype has is not a hole),
/// `includes` compares by SameValueZero and reads a hole as undefined; both are generic, read
/// "length" before they convert fromIndex, and answer at a length of 0 without converting it.
#[test]
fn index_of_skips_holes_and_includes_finds_nan_and_undefined_in_holes() {
    is_method("Array.prototype", "indexOf", 1);
    is_method("Array.prototype", "includes", 1);
    both_answer(
        "var a = [1, , NaN, undefined, \"1\", -0]; \
         [a.indexOf(1), a.indexOf(\"1\"), a.indexOf(NaN), a.indexOf(undefined), a.indexOf(0), \
          a.indexOf(2), [, ].indexOf(undefined)].join();",
        "\"0,4,-1,3,5,-1,-1\"",
    );
    both_answer(
        "var a = [1, , NaN, -0]; \
         [a.includes(NaN), a.includes(undefined), a.includes(0), a.includes(\"1\"), \
          [, ].includes(undefined), [].includes(undefined)].join();",
        "\"true,true,true,false,true,false\"",
    );
    both_answer(
        "Object.prototype[1] = \"p\"; var a = [0, , 2]; \
         a.indexOf(\"p\") + \",\" + a.includes(\"p\");",
        "\"1,true\"",
    );
    both_answer(
        "var o = { length: 3, 1: \"x\", 2: \"y\" }; \
         [Array.prototype.indexOf.call(o, \"y\"), Array.prototype.includes.call(o, \"x\"), \
          Array.prototype.indexOf.call(\"abc\", \"c\"), \
          Array.prototype.includes.call(\"abc\", \"b\")].join();",
        "\"2,true,2,true\"",
    );
    both_answer(
        "var log = \"\"; \
         var o = { get length() { log += \"l\"; return 2; } }; \
         var from = { valueOf: function () { log += \"f\"; return 0; } }; \
         Array.prototype.indexOf.call(o, 1, from); Array.prototype.includes.call(o, 1, from); \
         var never = { valueOf: function () { throw \"converted\"; } }; \
         log + \",\" + [].indexOf(1, never) + \",\" + [].includes(1, never);",
        "\"lflf,-1,false\"",
    );
    both_answer(
        "try { Array.prototype.indexOf.call(undefined, 1); \"none\"; } catch (e) { e.name; }",
        "\"TypeError\"",
    );
}

/// `#sec-array.prototype.lastindexof`: it searches down from fromIndex, or from the last
/// index when fromIndex is not present (an undefined fromIndex is present and is 0), skips
/// holes and compares by IsStrictlyEqual.
#[test]
fn last_index_of_searches_down_and_skips_holes() {
    is_method("Array.prototype", "lastIndexOf", 1);
    both_answer(
        "var a = [1, , 1, NaN, undefined]; \
         [a.lastIndexOf(1), a.lastIndexOf(NaN), a.lastIndexOf(undefined), a.lastIndexOf(1, 1), \
          a.lastIndexOf(1, undefined), [, ].lastIndexOf(undefined), [].lastIndexOf(1), \
          Array.prototype.lastIndexOf.call({ length: 2, 0: \"z\" }, \"z\")].join();",
        "\"2,-1,4,0,0,-1,-1,0\"",
    );
}

/// ToClampedIndex (`#sec-toclampedindex`) for `indexOf` and `includes`, and ToAbsoluteIndex
/// (`#sec-toabsoluteindex`) with the minimum of length - 1 for `lastIndexOf`: a negative
/// fromIndex counts from the end, one beyond either end is clamped (to 0 or the length for the
/// first two, to length - 1 for `lastIndexOf`, which finds nothing below index 0), the
/// infinities are clamped, and a fraction or a String is converted to an integer.
#[test]
fn from_index_is_clamped_from_both_ends() {
    both_answer(
        "var a = [1, 2, 3, 1]; \
         [a.indexOf(1, 1), a.indexOf(1, -1), a.indexOf(1, -10), a.indexOf(1, 4), \
          a.indexOf(1, Infinity), a.indexOf(1, -Infinity), a.indexOf(1, 0.9), \
          a.indexOf(1, \"1\")].join();",
        "\"3,3,0,-1,-1,0,0,3\"",
    );
    both_answer(
        "var a = [1, 2, 3, 1]; \
         [a.lastIndexOf(1, 2), a.lastIndexOf(1, -1), a.lastIndexOf(1, -4), \
          a.lastIndexOf(1, -5), a.lastIndexOf(1, 10), a.lastIndexOf(1, -Infinity), \
          a.lastIndexOf(1, Infinity), a.lastIndexOf(1, -0.5)].join();",
        "\"0,3,0,-1,3,-1,3,0\"",
    );
    both_answer(
        "var a = [1, 2, 3, 1]; \
         [a.includes(1, 4), a.includes(1, -1), a.includes(2, -1), a.includes(2, -10), \
          a.includes(3, NaN), a.includes(1, Infinity), a.includes(2, -Infinity)].join();",
        "\"false,true,false,true,true,false,true\"",
    );
}

/// `#sec-array.prototype.at`: ToAbsoluteIndex of the argument; an index outside the array
/// answers undefined without a Get; generic over an array-like and a String.
#[test]
fn at_reads_relative_to_the_end() {
    is_method("Array.prototype", "at", 1);
    both_answer(
        "var a = [1, 2, 3]; \
         [a.at(0), a.at(-1), a.at(-3), a.at(3), a.at(-4), a.at(1.7), a.at(\"2\"), a.at(), \
          a.at(Infinity), a.at(-Infinity), Array.prototype.at.call(\"xyz\", -1)].join();",
        "\"1,3,1,,,2,3,1,,,z\"",
    );
    both_answer(
        "var read = \"\"; \
         var o = { length: 2, get 5() { read += \"5\"; return 0; }, get 1() { read += \"1\"; \
           return \"one\"; } }; \
         Array.prototype.at.call(o, 5) + \",\" + Array.prototype.at.call(o, -1) + \",\" + read;",
        "\"undefined,one,1\"",
    );
}

/// `#sec-array.prototype.slice`: ToClampedIndex of start and end (end undefined is the
/// length), a hole of the range stays a hole, the result's "length" is set to the count, an
/// array-like gives an Array; the species part is a unit test of `search.rs`.
#[test]
fn slice_copies_its_range_and_keeps_holes() {
    is_method("Array.prototype", "slice", 2);
    both_answer(
        "var a = [0, 1, , 3, 4]; var s = a.slice(1, -1); \
         [s.length, s.join(\"-\"), 1 in s, Array.isArray(s), s === a].join();",
        "\"3,1--3,false,true,false\"",
    );
    both_answer(
        "var a = [0, 1, 2, 3]; \
         [a.slice().join(\"\"), a.slice(-2).join(\"\"), a.slice(2, 1).length, \
          a.slice(1, undefined).join(\"\"), a.slice(-Infinity, Infinity).join(\"\"), \
          a.slice(\"1\", 2.9).join(\"\")].join();",
        "\"0123,23,0,123,0123,1\"",
    );
    both_answer(
        "var s = Array.prototype.slice.call({ length: 3, 0: \"a\", 2: \"c\" }, 1); \
         [Array.isArray(s), s.length, 0 in s, s[1]].join();",
        "\"true,2,false,c\"",
    );
}

/// `#sec-array.prototype.concat` with IsConcatSpreadable (`#sec-isconcatspreadable`): the this
/// value and each argument that is an Array is spread, holes kept; any other value, an
/// array-like included, is one element; the result's "length" counts trailing holes; a
/// primitive this value is its wrapper. The @@isConcatSpreadable and the species parts are
/// unit tests of `search.rs`.
#[test]
fn concat_spreads_arrays_and_appends_other_values() {
    is_method("Array.prototype", "concat", 1);
    both_answer(
        "var o = { length: 1, 0: 7 }; \
         var c = [1, 2].concat(3, [4, , 6], o, \"s\"); \
         [c.length, 4 in c, c[6] === o, c.join(\"-\")].join();",
        "\"8,false,true,1-2-3-4--6-[object Object]-s\"",
    );
    both_answer(
        "var c = [1].concat([2, , ]); var d = Array.prototype.concat.call(1, 2); \
         [c.length, typeof d[0], d[0] + d[1], d.length, [].concat().length].join();",
        "\"3,object,3,2,0\"",
    );
}

/// `#sec-array.prototype.with`: a copy with one index replaced (relative to the end when
/// negative), holes read as undefined; an index outside the array throws a RangeError before
/// any element is read, and so does a length above 2^32 - 1 (ArrayCreate).
#[test]
fn with_replaces_one_index_and_refuses_one_outside() {
    is_method("Array.prototype", "with", 2);
    both_answer(
        "var a = [1, 2, 3]; var w = a.with(-1, 9); var h = [, 1].with(1, 2); \
         [w.join(\"\"), Array.isArray(w), 0 in h, h[0], h[1], \
          Array.prototype.with.call({ length: 2, 0: \"a\" }, 1, \"b\").join(\"\")].join();",
        "\"129,true,true,,2,ab\"",
    );
    both_answer(
        "var read = false; \
         var o = { length: 2, get 0() { read = true; return 0; } }; \
         var names = []; \
         try { Array.prototype.with.call(o, 2, 0); } catch (e) { names.push(e.name); } \
         try { Array.prototype.with.call(o, -3, 0); } catch (e) { names.push(e.name); } \
         var big = { length: 4294967296, get 0() { read = true; return 0; } }; \
         try { Array.prototype.with.call(big, 1, 0); } catch (e) { names.push(e.name); } \
         names.join() + \",\" + read;",
        "\"RangeError,RangeError,RangeError,false\"",
    );
}

/// `#sec-array.prototype.toreversed`: a new Array of the elements in reverse order, a hole read
/// as undefined; a length above 2^32 - 1 throws a RangeError before any element is read.
#[test]
fn to_reversed_reverses_and_reads_holes_as_undefined() {
    is_method("Array.prototype", "toReversed", 0);
    both_answer(
        "var r = [1, 2, 3].toReversed(); var h = [, 1].toReversed(); \
         var o = Array.prototype.toReversed.call({ length: 2, 0: \"a\", 1: \"b\" }); \
         [r.join(\"\"), Array.isArray(r), 1 in h, h[0], h[1], o.join(\"\")].join();",
        "\"321,true,true,1,,ba\"",
    );
    both_answer(
        "var read = false; \
         var big = { length: 4294967296, get 0() { read = true; return 0; } }; \
         var name; \
         try { Array.prototype.toReversed.call(big); } catch (e) { name = e.name; } \
         name + \",\" + read;",
        "\"RangeError,false\"",
    );
}

/// `#sec-array.prototype.tospliced`: a copy with skipCount elements at start removed and the
/// items inserted; without start nothing is removed, with start alone everything from it, an
/// undefined skipCount is 0, a skipCount is clamped between 0 and what follows start.
#[test]
fn to_spliced_removes_and_inserts() {
    is_method("Array.prototype", "toSpliced", 2);
    both_answer(
        "var a = [1, 2, 3, 4]; \
         [a.toSpliced(1, 2).join(), a.toSpliced(1, 0, \"x\", \"y\").join(), \
          a.toSpliced(-1).join(), a.toSpliced().join(), a.toSpliced(1, 1, \"x\").join(), \
          a.toSpliced(2, Infinity).join(), a.toSpliced(2, -1).join(), \
          a.toSpliced(1, undefined).join(), [, 1].toSpliced(2, 0, 2).join()].join(\"|\");",
        "\"1,4|1,x,y,2,3,4|1,2,3|1,2,3,4|1,x,3,4|1,2|1,2,3,4|1,2,3,4|,1,2\"",
    );
}

/// `#sec-array.prototype.tospliced`, steps 11 and 12: a new length above 2^53 - 1 throws a
/// TypeError and one above 2^32 - 1 a RangeError (ArrayCreate, `#sec-arraycreate`), both
/// before any element is read.
#[test]
fn to_spliced_of_a_length_above_2_32_throws_before_it_reads() {
    both_answer(
        "var read = false; var names = []; \
         var big = { length: 4294967296, get 0() { read = true; return 0; } }; \
         try { Array.prototype.toSpliced.call(big, 0, 0); } catch (e) { names.push(e.name); } \
         try { Array.prototype.toSpliced.call(big, 1, 0); } catch (e) { names.push(e.name); } \
         var huge = { length: 9007199254740991, get 0() { read = true; return 0; } }; \
         try { Array.prototype.toSpliced.call(huge, 0, 0, 1); } catch (e) { names.push(e.name); } \
         names.join() + \",\" + read;",
        "\"RangeError,RangeError,TypeError,false\"",
    );
}

/// `with`, `toReversed` and `toSpliced` never write to their receiver: after each call the
/// receiver reads as before, element by element and by "length", and the result is a new
/// Array; a receiver whose elements and "length" are not writable (made by
/// `Object.defineProperty`) gives the same results.
#[test]
fn with_to_reversed_and_to_spliced_leave_the_receiver_unchanged() {
    let fixed = "for (var i = 0; i < 3; i++) \
                   Object.defineProperty(a, i, { writable: false, configurable: false }); \
                 Object.defineProperty(a, \"length\", { writable: false });";
    for setup in ["", fixed] {
        both_answer(
            &format!(
                "var a = [1, 2, 3]; {setup} \
                 function state() {{ return a[0] + \",\" + a[1] + \",\" + a[2] + \",\" + \
                   a.length; }} \
                 var log = []; var made; \
                 made = a.with(0, 9); log.push(made.join(\"\"), made !== a, state()); \
                 made = a.toReversed(); log.push(made.join(\"\"), made !== a, state()); \
                 made = a.toSpliced(0, 1, 7, 8); log.push(made.join(\"\"), made !== a, state()); \
                 log.join(\"|\");"
            ),
            "\"923|true|1,2,3,3|321|true|1,2,3,3|7823|true|1,2,3,3\"",
        );
    }
}

/// `#sec-array.prototype.tolocalestring` with JoinIndexedProperties
/// (`#sec-joinindexedproperties`) and Invoke (`#sec-invoke`): each element that is neither
/// undefined nor null is asked for its own `toLocaleString`, with the element as the this value
/// and no argument, in index order, and ToString of the answer is joined with ","; a hole,
/// undefined and null are the empty String; an element whose `toLocaleString` is not callable
/// throws a TypeError. Every element here has a `toLocaleString` of its own: the one of
/// `Object.prototype` and of `Number.prototype` are other stories'.
#[test]
fn to_locale_string_invokes_each_element() {
    is_method("Array.prototype", "toLocaleString", 0);
    both_answer(
        "var log = []; \
         function e(n) { return { n: n, toLocaleString: function () { \
           log.push(this.n + \":\" + arguments.length); return \"e\" + this.n; } }; } \
         var a = [e(1), null, e(2), undefined, , e(3)]; \
         a.toLocaleString(\"de\", {}) + \"|\" + log.join();",
        "\"e1,,e2,,,e3|1:0,2:0,3:0\"",
    );
    both_answer(
        "var n = { toLocaleString: function () { return 5; } }; \
         var o = { toLocaleString: function () { return { toString: function () { \
           return \"o\"; } }; } }; \
         [n, o].toLocaleString() + \",\" + \
         Array.prototype.toLocaleString.call({ length: 1, 0: n }) + \",\" + [].toLocaleString();",
        "\"5,o,5,\"",
    );
    both_answer(
        "try { [{ toLocaleString: 1 }].toLocaleString(); \"none\"; } catch (e) { e.name; }",
        "\"TypeError\"",
    );
}

/// `#sec-array.of`: with a constructor as its this value it constructs it with the count of
/// items and defines each item, then sets "length" (a non-writable "length" throws a
/// TypeError); with any other this value it makes an Array.
#[test]
fn of_constructs_its_this_value() {
    is_method("Array", "of", 0);
    both_answer(
        "var a = Array.of(1, 2, 3); var log = \"\"; \
         function C(n) { log += \"C\" + arguments.length + \":\" + n; } \
         var c = Array.of.call(C, \"x\", \"y\"); \
         [a.length, a.join(\"\"), Array.isArray(a), Array.of().length, Array.of(7).length, \
          log, c instanceof C, Array.isArray(c), c.length, c[0] + c[1], \
          Array.isArray(Array.of.call(undefined, 1)), Array.isArray(Array.of.call({}, 1)), \
          Array.of.call(Math.max, 4)[0]].join();",
        "\"3,123,true,0,1,C1:2,true,false,2,xy,true,true,4\"",
    );
    both_answer(
        "function D() { Object.defineProperty(this, \"length\", { value: 0, writable: false }); } \
         try { Array.of.call(D); \"none\"; } catch (e) { e.name; }",
        "\"TypeError\"",
    );
}

/// The budget of 10,000 steps the searches below are ended by.
const SEARCH_STEPS: u64 = 10_000;

/// Each method loops over indices with a step per index (decision 2): over an Array of length
/// 2^32 - 1 that is all holes, and over an array-like of length 2^53 - 1, each ends in
/// `Terminated(StepBudget)` with a budget of 10,000 steps, in sloppy and in strict code; the
/// next entry runs afresh.
#[test]
fn a_search_of_2_32_minus_1_indices_ends_in_the_step_budget() {
    let calls = [
        "a.indexOf(1)",
        "a.lastIndexOf(1)",
        "a.includes(1)",
        "a.slice()",
        "a.concat()",
        "a.with(0, 1)",
        "a.toReversed()",
        "a.toSpliced(0, 0)",
        "a.toLocaleString()",
        "Array.prototype.indexOf.call(o, 1)",
        "Array.prototype.lastIndexOf.call(o, 1)",
        "Array.prototype.includes.call(o, 1)",
    ];
    let mut limits = limits();
    limits.steps_per_entry = SEARCH_STEPS;
    for call in calls {
        for prologue in ["", "\"use strict\";\n"] {
            let source = format!(
                "{prologue}var a = []; a.length = 4294967295; var o = {{ length: Infinity }}; \
                 {call};"
            );
            assert_eq!(
                run(limits, &source),
                Outcome::Terminated(Termination::StepBudget),
                "{source}"
            );
        }
    }
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var a = []; a.length = 4294967295; a.indexOf(1);"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "[1, 2].indexOf(2);"),
        Outcome::Normal("1".to_string())
    );
}
/// The steps an entry of `source` spends in `agent`'s realm; it must answer the String
/// `expected`, which `show` reads without a call into the agent: the ToString of a Number
/// would be an entry of its own, which starts the counter again.
/// Number would be an entry of its own, which starts the counter again.
fn steps_of(
    agent: &mut swell::Agent<support::host::TestHost>,
    realm: &swell::Realm,
    source: &str,
    expected: &str,
) -> u64 {
    assert_eq!(
        eval_text(agent, realm, source),
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
    agent.last_entry_steps().unwrap()
}

/// Each loop charges one step per index (decision 2), and the Get of an own element, the
/// comparison of two Numbers and the definition of an element of a new Array charge none: a
/// call over an Array of 200 own elements spends 100 steps more than the same call over one of
/// 100 (the answers are made Strings of the same length for both, so the concatenation that
/// makes them, a step per code unit, costs both the same). `toLocaleString` also appends the
/// separator, a step for its one code unit (`append_units`), so 200 more; `at` reads one
/// index, so the same.
#[test]
fn each_search_charges_a_step_per_index() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var small = [], large = [], nulls = [], more = []; \
             for (var i = 0; i < 100; i++) { small.push(0); nulls.push(null); } \
             for (var i = 0; i < 200; i++) { large.push(0); more.push(null); } \
             large.length;"
        ),
        Outcome::Normal("200".to_string())
    );
    let calls = [
        ("indexOf(1)", "-1", 100),
        ("lastIndexOf(1)", "-1", 100),
        ("includes(1)", "false", 100),
        ("slice().length", "", 100),
        ("concat().length", "", 100),
        ("with(0, 1).length", "", 100),
        ("toReversed().length", "", 100),
        ("toSpliced(0, 0).length", "", 100),
        ("at(-1)", "0", 0),
    ];
    for (call, answer, extra) in calls {
        let short = steps_of(
            &mut agent,
            &realm,
            &format!("small.{call} + \"\";"),
            if answer.is_empty() { "100" } else { answer },
        );
        let long = steps_of(
            &mut agent,
            &realm,
            &format!("large.{call} + \"\";"),
            if answer.is_empty() { "200" } else { answer },
        );
        assert_eq!(long - short, extra, "{call}: {short} and {long} steps");
    }
    // `typeof` answers the same String for both, where a concatenation would charge a step
    // per code unit of its result, and "199" has one more than "99".
    let short = steps_of(
        &mut agent,
        &realm,
        "typeof nulls.toLocaleString();",
        "string",
    );
    let long = steps_of(
        &mut agent,
        &realm,
        "typeof more.toLocaleString();",
        "string",
    );
    assert_eq!(
        long - short,
        200,
        "toLocaleString: {short} and {long} steps"
    );
}

/// The body of the stack test: a `valueOf` of fromIndex and an element's `toLocaleString`, one
/// agent each, to the re-entry limit.
fn reach_through_conversions_and_elements() {
    for through in [
        "[0].indexOf(0, { valueOf: f })",
        "[{ toLocaleString: f }].toLocaleString()",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through the two ways these builtins call back into script (the
/// conversion of an argument and Invoke of an element): on a thread with seven eighths of
/// `min_stack_bytes()` ([`stack_with_room`]) for `frame_depth` 10,000, `reentry_depth` 1,000 and
/// `parser_nesting` 1,000, a script reaches `reentry_depth` re-entries through each, parses
/// 1,000 nested brackets at the deepest and throws the `RangeError` of the limit, without a
/// stack overflow. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn conversions_and_elements_reach_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("conversions_and_elements_reach_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_conversions_and_elements,
    )
    .unwrap();
}
