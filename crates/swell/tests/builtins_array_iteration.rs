//! The iteration methods of `Array.prototype` (ECMA-262 23.1.3, `story:array-iteration-methods`):
//! `every` (`#sec-array.prototype.every`), `filter` (`#sec-array.prototype.filter`), `find`,
//! `findIndex`, `findLast` and `findLastIndex` (`#sec-array.prototype.find`,
//! `#sec-array.prototype.findindex`, `#sec-array.prototype.findlast`,
//! `#sec-array.prototype.findlastindex`) with FindViaPredicate (`#sec-findviapredicate`), `flat`
//! (`#sec-array.prototype.flat`) and `flatMap` (`#sec-array.prototype.flatmap`) with
//! FlattenIntoArray (`#sec-flattenintoarray`), `forEach` (`#sec-array.prototype.foreach`),
//! `reduce` (`#sec-array.prototype.reduce`), `reduceRight` (`#sec-array.prototype.reduceright`)
//! and `some` (`#sec-array.prototype.some`).
//!
//! Each builtin has a test of its property (clause 18: a function with its `length` and `name`,
//! writable, not enumerable, configurable, not a constructor) and of its steps, in sloppy and in
//! strict code; then the walks over holes, a length read once and changed while the walk runs,
//! the step budget (a step per index: a length of 2^32 - 1 or 2^53 - 1 ends in
//! `Terminated(StepBudget)`), the explicit stack of FlattenIntoArray under a step and a memory
//! budget, what a walk holds across a collection in its callback (run also with the feature
//! `collect-every-safe-point`), and one test per callback method that reaches the re-entry limit
//! on seven eighths of `min_stack_bytes()` (`tests/support/stack.rs`). The parts that need
//! `@@species`, which no script names before slice 6, are unit tests of
//! `src/builtins/array/iteration.rs`. No script here calls a builtin another story of the wave
//! delivers.
#![cfg(test)]

mod support;

use support::host::{agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits};
use swell::{Agent, AgentState, Outcome as HostOutcome, Realm, Termination};

use support::host::TestHost;

/// What `expected` names: `normal <value>` (as `support::script::show` writes it) or
/// `throw <name>`.
fn outcome(expected: &str) -> Outcome {
    if let Some(value) = expected.strip_prefix("normal ") {
        Outcome::Normal(value.to_string())
    } else if let Some(name) = expected.strip_prefix("throw ") {
        Outcome::Throw(name.to_string())
    } else {
        panic!("{expected:?} is neither `normal <value>` nor `throw <name>`")
    }
}

/// Each source answers its expectation, as written and with a `"use strict";` prologue.
fn check(cases: &[(&str, &str)]) {
    for &(source, expected) in cases {
        let expected = outcome(expected);
        let (sloppy, strict) = run_both(source);
        assert_eq!(sloppy, expected, "sloppy: {source}");
        assert_eq!(strict, expected, "strict: {source}");
    }
}

/// A script that reads `Array.prototype[name]`: its type, `length` and `name`, whether a
/// for-in over `Array.prototype` lists it, what `new` of it throws, whether an assignment
/// replaces it, whether `delete` removes it, and whether it is still an own property after.
fn property_script(name: &str) -> String {
    format!(
        "var f = Array.prototype.{name}, r = [typeof f, f.length, f.name], listed = false;\n\
         for (var k in Array.prototype) {{ if (k === \"{name}\") listed = true; }}\n\
         var made = \"made\";\n\
         try {{ new f(); }} catch (e) {{ made = e instanceof TypeError ? \"TypeError\" : \"other\"; }}\n\
         Array.prototype.{name} = 0;\n\
         r.push(listed, made, Array.prototype.{name} === 0, delete Array.prototype.{name},\n\
           Array.prototype.hasOwnProperty(\"{name}\"));\n\
         r.join();"
    )
}

/// Clause 18 for the method `name` of `Array.prototype` with "length" `length`, then `steps`.
fn property_and_steps(name: &str, length: u32, steps: &[(&str, &str)]) {
    let expected = format!("normal \"function,{length},{name},false,TypeError,true,true,false\"");
    let script = property_script(name);
    check(&[(script.as_str(), expected.as_str())]);
    check(steps);
}

/// `#sec-array.prototype.foreach`.
#[test]
fn for_each_has_its_property_and_steps() {
    property_and_steps(
        "forEach",
        1,
        &[
            (
                "var r = [], t = {};\n\
                 var x = [1, , 3].forEach(function (v, i, o) { r.push(v, i, o.length, this === t); }, t);\n\
                 r.push(x); r.join();",
                "normal \"1,0,3,true,3,2,3,true,\"",
            ),
            (
                "var r = []; Array.prototype.forEach.call(\"ab\", function (v, i, o) { r.push(v + i, typeof o); });\n\
                 r.join();",
                "normal \"a0,object,b1,object\"",
            ),
            ("[].forEach(1);", "throw TypeError"),
            (
                "Array.prototype.forEach.call(undefined, function () {});",
                "throw TypeError",
            ),
            (
                "var n = 0; try { [1, 2].forEach(function () { n++; throw \"x\"; }); } catch (e) { n + e; }",
                "normal \"1x\"",
            ),
            (
                "var o = {}, read = 0;\n\
                 Object.defineProperty(o, \"length\", { get: function () { read++; return 0; } });\n\
                 try { Array.prototype.forEach.call(o, null); } catch (e) { read + (e instanceof TypeError ? \"T\" : \"?\"); }",
                "normal \"1T\"",
            ),
        ],
    );
}

