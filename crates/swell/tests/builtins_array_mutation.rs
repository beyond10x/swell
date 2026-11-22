//! The methods of `Array.prototype` that change their receiver or sort (ECMA-262 23.1.3,
//! `story:array-mutation-and-sort-methods`): `copyWithin` (`#sec-array.prototype.copywithin`),
//! `fill` (`#sec-array.prototype.fill`), `pop` (`#sec-array.prototype.pop`), `reverse`
//! (`#sec-array.prototype.reverse`), `shift` (`#sec-array.prototype.shift`), `sort`
//! (`#sec-array.prototype.sort`) with SortIndexedProperties (`#sec-sortindexedproperties`) and
//! CompareArrayElements (`#sec-comparearrayelements`), `splice`
//! (`#sec-array.prototype.splice`), `toSorted` (`#sec-array.prototype.tosorted`) and `unshift`
//! (`#sec-array.prototype.unshift`); the step budget, the memory budget and the Rust stack
//! `Limits::min_stack_bytes` promises through a comparator and a conversion
//! (`docs/design/engine.md`, decisions 2 to 4).
//!
//! Each test runs scripts, in sloppy and in strict code. A result is compared element by
//! element or through `join`; the elements a sort test orders are literals or made in a loop
//! with `push`. A constructor with its own @@species needs a well-known symbol, which the host
//! interface does not hand out, so that part of `splice` is a unit test of
//! `builtins/array/mutation.rs`, beside the test of a receiver frozen by SetIntegrityLevel.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{AgentState, Limits, Termination};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Both modes of `source` with `limits` answer `expected`.
fn both_with(limits: Limits, source: &str, expected: &Outcome) {
    assert_eq!(&run(limits, source), expected, "sloppy: {source}");
    assert_eq!(
        &run(limits, &format!("\"use strict\";\n{source}")),
        expected,
        "strict: {source}"
    );
}

/// The nine methods in the order of their clauses, with their "length".
const METHODS: [(&str, u32); 9] = [
    ("copyWithin", 2),
    ("fill", 1),
    ("pop", 0),
    ("reverse", 0),
    ("shift", 0),
    ("sort", 1),
    ("splice", 2),
    ("toSorted", 1),
    ("unshift", 1),
];

/// Clause 18 and 23.1.3: each method is an own property of `Array.prototype`, writable, not
/// enumerable and configurable, a function with its "name" and its "length", and not a
/// constructor.
#[test]
fn each_method_is_a_property_of_array_prototype() {
    for (name, length) in METHODS {
        both_answer(
            &format!(
                "var d = Object.getOwnPropertyDescriptor(Array.prototype, \"{name}\");\n\
                 var f = d.value, constructs = true;\n\
                 try {{ new f(); }} catch (e) {{ constructs = !(e instanceof TypeError); }}\n\
                 [typeof f, d.writable, d.enumerable, d.configurable, f.name, f.length,\n\
                  constructs].join();"
            ),
            &format!("\"function,true,false,true,{name},{length},false\""),
        );
    }
}

/// `#sec-array.prototype.pop` and `#sec-array.prototype.shift`, step 3: an array-like whose
/// length is 0 (or reads as 0 by ToLength) gets the Number +0 as its "length" and answers
/// undefined. Otherwise `pop` answers the last element and `shift` the first, moving the others
/// down: a hole moves as a hole, and the last index is deleted.
#[test]
fn pop_and_shift_of_an_empty_array_like_set_length_zero() {
    for method in ["pop", "shift"] {
        both_answer(
            &format!(
                "var r = [];\n\
                 var lengths = [undefined, null, -5, \"0\", NaN, 0.5];\n\
                 for (var i = 0; i < lengths.length; i++) {{\n\
                   var o = {{ length: lengths[i] }};\n\
                   var v = Array.prototype.{method}.call(o);\n\
                   r.push(v === undefined && o.length === 0 && typeof o.length);\n\
                 }}\n\
                 var p = {{}};\n\
                 Array.prototype.{method}.call(p);\n\
                 r.push(p.length === 0);\n\
                 r.join();"
            ),
            "\"number,number,number,number,number,number,true\"",
        );
    }
    both_answer(
        "var a = [1, 2, 3];\n\
         var v = a.pop();\n\
         var o = { length: 2, 0: \"x\", 1: \"y\" };\n\
         var w = Array.prototype.pop.call(o);\n\
         [v, a.length, a.join(), w, o.length, o.hasOwnProperty(\"1\")].join();",
        "\"3,2,1,2,y,1,false\"",
    );
    both_answer(
        "var a = [1, , 3, 4];\n\
         var v = a.shift();\n\
         var o = { length: 3, 0: \"x\", 2: \"z\" };\n\
         var w = Array.prototype.shift.call(o);\n\
         [v, a.length, 0 in a, a.join(), w, o.length, 0 in o, o[1], 2 in o].join();",
        "\"1,3,false,,3,4,x,2,false,z,false\"",
    );
}

