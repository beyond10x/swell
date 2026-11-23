//! The methods of `%RegExp.prototype%` keyed by `%Symbol.match%`
//! (`#sec-regexp.prototype-%symbol.match%`), `%Symbol.replace%`
//! (`#sec-regexp.prototype-%symbol.replace%`), `%Symbol.search%`
//! (`#sec-regexp.prototype-%symbol.search%`) and `%Symbol.split%`
//! (`#sec-regexp.prototype-%symbol.split%`), SpeciesConstructor (`#sec-speciesconstructor`),
//! and `String.prototype.match` (`#sec-string.prototype.match`) and `search`
//! (`#sec-string.prototype.search`), written by
//! `story:regexp-symbol-methods-and-string-match-search`: what each answers, the order of its
//! reads, the steps of its loops and the step budget, the results `@@replace` holds across the
//! calls of a replacer, the re-entries of a replacer and of a species constructor on the
//! promised stack, and the calls the first consumer makes (`docs/plan/slice-05/consumer-regexps.tsv`).
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, describe, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, Limits, Realm, Termination, Value};

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
fn mode_count(default: usize, collect: usize) -> usize {
    if cfg!(feature = "collect-every-safe-point") {
        collect
    } else {
        default
    }
}

/// Set the global `name` of `realm` to a String of `units` the host makes, outside any entry
/// of a script.
fn host_string(agent: &mut Agent<TestHost>, realm: &Realm, name: &str, units: &[u16]) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let string = cx.new_string_utf16(units).unwrap();
            let name = key(cx, name);
            cx.set(global, name, Value::String(string), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// The script that answers whether `RegExp.prototype[Symbol.<symbol>]` is a method as clause
/// 18 makes one: a function in a data property { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true }, whose "length" is `length` and "name" is `[Symbol.<symbol>]`,
/// with no "prototype" and no [[Construct]].
fn symbol_method_script(symbol: &str, length: u32) -> String {
    format!(
        "var d = Object.getOwnPropertyDescriptor(RegExp.prototype, Symbol.{symbol});\n\
         var m = d.value;\n\
         var constructed;\n\
         try {{ new m(); constructed = true; }} catch (e) {{ constructed = !(e instanceof TypeError); }}\n\
         typeof m === \"function\" && d.writable === true && d.enumerable === false &&\n\
           d.configurable === true && m.length === {length} && m.name === \"[Symbol.{symbol}]\" &&\n\
           !m.hasOwnProperty(\"prototype\") && !constructed"
    )
}

/// `#sec-regexp.prototype-%symbol.match%`: a method named `[Symbol.match]` of length 1. Without
/// `g` in the String of Get of `flags` it answers RegExpExec, the match array or null; with `g`
/// it sets `lastIndex` to 0 and answers an Array of every match's String, or null when none
/// matched, and leaves `lastIndex` at 0. The this value must be an Object, its String argument is
/// converted first, and `flags` is read by Get, so an object with `flags` and `exec` of its own
/// is matched through them.
#[test]
fn match_answers_the_first_match_or_every_match_with_g() {
    all_true(&[
        &symbol_method_script("match", 1),
        r#"var m = /(\d)(\d)/[Symbol.match]("a12b34");
           var all = /\d\d/g[Symbol.match]("a12b34c5");
           var g = /x/g;
           g.lastIndex = 3;
           var none = g[Symbol.match]("abc");
           Array.isArray(m) && m.length === 3 && m[0] === "12" && m[1] === "1" && m.index === 1 &&
             Array.isArray(all) && all.length === 2 && all.join() === "12,34" &&
             Object.getPrototypeOf(all) === Array.prototype && !("index" in all) &&
             none === null && g.lastIndex === 0 &&
             /z/[Symbol.match]("abc") === null"#,
        r#"var log = [];
           var calls = 0;
           var o = {
             get flags() { log.push("flags"); return "g"; },
             set lastIndex(v) { log.push("set " + v); },
             get lastIndex() { log.push("get"); return 0; },
             exec: function (s) { calls++; return calls < 3 ? { 0: "m" + calls } : null; }
           };
           var r = RegExp.prototype[Symbol.match].call(o, { toString: function () { log.push("string"); return "s"; } });
           r.join() === "m1,m2" && log.join() === "string,flags,set 0" && calls === 3"#,
        r#"var thrown = 0;
           [1, "x", undefined].forEach(function (v) {
             try { RegExp.prototype[Symbol.match].call(v, "a"); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           var log = [];
           try {
             RegExp.prototype[Symbol.match].call({ get flags() { log.push("flags"); return ""; } },
               { toString: function () { throw 7; } });
           } catch (e) { log.push(e); }
           thrown === 3 && log.join() === "7""#,
    ]);
}

/// `#sec-regexp.prototype-%symbol.match%`, step 6.b.iii: a global match whose String is empty
/// moves `lastIndex` by AdvanceStringIndex (`#sec-advancestringindex`), past a whole surrogate
/// pair with `u` (or `v`, in the String of `flags`) and by one code unit without; the match
/// answers one empty String per position it stopped at.
#[test]
fn a_global_match_advances_past_an_empty_match_by_a_code_point_with_u() {
    all_true(&[r#"var s = "a😀b";
           var u = /(?:)/gu[Symbol.match](s);
           var plain = /(?:)/g[Symbol.match](s);
           var log = [];
           var o = {
             flags: "gv",
             i: 0,
             get lastIndex() { return this.i; },
             set lastIndex(v) { log.push(v); this.i = v; },
             exec: function () { return this.i < 4 ? { 0: "" } : null; }
           };
           RegExp.prototype[Symbol.match].call(o, s);
           u.length === 4 && plain.length === 5 && u.join("|") === "|||" &&
             log.join() === "0,1,3,4" && "".match(/(?:)/g).length === 1"#]);
}

/// `#sec-regexp.prototype-%symbol.replace%` with a String replacement: a method named
/// `[Symbol.replace]` of length 2; GetSubstitution (`#sec-getsubstitution`) reads the captures
/// (`$1`, `$01`, a capture that did not participate as the empty String), the match (`$&`), the
/// Strings before and after it and the named groups (`$<name>`, from `groups` converted by
/// ToObject); without `g` only the first match is replaced, with `g` every match, an empty one
/// advancing `lastIndex` by AdvanceStringIndex; a result whose position lies before the end of
/// the last one is skipped.
#[test]
fn replace_substitutes_the_captures_and_named_groups() {
    all_true(&[
        &symbol_method_script("replace", 2),
        r#""2024-11".replace(/(\d+)-(\d+)/, "$2/$1") === "11/2024" &&
           "a-b-c".replace(/-/g, "+") === "a+b+c" && "a-b-c".replace(/-/, "+") === "a+b-c" &&
           "abc".replace(/b/, "[$`|$&|$'|$$]") === "a[a|b|c|$]c" &&
           "ab".replace(/(x)?b/, "[$1]") === "a[]" && "ab".replace(/(b)/, "$01$2") === "ab$2" &&
           "abc".replace(/z/g, "-") === "abc" && "abc".replace(/(?:)/g, "-") === "-a-b-c-" &&
           "😀".replace(/(?:)/gu, "-") === "-😀-" &&
           "😀".replace(/(?:)/g, "-") === "-\ud83d-\ude00-""#,
        r#""2024-11".replace(/(?<y>\d+)-(?<m>\d+)/, "$<m>.$<y>.$<none>") === "11.2024." &&
           "ab".replace(/(?<x>a)/, "[$<x]") === "[$<x]b" &&
           "ab".replace(/(a)/, "[$<x>]") === "[$<x>]b""#,
        r#"var n = 0;
           var o = {
             flags: "g",
             lastIndex: 0,
             exec: function () {
               n++;
               if (n === 1) return { 0: "bc", index: 1, length: 1 };
               if (n === 2) return { 0: "a", index: 0, length: 1 };
               if (n === 3) return { 0: "d", index: 3, length: 2, 1: { toString: function () { return "D"; } }, groups: { g: "G" } };
               return null;
             }
           };
           var r = RegExp.prototype[Symbol.replace].call(o, "abcde", "<$1$<g>>");
           r === "a<$1$<g>><DG>e" && n === 4"#,
        r#"var log = [];
           var o = {
             get flags() { log.push("flags"); return ""; },
             exec: function (s) { log.push("exec " + s); return null; }
           };
           var r = RegExp.prototype[Symbol.replace].call(o,
             { toString: function () { log.push("string"); return "s"; } },
             { toString: function () { log.push("replace"); return "r"; } });
           r === "s" && log.join() === "string,replace,flags,exec s""#,
    ]);
}

/// `#sec-regexp.prototype-%symbol.replace%` with a function: it is called once per result, in
/// order, with undefined as its this value and (the match, each capture converted by ToString or
/// undefined, the position, the String) and, when `groups` is not undefined, the groups object;
/// its answer is converted by ToString. The results are all collected before the first call.
#[test]
fn replace_with_a_function_receives_the_captures_the_position_the_string_and_the_groups() {
    all_true(&[
        r#"var seen = [];
           var r = "a1b22".replace(/(\d)(x)?/g, function () {
             "use strict";
             seen.push([this === undefined, arguments.length, arguments[0], arguments[1],
               arguments[2] === undefined, arguments[3], arguments[4]].join(":"));
             return "<" + arguments[0] + ">";
           });
           r === "a<1>b<2><2>" &&
             seen.join("|") === "true:5:1:1:true:1:a1b22|true:5:2:2:true:3:a1b22|true:5:2:2:true:4:a1b22""#,
        r#"var groups;
           var r = "x-y".replace(/(?<a>x)-(?<b>y)/, function (m, p1, p2, pos, s, g) {
             groups = g;
             return [m, p1, p2, pos, s].join(",") + "," + arguments.length;
           });
           r === "x-y,x,y,0,x-y,6" && Object.getPrototypeOf(groups) === null &&
             groups.a === "x" && groups.b === "y" &&
             "ab".replace(/b/, function () { return { toString: function () { return 5; } }; }) === "a5""#,
        r#"var calls = [];
           var execs = 0;
           var o = {
             flags: "g",
             lastIndex: 0,
             exec: function () { execs++; return execs < 3 ? { 0: "x", index: execs - 1, length: 1 } : null; }
           };
           var r = RegExp.prototype[Symbol.replace].call(o, "ab", function (m, p) { calls.push(execs + ":" + p); return "-"; });
           r === "--" && calls.join() === "3:0,3:1""#,
    ]);
}

/// `#sec-regexp.prototype-%symbol.search%`: a method named `[Symbol.search]` of length 1 that
/// answers the `index` of the first match or -1, sets `lastIndex` to +0 before the match only
/// when it is not +0 (so -0 is set), and restores the value it read when SameValue says it
/// changed, `g` and `y` included.
#[test]
fn search_restores_last_index() {
    all_true(&[
        &symbol_method_script("search", 1),
        r#"var g = /b/g;
           g.lastIndex = 2;
           var y = /b/y;
           y.lastIndex = 5;
           var found = g[Symbol.search]("abc");
           var sticky = y[Symbol.search]("bcd");
           found === 1 && g.lastIndex === 2 && sticky === 0 && y.lastIndex === 5 &&
             /z/[Symbol.search]("abc") === -1 && "abc".search(/c/) === 2"#,
        r#"var log = [];
           var o = {
             v: -0,
             get lastIndex() { log.push("get"); return this.v; },
             set lastIndex(v) { log.push("set " + (1 / v)); this.v = v; },
             exec: function () { return { index: 7 }; }
           };
           var r = RegExp.prototype[Symbol.search].call(o, "s");
           var plain = { lastIndex: 0, exec: function () { this.lastIndex = 3; return null; } };
           var p = RegExp.prototype[Symbol.search].call(plain, "s");
           r === 7 && log.join() === "get,set Infinity,get,set -Infinity" &&
             p === -1 && plain.lastIndex === 0"#,
    ]);
}

/// `#sec-regexp.prototype-%symbol.split%`, steps 4 to 10: a method named `[Symbol.split]` of
/// length 2; the splitter is constructed by SpeciesConstructor (`#sec-speciesconstructor`) of
/// the this value with `%RegExp%` as its default, given the this value and the String of
/// `flags` with `y` appended unless it has one; it is matched by `lastIndex` from each position,
/// so a match that starts later is not taken, and `lastIndex` of the this value is not touched.
#[test]
fn split_constructs_a_sticky_splitter_through_the_species_constructor() {
    all_true(&[
        &symbol_method_script("split", 2),
        r#"var made = [];
           function C(pattern, flags) {
             made.push(flags, pattern === r || pattern === sticky);
             return new RegExp(pattern, flags);
           }
           var r = /-/g;
           r.lastIndex = 9;
           r.constructor = { [Symbol.species]: C };
           var parts = r[Symbol.split]("a-b-c");
           var sticky = /-/y;
           sticky.constructor = { [Symbol.species]: C };
           sticky[Symbol.split]("a-b");
           parts.join() === "a,b,c" && made.join() === "gy,true,y,true" && r.lastIndex === 9"#,
        r#"var execs = [];
           function S(pattern, flags) {
             var inner = new RegExp(pattern, flags);
             return { set lastIndex(v) { execs.push("at " + v); inner.lastIndex = v; },
                      get lastIndex() { return inner.lastIndex; },
                      exec: function (s) { return inner.exec(s); } };
           }
           var r = /b/;
           r.constructor = { [Symbol.species]: S };
           var parts = r[Symbol.split]("abc");
           parts.join() === "a,c" && execs.join() === "at 0,at 1,at 2""#,
        r#"var log = [];
           var o = {
             get constructor() { log.push("constructor"); return undefined; },
             get flags() { log.push("flags"); return "u"; },
             source: "x"
           };
           o[Symbol.match] = true;
           var parts = RegExp.prototype[Symbol.split].call(o,
             { toString: function () { log.push("string"); return "axb"; } });
           parts.join() === "a,b" && log.join() === "string,constructor,flags""#,
    ]);
}

/// `#sec-regexp.prototype-%symbol.split%`, steps 11 to 25: the captures of each match are put
/// between the parts (undefined for one that did not participate), the count of parts stops at
/// the limit (ToUint32, after the splitter is made), an empty match at the position after the
/// last match is not a separator, the empty String is no part when the splitter matches it and
/// itself when it does not, and a pair is not split by an empty match with `u`.
#[test]
fn split_with_captures_and_a_limit() {
    all_true(&[
        r#"var a = "a<B>c</B>".split(/<(\/)?([^<>]+)>/);
           a.length === 7 && a[0] === "a" && a[1] === undefined && a[2] === "B" &&
             a[3] === "c" && a[4] === "/" && a[5] === "B" && a[6] === "" &&
             "a,b,c".split(/,/, 2).join("|") === "a|b" && "a,b".split(/,/, 0).length === 0 &&
             "a1b2c".split(/(\d)/, 3).join("|") === "a|1|b" &&
             "a,b".split(/,/, -1).length === 2 && "a,b".split(/,/, 4294967297).length === 1"#,
        r#""ab".split(/a*?/).join("|") === "a|b" && "ab".split(/a*/).join("|") === "|b" &&
           "abc".split(/(?:)/).join("|") === "a|b|c" && "".split(/a/).length === 1 &&
           "".split(/a/)[0] === "" && "".split(/(?:)/).length === 0 &&
           "😀x".split(/(?:)/u).length === 2 &&
           "😀x".split(/(?:)/).length === 3 && "abc".split(/z/).join() === "abc""#,
        r#"var log = [];
           var r = /,/;
           r.constructor = { [Symbol.species]: function (p, f) { log.push("construct"); return new RegExp(p, f); } };
           r[Symbol.split]("a,b", { valueOf: function () { log.push("limit"); return 1; } });
           log.join() === "construct,limit""#,
    ]);
}

/// `#sec-string.prototype.match` and `#sec-string.prototype.search`: methods of length 1. A
/// pattern that is not an Object (undefined and a String included) is made a RegExp by
/// RegExpCreate with undefined flags, and the method keyed by the symbol is invoked on it with
/// ToString of the this value; RequireObjectCoercible comes first. A primitive's prototype is
/// never searched for the method.
#[test]
fn string_match_and_search_create_a_regexp_from_a_string() {
    all_true(&[
        r#"var m = "a.b".match(".");
           var d = Object.getOwnPropertyDescriptor(String.prototype, "match");
           var e = Object.getOwnPropertyDescriptor(String.prototype, "search");
           m[0] === "a" && m.index === 0 && "a.b".search("\\.") === 1 && "abc".search("z") === -1 &&
             "abc".match(undefined)[0] === "" && "abc".search() === 0 &&
             "a1".match(1)[0] === "1" && String.prototype.match.call(123, /2/).index === 1 &&
             d.value.length === 1 && d.value.name === "match" && d.writable && !d.enumerable &&
             d.configurable && e.value.length === 1 && e.value.name === "search""#,
        r#"var thrown = 0;
           [undefined, null].forEach(function (v) {
             try { String.prototype.match.call(v, /a/); } catch (e) { if (e instanceof TypeError) thrown++; }
             try { String.prototype.search.call(v, /a/); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           try { "a".match("("); } catch (e) { if (e instanceof SyntaxError) thrown++; }
           Object.defineProperty(String.prototype, Symbol.match, { get: function () { throw 1; } });
           Object.defineProperty(String.prototype, Symbol.search, { get: function () { throw 1; } });
           thrown === 5 && "abc".match("b").index === 1 && "abc".search("c") === 2"#,
        r#"var log = [];
           var saved = RegExp.prototype[Symbol.match];
           RegExp.prototype[Symbol.match] = function (s) { log.push(this.source + ":" + s + ":" + this.flags); return 9; };
           var r = String.prototype.match.call({ toString: function () { log.push("this"); return "T"; } }, "p+");
           RegExp.prototype[Symbol.match] = saved;
           r === 9 && log.join() === "this,p+:T:""#,
    ]);
}

/// `#sec-string.prototype.match`, `search`, `replace`, `replaceAll` and `split`, the first steps:
/// an Object argument is handed to its method keyed by the symbol, with the this value
/// unconverted, so a RegExp answers through `RegExp.prototype`'s methods and an object of a
/// script through its own.
#[test]
fn string_methods_hand_a_regexp_to_its_symbol_method() {
    all_true(&[
        r##"var o = {};
           o[Symbol.match] = function (s) { return [this === o, typeof s, s, arguments.length].join(); };
           o[Symbol.search] = function (s) { return [this === o, typeof s, s, arguments.length].join(); };
           "ab".match(o) === "true,string,ab,1" && String.prototype.search.call(5, o) === "true,number,5,1" &&
             "a1b2".match(/\d/g).join() === "1,2" && "a1b2".search(/\d/) === 1 &&
             "a1b2".replace(/\d/g, "#") === "a#b#" && "a1b2".replaceAll(/\d/g, "#") === "a#b#" &&
             "a1b2".split(/\d/).join() === "a,b,""##,
        r#"var r = /b/;
           var log = [];
           r[Symbol.match] = function () { log.push("match"); return 1; };
           r[Symbol.search] = function () { log.push("search"); return 2; };
           r[Symbol.replace] = function () { log.push("replace"); return 3; };
           r[Symbol.split] = function () { log.push("split"); return 4; };
           "b".match(r) + "b".search(r) + "b".replace(r) + "b".split(r) === 10 &&
             log.join() === "match,search,replace,split""#,
    ]);
}

/// SpeciesConstructor (`#sec-speciesconstructor`), as `@@split` reads it: Get of `constructor`,
/// the default `%RegExp%` when it is undefined, a TypeError when it is not an Object; Get of
/// `@@species` on it, the default when that is undefined or null, the value when it is a
/// constructor and a TypeError otherwise. The default is the `RegExp` of the current realm.
#[test]
fn species_constructor_reads_constructor_and_species() {
    all_true(&[
        r#"var thrown = [];
           function split(constructor) {
             var r = /,/;
             r.constructor = constructor;
             try { return r[Symbol.split]("a,b").join("|"); } catch (e) { thrown.push(e.constructor.name); return "thrown"; }
           }
           split(undefined) === "a|b" && split({}) === "a|b" &&
             split({ [Symbol.species]: null }) === "a|b" &&
             split({ [Symbol.species]: undefined }) === "a|b" &&
             split(1) === "thrown" && split("RegExp") === "thrown" &&
             split({ [Symbol.species]: 1 }) === "thrown" &&
             split({ [Symbol.species]: {} }) === "thrown" &&
             split({ [Symbol.species]: Math.max }) === "thrown" &&
             split({ [Symbol.species]: () => 0 }) === "thrown" &&
             thrown.join() === "TypeError,TypeError,TypeError,TypeError,TypeError,TypeError" &&
             /,/[Symbol.split].call({ constructor: undefined, flags: "", source: ",", [Symbol.match]: true }, "a,b").join() === "a,b""#,
        r#"var log = [];
           var species = {};
           Object.defineProperty(species, Symbol.species, { get: function () { log.push("species"); return undefined; } });
           var r = /,/;
           Object.defineProperty(r, "constructor", { get: function () { log.push("constructor"); return species; } });
           r[Symbol.split]("a,b").join() === "a,b" && log.join() === "constructor,species""#,
    ]);
}

/// The steps of the entry of `source`, which must answer true, in a new agent with the test
/// limits and the global `s` of `units`, which the host makes before the entry. The steps are
/// read before the answer is: a conversion through the host interface is an entry of its own.
fn steps_with(units: &[u16], source: &str) -> u64 {
    let (mut agent, realm) = agent_and_realm(limits());
    host_string(&mut agent, &realm, "s", units);
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert_eq!(describe(&mut agent, &realm, outcome), yes(), "{source}");
    steps
}

/// The steps a `source` over `s` of `count` code units "a" costs per code unit beside the same
/// over `2 × count`: the difference of the two entries, divided by `count`, checked to divide
/// evenly.
fn steps_per_unit(count: usize, source: &str) -> u64 {
    let a = u16::from(b'a');
    let once = steps_with(&vec![a; count], source);
    let twice = steps_with(&vec![a; 2 * count], source);
    let difference = twice - once;
    let count = u64::try_from(count).unwrap();
    assert_eq!(difference % count, 0, "{difference} steps over {count}");
    difference / count
}

/// The steps of one match of `s.replace(/a/g, "b")`, of `s.match(/a/g)` and of `s.split(/a/)`,
/// measured with [`steps_per_unit`] on this story's code, the same in both collection modes and
/// at every count from 10 to 400 (2026-11-23).
const REPLACE_STEPS_PER_MATCH: u64 = 11;
const MATCH_STEPS_PER_MATCH: u64 = 7;
const SPLIT_STEPS_PER_MATCH: u64 = 7;

/// `docs/design/engine.md`, decision 3, "The step budget", and decision 7: a global `@@replace`
/// pays per match the steps of its two loops (a step per pass of each), of RegExpExec through
/// the builtin `exec` (the call, the matcher, the match array), of the reads of the result and
/// of the replacement, so the steps of `s.replace(/a/g, "b")` over 2n code units "a" exceed
/// those over n by n times [`REPLACE_STEPS_PER_MATCH`], exactly, at every n; `s.match(/a/g)`
/// and `s.split(/a/)` the same with [`MATCH_STEPS_PER_MATCH`] and [`SPLIT_STEPS_PER_MATCH`].
/// The answers are checked by their length: `"b".repeat(n)` is not linear in n at small n. The
/// counts: 200 in the default mode, 20 with a collection at every safe point.
#[test]
fn a_global_replace_charges_each_match() {
    let count = mode_count(200, 20);
    for (source, expected) in [
        (
            "s.replace(/a/g, \"b\").length === s.length",
            REPLACE_STEPS_PER_MATCH,
        ),
        ("s.match(/a/g).length === s.length", MATCH_STEPS_PER_MATCH),
        (
            "s.split(/a/).length === s.length + 1",
            SPLIT_STEPS_PER_MATCH,
        ),
    ] {
        let once = steps_per_unit(count, source);
        let twice = steps_per_unit(2 * count, source);
        println!("{source}: {once} steps per match at {count}, {twice} at twice");
        assert_eq!(twice, once, "{source} at twice the count");
        assert_eq!(once, expected, "{source}");
    }
}

/// The step budget ends a global match: with `steps_per_entry` 50,000, `s.match(/a/g)` of 100
/// code units "a" answers its 100 matches, and that of 100,000 ends the entry in
/// `Terminated(StepBudget)` with no more than 50,000 steps spent; `s.replace(/a/g, "b")` and
/// `s.split(/a/)` alike. With a collection at every safe point the budget is 5,000 and the
/// long String 10,000 code units ([`mode_count`]): each collection there marks the whole heap,
/// which grows with each match kept, and at 50,000 the test ran past 60 s in that mode.
#[test]
fn a_global_match_over_a_long_string_ends_at_the_step_budget() {
    let budget = mode_count(50_000, 5_000);
    let mut limits = limits();
    limits.steps_per_entry = u64::try_from(budget).unwrap();
    let a = u16::from(b'a');
    let (mut agent, realm) = agent_and_realm(limits);
    host_string(&mut agent, &realm, "short", &[a; 100]);
    host_string(&mut agent, &realm, "long", &vec![a; 2 * budget]);
    assert_eq!(
        eval_text(&mut agent, &realm, "short.match(/a/g).length;"),
        Outcome::Normal("100".to_string())
    );
    for source in [
        "long.match(/a/g);",
        "long.replace(/a/g, \"b\");",
        "long.split(/a/);",
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert!(
            agent.last_entry_steps().unwrap() <= limits.steps_per_entry,
            "{source}"
        );
    }
}

/// `#sec-regexp.prototype-%symbol.replace%`, steps 11 and 14: the results of a global
/// `@@replace` are collected before the replacer is first called and read after its calls,
/// so they are rooted while it runs. Each result, its match String and its captures are made
/// by a user `exec` and held by nothing else; each call of the replacer makes garbage. With the
/// feature `collect-every-safe-point` every instruction collects, so a result held without a
/// root is freed and read back wrong; in the default mode the replacer makes 2 MiB of Strings
/// at its first call, past the 1 MiB at least that starts a collection (`heap/mod.rs`,
/// `collection_due`).
#[test]
fn the_results_of_replace_stay_alive_across_a_collection_in_the_replacer() {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 24;
    let (mut agent, realm) = agent_and_realm(limits);
    let script = "var n = 0;\n\
         var o = {\n\
           flags: \"g\",\n\
           lastIndex: 0,\n\
           exec: function () {\n\
             n++;\n\
             if (n > 3) return null;\n\
             return { 0: \"m\" + n, 1: \"c\" + n, index: 2 * (n - 1), length: 2,\n\
                      groups: { g: \"g\" + n } };\n\
           }\n\
         };\n\
         var calls = 0;\n\
         var out = RegExp.prototype[Symbol.replace].call(o, \"abcdef\", function (m, c, p, s, g) {\n\
           calls++;\n\
           var junk = [];\n\
           if (calls === 1) for (var i = 0; i < 2; i++) junk.push(\"x\".repeat(1 << 19) + i);\n\
           for (var j = 0; j < 20; j++) junk.push({ j: j });\n\
           return [m, c, p, g.g].join(\".\") + \";\";\n\
         });\n\
         out + calls;";
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Outcome::Normal("\"m1.c1.0.g1;m2.c2.2.g2;m3.c3.4.g3;3\"".to_string())
    );
}

/// The re-entries of `@@replace` and `@@match`, each called directly so that one builtin stands
/// between two calls of `f` (`String.prototype.replace` handing a RegExp to `@@replace` would be
/// a second): a functional replacer of a single and of a global replace, a user `exec` called by
/// RegExpExec from `@@match`, and the method keyed by `@@match` of an Object given to `match`.
/// Each RegExp has an own `flags` that is data, and the two of `@@replace` an own `exec` that is
/// `null` (RegExpExec, step 3, then runs RegExpBuiltinExec without a call), so that `f` is the
/// first function the method calls: at the deepest re-entry the getter `flags` or the builtin
/// `exec` would be a call past `reentry_depth` before `f`.
const THROUGH_REPLACE: [&str; 4] = [
    "Object.defineProperties(/x/, { flags: { value: \"\" }, exec: { value: null } })[Symbol.replace](\"x\", f)",
    "Object.defineProperties(/x/g, { flags: { value: \"g\" }, exec: { value: null } })[Symbol.replace](\"xx\", f)",
    "Object.defineProperties(/x/, { flags: { value: \"\" }, exec: { value: f } })[Symbol.match](\"x\")",
    "\"x\".match({ [Symbol.match]: f })",
];

/// The re-entries of `@@split`, called as a method of a plain object so that no `call` stands
/// between: the Get of `@@species` of SpeciesConstructor, the species constructor called by
/// Construct, and the Get of `flags`; and the method keyed by `@@search` of an Object given to
/// `search`.
const THROUGH_SPECIES: [&str; 4] = [
    "({ constructor: Object.defineProperty({}, Symbol.species, { get: f }), s: RegExp.prototype[Symbol.split] }).s(\"x\")",
    "({ constructor: { [Symbol.species]: f }, s: RegExp.prototype[Symbol.split] }).s(\"x\")",
    "Object.defineProperty({ s: RegExp.prototype[Symbol.split] }, \"flags\", { get: f }).s(\"x\")",
    "\"x\".search({ [Symbol.search]: f })",
];

/// The `reentry_depth` of the two stack tests: [`BUILTINS_REENTRY`] in the default mode, 25 with
/// a collection at every safe point, where each instruction marks the whole heap. At the 50 of
/// [`BUILTINS_REENTRY`] in that mode the run on the bounded stack alone took 1.8 to 2.1 CPU
/// seconds per test and the whole test 2.7 to 3.2 (measured on 2026-11-23).
const STACK_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    25
} else {
    BUILTINS_REENTRY
};

/// The small `reentry_depth`s each expression of the stack tests reaches exactly: 0, 1 and 7 in
/// the default mode, 7 with a collection at every safe point (each depth is a new agent, and
/// the depths 0 and 1 together took 0.3 to 1.0 CPU seconds per test in that mode).
const SMALL_DEPTHS: &[u32] = if cfg!(feature = "collect-every-safe-point") {
    &[7]
} else {
    &[0, 1, 7]
};

/// The body of the stack test of [`THROUGH_REPLACE`]: one agent per expression.
fn reach_through_replace() {
    for through in THROUGH_REPLACE {
        reach_through(through, STACK_REENTRY);
    }
}

/// The body of the stack test of [`THROUGH_SPECIES`]: one agent per expression.
fn reach_through_species() {
    for through in THROUGH_SPECIES {
        reach_through(through, STACK_REENTRY);
    }
}

/// On a thread with seven eighths of `min_stack_bytes()` for the stack limits, a replacer that
/// calls `replace` again reaches the re-entry limit (a `RangeError`), as do the other
/// expressions of [`THROUGH_REPLACE`], and parses 1,000 nested brackets at the deepest, without
/// a stack overflow (`tests/support/stack.rs`, [`reach_through`]), at the `reentry_depth`
/// [`STACK_REENTRY`]; each also reaches the small depths of [`SMALL_DEPTHS`] exactly.
#[test]
fn a_replacer_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(STACK_REENTRY));
    on_bounded_stack(
        test_path!("a_replacer_re_enters_on_the_promised_stack"),
        stack,
        reach_through_replace,
    )
    .unwrap();
    for &depth in SMALL_DEPTHS {
        for through in THROUGH_REPLACE {
            reach_through(through, depth);
        }
    }
}

/// As [`a_replacer_re_enters_on_the_promised_stack`], through the expressions of
/// [`THROUGH_SPECIES`].
#[test]
fn a_species_constructor_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(STACK_REENTRY));
    on_bounded_stack(
        test_path!("a_species_constructor_re_enters_on_the_promised_stack"),
        stack,
        reach_through_species,
    )
    .unwrap();
    for &depth in SMALL_DEPTHS {
        for through in THROUGH_SPECIES {
            reach_through(through, depth);
        }
    }
}

/// The calls of the consumer's table (`docs/plan/slice-05/consumer-regexps.tsv`), one per row,
/// each on a literal written for this test with the constructs and the flags of that row and
/// the call the row names: `match` (rows 1 and 10), `test` (row 2), `replace` with `$1` (row 3),
/// with a String (rows 4 and 7), with a function (row 5), with the empty String (row 6), with
/// `$&` (row 9) and with an arrow function (row 12), and `split` (rows 8 and 11); and `replace`
/// with a RegExp made from a String by the constructor, as the harness makes one. The patterns,
/// the inputs and the answers are this test's own.
#[test]
fn the_calls_of_the_consumer_table_answer() {
    let rows: [&str; 13] = [
        r#"var m = "  {a}.c".match(/^\s*{(a|b)}\.(?:c)$/);
           m !== null && m.length === 2 && m[0] === "  {a}.c" && m[1] === "a" &&
             "{c}.c".match(/^\s*{(a|b)}\.(?:c)$/) === null"#,
        r#"/[\u0000-\u001f\u007f-\u009f]/.test("a\u0085") && !/[\u0000-\u001f\u007f-\u009f]/.test("abc")"#,
        r#""host:80:90".replace(/^([^:]+)(:\d+)*$/, "$1") === "host" &&
           "plain".replace(/^([^:]+)(:\d+)*$/, "[$1]") === "[plain]""#,
        r#""a\nb\nc".replace(/\n/g, "\\n") === "a\\nb\\nc""#,
        r#"var s = "a\ud800b\udc00c😀";
           var r = s.replace(/([\ud800-\udbff](?![\udc00-\udfff]))|(^|[^\ud800-\udbff])([\udc00-\udfff])/g,
             function (all, lead, before, trail) {
               if (lead) return "<" + lead.charCodeAt(0).toString(16) + ">";
               return before + "<" + trail.charCodeAt(0).toString(16) + ">";
             });
           r === "a<d800>b<dc00>c😀""#,
        r#""a b\tc\n d".replace(/\s/g, "") === "abcd""#,
        r##""a1.bx c2.d e3.-".replace(/\w\d\.[^\-](?:x)?/g, "#") === "# # e3.-""##,
        r#"var parts = "a${x}b${y}c".split(/\$\{([^}]+)\}/g);
           parts.length === 5 && parts.join("|") === "a|x|b|y|c""#,
        r#""a.b*c(d)".replace(/[\\^$.*+?()[\]{}|]/g, "\\$&") === "a\\.b\\*c\\(d\\)""#,
        r#"var m = "-x.".match(/^(\-|)x\.$/);
           var n = "x.".match(/^(\-|)x\.$/);
           m[1] === "-" && n[0] === "x." && n[1] === "" && "y.".match(/^(\-|)x\.$/) === null"#,
        r#"var parts = "a{b}c{}d".split(/{([^}]*)}/g);
           parts.length === 5 && parts.join("|") === "a|b|c||d""#,
        r#""a\u0001b\u001f".replace(/[\x00-\x1f]/g, c => "<" + c.charCodeAt(0) + ">") === "a<1>b<31>""#,
        r#"var re = new RegExp("[.*+?^${}()|[\\]\\\\]", "g");
           "a.b$c".replace(re, "\\$&") === "a\\.b\\$c""#,
    ];
    let (mut agent, realm) = agent_and_realm(limits());
    for (row, script) in rows.iter().enumerate() {
        assert_eq!(
            eval_text(&mut agent, &realm, script),
            yes(),
            "row {}: {script}",
            row + 1
        );
    }
    assert_eq!(agent.refusals(), 0);
}

/// `story:regexp-objects-exec-and-literals` gave `RegExp.prototype[@@replace]` and `[@@split]`
/// a counted refusal; this story's methods replace them: `"aXbX".replace(/X/g, "-")`,
/// `"aXbX".replaceAll(/X/g, "-")` and `"aXb".split(/X/)` answer the standard's Strings and
/// Array, each of the four methods called directly answers, and `refusals()` stays 0.
#[test]
fn replace_replace_all_and_split_given_a_regexp_refuse_nothing() {
    let (mut agent, realm) = agent_and_realm(limits());
    for (source, answer) in [
        ("\"aXbX\".replace(/X/g, \"-\")", "\"a-b-\""),
        ("\"aXbX\".replace(/X/, \"-\")", "\"a-bX\""),
        ("\"aXbX\".replaceAll(/X/g, \"-\")", "\"a-b-\""),
        ("\"aXb\".split(/X/).join(\"|\")", "\"a|b\""),
        ("/X/[Symbol.replace](\"aX\", \"b\")", "\"ab\""),
        ("/X/[Symbol.split](\"aXb\").length", "2"),
        ("/X/[Symbol.match](\"aX\").index", "1"),
        ("/X/[Symbol.search](\"aX\")", "1"),
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal(answer.to_string()),
            "{source}"
        );
    }
    assert_eq!(agent.refusals(), 0);
}