/// `#sec-array.prototype.every`.
#[test]
fn every_has_its_property_and_steps() {
    property_and_steps(
        "every",
        1,
        &[
            (
                "var r = []; var x = [1, 2, 3].every(function (v, i) { r.push(i); return v < 2; });\n\
                 r.push(x); r.join();",
                "normal \"0,1,false\"",
            ),
            ("[].every(function () { return false; });", "normal true"),
            ("[, ,].every(function () { return false; });", "normal true"),
            (
                "var t = {}; [1].every(function () { return this === t; }, t);",
                "normal true",
            ),
            (
                "[1, 2].every(function (v) { return v ? {} : \"\"; });",
                "normal true",
            ),
            (
                "[1, 0].every(function (v) { return v ? {} : \"\"; });",
                "normal false",
            ),
            ("[].every(null);", "throw TypeError"),
        ],
    );
}

/// `#sec-array.prototype.some`.
#[test]
fn some_has_its_property_and_steps() {
    property_and_steps(
        "some",
        1,
        &[
            (
                "var r = []; var x = [1, 2, 3].some(function (v, i) { r.push(i); return v > 1; });\n\
                 r.push(x); r.join();",
                "normal \"0,1,true\"",
            ),
            ("[].some(function () { return true; });", "normal false"),
            ("[, ,].some(function () { return true; });", "normal false"),
            (
                "var t = {}; [1].some(function () { return this === t; }, t);",
                "normal true",
            ),
            (
                "[0, \"\"].some(function (v) { return v; });",
                "normal false",
            ),
            ("[].some({});", "throw TypeError"),
        ],
    );
}

/// `#sec-array.prototype.filter`.
#[test]
fn filter_has_its_property_and_steps() {
    property_and_steps(
        "filter",
        1,
        &[
            (
                "var r = [1, , 3, 4, 5].filter(function (v, i, o) { return v % 2 && o.length === 5; });\n\
                 \"\" + Array.isArray(r) + r.length + \":\" + r.join();",
                "normal \"true3:1,3,5\"",
            ),
            (
                "var a = [1, 2, 3]; a.filter(function (v, i) { a[i] = 0; return true; }).join();",
                "normal \"1,2,3\"",
            ),
            (
                "var t = {}; [1, 2].filter(function () { return this === t; }, t).length;",
                "normal 2",
            ),
            (
                "Array.prototype.filter.call({ length: 2, 0: \"a\", 1: \"b\" }, function (v) { return v === \"b\"; }).join();",
                "normal \"b\"",
            ),
            ("[].filter(undefined);", "throw TypeError"),
            (
                "var a = [1]; a.constructor = 0; a.filter(function () { return true; });",
                "throw TypeError",
            ),
        ],
    );
}

/// `#sec-array.prototype.reduce`.
#[test]
fn reduce_has_its_property_and_steps() {
    property_and_steps(
        "reduce",
        1,
        &[
            (
                "[1, 2, 3].reduce(function (a, v, i) { return a + v * 10 + i; });",
                "normal 54",
            ),
            (
                "[1, 2].reduce(function (a, v) { return a + v; }, \"s\");",
                "normal \"s12\"",
            ),
            (
                "var r = [];\n\
                 [, 1, , 2].reduce(function (a, v, i, o) { r.push(a, v, i, o.length, arguments.length); return a + v; });\n\
                 r.join();",
                "normal \"1,2,3,4,4\"",
            ),
            ("[].reduce(function () {});", "throw TypeError"),
            ("[].reduce(1, 0);", "throw TypeError"),
            (
                "Array.prototype.reduce.call(\"abc\", function (a, v) { return v + a; });",
                "normal \"cba\"",
            ),
        ],
    );
}

/// `#sec-array.prototype.reduceright`.
#[test]
fn reduce_right_has_its_property_and_steps() {
    property_and_steps(
        "reduceRight",
        1,
        &[
            (
                "[\"a\", \"b\", \"c\"].reduceRight(function (a, v, i) { return a + v + i; });",
                "normal \"cb1a0\"",
            ),
            (
                "[\"a\", \"b\", \"c\"].reduceRight(function (a, v, i) { return a + v + i; }, \"s\");",
                "normal \"sc2b1a0\"",
            ),
            (
                "var r = []; [1, , 2, ,].reduceRight(function (a, v, i) { r.push(a, v, i); return a + v; });\n\
                 r.join();",
                "normal \"2,1,0\"",
            ),
            ("[, ,].reduceRight(function () {});", "throw TypeError"),
            ("[].reduceRight(function () {}, 7);", "normal 7"),
            ("[].reduceRight(null, 0);", "throw TypeError"),
        ],
    );
}