/// `#sec-array.prototype.unshift`, step 4.a: with items, a length plus their count above
/// 2^53 - 1 is a TypeError before anything is written; without items nothing moves and the
/// length is set again. Otherwise the elements move up by the count, holes as holes, and the
/// items are written from index 0.
#[test]
fn unshift_refuses_a_length_above_2_53() {
    both_answer(
        "var o = { length: 9007199254740991 }, thrown = false;\n\
         try { Array.prototype.unshift.call(o, 1); } catch (e) { thrown = e instanceof TypeError; }\n\
         var n = Array.prototype.unshift.call(o);\n\
         [thrown, o.length, n, 0 in o].join();",
        "\"true,9007199254740991,9007199254740991,false\"",
    );
    both_answer(
        "var o = { length: 9007199254740990 }, thrown = false;\n\
         try { Array.prototype.unshift.call(o, 1, 2); } catch (e) { thrown = e instanceof TypeError; }\n\
         [thrown, o.length].join();",
        "\"true,9007199254740990\"",
    );
    both_answer(
        "var a = [1, , 3];\n\
         var n = a.unshift(\"a\", \"b\");\n\
         [n, a.join(), 3 in a, a.length].join();",
        "\"5,a,b,1,,3,false,5\"",
    );
}

/// `#sec-array.prototype.splice` with each count of arguments: none deletes nothing; a start
/// alone deletes to the end; a delete count is clamped between 0 and what follows the start;
/// items replace the deleted elements, the rest moving down or up, holes as holes; a negative
/// start counts from the end. The array of the deleted elements is made by ArraySpeciesCreate
/// (`#sec-arrayspeciescreate`): an Array for an array-like; for an Array whose "constructor" is
/// undefined or a function without @@species, an Array; one whose "constructor" is neither an
/// object nor undefined throws a TypeError.
#[test]
fn splice_of_every_argument_count_and_the_species() {
    both_answer(
        "var r = [];\n\
         function show(d, a) { r.push(d.length + \":\" + d.join(\"\") + \"/\" + a.join(\"\")); }\n\
         var a = [1, 2, 3, 4, 5]; show(a.splice(), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(2), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, 2), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, undefined), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, -3), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, 10), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, 0, \"x\", \"y\"), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(-2, 1, \"x\"), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(1, 3, \"x\"), a);\n\
         a = [1, 2, 3, 4, 5]; show(a.splice(-10, 1, \"x\", \"y\", \"z\"), a);\n\
         r.join();",
        "\"0:/12345,3:345/12,2:23/145,0:/12345,0:/12345,4:2345/1,0:/1xy2345,1:4/123x5,3:234/1x5,1:1/xyz2345\"",
    );
    both_answer(
        "var a = [1, , 3, , 5];\n\
         var d = a.splice(1, 2);\n\
         [d.length, 0 in d, d[1], a.length, 1 in a, a.join(), 3 in a].join();",
        "\"2,false,3,3,false,1,,5,false\"",
    );
    both_answer(
        "var o = { length: 3, 0: \"a\", 1: \"b\", 2: \"c\" };\n\
         var d = Array.prototype.splice.call(o, 0, 1);\n\
         [Array.isArray(d), d.join(), o.length, o[0], o[1], 2 in o].join();",
        "\"true,a,2,b,c,false\"",
    );
    both_answer(
        "var r = [];\n\
         var a = [1, 2, 3]; a.constructor = undefined;\n\
         var d = a.splice(0, 1); r.push(Array.isArray(d), Object.getPrototypeOf(d) === Array.prototype);\n\
         a = [1, 2, 3]; a.constructor = function () { throw 1; };\n\
         d = a.splice(0, 1); r.push(Array.isArray(d));\n\
         a = [1, 2, 3]; a.constructor = 1;\n\
         try { a.splice(0, 1); r.push(\"no throw\"); } catch (e) { r.push(e instanceof TypeError, a.length); }\n\
         r.join();",
        "\"true,true,true,true,3\"",
    );
}

