//! RegExp objects (ECMA-262 22.2, `#sec-regexp-regular-expression-objects`): the constructor
//! (`#sec-regexp-pattern-flags`) with RegExpAlloc and RegExpInitialize, `%RegExp.prototype%`
//! with `exec`, `test`, `toString` and the accessors of the flags and the source, RegExpExec and
//! RegExpBuiltinExec with the match array, the literal evaluated to a new object at each
//! evaluation (`#sec-regular-expression-literals-runtime-semantics-evaluation`), and the bounds
//! of `docs/design/engine.md`, decision 7: the steps of the matcher and of the pattern, the
//! memory of the backtrack stack and of the program, the interrupt and the re-entries. Run
//! also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, parse, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    Agent, Completion, Cx, HostCall, Limits, Outcome as HostOutcome, Realm, Termination, Value,
};

/// `true`, as an outcome.
fn yes() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// Each script answers `true`, as written and with a `"use strict";` prologue, each run in an
/// agent of its own.
fn all_true(scripts: &[&str]) {
    for script in scripts {
        let (sloppy, strict) = run_both(script);
        assert_eq!(sloppy, yes(), "sloppy: {script}");
        assert_eq!(strict, yes(), "strict: {script}");
    }
}

/// An agent with `limits` and a realm.
fn agent_and_realm(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// A count of a test whose claim shows at any count: `default` in the default mode and
/// `collect` with the feature `collect-every-safe-point`, where every instruction collects.
fn mode_count(default: u32, collect: u32) -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        collect
    } else {
        default
    }
}

/// `#sec-regexp-pattern-flags`: `RegExp` called and constructed with a String, with a RegExp
/// (read by its slots, the flags replaced when given) and with a RegExp-like object (an object
/// whose `@@match` is true, read by Get of `source` and `flags`); called with a RegExp or a
/// RegExp-like object whose `constructor` is `RegExp` and no flags it answers its argument.
/// RegExpAlloc gives the object `%RegExp.prototype%` and an own `lastIndex` { [[Writable]]:
/// true, [[Enumerable]]: false, [[Configurable]]: false } that RegExpInitialize sets to 0; the
/// constructor's `length` is 2 and `RegExp[@@species]` answers its this value.
#[test]
fn the_constructor_called_and_constructed_with_a_string_a_regexp_and_a_regexp_like_object() {
    all_true(&[
        r#"var r = new RegExp("a+", "g");
           var d = Object.getOwnPropertyDescriptor(r, "lastIndex");
           r instanceof RegExp && Object.getPrototypeOf(r) === RegExp.prototype &&
             r.source === "a+" && r.flags === "g" && r.lastIndex === 0 &&
             d.writable === true && d.enumerable === false && d.configurable === false &&
             Object.getOwnPropertyNames(r).join() === "lastIndex" &&
             RegExp("b") instanceof RegExp && RegExp("b").source === "b" &&
             new RegExp().source === "(?:)" && new RegExp(undefined, undefined).flags === "" &&
             new RegExp(1).source === "1" && RegExp.length === 2 && RegExp.name === "RegExp""#,
        r#"var s = /x/g;
           var t = new RegExp(s);
           var u = new RegExp(s, "y");
           RegExp(s) === s && t !== s && t.source === "x" && t.flags === "g" &&
             u.source === "x" && u.flags === "y" && RegExp(s, "m") !== s &&
             RegExp(s, "m").flags === "m""#,
        r#"var like = { source: "q", flags: "m", constructor: RegExp };
           like[Symbol.match] = true;
           var c = new RegExp(like);
           var d = new RegExp(like, "g");
           RegExp(like) === like && c !== like && c.source === "q" && c.flags === "m" &&
             d.source === "q" && d.flags === "g" &&
             new RegExp({ source: "q", toString: function () { return "z"; } }).source === "z""#,
        r#"var d = Object.getOwnPropertyDescriptor(RegExp, Symbol.species);
           var p = Object.getOwnPropertyDescriptor(RegExp, "prototype");
           RegExp[Symbol.species] === RegExp && d.set === undefined &&
             d.get.name === "get [Symbol.species]" && d.get.call(1) === 1 &&
             d.enumerable === false && d.configurable === true &&
             p.writable === false && p.enumerable === false && p.configurable === false &&
             RegExp.prototype.constructor === RegExp &&
             Object.getPrototypeOf(RegExp.prototype) === Object.prototype"#,
    ]);
}