/// `#sec-array.prototype.find` with `#sec-findviapredicate`.
#[test]
fn find_has_its_property_and_steps() {
    property_and_steps(
        "find",
        1,
        &[
            (
                "var r = []; var x = [1, , 3].find(function (v, i) { r.push(v, i); return v === 3; });\n\
                 r.push(x); r.join();",
                "normal \"1,0,,1,3,2,3\"",
            ),
            (
                "[1, 2].find(function () { return false; });",
                "normal undefined",
            ),
            (
                "var a = [1, 2], r = []; a.find(function (v) { r.push(v); delete a[1]; }); r.join();",
                "normal \"1,\"",
            ),
            (
                "var t = {}; [5].find(function () { return this === t; }, t);",
                "normal 5",
            ),
            ("[].find(1);", "throw TypeError"),
            (
                "Array.prototype.find.call({ length: 1, 0: \"x\" }, function (v, i, o) { return o.length === 1; });",
                "normal \"x\"",
            ),
        ],
    );
}

/// `#sec-array.prototype.findindex` with `#sec-findviapredicate`.
#[test]
fn find_index_has_its_property_and_steps() {
    property_and_steps(
        "findIndex",
        1,
        &[
            (
                "[5, , 6].findIndex(function (v) { return v === undefined; });",
                "normal 1",
            ),
            (
                "[5, 6].findIndex(function () { return false; });",
                "normal -1",
            ),
            (
                "var r = []; [7, 8].findIndex(function (v, i, o) { r.push(v, i, o.length); }); r.join();",
                "normal \"7,0,2,8,1,2\"",
            ),
            ("[].findIndex(null);", "throw TypeError"),
            ("[].findIndex(function () { return true; });", "normal -1"),
        ],
    );
}

/// `#sec-array.prototype.findlast` with `#sec-findviapredicate`.
#[test]
fn find_last_has_its_property_and_steps() {
    property_and_steps(
        "findLast",
        1,
        &[
            (
                "var r = []; var x = [1, 2, 3].findLast(function (v, i) { r.push(i); return v < 3; });\n\
                 r.push(x); r.join();",
                "normal \"2,1,2\"",
            ),
            (
                "var r = []; [1, , 3].findLast(function (v, i) { r.push(v, i); }); r.join();",
                "normal \"3,2,,1,1,0\"",
            ),
            (
                "[1, 2].findLast(function () { return false; });",
                "normal undefined",
            ),
            ("[].findLast(1);", "throw TypeError"),
            (
                "var t = {}; [5, 6].findLast(function () { return this === t; }, t);",
                "normal 6",
            ),
        ],
    );
}

/// `#sec-array.prototype.findlastindex` with `#sec-findviapredicate`.
#[test]
fn find_last_index_has_its_property_and_steps() {
    property_and_steps(
        "findLastIndex",
        1,
        &[
            (
                "[5, 6, 5].findLastIndex(function (v) { return v === 5; });",
                "normal 2",
            ),
            (
                "[5, 6].findLastIndex(function () { return false; });",
                "normal -1",
            ),
            (
                "var r = []; [, 8].findLastIndex(function (v, i, o) { r.push(v, i, o.length); }); r.join();",
                "normal \"8,1,2,,0,2\"",
            ),
            ("[].findLastIndex({});", "throw TypeError"),
        ],
    );
}

/// `#sec-array.prototype.flat` with `#sec-flattenintoarray`.
#[test]
fn flat_has_its_property_and_steps() {
    property_and_steps(
        "flat",
        0,
        &[
            (
                "var r = [1, [2, [3, [4]]]].flat(); r.length + \":\" + r[2].length;",
                "normal \"3:2\"",
            ),
            (
                "var r = [1, [2]].flat(0); r.length + \":\" + Array.isArray(r[1]);",
                "normal \"2:true\"",
            ),
            (
                "[1, [2, [3, [4]]]].flat(Infinity).join(\"|\");",
                "normal \"1|2|3|4\"",
            ),
            ("[1, , [2, , 3]].flat().join(\"|\");", "normal \"1|2|3\""),
            (
                "var r = [[[1]]].flat(\"2\"); [[1], [2]].flat(-1).length + \":\" + r.length + typeof r[0];",
                "normal \"2:1number\"",
            ),
            (
                "var r = [{ length: 1, 0: \"x\" }].flat(); r.length + typeof r[0];",
                "normal \"1object\"",
            ),
            ("Array.prototype.flat.call(null);", "throw TypeError"),
        ],
    );
}