/// `#sec-array.prototype.reverse`: the pairs from both ends are swapped; where one of a pair is
/// a hole, the other is deleted and its value written in its place, so a hole stays a hole at
/// its mirrored index, on an Array and on an array-like.
#[test]
fn reverse_keeps_holes_as_holes() {
    both_answer(
        "var a = [1, , 3, 4, , ];\n\
         var b = a.reverse();\n\
         [b === a, a.length, 0 in a, a[1], a[2], 3 in a, a[4], a.join()].join();",
        "\"true,5,false,4,3,false,1,,4,3,,1\"",
    );
    both_answer(
        "var o = { length: 4, 0: \"a\", 3: \"d\", 1: \"b\" };\n\
         Array.prototype.reverse.call(o);\n\
         [o[0], 1 in o, o[2], o[3], o.length].join();",
        "\"d,false,b,a,4\"",
    );
    both_answer("[].reverse().length + [7].reverse().join();", "\"07\"");
}

/// `#sec-array.prototype.fill` and `#sec-array.prototype.copywithin` with ToClampedIndex
/// (`#sec-toclampedindex`): a negative index counts from the end and both ends are clamped
/// between 0 and the length; an undefined end is the length; NaN is 0. `copyWithin` copies
/// backwards when the target overlaps the source from above, and a hole of the source deletes
/// the index it would be copied to.
#[test]
fn fill_and_copy_within_clamp_from_both_ends() {
    both_answer(
        "var r = [];\n\
         function f(s, e) { r.push([1, 2, 3, 4, 5].fill(0, s, e).join(\"\")); }\n\
         f(); f(-2); f(1, -1); f(-10, 10); f(Infinity); f(NaN, 2); f(3, 1); f(-Infinity, undefined);\n\
         r.push(Array.prototype.fill.call({ length: 2 }, \"x\")[1]);\n\
         r.join();",
        "\"00000,12300,10005,00000,12345,00345,12345,00000,x\"",
    );
    both_answer(
        "var r = [];\n\
         function c(t, s, e) { r.push([1, 2, 3, 4, 5].copyWithin(t, s, e).join(\"\")); }\n\
         c(0, 3); c(-2, 0); c(1, 0); c(0, 1, -1); c(10, 0); c(0, -10, 2); c(2, 0, Infinity); c(0, 4, 2);\n\
         var h = [1, , 3].copyWithin(0, 1);\n\
         r.push(0 in h, h[1], h[2]);\n\
         r.join();",
        "\"45345,12312,11234,23445,12345,12345,12123,12345,false,3,3\"",
    );
}

/// The receiver that refuses writes: an Array of three elements whose elements and "length"
/// were defined again by `Object.defineProperty` as neither writable nor configurable. Each of
/// the eight methods that write throws a TypeError, and every element and the length are as
/// before; `toSorted` answers a new Array.
#[test]
fn a_receiver_that_refuses_writes_throws_and_keeps_its_elements() {
    let calls = [
        "copyWithin(0, 1)",
        "fill(9)",
        "pop()",
        "reverse()",
        "shift()",
        "sort()",
        "splice(0, 1)",
        "unshift(9)",
    ];
    for call in calls {
        both_answer(
            &format!(
                "var a = [3, 1, 2];\n\
                 for (var i = 0; i < 3; i++)\n\
                   Object.defineProperty(a, i, {{ writable: false, configurable: false }});\n\
                 Object.defineProperty(a, \"length\", {{ writable: false }});\n\
                 var thrown = false;\n\
                 try {{ a.{call}; }} catch (e) {{ thrown = e instanceof TypeError; }}\n\
                 [thrown, a.length, a.join()].join();"
            ),
            "\"true,3,3,1,2\"",
        );
    }
    both_answer(
        "var a = [3, 1, 2];\n\
         for (var i = 0; i < 3; i++)\n\
           Object.defineProperty(a, i, { writable: false, configurable: false });\n\
         Object.defineProperty(a, \"length\", { writable: false });\n\
         var b = a.toSorted();\n\
         [b !== a, Array.isArray(b), b.join(), a.join()].join();",
        "\"true,true,1,2,3,3,1,2\"",
    );
}