/// RegExpInitialize (`#sec-regexpinitialize`): a flag that is not one of `d g i m s u v y`, a
/// flag written twice and a pattern that is not valid each throw a `SyntaxError` of the
/// current realm, the `RegExp` of another realm one of that realm, with the message of
/// `swell-regexp` and no position after it; none is a refusal, so `refusals()` stays 0.
#[test]
fn an_invalid_pattern_or_flags_throw_a_syntax_error_of_the_realm() {
    let (mut agent, realm) = agent_and_realm(limits());
    for (source, message) in [
        (r#"new RegExp("(")"#, "unterminated group"),
        (
            r#"new RegExp("a", "gg")"#,
            "repeated regular expression flag",
        ),
        (r#"new RegExp("a", "x")"#, "invalid regular expression flag"),
        (
            r#"RegExp("a{2,1}")"#,
            "numbers out of order in {} quantifier",
        ),
        (r#"new RegExp("a", "uv")"#, "the flags u and v together"),
        (
            r#"new RegExp("a", "gimsuydx")"#,
            "invalid regular expression flag",
        ),
        (
            r#"new RegExp("\\k<a>(?<b>c)")"#,
            "a reference to a group name no group has",
        ),
    ] {
        let script = format!(
            "try {{ {source}; \"no error\"; }} catch (e) {{ (e instanceof SyntaxError) + \": \" + e.message; }}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &script),
            Outcome::Normal(format!("\"true: {message}\"")),
            "{source}"
        );
    }
    assert_eq!(agent.refusals(), 0);

    let other = support::host::realm(&mut agent);
    let other_global = global(&mut agent, &other);
    let this_global = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let this = object(cx, &this_global);
            let that = object(cx, &other_global);
            let name = key(cx, "other");
            cx.set(this, name, Value::Object(that), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "try { new other.RegExp(\"(\"); false; } catch (e) { e instanceof other.SyntaxError && !(e instanceof SyntaxError); }"
        ),
        yes()
    );
    assert_eq!(agent.refusals(), 0);
}

/// The accessors of `%RegExp.prototype%`: `flags` (`#sec-get-regexp.prototype.flags`) reads
/// the eight flags by Get in the order `d g i m s u v y`, of any object; each flag's getter
/// reads [[OriginalFlags]] (RegExpHasFlag, `#sec-regexphasflag`); `source` answers
/// EscapeRegExpPattern (`#sec-escaperegexppattern`) of [[OriginalSource]]: `/` and the line
/// terminators escaped, `(?:)` for the empty pattern, and a pattern that evaluates to the same
/// matcher.
#[test]
fn flags_and_source_read_the_original_flags_and_pattern() {
    all_true(&[
        r#"var r = new RegExp("a", "yusmgd");
           r.flags === "dgmsuy" && r.hasIndices && r.global && r.multiline && r.dotAll &&
             r.unicode && r.sticky && r.ignoreCase === false && r.unicodeSets === false &&
             /a/.flags === "" && /a/.global === false && /a/g.global === true &&
             /a/m.multiline && /a/s.dotAll && /a/u.unicode && /a/y.sticky && /a/d.hasIndices"#,
        r#"var get = Object.getOwnPropertyDescriptor(RegExp.prototype, "flags").get;
           var read = [];
           var o = {};
           ["hasIndices", "global", "ignoreCase", "multiline", "dotAll", "unicode",
            "unicodeSets", "sticky"].forEach(function (name) {
             Object.defineProperty(o, name, { get: function () { read.push(name); return name !== "global"; } });
           });
           get.call(o) === "dimsuvy" && read.join() ===
             "hasIndices,global,ignoreCase,multiline,dotAll,unicode,unicodeSets,sticky" &&
             get.call({ global: 1, sticky: "x", unicode: 0 }) === "gy""#,
        r#"new RegExp("/").source === "\\/" && new RegExp("\n").source === "\\n" &&
             new RegExp("\r  ").source === "\\r\\u2028\\u2029" &&
             new RegExp("").source === "(?:)" && /[/]/.source === "[/]" &&
             new RegExp("\\/").source === "\\/" && /a\/b/.source === "a\\/b" &&
             eval("/" + new RegExp("a/b\n").source + "/").test("a/b\n") &&
             eval("/" + new RegExp("[/]").source + "/").test("/")"#,
    ]);
}

/// RegExpBuiltinExec (`#sec-regexpbuiltinexec`): the match array has the captures, `index`,
/// `input` and `groups` (an object with a null prototype holding the named captures, or
/// undefined for a pattern with no group name), and under `d` `indices`, an Array of the pairs
/// of each capture with its own `groups` (MakeMatchIndicesIndexPairArray,
/// `#sec-makematchindicesindexpairarray`); a capture that did not participate is undefined,
/// and no match is `null`. Of two groups of one name the one that participated answers.
#[test]
fn exec_answers_the_match_array_with_index_input_groups_and_indices() {
    all_true(&[
        r#"var m = /(?<y>\d{4})-(\d\d)(x)?/d.exec("on 2024-11 now");
           Array.isArray(m) && m.length === 4 && m.index === 3 && m.input === "on 2024-11 now" &&
             m[0] === "2024-11" && m[1] === "2024" && m[2] === "11" && m[3] === undefined &&
             Object.keys(m).join() === "0,1,2,3,index,input,groups,indices" &&
             Object.getPrototypeOf(m.groups) === null && m.groups.y === "2024" &&
             Object.keys(m.groups).join() === "y" &&
             m.indices.length === 4 && m.indices[0].join() === "3,10" &&
             m.indices[1].join() === "3,7" && m.indices[2].join() === "8,10" &&
             m.indices[3] === undefined && Object.getPrototypeOf(m.indices.groups) === null &&
             m.indices.groups.y.join() === "3,7""#,
        r#"var m = /a(b)?/.exec("xa");
           var n = /(?<k>a)|(?<k>b)/.exec("b");
           var e = /(?:)/.exec("");
           /a/.exec("b") === null && m.groups === undefined && !("indices" in m) &&
             m.length === 2 && m[1] === undefined && m.index === 1 &&
             n.groups.k === "b" && n[1] === undefined && n[2] === "b" &&
             e.length === 1 && e[0] === "" && e.index === 0 &&
             /(?<k>a)|(?<k>b)/d.exec("b").indices.groups.k.join() === "0,1" &&
             RegExp.prototype.exec.call(/b/, { toString: function () { return "ab"; } }).index === 1"#,
        r#"var thrown = 0;
           [1, {}, RegExp.prototype].forEach(function (v) {
             try { RegExp.prototype.exec.call(v, "a"); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           thrown === 3"#,
    ]);
}

/// Positions are code unit indices in every mode (`docs/design/engine.md`, decision 7, "How
/// `swell-regexp` charges its work", item 3): under `u` `index`, `lastIndex` and `indices`
/// count the code units of a surrogate pair as two; a match does not start on the second unit
/// of a pair, and a `lastIndex` there starts the try at the pair's first unit while `index` is
/// the `lastIndex` the search was given (RegExpBuiltinExec, step 13.b).
#[test]
fn positions_are_code_unit_indices_under_u() {
    all_true(&[r#"var s = "😀ab";
           var g = /b/gu;
           var first = g.exec(s);
           var d = /a/du.exec(s);
           var one = /./gu;
           one.lastIndex = 1;
           var m = one.exec(s);
           var plain = /./g;
           plain.lastIndex = 1;
           var p = plain.exec(s);
           var dot = /./ug;
           dot.exec("𝌆");
           first.index === 3 && g.lastIndex === 4 && d.index === 2 &&
             d.indices[0].join() === "2,3" && m.index === 1 && m[0] === "\ude00" &&
             one.lastIndex === 2 && p.index === 1 && p[0] === "\ude00" && plain.lastIndex === 2 &&
             /\udf06/u.exec("𝌆") === null && /\udf06/.exec("𝌆").index === 1 &&
             dot.lastIndex === 2"#]);
}

/// RegExpBuiltinExec, steps 2 to 13: with `g` or `y` a match sets `lastIndex` to its end and a
/// failure to 0, also past the end of the String; `y` tries only at `lastIndex`; without
/// either `lastIndex` is read (ToLength) and left as it is. A `lastIndex` that cannot be set
/// throws a `TypeError`.
#[test]
fn exec_moves_last_index_with_g_and_y_and_resets_it_on_a_failure() {
    all_true(&[
        r#"var g = /a/g;
           var s = "aba";
           g.exec(s).index === 0 && g.lastIndex === 1 && g.exec(s).index === 2 &&
             g.lastIndex === 3 && g.exec(s) === null && g.lastIndex === 0 &&
             g.exec(s).index === 0"#,
        r#"var y = /a/y;
           var first = y.exec("ba");
           var after = y.lastIndex;
           y.lastIndex = 1;
           var second = y.exec("ba");
           first === null && after === 0 && second.index === 1 && y.lastIndex === 2"#,
        r#"var n = /a/;
           n.lastIndex = 5;
           var big = /a/g;
           big.lastIndex = 10;
           var read = 0;
           var counted = /a/;
           counted.lastIndex = { valueOf: function () { read++; return 7; } };
           counted.exec("a");
           n.exec("a").index === 0 && n.lastIndex === 5 && big.exec("a") === null &&
             big.lastIndex === 0 && read === 1 && typeof counted.lastIndex === "object""#,
        r#"var f = /a/g;
           Object.defineProperty(f, "lastIndex", { writable: false, value: 0 });
           var thrown = false;
           try { f.exec("b"); } catch (e) { thrown = e instanceof TypeError; }
           thrown"#,
    ]);
}

/// `RegExp.prototype.test` (`#sec-regexp.prototype.test`) through RegExpExec
/// (`#sec-regexpexec`): a callable `exec` is called with the String and its answer, an object
/// or null, decides; another answer is a `TypeError`; without a callable `exec` an object with
/// [[RegExpMatcher]] is matched by RegExpBuiltinExec and any other object is a `TypeError`.
#[test]
fn test_answers_through_a_user_exec() {
    all_true(&[
        r#"var calls = [];
           var o = { exec: function (s) { calls.push(this === o && s); return s === "x" ? {} : null; } };
           var r = /a/;
           r.exec = 1;
           var q = /a/;
           q.exec = function () { return null; };
           RegExp.prototype.test.call(o, "x") === true && RegExp.prototype.test.call(o, "y") === false &&
             calls.join() === "x,y" && r.test("a") === true && q.test("a") === false &&
             /a/.test("ba") === true && /a/.test("b") === false"#,
        r#"var thrown = 0;
           [{ exec: function () { return 1; } }, { exec: 1 }, { exec: function () { return "x"; } }]
             .forEach(function (o) {
               try { RegExp.prototype.test.call(o, "a"); } catch (e) { if (e instanceof TypeError) thrown++; }
             });
           try { RegExp.prototype.test.call(1, "a"); } catch (e) { if (e instanceof TypeError) thrown++; }
           thrown === 4"#,
    ]);
}

/// `RegExp.prototype.toString` (`#sec-regexp.prototype.tostring`): `/`, ToString of Get of
/// `source`, `/` and ToString of Get of `flags`, of any object; a primitive this is a
/// `TypeError`.
#[test]
fn to_string_reads_source_and_flags_by_get() {
    all_true(&[r#"var thrown = false;
           try { RegExp.prototype.toString.call(1); } catch (e) { thrown = e instanceof TypeError; }
           RegExp.prototype.toString.call({ source: "x", flags: "yz" }) === "/x/yz" &&
             /a\/b/g.toString() === "/a\\/b/g" && String(new RegExp("/")) === "/\\//" &&
             RegExp.prototype.toString() === "/(?:)/" && String(new RegExp("", "dg")) === "/(?:)/dg" &&
             thrown"#]);
}

/// RegExpHasFlag (`#sec-regexphasflag`) and `get RegExp.prototype.source`
/// (`#sec-get-regexp.prototype.source`): for `%RegExp.prototype%` itself the getters of the
/// flags answer undefined and `source` answers `(?:)`; any other object without the slots and
/// any primitive is a `TypeError`. Each is an accessor with no setter, named `get <name>`, not
/// enumerable and configurable.
#[test]
fn the_flag_getters_answer_undefined_for_the_prototype_and_throw_for_other_objects() {
    all_true(&[
        r#"var names = ["dotAll", "global", "hasIndices", "ignoreCase", "multiline",
                        "sticky", "unicode", "unicodeSets"];
           var ok = true;
           names.forEach(function (name) {
             var d = Object.getOwnPropertyDescriptor(RegExp.prototype, name);
             ok = ok && RegExp.prototype[name] === undefined && d.set === undefined &&
               d.get.name === "get " + name && d.get.length === 0 && d.enumerable === false &&
               d.configurable === true;
             [{}, 1, "g"].forEach(function (v) {
               try { d.get.call(v); ok = false; } catch (e) { ok = ok && e instanceof TypeError; }
             });
           });
           var source = Object.getOwnPropertyDescriptor(RegExp.prototype, "source").get;
           try { source.call({}); ok = false; } catch (e) { ok = ok && e instanceof TypeError; }
           ok && RegExp.prototype.source === "(?:)" && RegExp.prototype.flags === """#,
    ]);
}

/// `#sec-regular-expression-literals-runtime-semantics-evaluation`: each evaluation of a
/// literal makes a new RegExp object by RegExpCreate, with its own `lastIndex` at 0, in a
/// loop and from a function called twice alike.
#[test]
fn each_evaluation_of_a_literal_makes_a_new_object() {
    all_true(&[r#"var a = [];
           for (var i = 0; i < 3; i++) {
             var r = /a/g;
             a.push(r.lastIndex);
             r.lastIndex = i + 1;
             a.push(r);
           }
           function f() { return /b/y; }
           var x = f();
           x.exec("b");
           a[1] !== a[3] && a[3] !== a[5] && a[0] === 0 && a[2] === 0 && a[4] === 0 &&
             a[1].lastIndex === 1 && a[5].lastIndex === 3 && f() !== f() &&
             x.lastIndex === 1 && f().lastIndex === 0"#]);
}

/// The steps of `source` in a fresh agent: the reading of `last_entry_steps()` after it, which
/// must complete normally.
fn entry_steps(source: &str) -> u64 {
    let (mut agent, realm) = agent_and_realm(limits());
    let outcome = eval(&mut agent, &realm, source);
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{source}");
    agent.last_entry_steps().unwrap()
}

/// The steps one evaluation of a literal of `length` code units costs in a loop of `count`,
/// beside the loop without it: the difference of the two entries, divided by `count`, checked
/// to divide evenly.
fn literal_steps(length: usize, count: u32) -> u64 {
    let body = "a".repeat(length);
    let with = entry_steps(&format!(
        "for (var i = 0; i < {count}; i++) {{ /{body}/; }}"
    ));
    let without = entry_steps(&format!("for (var i = 0; i < {count}; i++) {{ ; }}"));
    let difference = with - without;
    assert_eq!(
        difference % u64::from(count),
        0,
        "{difference} steps over {count}"
    );
    difference / u64::from(count)
}

/// `docs/design/engine.md`, decision 7, "A regular expression literal: checked at parse time,
/// compiled at each evaluation", rule 2: a loop of n evaluations of a literal of L code units
/// spends at least n × (L + 1) steps more than the loop without it, the same at every n, and a
/// literal 10 code units longer exactly 10 steps more at each evaluation. The counts: 100
/// evaluations in the default mode, 10 with a collection at every safe point; a difference of
/// steps divided by n is exact at any n, and the assertions fail at either count when the
/// pattern is not charged per evaluation.
#[test]
fn a_literal_pays_a_step_per_code_unit_at_each_evaluation() {
    let count = mode_count(100, 10);
    let short = literal_steps(10, count);
    let long = literal_steps(20, count);
    assert!(short >= 11, "{short} steps per evaluation of 10 code units");
    assert_eq!(long - short, 10, "{short} and {long} steps per evaluation");
    assert_eq!(
        literal_steps(10, count * 2),
        short,
        "the same at twice the count"
    );
}

/// The consumer's table (`docs/plan/slice-05/consumer-regexps.tsv`): one literal per row,
/// written for this test with the constructs and the flags of that row, each with the body as
/// `String.raw` reads it: the object is a RegExp for `Object.prototype.toString`, and `source`
/// and `flags` answer the body and the flags written. The call of row 2 (`test` of a class of
/// `\u` escapes) answers, and so do a construction from a String and its `test`, as
/// `testharness.js` makes them.
#[test]
fn the_constructs_of_the_consumer_table_evaluate() {
    let rows: [(&str, &str); 12] = [
        (r"^\s*{(a|b)}\.(?:c)$", ""),
        (r"[\u0000-\u001f\u007f-\u009f]", ""),
        (r"^([^:]+)(:\d+)*$", ""),
        (r"\n", "g"),
        (
            r"([\ud800-\udbff](?![\udc00-\udfff]))|(^|[^\ud800-\udbff])([\udc00-\udfff])",
            "g",
        ),
        (r"\s", "g"),
        (r"\w\d\.[^\-](?:x)?", "g"),
        (r"\$\{([^}]+)\}", "g"),
        (r"[\\^$.*+?()[\]{}|]", "g"),
        (r"^(\-|)x\.$", ""),
        (r"{([^}]*)}", "g"),
        (r"[\x00-\x1f]", "g"),
    ];
    let mut script = String::from("var ok = true;\n");
    for (body, flags) in rows {
        script.push_str(&format!(
            "var r = /{body}/{flags};\n\
             ok = ok && Object.prototype.toString.call(r) === \"[object RegExp]\" &&\n\
               r.source === String.raw`{body}` && r.flags === \"{flags}\";\n"
        ));
    }
    script.push_str(
        "ok && /[\\u0000-\\u001f\\u007f-\\u009f]/.test(\"a\\u0001\") &&\n\
           !/[\\u0000-\\u001f\\u007f-\\u009f]/.test(\"abc\") &&\n\
           new RegExp(\"^\\\\s*\" + \"x\").test(\"  x\") && new RegExp(\"[.]\", \"g\").test(\"a.b\")",
    );
    all_true(&[&script]);
}

/// IsRegExp (`#sec-isregexp`), step 4, and `Object.prototype.toString`
/// (`#sec-object.prototype.tostring`), step 13, read [[RegExpMatcher]]: a RegExp is a regular
/// expression to `startsWith`, `endsWith`, `includes` and `replaceAll`, unless its own
/// `@@match` says otherwise, and its builtinTag is `RegExp`; `%RegExp.prototype%` has no
/// [[RegExpMatcher]] and its builtinTag is `Object`, but it is a regular expression to IsRegExp,
/// whose step 3 finds its `@@match` callable, so `startsWith` given it throws a `TypeError`.
#[test]
fn is_reg_exp_and_object_to_string_see_a_regexp() {
    all_true(&[r#"var thrown = 0;
           ["startsWith", "endsWith", "includes"].forEach(function (name) {
             try { "/a/"[name](/a/); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           try { "a".replaceAll(/a/, "b"); } catch (e) { if (e instanceof TypeError) thrown++; }
           var r = /a/;
           r[Symbol.match] = false;
           try { "/a/".startsWith(RegExp.prototype); } catch (e) { if (e instanceof TypeError) thrown++; }
           thrown === 5 && "/a/".startsWith(r) && "x/a/".endsWith(r) && "x/a/x".includes(r) &&
             Object.prototype.toString.call(/a/) === "[object RegExp]" &&
             Object.prototype.toString.call(RegExp.prototype) === "[object Object]" &&
             String(Object.prototype.toString.call(new RegExp("b", "g"))) === "[object RegExp]""#]);
}

/// A pattern of `n` nested capturing groups around `a`.
fn groups(n: usize) -> String {
    format!("{}a{}", "(".repeat(n), ")".repeat(n))
}

/// HOST-143 (`docs/design/host-interface.md`; decision 3, "Depth bounds"): with
/// `parser_nesting` 1,000, `new RegExp(p)` of a pattern of 1,001 and of 2,000 nested groups
/// throws a `SyntaxError` of the realm whose message is `nesting deeper than parser_nesting
/// (1000)`, which the script catches, and `refusals()` does not move; a pattern of 1,000 nested
/// groups constructs and matches.
#[test]
fn host_143_a_pattern_past_parser_nesting_throws_a_catchable_syntax_error() {
    let (mut agent, realm) = agent_and_realm(limits());
    for depth in [1_001, 2_000] {
        let script = format!(
            "try {{ new RegExp(\"{}\"); \"no error\"; }} catch (e) {{ (e instanceof SyntaxError) + \": \" + e.message; }}",
            groups(depth)
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &script),
            Outcome::Normal("\"true: nesting deeper than parser_nesting (1000)\"".to_string()),
            "{depth}"
        );
    }
    let script = format!(
        "var m = new RegExp(\"{}\").exec(\"ba\"); m.index === 1 && m.length === 1001 && m[1000] === \"a\"",
        groups(1_000)
    );
    assert_eq!(eval_text(&mut agent, &realm, &script), yes());
    assert_eq!(agent.refusals(), 0);
}

/// The acceptance 5 of `epic:slice-05-regexp` (`docs/design/engine.md`, decision 7, "What one
/// step of the matcher is"): `/(a+)+$/` against 10,000 `a` and `!`, with `steps_per_entry`
/// 2,000,000, ends in `Terminated(StepBudget)`, with `last_entry_steps()` equal to the budget.
#[test]
fn a_catastrophic_pattern_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 2_000_000;
    let (mut agent, realm) = agent_and_realm(limits);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var s = \"a\".repeat(10000) + \"!\"; /(a+)+$/.test(s);",
    );
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert_eq!(agent.last_entry_steps().unwrap(), 2_000_000);
}

/// `docs/design/engine.md`, decision 7, "How `swell-regexp` charges its work": a match whose
/// backtrack stack passes the memory budget ends the entry in `Terminated(MemoryBudget)`, where
/// no `catch` runs; after a collection `heap_bytes()` reads what it read before the match, so
/// nothing the match charged stays charged. `/a*b/` against 2^20 `a` pushes an entry for each
/// `a` before it fails, with a memory budget of 8 MiB.
#[test]
fn a_backtrack_stack_past_the_memory_budget_terminates_the_entry() {
    let mut limits = limits();
    limits.memory_bytes = 8 << 20;
    limits.steps_per_entry = 1 << 30;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var s = \"a\".repeat(1 << 20); var caught = false; s.length"
        ),
        Outcome::Normal("1048576".to_string())
    );
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "try { /a*b/.test(s); } catch (e) { caught = true; }"
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    agent.collect().unwrap();
    assert_eq!(agent.heap_bytes().unwrap(), before);
    assert_eq!(
        eval_text(&mut agent, &realm, "caught"),
        Outcome::Normal("false".to_string())
    );
}

/// Set by `started` when the script reaches its match.
static STARTED: AtomicBool = AtomicBool::new(false);

/// A host function that records that the script has started.
fn started(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    STARTED.store(true, Ordering::SeqCst);
    Ok(Value::Undefined)
}

/// `docs/design/engine.md`, decision 7, "What one step of the matcher is": the matcher reads
/// the interrupt at each grant of at most 4,096 steps, so a match that would run for hours
/// ends in `Terminated(Interrupted)` when another thread sets the flag, more than 10,000 steps
/// into the entry, with the flag cleared. The script calls `started` from the `valueOf` of
/// `lastIndex`, which ToLength reads just before the search, and the other thread sets the flag
/// 100 ms after that call.
#[test]
fn an_interrupt_ends_a_long_match() {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 40;
    let (mut agent, realm) = agent_and_realm(limits);
    define_function(&mut agent, &realm, "started", 0, started, false);
    let script = parse(
        &mut agent,
        &realm,
        "var s = \"a\".repeat(40) + \"!\"; var r = /(a+)+$/;\n\
         r.lastIndex = { valueOf: function () { started(); return 0; } }; r.test(s);",
    );
    let handle = agent.interrupt_handle();
    let interrupter = thread::spawn(move || {
        while !STARTED.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
        }
        thread::sleep(Duration::from_millis(100));
        handle.interrupt();
    });
    let outcome = agent.evaluate_script(&script).unwrap();
    interrupter.join().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::Interrupted)),
        "{outcome:?}"
    );
    assert!(agent.last_entry_steps().unwrap() > 10_000);
    assert!(!agent.interrupt_handle().is_set());
}

/// `docs/design/engine.md`, decision 7, "Where a RegExp object keeps its program": the program
/// of `new RegExp(p)` is charged while the object lives, and the `collect()` that frees the
/// object lowers `heap_bytes()` by at least the program's bytes, as `swell-regexp` reports them
/// for the same pattern compiled on its own. `p` is 2,000 code units of classes and groups.
#[test]
fn the_program_is_charged_while_the_object_lives() {
    let pattern = "([a-z]|\\d)".repeat(200);
    let units: Vec<u16> = pattern.encode_utf16().collect();
    let mut unlimited = swell_regexp::Unlimited;
    let tree = swell_regexp::parse_pattern(
        &units,
        swell_regexp::Flags::default(),
        1_000,
        &mut unlimited,
    )
    .unwrap();
    let program = swell_regexp::compile_pattern(&tree, &mut unlimited).unwrap();
    let program_bytes = u64::try_from(program.heap_bytes()).unwrap();
    assert!(program_bytes > 0);

    let (mut agent, realm) = agent_and_realm(limits());
    let source = format!(
        "var r = new RegExp(\"{}\"); r.test(\"a\".repeat(200))",
        pattern.replace('\\', "\\\\")
    );
    assert_eq!(eval_text(&mut agent, &realm, &source), yes());
    agent.collect().unwrap();
    let alive = agent.heap_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "r = null; 0"),
        Outcome::Normal("0".to_string())
    );
    agent.collect().unwrap();
    let freed = agent.heap_bytes().unwrap();
    assert!(
        alive - freed >= program_bytes,
        "heap_bytes() fell by {} when the RegExp was freed, its program holds {program_bytes}",
        alive - freed
    );
}

/// The re-entries of this story: ToLength of `lastIndex` (`#sec-regexpbuiltinexec`, step 2),
/// ToString of the String of `exec`, and the call of a user `exec` by RegExpExec
/// (`#sec-regexpexec`) through `test`.
const THROUGH_REGEXP: [&str; 3] = [
    "Object.assign(/a/, { lastIndex: { valueOf: f } }).exec(\"a\")",
    "/a/.exec({ toString: f })",
    "Object.assign(/a/, { exec: f }).test(\"a\")",
];

/// The body of the stack test: one agent per expression.
fn reach_through_regexp() {
    for through in THROUGH_REGEXP {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// On a thread with seven eighths of `min_stack_bytes()` for the stack limits, a `valueOf` of
/// `lastIndex` that calls `exec` again reaches the re-entry limit (a `RangeError`), as do the
/// other expressions of [`THROUGH_REGEXP`], and parses 1,000 nested brackets at the deepest,
/// without a stack overflow (`tests/support/stack.rs`, [`reach_through`]); each also reaches a
/// small `reentry_depth` exactly.
#[test]
fn last_index_conversion_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("last_index_conversion_re_enters_on_the_promised_stack"),
        stack,
        reach_through_regexp,
    )
    .unwrap();
    for depth in [0_u32, 1, 7] {
        for through in THROUGH_REGEXP {
            reach_through(through, depth);
        }
    }
}