/// `#sec-array.prototype.flatmap` with `#sec-flattenintoarray`.
#[test]
fn flat_map_has_its_property_and_steps() {
    property_and_steps(
        "flatMap",
        1,
        &[
            (
                "var r = [1, 2].flatMap(function (v, i) { return [v, [i]]; }); r.length + \":\" + Array.isArray(r[1]);",
                "normal \"4:true\"",
            ),
            (
                "[1, 2].flatMap(function (v) { return v * 2; }).join();",
                "normal \"2,4\"",
            ),
            (
                "var t = {}; [1].flatMap(function (v, i, o) { return [this === t, i, o.length]; }, t).join();",
                "normal \"true,0,1\"",
            ),
            ("[].flatMap(1);", "throw TypeError"),
            (
                "[1, , 2].flatMap(function (v) { return [v]; }).join();",
                "normal \"1,2\"",
            ),
        ],
    );
}

/// The walks of the callback methods and of `flat` (each `#sec-array.prototype.<name>`): over
/// an array-like with elements at 0 and 2 whose "length" is a getter, each reads "length" once,
/// the methods that skip holes call back for 0 and 2, the four of FindViaPredicate for 0, 1 and
/// 2 (descending for the last two), `reduce` without an initial value for 2 only; over an Array
/// whose callback deletes two elements and pushes one at the first call, or sets "length" to 2,
/// the range of indices stays the length read before the walk, the deleted elements are not
/// visited by the methods that skip holes and are visited by FindViaPredicate.
#[test]
fn walks_skip_holes_and_see_a_length_read_once() {
    // (method, callback answer, more arguments, indices over the array-like, after the
    // deletions, after the truncation)
    let walks: [(&str, &str, &str, &str, &str, &str); 13] = [
        ("forEach", "false", "", "0,2", "0,3", "0,1"),
        ("every", "true", "", "0,2", "0,3", "0,1"),
        ("some", "false", "", "0,2", "0,3", "0,1"),
        ("filter", "false", "", "0,2", "0,3", "0,1"),
        ("reduce", "false", ", 0", "0,2", "0,3", "0,1"),
        ("reduce", "false", "", "2", "1,3", "1"),
        ("reduceRight", "false", ", 0", "2,0", "3,0", "3,1,0"),
        ("find", "false", "", "0,1,2", "0,1,2,3", "0,1,2,3"),
        ("findIndex", "false", "", "0,1,2", "0,1,2,3", "0,1,2,3"),
        ("findLast", "false", "", "2,1,0", "3,2,1,0", "3,2,1,0"),
        ("findLastIndex", "false", "", "2,1,0", "3,2,1,0", "3,2,1,0"),
        ("flatMap", "false", "", "0,2", "0,3", "0,1"),
        ("flat", "false", "", "", "", ""),
    ];
    for (method, answer, more, like, deleted, truncated) in walks {
        let callback = if method == "flat" {
            String::new()
        } else {
            format!("f{more}")
        };
        let after_this = if callback.is_empty() {
            String::new()
        } else {
            format!(", {callback}")
        };
        let like_source = format!(
            "var reads = 0, seen = [], o = {{ 0: \"a\", 2: \"c\" }};\n\
             Object.defineProperty(o, \"length\", {{ get: function () {{ reads++; return 3; }} }});\n\
             function f() {{ seen.push(arguments[arguments.length - 2]); return {answer}; }}\n\
             Array.prototype.{method}.call(o{after_this});\n\
             reads + \":\" + seen.join();"
        );
        let like_expected = format!("normal \"1:{like}\"");
        let deleted_source = format!(
            "var seen = [], first = true, a = [1, 2, 3, 4];\n\
             function f() {{\n\
               seen.push(arguments[arguments.length - 2]);\n\
               if (first) {{ first = false; delete a[1]; delete a[2]; a.push(5); }}\n\
               return {answer};\n\
             }}\n\
             a.{method}({callback}); seen.join() + \":\" + a.length;"
        );
        let deleted_expected = format!(
            "normal \"{deleted}:{}\"",
            if method == "flat" { 4 } else { 5 }
        );
        let truncated_source = format!(
            "var seen = [], first = true, a = [1, 2, 3, 4];\n\
             function f() {{\n\
               seen.push(arguments[arguments.length - 2]);\n\
               if (first) {{ first = false; a.length = 2; }}\n\
               return {answer};\n\
             }}\n\
             a.{method}({callback}); seen.join() + \":\" + a.length;"
        );
        let truncated_expected = format!(
            "normal \"{truncated}:{}\"",
            if method == "flat" { 4 } else { 2 }
        );
        check(&[
            (like_source.as_str(), like_expected.as_str()),
            (deleted_source.as_str(), deleted_expected.as_str()),
            (truncated_source.as_str(), truncated_expected.as_str()),
        ]);
    }
}