/// The elements of a stability test: `n` objects pushed in a loop, each with a key `k` of 13
/// values spread over the indices and its index `i`, sorted by `k` with a comparator; the
/// result is ordered by `k`, and by `i` where `k` is equal (`#sec-sortindexedproperties`: "the
/// sort is stable"). The same with `toSorted`, which leaves the receiver as it was.
fn stable_sort_source(n: u32) -> String {
    format!(
        "var n = {n}, a = [];\n\
         for (var i = 0; i < n; i++) a.push({{ k: (i * 7919) % 13, i: i }});\n\
         function by(x, y) {{ return x.k - y.k; }}\n\
         function ordered(s) {{\n\
           if (s.length !== n) return false;\n\
           for (var j = 1; j < n; j++) {{\n\
             var p = s[j - 1], q = s[j];\n\
             if (p.k > q.k || (p.k === q.k && p.i >= q.i)) return false;\n\
           }}\n\
           return true;\n\
         }}\n\
         var t = a.toSorted(by);\n\
         var unchanged = a[0].i === 0 && a[n - 1].i === n - 1;\n\
         var s = a.sort(by);\n\
         [ordered(t), unchanged, s === a, ordered(a)].join();"
    )
}

/// `#sec-sortindexedproperties`: the sort is stable for 11, 513 and 2,048 elements (128 for
/// 2,048 in the gate). With the
/// feature `collect-every-safe-point` every instruction of the comparator collects the whole
/// heap: with 11 and 513 elements this test took 258 s alone in that mode (1.6 s with 2,048
/// more in the default one, measured), so that mode sorts 11 and 17 elements. A value held
/// without a root shows at any size, and 17 still merges a run of one and copies the result
/// back from the buffer after an odd number of passes, as 2,048 does. The default mode's gate
/// sorts 128 elements where the lane `slow` (`SWELL_FULL_COUNTS=1`) sorts 2,048: the merge
/// sort has no size threshold, and 128 takes it through 7 passes, an odd number, so the result
/// is copied back from the buffer as after the 11 passes of 2,048, while 513 merges a run of one
/// after 10 (the test took 1.60 CPU seconds alone with 2,048, measured on 2026-11-22).
#[test]
fn sort_is_stable_for_11_513_and_2048_elements() {
    let sizes: &[u32] = if cfg!(feature = "collect-every-safe-point") {
        &[11, 17]
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        &[11, 513, 2048]
    } else {
        &[11, 513, 128]
    };
    let mut limits = limits();
    limits.steps_per_entry = 10_000_000;
    for &n in sizes {
        both_with(
            limits,
            &stable_sort_source(n),
            &Outcome::Normal("\"true,true,true,true\"".to_string()),
        );
    }
}

/// `#sec-comparearrayelements`, steps 5 to 9: without a comparator both values go through
/// ToString and the Strings are compared by their code units, not by their code points or as
/// Numbers: "10" before "9", "B" before "a", "b" before "null" (which `join` writes as the
/// empty String) before "true", U+1F600 (D83D DE00) before U+FF61; equal Strings keep their
/// order, and an object is compared by its `toString`.
#[test]
fn sort_without_a_comparator_compares_strings_by_code_unit() {
    both_answer(
        "var a = [\"b\", 9, \"a\", 10, \"B\", true, null, 1, \"\"];\n\
         a.sort();\n\
         a.join(\"|\");",
        "\"|1|10|9|B|a|b||true\"",
    );
    both_answer(
        "var a = [\"\\uFF61\", \"\\uD83D\\uDE00\", \"\\uD800\", \"z\"];\n\
         a.sort();\n\
         [a[0] === \"z\", a[1] === \"\\uD800\", a[2] === \"\\uD83D\\uDE00\", a[3] === \"\\uFF61\"].join();",
        "\"true,true,true,true\"",
    );
    both_answer(
        "var x = { toString: function () { return \"m\"; } };\n\
         var y = { toString: function () { return \"m\"; } };\n\
         var a = [\"z\", x, \"a\", y];\n\
         a.sort();\n\
         [a[0], a[1] === x, a[2] === y, a[3]].join();",
        "\"a,true,true,z\"",
    );
}

/// `#sec-comparearrayelements`, steps 1 to 3, and `#sec-array.prototype.sort`, steps 8 and 9
/// (its Note 1): undefined sorts after every other value and is never passed to the comparator;
/// the holes come after the undefined values, as holes, and the length stays.
#[test]
fn sort_puts_undefined_last_and_holes_after_it() {
    both_answer(
        "var a = [3, undefined, , 1, undefined, , 2];\n\
         var seen = false;\n\
         a.sort(function (x, y) { if (x === undefined || y === undefined) seen = true; return x - y; });\n\
         [seen, a.length, a[0], a[1], a[2], a[3] === undefined, 3 in a, a[4] === undefined, 4 in a,\n\
          5 in a, 6 in a].join();",
        "\"false,7,1,2,3,true,true,true,true,false,false\"",
    );
    both_answer(
        "var o = { length: 4, 0: undefined, 2: \"b\", 3: \"a\" };\n\
         Array.prototype.sort.call(o);\n\
         [o[0], o[1], o[2] === undefined, 2 in o, 3 in o, o.length].join();",
        "\"a,b,true,true,false,4\"",
    );
}

/// The elements of the arrays the lying comparators of
/// [`sort_with_a_comparator_that_throws_or_lies_ends`] sort: 40, the full count, in the default
/// mode; with the feature `collect-every-safe-point`, where every comparator call collects, 12
/// in the gate and 20 in the lane `slow` (`SWELL_FULL_COUNTS=1`) (with 40 the test took 22.1 s
/// there at a load of 100 to 110 on 17 cores, cargo-nextest, and 6.0 CPU seconds alone in the
/// lane; 12 took 1.8, measured on 2026-11-22). The sort is a merge sort bottom up from runs of
/// one, so 12 elements still take it through four passes and 20 through five, with every lie
/// answered in each.
fn lying_sort_elements() -> u32 {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    match (cfg!(feature = "collect-every-safe-point"), full) {
        (true, false) => 12,
        (true, true) => 20,
        (false, _) => 40,
    }
}

/// `#sec-sortindexedproperties`, step 4: a comparator that throws ends the sort in its throw
/// before anything is written; one that answers inconsistently (in turn negative, zero and
/// positive, NaN, an object with a `valueOf`) or changes the array while it runs ends the sort
/// with the elements it had, in some order ([`lying_sort_elements`] of them).
#[test]
fn sort_with_a_comparator_that_throws_or_lies_ends() {
    both_answer(
        "var a = [5, 4, 3, 2, 1], c = 0, caught;\n\
         try { a.sort(function () { if (++c === 3) throw \"stop\"; return 1; }); }\n\
         catch (e) { caught = e; }\n\
         [caught, c, a.join(\"\")].join();",
        "\"stop,3,54321\"",
    );
    let n = lying_sort_elements();
    let last = n - 1;
    both_answer(
        &format!(
            "function permutation(s) {{\n\
               var seen = [], count = 0;\n\
               for (var i = 0; i < {n}; i++) if (!seen[s[i]]) {{ seen[s[i]] = true; count++; }}\n\
               return count === {n} && seen.length === {n};\n\
             }}\n\
             var r = [];\n\
             var c = 0;\n\
             var lies = [\n\
               function () {{ return (c++ % 3) - 1; }},\n\
               function () {{ return NaN; }},\n\
               function () {{ return {{ valueOf: function () {{ return c++ % 2 ? 1 : -1; }} }}; }},\n\
               function (x, y) {{ a.push(x); return y - x; }}\n\
             ];\n\
             for (var k = 0; k < lies.length; k++) {{\n\
               var a = [];\n\
               for (var i = 0; i < {n}; i++) a.push(i);\n\
               a.sort(lies[k]);\n\
               r.push(permutation(a) && (k === 3 ? a.length > {n} && a[0] === {last} : a.length === {n}));\n\
             }}\n\
             r.join();"
        ),
        "\"true,true,true,true\"",
    );
}