/// `#sec-array.prototype.reduce` step 4 and `#sec-array.prototype.reduceright` step 4: without
/// an initial value an empty array, an array of holes and an empty array-like throw a
/// `TypeError` before any call; an initial value that is undefined is present; one element
/// present and no initial value answers that element without a call.
#[test]
fn reduce_without_an_initial_value_on_an_empty_array_throws() {
    check(&[
        ("[].reduce(function () {});", "throw TypeError"),
        ("[, , ,].reduce(function () {});", "throw TypeError"),
        ("[].reduceRight(function () {});", "throw TypeError"),
        ("[, , ,].reduceRight(function () {});", "throw TypeError"),
        (
            "Array.prototype.reduce.call({ length: 0 }, function () {});",
            "throw TypeError",
        ),
        (
            "Array.prototype.reduceRight.call({ length: 2 }, function () {});",
            "throw TypeError",
        ),
        (
            "var n = 0; try { [, ,].reduce(function () { n++; }); } catch (e) { n + (e instanceof TypeError ? \"T\" : \"?\"); }",
            "normal \"0T\"",
        ),
        ("[].reduce(function () {}, undefined);", "normal undefined"),
        (
            "[].reduceRight(function () {}, undefined);",
            "normal undefined",
        ),
        ("[, 5, ,].reduce(function () { throw 1; });", "normal 5"),
        (
            "[, 5, ,].reduceRight(function () { throw 1; });",
            "normal 5",
        ),
    ]);
}

/// `#sec-findviapredicate`: every index below the length is visited, a hole as undefined (or
/// what the prototype chain holds there), ascending for `find` and `findIndex`, descending for
/// `findLast` and `findLastIndex`.
#[test]
fn find_visits_holes_as_undefined_and_find_last_walks_backward() {
    let visit = "function (v, i) { r.push(i + \":\" + v); }";
    let sources = [
        ("find", "0:undefined,1:b,2:undefined"),
        ("findIndex", "0:undefined,1:b,2:undefined"),
        ("findLast", "2:undefined,1:b,0:undefined"),
        ("findLastIndex", "2:undefined,1:b,0:undefined"),
    ]
    .map(|(method, seen)| {
        (
            format!("var r = []; [, \"b\", ,].{method}({visit}); r.join();"),
            format!("normal \"{seen}\""),
        )
    });
    let cases: Vec<(&str, &str)> = sources
        .iter()
        .map(|(source, expected)| (source.as_str(), expected.as_str()))
        .collect();
    check(&cases);
    check(&[
        (
            "Array.prototype[0] = \"p\"; [, 1].find(function () { return true; });",
            "normal \"p\"",
        ),
        (
            "[, ,].findIndex(function (v) { return v === undefined; });",
            "normal 0",
        ),
        (
            "[, ,].findLastIndex(function (v) { return v === undefined; });",
            "normal 1",
        ),
        (
            "var r = []; [1, 2, 3].findLast(function (v) { r.push(v); return v === 2; }); r.join();",
            "normal \"3,2\"",
        ),
    ]);
}

/// `#sec-array.prototype.filter` with `#sec-arrayspeciescreate`, the part a script reaches
/// (the part through `@@species` is the unit test of the same name in
/// `src/builtins/array/iteration.rs`): the selected values keep their order and are numbered
/// from 0; an array-like this, an Array whose `constructor` is undefined and one whose
/// `constructor` is an object without `@@species` make an Array; a `constructor` that is
/// neither an object nor undefined throws a `TypeError`.
#[test]
fn filter_uses_the_species_and_keeps_the_order() {
    check(&[
        (
            "[5, 1, 4, 2, 3].filter(function (v) { return v > 1; }).join();",
            "normal \"5,4,2,3\"",
        ),
        (
            "[, , 7].filter(function () { return true; })[0];",
            "normal 7",
        ),
        (
            "Array.isArray(Array.prototype.filter.call({ length: 1, 0: 1 }, function () { return true; }));",
            "normal true",
        ),
        (
            "var a = [1]; a.constructor = undefined; Array.isArray(a.filter(function () { return true; }));",
            "normal true",
        ),
        (
            "var a = [1]; a.constructor = {}; Array.isArray(a.filter(function () { return true; }));",
            "normal true",
        ),
        (
            "var a = [1]; a.constructor = 1; a.filter(function () { return true; });",
            "throw TypeError",
        ),
    ]);
}