/// `#sec-array.prototype.tosorted`: a new Array of the receiver's length; a hole is read as
/// undefined (read-through-holes) and sorts last, as an own element; the receiver keeps its
/// elements and its hole; an array-like receiver gives an Array; a length above 2^32 - 1 is a
/// RangeError of ArrayCreate.
#[test]
fn to_sorted_leaves_the_receiver_unchanged_and_reads_holes_as_undefined() {
    both_answer(
        "var a = [3, , 1];\n\
         var b = a.toSorted();\n\
         [b !== a, Array.isArray(b), b.length, b[0], b[1], b[2] === undefined, b.hasOwnProperty(2),\n\
          a.join(), 1 in a].join();",
        "\"true,true,3,1,3,true,true,3,,1,false\"",
    );
    both_answer(
        "var o = { length: 2, 0: \"b\", 1: \"a\" };\n\
         var b = Array.prototype.toSorted.call(o, function (x, y) { return x < y ? -1 : 1; });\n\
         [Array.isArray(b), b.join(), o[0]].join();",
        "\"true,a,b,b\"",
    );
    both_answer(
        "var r;\n\
         try { Array.prototype.toSorted.call({ length: 4294967296 }); r = \"no throw\"; }\n\
         catch (e) { r = e instanceof RangeError; }\n\
         r;",
        "true",
    );
}

/// `#sec-array.prototype.sort` and `#sec-array.prototype.tosorted`, step 1: a comparator that is
/// neither undefined nor callable is a TypeError before the this value is converted and before
/// "length" is read.
#[test]
fn sort_refuses_a_comparator_that_is_not_callable_before_it_reads() {
    for method in ["sort", "toSorted"] {
        both_answer(
            &format!(
                "var read = false, r = [];\n\
                 var o = {{ get length() {{ read = true; return 2; }}, 0: 2, 1: 1 }};\n\
                 var bad = [null, 1, \"f\", {{}}, true];\n\
                 for (var i = 0; i < bad.length; i++) {{\n\
                   try {{ Array.prototype.{method}.call(o, bad[i]); r.push(\"no throw\"); }}\n\
                   catch (e) {{ r.push(e instanceof TypeError); }}\n\
                 }}\n\
                 try {{ Array.prototype.{method}.call(undefined, 1); }}\n\
                 catch (e) {{ r.push(e instanceof TypeError); }}\n\
                 Array.prototype.{method}.call(o, undefined);\n\
                 r.push(read);\n\
                 r.join();"
            ),
            "\"true,true,true,true,true,true,true\"",
        );
    }
}

/// The steps an entry of `sort` of `n` Numbers in descending order spends, without a
/// comparator: the Array is made by a first entry, and the second sorts it and answers
/// whether it starts with 0, 1 and 10 (the order of their Strings).
fn sort_steps(n: u32) -> u64 {
    let mut limits = limits();
    limits.steps_per_entry = 10_000_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var a = []; for (var i = 0; i < {n}; i++) a.push({n} - 1 - i); a.length;")
        ),
        Outcome::Normal(n.to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "a.sort(); a[0] === 0 && a[1] === 1 && a[2] === 10;"
        ),
        Outcome::Normal("true".to_string())
    );
    agent.last_entry_steps().unwrap()
}