/// `#sec-array.prototype.flat` step 4 and `#sec-flattenintoarray`: depth 0 copies the
/// elements, an undefined depth is 1, a depth is ToIntegerOrInfinity of the argument with a
/// negative one 0, `Infinity` flattens every level; an element that is an array-like but not an
/// Array is not flattened, an Array element of an array-like this is.
#[test]
fn flat_of_depth_zero_one_and_infinity() {
    check(&[
        (
            "var a = [1, [2, [3, [4, [5]]]]];\n\
             [a.flat(0).length, a.flat().length, a.flat(1).length, a.flat(2).length,\n\
              a.flat(Infinity).length, a.flat(Infinity).join(\"\")].join();",
            "normal \"2,3,3,4,5,12345\"",
        ),
        (
            "[[1]].flat(undefined).length + typeof [[1]].flat(undefined)[0];",
            "normal \"1number\"",
        ),
        ("typeof [[[1]]].flat(0.9)[0][0];", "normal \"object\""),
        ("typeof [[[1]]].flat(-Infinity)[0];", "normal \"object\""),
        ("[[[[1]]]].flat(1e9).join();", "normal \"1\""),
        (
            "Array.prototype.flat.call({ length: 2, 0: [1, 2], 1: 3 }).join();",
            "normal \"1,2,3\"",
        ),
        (
            "typeof [{ length: 1, 0: [1] }].flat(Infinity)[0];",
            "normal \"object\"",
        ),
    ]);
}

/// `#sec-array.prototype.flatmap`: FlattenIntoArray with depth 1, so an Array the mapper
/// answers is flattened one level and an Array inside it is kept; a value that is not an Array
/// is one element; holes of the source are not mapped and holes of an answer are skipped.
#[test]
fn flat_map_flattens_one_level_only() {
    check(&[
        (
            "var r = [1].flatMap(function () { return [[2, [3]]]; }); r.length + \":\" + r[0].length;",
            "normal \"1:2\"",
        ),
        (
            "[1, 2].flatMap(function (v) { return [v, v]; }).join();",
            "normal \"1,1,2,2\"",
        ),
        (
            "typeof [1].flatMap(function () { return { length: 1, 0: \"x\" }; })[0];",
            "normal \"object\"",
        ),
        (
            "[1].flatMap(function () { return []; }).length;",
            "normal 0",
        ),
        (
            "[1].flatMap(function () { return [, 2]; }).join();",
            "normal \"2\"",
        ),
        (
            "var n = 0; [1, , 2].flatMap(function (v) { n++; return v; }); n;",
            "normal 2",
        ),
    ]);
}

/// An agent with the test limits changed by `change`, and a realm.
fn agent_and_realm(change: impl FnOnce(&mut swell::Limits)) -> (Agent<TestHost>, Realm) {
    let mut limits = limits();
    change(&mut limits);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// `#sec-flattenintoarray` on an explicit stack: an Array that holds itself, flattened with
/// depth `Infinity` (and with a finite depth of 10^9), never ends by itself. Under a budget of
/// 10,000 steps it ends in `Terminated(StepBudget)` (a step per index of each level); under a
/// memory budget of 1 MiB beyond the realm (each open level is charged while it is open) and a
/// step budget it does not reach, in `Terminated(MemoryBudget)`; never in a Rust stack
/// overflow. The agent answers the next script after either.
#[test]
fn flat_of_an_array_that_holds_itself_ends_in_a_budget() {
    let base = {
        let (mut agent, _realm) = agent_and_realm(|_| {});
        agent.collect().unwrap();
        agent.heap_bytes().unwrap()
    };
    for depth in ["Infinity", "1e9"] {
        let source = format!("var a = [0]; a[0] = a; a.flat({depth});");
        let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 10_000);
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Terminated(Termination::StepBudget),
            "depth {depth}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "1;"),
            Outcome::Normal("1".to_string())
        );
        let (mut agent, realm) = agent_and_realm(|limits| {
            limits.memory_bytes = base + (1 << 20);
            limits.steps_per_entry = u64::MAX;
        });
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Terminated(Termination::MemoryBudget),
            "depth {depth}"
        );
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "1;"),
            Outcome::Normal("1".to_string())
        );
    }
}

/// The calls of the twelve methods in the tests of the step budget: the method, then what
/// follows the this value (the callback, a function that answers false, and an initial value).
const CALLS: [(&str, &str); 14] = [
    ("forEach", ", p"),
    ("every", ", p"),
    ("some", ", p"),
    ("filter", ", p"),
    ("reduce", ", p, 0"),
    ("reduce", ", p"),
    ("reduceRight", ", p, 0"),
    ("reduceRight", ", p"),
    ("find", ", p"),
    ("findIndex", ", p"),
    ("findLast", ", p"),
    ("findLastIndex", ", p"),
    ("flat", ""),
    ("flatMap", ", p"),
];

/// The step budget of [`a_walk_of_2_32_minus_1_indices_ends_in_the_step_budget`]: 10,000 in
/// the default mode; with the feature `collect-every-safe-point`, 1,000, which ends every walk
/// the same way. Each predicate call of FindViaPredicate passes safe points, and a collection at
/// each of them made the test take 40.9 s in that mode with 10,000 (1.3 s in the default one,
/// cargo-nextest, measured).
const WALK_STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    1_000
} else {
    10_000
};

/// Each walk loops over every index below the length, a step each (decision 2): over an
/// array-like whose "length" is 2^32 - 1 or 2^53 - 1 and over an Array of length 2^32 - 1, all
/// without elements, each of the twelve methods (`reduce` and `reduceRight` with and without an
/// initial value) ends in `Terminated(StepBudget)` under a budget of [`WALK_STEPS`] steps, not
/// in a hang, and the agent stays `Alive` and answers the next script.
#[test]
fn a_walk_of_2_32_minus_1_indices_ends_in_the_step_budget() {
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = WALK_STEPS);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var p = function () { return false; }, objects = [{ length: 4294967295 },\n\
             { length: 9007199254740991 }, new Array(4294967295)]; objects.length;"
        ),
        Outcome::Normal("3".to_string())
    );
    for (method, rest) in CALLS {
        for index in 0..3 {
            let source = format!("Array.prototype.{method}.call(objects[{index}]{rest});");
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                Outcome::Terminated(Termination::StepBudget),
                "{source}"
            );
            assert_eq!(agent.state(), AgentState::Alive);
        }
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "objects[2].length;"),
        Outcome::Normal("4294967295".to_string())
    );
}

/// The steps of the entry that runs `source` in `realm`, a normal completion or a throw, read
/// before anything else enters the agent (describing a Number outcome converts it in an entry
/// of its own, which starts its count at 0).
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> u64 {
    let outcome = eval(agent, realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert!(
        !matches!(outcome, HostOutcome::Terminated(_)),
        "{source}: {outcome:?}"
    );
    steps
}

/// A step per index (decision 2), measured with `last_entry_steps()` over N indices: 1,000 in
/// the default mode; with the feature `collect-every-safe-point`, 100, since the callbacks pass
/// safe points that each collect (11.6 s in that mode with 1,000, cargo-nextest, measured). A
/// lookup of a key an object does not have charges a step for the prototype it passes
/// (OrdinaryHasProperty and OrdinaryGet in `src/object/ordinary.rs`), one for an array-like
/// whose prototype is `%Object.prototype%`. So for each method that skips holes (and `reduce`
/// and `reduceRight` without an initial value, which search for the first element), a walk over
/// an array-like of N holes takes exactly 2 N steps more than the same call over one of length
/// 0: N of the walk and N of its HasProperty. The four of FindViaPredicate call the predicate
/// at every index: over N own elements each takes exactly as many more steps as `forEach` with
/// the same callback, which spends a step per index and the same calls, and over N holes N more
/// than that, the steps of its Get.
#[test]
fn each_walk_charges_a_step_per_index() {
    const N: u64 = if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        1_000
    };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var p = function () {{ return false; }}, holes = {{ length: {N} }}, none = {{ length: 0 }},\n\
                 full = {{ length: {N} }}; for (var i = 0; i < {N}; i++) full[i] = 0; i;"
            )
        ),
        Outcome::Normal(N.to_string())
    );
    let difference = |agent: &mut Agent<TestHost>, method: &str, rest: &str, over: &str| {
        let walked = steps_of(
            agent,
            &realm,
            &format!("Array.prototype.{method}.call({over}{rest});"),
        );
        let empty = steps_of(
            agent,
            &realm,
            &format!("Array.prototype.{method}.call(none{rest});"),
        );
        walked - empty
    };
    let for_each_full = difference(&mut agent, "forEach", ", p", "full");
    assert!(for_each_full > 2 * N, "{for_each_full}");
    for (method, rest) in CALLS {
        let walked = difference(&mut agent, method, rest, "holes");
        if method.starts_with("find") {
            let over_full = difference(&mut agent, method, rest, "full");
            assert_eq!(over_full, for_each_full, "{method}");
            assert_eq!(walked, for_each_full + N, "{method}");
        } else {
            assert_eq!(walked, 2 * N, "{method}{rest}");
        }
    }
}