/// The step budget (decision 2): a sort is charged a step per comparison and per element moved,
/// and its reads and writes a step per index, so the entry that sorts `n` elements spends at
/// least `n` steps and at most 8 n log2 n (2,048 elements; with the feature
/// `collect-every-safe-point` 513 in the lane `slow`, `SWELL_FULL_COUNTS=1`, and 129 in the
/// gate: with 513 the test took 2.1 CPU seconds there, run alone, most of it in the loop that
/// makes the Array, which collects at every instruction. 129 is 2^7 + 1 as 513 is 2^9 + 1, so
/// the bottom-up merge sort still carries a lone last run through every pass).
#[test]
fn sort_charges_a_step_per_comparison() {
    let n: u32 = if !cfg!(feature = "collect-every-safe-point") {
        2048
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        513
    } else {
        129
    };
    let steps = sort_steps(n);
    let n = f64::from(n);
    let bound = 8.0 * n * n.log2();
    assert!(steps as f64 >= n, "{steps} steps for {n} elements");
    assert!(
        (steps as f64) <= bound,
        "{steps} steps for {n} elements, above {bound}"
    );
}

/// Every loop of the nine methods charges a step per index (decision 2): on an array-like of
/// length 2^32 - 1 without elements each of `sort`, `toSorted`, `reverse`, `shift`, `unshift`,
/// `splice`, `fill` and `copyWithin` ends in `Terminated(StepBudget)` under a budget of 10,000
/// steps, and `pop` answers.
#[test]
fn step_budget_ends_a_sort_and_a_reverse() {
    let mut limits = limits();
    limits.steps_per_entry = 10_000;
    for call in [
        "sort()",
        "toSorted()",
        "reverse()",
        "shift()",
        "unshift(1)",
        "splice(0, 0, 1)",
        "splice(1)",
        "fill(0)",
        "copyWithin(1, 0)",
    ] {
        let (method, args) = call.split_once('(').unwrap();
        let args = args.trim_end_matches(')');
        let args = if args.is_empty() {
            String::new()
        } else {
            format!(", {args}")
        };
        both_with(
            limits,
            &format!("var o = {{ length: 4294967295 }}; Array.prototype.{method}.call(o{args});"),
            &Outcome::Terminated(Termination::StepBudget),
        );
    }
    both_with(
        limits,
        "var o = { length: 4294967295 }; Array.prototype.pop.call(o); o.length;",
        &Outcome::Normal("4294967294".to_string()),
    );
}

/// The bytes an agent with the test limits holds after `create_realm`.
fn realm_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// The memory budget (decision 4): the list a sort holds is charged as it grows, so `toSorted`
/// of an array-like of 100,000 indices, all read as undefined, under 512 KiB beyond the realm
/// ends in `Terminated(MemoryBudget)` with `heap_bytes()` within the budget; the agent stays
/// alive and answers the next script.
#[test]
fn memory_budget_ends_a_sort_of_a_long_array_like() {
    let mut limits = limits();
    limits.memory_bytes = realm_bytes() + (512 << 10);
    limits.steps_per_entry = 10_000_000;
    let budget = limits.memory_bytes;
    for call in [
        "Array.prototype.toSorted.call({ length: 100000 });",
        "Array.prototype.toSorted.call({ length: 100000 }, function () { return 0; });",
    ] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, call),
            Outcome::Terminated(Termination::MemoryBudget),
            "{call}"
        );
        let bytes = agent.heap_bytes().unwrap();
        assert!(bytes <= budget, "{bytes} > {budget}");
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "[2, 1].toSorted().join();"),
            Outcome::Normal("\"1,2\"".to_string())
        );
    }
}

/// The body of the stack test: the three runs, one agent each.
fn reach_through_mutation_builtins() {
    for through in [
        "[1, 0].sort(f)",
        "[1, 0].toSorted(f)",
        "[0].fill(0, { valueOf: f })",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through the comparator of `sort` and of `toSorted` and the
/// `valueOf` ToClampedIndex calls in `fill` (decision 3): on a thread with seven eighths of
/// `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting`
/// 1,000, a script reaches `reentry_depth` re-entries through each and parses 1,000 nested
/// brackets at the deepest, without a stack overflow. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn comparator_and_conversions_reach_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("comparator_and_conversions_reach_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_mutation_builtins,
    )
    .unwrap();
}