/// What each walk holds across a call into code is kept (decision 4): O made by ToObject of a
/// String, an element the callback deleted from O (`filter`, `find`), the accumulator of
/// `reduce` and `reduceRight` across a getter of the next element, the Arrays `flatMap`'s
/// mapper answers and an Array `flat` is flattening that a getter deleted from its parent. With
/// `collect-every-safe-point` each callback and getter runs at safe points that collect, and
/// every result still reads what was held; in the default mode the same scripts answer the
/// same.
#[test]
fn what_a_walk_holds_survives_a_collection_in_its_callback() {
    let garbage = "var g = []; for (var j = 0; j < 4; j++) g.push({ j: j });";
    let sources = [
        (
            format!(
                "var r = []; Array.prototype.forEach.call(\"abc\", function (v, i, o) {{ {garbage} r.push(v + o.length); }});\n\
                 r.join();"
            ),
            "normal \"a3,b3,c3\"",
        ),
        (
            format!(
                "var r = []; Array.prototype.every.call(\"abc\", function (v, i, o) {{ {garbage} r.push(o[i]); return true; }});\n\
                 Array.prototype.some.call(\"de\", function (v, i, o) {{ {garbage} r.push(o[i]); return false; }}); r.join();"
            ),
            "normal \"a,b,c,d,e\"",
        ),
        (
            format!(
                "var a = [], i; for (i = 0; i < 8; i++) a.push({{ v: i }});\n\
                 var r = a.filter(function (x, k, o) {{ delete o[k]; {garbage} return true; }});\n\
                 var s = []; for (i = 0; i < r.length; i++) s.push(r[i].v); s.join();"
            ),
            "normal \"0,1,2,3,4,5,6,7\"",
        ),
        (
            format!(
                "var r = Array.prototype.filter.call(\"abc\", function (v) {{ {garbage} return v !== \"b\"; }}); r.join();"
            ),
            "normal \"a,c\"",
        ),
        (
            format!(
                "var a = [{{ v: 3 }}, {{ v: 4 }}];\n\
                 var x = a.find(function (v, k, o) {{ delete o[k]; {garbage} return true; }});\n\
                 var y = a.findLast(function (v, k, o) {{ delete o[k]; {garbage} return true; }}); x.v + \":\" + y.v;"
            ),
            "normal \"3:4\"",
        ),
        (
            format!(
                "var o = {{ length: 3 }};\n\
                 Object.defineProperty(o, \"1\", {{ get: function () {{ {garbage} return 1; }} }});\n\
                 Object.defineProperty(o, \"2\", {{ get: function () {{ {garbage} return 2; }} }});\n\
                 var r = Array.prototype.reduce.call(o, function (acc, v) {{ return {{ n: acc.n + v }}; }}, {{ n: 0 }});\n\
                 var q = {{ length: 3 }};\n\
                 Object.defineProperty(q, \"0\", {{ get: function () {{ {garbage} return 1; }} }});\n\
                 Object.defineProperty(q, \"1\", {{ get: function () {{ {garbage} return 2; }} }});\n\
                 var l = Array.prototype.reduceRight.call(q, function (acc, v) {{ return {{ n: acc.n + v }}; }}, {{ n: 0 }});\n\
                 r.n + \":\" + l.n;"
            ),
            "normal \"3:3\"",
        ),
        (
            format!(
                "var r = [0, 1].flatMap(function (v) {{\n\
                   {garbage} var x = [{{ v: v }}, 0, {{ v: v + 10 }}];\n\
                   Object.defineProperty(x, \"1\", {{ get: function () {{ {garbage} return {{ v: v + 5 }}; }} }});\n\
                   return x;\n\
                 }});\n\
                 var s = []; for (var i = 0; i < r.length; i++) s.push(r[i].v); s.join();"
            ),
            "normal \"0,5,10,1,6,11\"",
        ),
        (
            format!(
                "var inner = [{{ v: 1 }}, 0, {{ v: 3 }}], outer = [inner];\n\
                 Object.defineProperty(inner, \"1\", {{ get: function () {{ delete outer[0]; {garbage} return {{ v: 2 }}; }} }});\n\
                 inner = null;\n\
                 var r = outer.flat(), s = []; for (var i = 0; i < r.length; i++) s.push(r[i].v); s.join();"
            ),
            "normal \"1,2,3\"",
        ),
    ];
    let cases: Vec<(&str, &str)> = sources
        .iter()
        .map(|(source, expected)| (source.as_str(), *expected))
        .collect();
    check(&cases);
}

/// A test that `f` re-enters itself through `$through` until the re-entry limit, on seven
/// eighths of `min_stack_bytes()` (`AGENTS.md`, "Commands and the gate"), in a child process
/// (`support::stack::on_bounded_stack`): one test per callback method, so each is timed alone.
macro_rules! reaches_the_reentry_limit {
    ($name:ident, $through:literal) => {
        #[test]
        fn $name() {
            fn body() {
                reach_through($through, BUILTINS_REENTRY);
            }
            let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
            on_bounded_stack(stringify!($name), stack, body).unwrap();
        }
    };
}

reaches_the_reentry_limit!(
    for_each_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].forEach(f)"
);
reaches_the_reentry_limit!(
    every_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].every(f)"
);
reaches_the_reentry_limit!(
    some_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].some(f)"
);
reaches_the_reentry_limit!(
    filter_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].filter(f)"
);
reaches_the_reentry_limit!(
    reduce_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].reduce(f, 0)"
);
reaches_the_reentry_limit!(
    reduce_right_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].reduceRight(f, 0)"
);
reaches_the_reentry_limit!(
    find_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].find(f)"
);
reaches_the_reentry_limit!(
    find_index_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].findIndex(f)"
);
reaches_the_reentry_limit!(
    find_last_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].findLast(f)"
);
reaches_the_reentry_limit!(
    find_last_index_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].findLastIndex(f)"
);
reaches_the_reentry_limit!(
    flat_map_callback_reaches_the_reentry_limit_on_the_promised_stack,
    "[0].flatMap(f)"
);
