//! `JSON.parse` (`#sec-json.parse`) with ParseJSON (`#sec-ParseJSON`), the JSON Parse Record
//! (`#sec-json-parse-record`), CreateJSONParseRecord (`#sec-createjsonparserecord`) and
//! InternalizeJSONProperty (`#sec-internalizejsonproperty`); `JSON.rawJSON`
//! (`#sec-json.rawjson`), `JSON.isRawJSON` (`#sec-json.israwjson`) and the step of
//! SerializeJSONProperty (`#sec-serializejsonproperty`) that writes a raw JSON object as its
//! text. The parser and the reviver walk run on stacks of their own, charged to the memory
//! budget, with a step per code unit read and per value made (`docs/design/engine.md`,
//! decision 3); a String longer than `string_length` is a `RangeError`
//! (`docs/design/host-interface.md`, section 2). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, is_error_of, key, limits, object, realm,
    stack_with_room, text, thrown,
};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Abrupt, Completion, Cx, HostCall, Limits, ObjectRef, PropertyKey, Termination, Value};

/// `JSON.parse(args)` through `cx.call`.
fn parse_call(cx: &mut Cx<'_, TestHost>, global: ObjectRef, args: &[Value]) -> Completion<Value> {
    let json = get_object(cx, global, "JSON");
    let method = get(cx, json, "parse");
    cx.call(method, Value::Object(json), args).unwrap()
}

/// A String of the code units `units`, made by the host (bounded by the memory budget only).
fn units_value(cx: &mut Cx<'_, TestHost>, units: &[u16]) -> Value {
    Value::String(cx.new_string_utf16(units).unwrap())
}

/// The code units of `text`.
fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// Each script answers `expected` as written and in strict mode.
fn check_all(cases: &[(&str, &str)]) {
    for (source, expected) in cases {
        let expected = Outcome::Normal((*expected).to_string());
        let (sloppy, strict) = run_both(source);
        assert_eq!(sloppy, expected, "{source}");
        assert_eq!(strict, expected, "strict: {source}");
    }
}

/// `#sec-json.parse` and `#sec-ParseJSON`: every kind of JSON value with the four white space
/// code points around it; a number by the JSON grammar, -0 included; a String with every
/// escape, a lone surrogate and a code point outside ASCII; arrays and objects of
/// `%Array.prototype%` and `%Object.prototype%` with their properties writable, enumerable and
/// configurable in the order OrdinaryOwnPropertyKeys gives; ToString of a text that is no
/// String; the function's `length` and `name`.
#[test]
fn parse_of_every_kind_of_value() {
    check_all(&[
        ("JSON.parse('null');", "null"),
        ("JSON.parse(' \\t\\n\\rtrue\\r\\n\\t ');", "true"),
        ("JSON.parse('false');", "false"),
        ("JSON.parse('0');", "0"),
        ("1 / JSON.parse('-0');", "-Infinity"),
        ("JSON.parse('-12.25E-1');", "-1.225"),
        ("JSON.parse('1.5e+2');", "150"),
        ("JSON.parse('1e400');", "Infinity"),
        (
            "JSON.parse('123456789012345678901234567890');",
            "1.2345678901234568e+29",
        ),
        ("JSON.parse('\"\"');", "\"\""),
        (
            "JSON.parse('\"a\\\\\"\\\\\\\\\\\\/\\\\b\\\\f\\\\n\\\\r\\\\t\\\\u0041\\\\u00e9\"') === \
             'a\"\\\\/\\b\\f\\n\\r\\tA\\u00e9';",
            "true",
        ),
        ("JSON.parse('\"\\\\ud800\"') === '\\ud800';", "true"),
        (
            "JSON.parse('\"\\\\uD83D\\\\uDE00\"') === '\\ud83d\\ude00';",
            "true",
        ),
        (
            "JSON.parse('\"\\u00e9\\u2028\\ud800\"') === '\\u00e9\\u2028\\ud800';",
            "true",
        ),
        (
            "JSON.stringify(JSON.parse(' { \"a\" : [1, {\"b\": null}, [], \"s\"], \"c\": {} } '));",
            "\"{\"a\":[1,{\"b\":null},[],\"s\"],\"c\":{}}\"",
        ),
        ("Array.isArray(JSON.parse('[]'));", "true"),
        (
            "Object.getPrototypeOf(JSON.parse('[1]')) === Array.prototype;",
            "true",
        ),
        (
            "Object.getPrototypeOf(JSON.parse('{}')) === Object.prototype;",
            "true",
        ),
        ("JSON.parse('[1, 2, 3]').length;", "3"),
        (
            "Object.getOwnPropertyNames(JSON.parse('{\"b\":1,\"a\":2,\"1\":3}')).join();",
            "\"1,b,a\"",
        ),
        (
            "var d = Object.getOwnPropertyDescriptor(JSON.parse('{\"a\":1}'), 'a'); \
             [d.value, d.writable, d.enumerable, d.configurable].join();",
            "\"1,true,true,true\"",
        ),
        (
            "var d = Object.getOwnPropertyDescriptor(JSON.parse('[1]'), '0'); \
             [d.value, d.writable, d.enumerable, d.configurable].join();",
            "\"1,true,true,true\"",
        ),
        ("JSON.parse(1);", "1"),
        ("JSON.parse(null);", "null"),
        (
            "JSON.parse({ toString: function () { return '[2]'; } })[0];",
            "2",
        ),
        ("JSON.parse.length + JSON.parse.name;", "\"2parse\""),
        ("JSON.parse('[]', null).length;", "0"),
        ("JSON.parse('[]', {}).length;", "0"),
    ]);
}

/// `#sec-ParseJSON` step 1: a text that is not JSON as ECMA-404 states it throws a
/// `SyntaxError` of the realm: no leading zero, no trailing comma, no single quote, no
/// unescaped control character, no other white space than the four, no comment, no
/// ECMAScript-only form, nothing after the value.
#[test]
fn parse_refuses_what_is_not_json() {
    let texts: &[&str] = &[
        "",
        " ",
        "01",
        "-",
        "-01",
        "- 1",
        "1.",
        ".5",
        "1.e1",
        "1e",
        "1e+",
        "+1",
        "0x10",
        "1_0",
        "Infinity",
        "-Infinity",
        "NaN",
        "undefined",
        "tru",
        "nul",
        "True",
        "[1,]",
        "[,1]",
        "[1 2]",
        "[1",
        "]",
        "{\"a\":1,}",
        "{,}",
        "{\"a\"}",
        "{\"a\":}",
        "{\"a\" 1}",
        "{\"a\":1",
        "{a:1}",
        "{'a':1}",
        "{1:1}",
        "'a'",
        "\"abc",
        "\"a\u{0}\"",
        "\"\t\"",
        "\"\n\"",
        "\"\u{1f}\"",
        "\"\\x41\"",
        "\"\\u00G0\"",
        "\"\\u12\"",
        "\"\\'\"",
        "\"\\u{41}\"",
        "\"\\a\"",
        "1 2",
        "true false",
        "[1]x",
        "\u{a0}1",
        "\u{feff}1",
        "\u{2028}1",
        "\u{b}1",
        "\u{c}1",
        "\u{3000}1",
        "//c\n1",
        "/*c*/1",
    ];
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            for source in texts {
                let value = units_value(cx, &utf16(source));
                let error = thrown(parse_call(cx, global, &[value]));
                assert!(
                    is_error_of(cx, global, error, "SyntaxError"),
                    "{source:?} did not throw a SyntaxError"
                );
            }
        })
        .unwrap();
}

/// The nesting of the tests of 100,000 brackets: 2,000 with the feature
/// `collect-every-safe-point`, as `stringify_of_100000_nested_arrays_needs_no_rust_stack` of
/// `tests/builtins_json.rs` nests, where a collection at every call of the reviver traces every
/// array made so far.
const DEPTH: usize = if cfg!(feature = "collect-every-safe-point") {
    2_000
} else {
    100_000
};

/// The nesting of the tests of the reviver on 100,000 nested arrays in the gate, as
/// [`reviver_depth`] gives it: 40,000 in the default mode, 200 with the feature
/// `collect-every-safe-point`.
///
/// The lane `slow` (`SWELL_FULL_COUNTS=1`) nests 60,000 in the default mode and 1,000, half of
/// [`DEPTH`], in the collect mode ([`reviver_depth`]). At [`DEPTH`] in the default mode the
/// two runs took 16.7 s together, and in the collect mode, where every call of the reviver
/// collects, the test was terminated at 60 s (measured at a load of 100 to 110 on 17 cores,
/// cargo-nextest). The
/// gate's nesting still reaches what the test is for. In the default mode the walk runs on
/// [`nesting_stack`], the stack of the full nesting scaled by 40,000 / 100,000, so a walk that
/// overflows at the full nesting overflows at the gate's; it ran on the whole stack before,
/// where only a Rust call per level of 88 bytes or more would have overflowed before level
/// 40,000. 40,000 arrays still pass the memory budget of 1 MiB. In the collect mode every call
/// of the reviver collects while the arrays of all 200 levels and the records of the walk are
/// held by the walk alone, which shows a value held without a root at any depth.
const GATE_REVIVER_DEPTH: usize = if cfg!(feature = "collect-every-safe-point") {
    200
} else {
    40_000
};

/// The nesting of the reviver tests: 60,000 under `SWELL_FULL_COUNTS=1` (the lane `slow`) in
/// the default mode, on [`nesting_stack`] of it (at [`DEPTH`] the result test took 3.3 CPU
/// seconds alone, measured on 2026-11-22, a reviver call per level),
/// [`GATE_REVIVER_DEPTH`] in the gate. With the feature `collect-every-safe-point` the lane
/// nests 1,000, half of [`DEPTH`]: every call of the reviver collects while the walk holds
/// every level, so the cost grows with the square of the nesting (at 2,000 the result test
/// took 7.3 CPU seconds alone, measured on 2026-11-22), and a value held without a root shows
/// at any depth.
fn reviver_depth() -> usize {
    if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        if cfg!(feature = "collect-every-safe-point") {
            DEPTH / 2
        } else {
            60_000
        }
    } else {
        GATE_REVIVER_DEPTH
    }
}

/// The stack a parse or a reviver walk nested `depth` levels runs on: seven eighths of
/// `min_stack_bytes()` of the test limits (3,505,824 bytes) at [`DEPTH`], scaled by `depth` /
/// [`DEPTH`] in the default mode, as `deep_stack` of `crates/syntax/tests/sources.rs` scales
/// its stack. With a fixed part of f bytes and p bytes a level, the run at 100,000 levels
/// overflows its stack S when f + 100,000 p > S, and then the gate's run at 40,000 has
/// f + 40,000 p ≥ 0.4 (f + 100,000 p) > 0.4 S: it overflows whenever the one at the full
/// nesting does, whatever f is. The other way it is stricter by 0.6 f, which leaves 1,402,329
/// bytes for the fixed part. The collect mode nests [`DEPTH`] or less on the whole stack: it
/// shows that a value held across a collection is rooted, and the default mode shows the stack
/// bound.
fn nesting_stack(depth: usize) -> usize {
    let full = stack_with_room(limits());
    if cfg!(feature = "collect-every-safe-point") {
        full
    } else {
        let scaled = u64::try_from(full).unwrap() * u64::try_from(depth).unwrap()
            / u64::try_from(DEPTH).unwrap();
        usize::try_from(scaled).unwrap()
    }
}

/// `depth` "[" then `depth` "]".
fn nested_text(depth: usize) -> Vec<u16> {
    let mut units = vec![u16::from(b'['); depth];
    units.extend(std::iter::repeat_n(u16::from(b']'), depth));
    units
}

/// Follow the element "0" from `value` down `depth - 1` levels: each an Array of length 1, the
/// innermost of length 0.
///
/// The keys `length` and `0` are made once. A key String made for each read stays live until
/// the scope ends (HOST-170) and is charged to the budget: at 100,000 levels, after a normal
/// parse that left `heap_bytes()` at 55,858,499 bytes, 130,783 such Strings were made and the
/// next one, at level 65,392, ended in `Terminated(MemoryBudget)` at 63,105,353 bytes of the
/// 64 MiB of the test limits: the strings arena's next doubling did not fit (measured on
/// `cc41ae6`, with the arenas charged by capacity).
fn check_nesting(cx: &mut Cx<'_, TestHost>, value: Value, depth: usize) {
    let Value::Object(mut array) = value else {
        panic!("{value:?}");
    };
    let length = key(cx, "length");
    let first = key(cx, "0");
    for level in 1..depth {
        let found = cx.get_property(array, length).unwrap();
        assert_eq!(found, Ok(Value::Number(1.0)), "{level}");
        array = match cx.get_property(array, first).unwrap() {
            Ok(Value::Object(next)) => next,
            other => panic!("{level}: {other:?}"),
        };
    }
    let found = cx.get_property(array, length).unwrap();
    assert_eq!(found, Ok(Value::Number(0.0)));
}

/// What a parse of the text nested `depth` levels answers with `limits`, through
/// `JSON.parse(text, reviver)` when `reviver` is a script function; a normal result is checked
/// level by level.
fn parse_nested(limits: Limits, depth: usize, reviver: Option<&str>) -> Result<(), Termination> {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let reviver = reviver.map(|source| match eval(&mut agent, &realm, source) {
        swell::Outcome::Normal(root) => root,
        other => panic!("{source}: {other:?}"),
    });
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let text = units_value(cx, &nested_text(depth));
            let mut args = vec![text];
            if let Some(root) = &reviver {
                args.push(cx.get(root).unwrap());
            }
            match parse_call(cx, global, &args) {
                Ok(value) => {
                    check_nesting(cx, value, depth);
                    Ok(())
                }
                Err(Abrupt::Terminated(termination)) => Err(termination),
                Err(other) => panic!("{other:?}"),
            }
        })
        .unwrap()
}

/// The charged reading, `heap_bytes()`, of the setup of [`parse_nested`] without a reviver for
/// `depth` levels: an agent under the test limits, its realm, the text as a String, and
/// `JSON.parse` read from the global object. Measured in an agent of its own, which makes the
/// steps of `parse_nested` in its order.
fn setup_charge(depth: usize) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let _text = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let text = units_value(cx, &nested_text(depth));
            let json = get_object(cx, global, "JSON");
            get(cx, json, "parse");
            cx.root(text).unwrap()
        })
        .unwrap();
    agent.heap_bytes().unwrap()
}

/// The room of the memory budget above [`setup_charge`] under which the parse of the nested
/// text must end in `Terminated(MemoryBudget)`: 1 MiB, the budget of the epic's acceptance 6,
/// given to the parse alone.
///
/// Measured on `cc41ae6` in the default mode (100,000 levels): the setup reads 372,099 bytes
/// (the realm 172,090, the text 200,000 at a byte per code unit, the two keys 9). The parse
/// first charges a copy of the text, 400,000 bytes, then an Array and an open entry per level;
/// at the setup the objects arena has 108 of its 256 slots free and doubles at the 109th,
/// 365th and 877th object made after it (52,224, 104,448 and 208,896 bytes, 204 per slot). Any
/// room from 885,896 to 1,094,879 bytes ends the parse at the same charge: the array of level
/// 877, whose doubling of the objects arena does not fit. 1 MiB is 162,680 bytes above the
/// charge before that doubling and 46,304 below the charge after it; with 46,304 bytes more the
/// parse ends at the next doubling (417,792 bytes, at the 1,901st object). The outcome rests
/// on the room only being smaller than the parse: a full parse leaves `heap_bytes()` at
/// 55,858,499, so every room up to about 55 MB ends in the memory budget, and a room below
/// 400,000 ends it before the first level. A setup that charges more moves the budget with it.
const PARSE_ROOM: u64 = 1 << 20;

/// The nesting of the parse that must end in a result in
/// [`parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget`]: [`DEPTH`] in the
/// lane `slow` (`SWELL_FULL_COUNTS=1`) and with the feature `collect-every-safe-point`; 40,000
/// in the default mode's gate, as [`GATE_REVIVER_DEPTH`] nests, on the stack of the full
/// nesting scaled to it ([`nesting_stack`]), so a parse that overflows at 100,000 overflows at
/// 40,000 too. At 100,000 the test took 1.60 CPU seconds alone in the default mode (measured on
/// 2026-11-22), most of it the parse and the walk of every level after it. The parse under
/// the memory budget keeps [`DEPTH`] in every run: it ends at level 877 ([`PARSE_ROOM`]).
fn result_depth() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        || std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        DEPTH
    } else {
        40_000
    }
}

/// The epic's acceptance 6 (`docs/design/engine.md`, decision 3, "Depth bounds"): a text of
/// 100,000 nested brackets ([`DEPTH`] in collect mode, [`result_depth`] in the default mode's
/// gate), on a thread of seven eighths of `min_stack_bytes()` ([`nesting_stack`] of the
/// nesting), ends in a result under the test limits, every level an Array, and in
/// `Terminated(MemoryBudget)` under a memory budget that the setup fits with 1 MiB to spare
/// ([`setup_charge`] plus [`PARSE_ROOM`]) and the parse does not (or a result, in collect mode,
/// where the nesting is smaller), never in a Rust stack overflow.
#[test]
fn parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget() {
    on_bounded_stack(
        test_path!("parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget"),
        nesting_stack(result_depth()),
        || {
            assert_eq!(parse_nested(limits(), result_depth(), None), Ok(()));
            let mut small = limits();
            small.memory_bytes = setup_charge(DEPTH) + PARSE_ROOM;
            let outcome = parse_nested(small, DEPTH, None);
            if cfg!(feature = "collect-every-safe-point") {
                assert!(
                    matches!(outcome, Ok(()) | Err(Termination::MemoryBudget)),
                    "{outcome:?}"
                );
            } else {
                assert_eq!(outcome, Err(Termination::MemoryBudget));
            }
        },
    )
    .unwrap();
}

/// The steps of `source` in a fresh agent with the test limits; it must answer normally.
fn steps_of(source: &str) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let outcome = eval_text(&mut agent, &realm, source);
    assert!(
        matches!(outcome, Outcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    agent.last_entry_steps().unwrap()
}

/// Decision 2 of the design, "per node for `JSON.parse`": scripts with the same instructions
/// and texts that differ only in their length or their values differ in steps by one per code
/// unit and one per value: `[  ]` two code units more than `[]`, `[0,0]` three code units and
/// two values more, `{"a":0}` five code units and one value more, a String of ten code units
/// eight more than `""` and none for the value.
#[test]
fn parse_charges_a_step_per_code_unit_and_per_node() {
    let base = steps_of("JSON.parse('[]');");
    for (source, more) in [
        ("JSON.parse('[  ]');", 2),
        ("JSON.parse('[0,0]');", 3 + 2),
        ("JSON.parse('{\"a\":0}');", 5 + 1),
        ("JSON.parse('\"abcdefgh\"');", 8),
    ] {
        assert_eq!(steps_of(source), base + more, "{source}");
    }
}

/// A step budget of 1,000 ends the parse of a text of 30,001 code units in
/// `Terminated(StepBudget)`, with no more steps spent than the budget: a long array, a long run
/// of white space and a long String.
#[test]
fn step_budget_ends_a_parse_of_a_long_text() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let long_array = format!("[{}0]", "0,".repeat(14_999));
    let long_space = format!("{}1", " ".repeat(30_000));
    let long_string = format!("\"{}\"", "a".repeat(29_999));
    for source in [long_array, long_space, long_string] {
        let outcome = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let value = units_value(cx, &utf16(&source));
                parse_call(cx, global, &[value])
            })
            .unwrap();
        assert_eq!(outcome, Err(Abrupt::Terminated(Termination::StepBudget)));
        assert!(agent.last_entry_steps().unwrap() <= 1_000);
    }
}

/// `string_length` (`docs/design/host-interface.md`, section 2) bounds every String the parse
/// makes from the text: with `string_length` 10, a String of 10 code units is made, one of 11
/// throws a `RangeError`, as a value and as a key; the text itself, made by the host, is
/// bounded by the memory budget only.
#[test]
fn parse_past_string_length_throws_range_error() {
    let mut limits = limits();
    limits.string_length = 10;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let fits = units_value(cx, &utf16("\"abcdefghij\""));
            let made = parse_call(cx, global, &[fits]).unwrap();
            assert_eq!(text(cx, made), "abcdefghij");
            for over in [
                "\"abcdefghijk\"",
                "[\"abcdefghijk\"]",
                "{\"abcdefghijk\":1}",
            ] {
                let value = units_value(cx, &utf16(over));
                let error = thrown(parse_call(cx, global, &[value]));
                assert!(is_error_of(cx, global, error, "RangeError"), "{over}");
            }
        })
        .unwrap();
}

/// `#sec-ParseJSON`, note 2, and `#sec-json.parse`'s use of 13.2.5: a duplicate key keeps the
/// last value in the place of the first, and `__proto__` is an own data property that leaves
/// the [[Prototype]] alone, twice too.
#[test]
fn duplicate_keys_and_proto_are_own_data_properties() {
    check_all(&[
        (
            "var o = JSON.parse('{\"a\":1,\"b\":2,\"a\":3}'); \
             Object.getOwnPropertyNames(o).join() + ':' + o.a + o.b;",
            "\"a,b:32\"",
        ),
        (
            "var o = JSON.parse('{\"__proto__\":[1]}'); \
             [Object.getPrototypeOf(o) === Object.prototype, o.hasOwnProperty('__proto__'), \
              Array.isArray(o.__proto__), Object.getOwnPropertyNames(o).join()].join();",
            "\"true,true,true,__proto__\"",
        ),
        (
            "var o = JSON.parse('{\"__proto__\":1,\"__proto__\":2}'); \
             o.__proto__ + ':' + Object.getOwnPropertyNames(o).length;",
            "\"2:1\"",
        ),
    ]);
}

/// `#sec-internalizejsonproperty`: the reviver is called with the holder as this value, the
/// leaves before the object that holds them, in key order, the root last with the key ""; an
/// undefined answer deletes the property, another replaces it; the root's answer is the
/// result.
#[test]
fn reviver_visits_leaves_first_and_may_delete_and_replace() {
    check_all(&[
        (
            "var log = []; \
             var r = JSON.parse('{\"a\":[1,2],\"b\":{\"c\":3,\"d\":4}}', function (k, v) { \
               log.push(k + (typeof v === 'object' ? '' : '=' + v)); \
               if (k === 'c') return undefined; \
               if (k === '0') return 10; \
               return v; \
             }); \
             log.join(' ') + '|' + JSON.stringify(r);",
            "\"0=1 1=2 a c=3 d=4 b |{\"a\":[10,2],\"b\":{\"d\":4}}\"",
        ),
        (
            "var holders = []; \
             var r = JSON.parse('{\"a\":{\"b\":1}}', function (k, v) { holders.push(this); return v; }); \
             [holders[0] === r.a, holders[1] === r, Object.getOwnPropertyNames(holders[2]).join()].join();",
            "\"true,true,\"",
        ),
        (
            "JSON.parse('[1]', function (k, v) { return k === '' ? 'root' : v; });",
            "\"root\"",
        ),
        (
            "JSON.parse('[1, 2]', function (k, v) { return k === '' ? v : undefined; }).length;",
            "2",
        ),
        (
            "var r = JSON.parse('[1, 2]', function (k, v) { return k === '' ? v : undefined; }); \
             r.hasOwnProperty('0') + ',' + r.hasOwnProperty('1');",
            "\"false,false\"",
        ),
    ]);
}

/// `#sec-internalizejsonproperty` step 3 and `#sec-createjsonparserecord`: the context object
/// of a primitive value the walk has not seen changed has `source`, the text the value was
/// parsed from (the minus sign of a number, the quotes and escapes of a String); an object or
/// array has none, and neither has a value the reviver changed before its turn, unless it set
/// the same value again (SameValue); a duplicate key gives the source of its last value.
#[test]
fn reviver_gets_the_source_of_an_unchanged_primitive_only() {
    check_all(&[
        (
            "var seen = []; \
             JSON.parse('[1.0, -0, \"x\\\\u0041\", true, null, {\"k\": 1e2, \"k\": -5E-1}, [2]]', \
               function (k, v, c) { \
                 seen.push(k + '=' + (c.hasOwnProperty('source') ? c.source : '-')); \
                 return v; \
               }); \
             seen.join(' ');",
            "\"0=1.0 1=-0 2=\"x\\u0041\" 3=true 4=null k=-5E-1 5=- 0=2 6=- =-\"",
        ),
        (
            "var seen = []; \
             JSON.parse('[1, 2, 3]', function (k, v, c) { \
               if (k === '0') { this[1] = 20; this[2] = 3; } \
               seen.push(k + '=' + (c.hasOwnProperty('source') ? c.source : '-')); \
               return v; \
             }); \
             seen.join(' ');",
            "\"0=1 1=- 2=3 =-\"",
        ),
        (
            "var seen = []; \
             JSON.parse('{\"a\": 1, \"b\": [2]}', function (k, v, c) { \
               if (k === 'a') this.b = [2]; \
               seen.push(k + '=' + (c.hasOwnProperty('source') ? c.source : '-')); \
               return v; \
             }); \
             seen.join(' ');",
            "\"a=1 0=- b=- =-\"",
        ),
        (
            "var c0; JSON.parse('1', function (k, v, c) { c0 = c; return v; }); \
             [Object.getPrototypeOf(c0) === Object.prototype, \
              Object.getOwnPropertyNames(c0).join()].join();",
            "\"true,source\"",
        ),
    ]);
}

/// The reviver of the tests on 100,000 nested arrays: a script function that answers its value.
const IDENTITY_REVIVER: Option<&str> = Some("(function (k, v) { return v; });");

/// The limits of the reviver tests on 100,000 nested arrays with steps and memory enough
/// (512 MiB: the arrays, their records and the frames of the walk pass the 64 MiB of the test
/// limits, measured).
fn roomy_limits() -> Limits {
    let mut roomy = limits();
    roomy.steps_per_entry = 1 << 40;
    roomy.memory_bytes = 512 << 20;
    roomy
}

/// The reviver on 100,000 nested arrays ([`reviver_depth`] levels), a script function called
/// once per array, on a thread of seven eighths of `min_stack_bytes()` ([`nesting_stack`]):
/// the walk is a loop on a stack of its own, so with steps and memory enough
/// ([`roomy_limits`]) it ends in a result, each level an Array, never in a Rust stack overflow.
/// The run under a memory budget of 1 MiB is the test after it, in a process of its own, so
/// that each fits under the 60 s of cargo-nextest at the full nesting of the collect mode.
#[test]
fn reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget() {
    on_bounded_stack(
        test_path!("reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget"),
        nesting_stack(reviver_depth()),
        || {
            assert_eq!(
                parse_nested(roomy_limits(), reviver_depth(), IDENTITY_REVIVER),
                Ok(())
            );
        },
    )
    .unwrap();
}

/// The reviver on 100,000 nested arrays ([`reviver_depth`] levels), as in
/// [`reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget`], under a memory budget of
/// 1 MiB: `Terminated(MemoryBudget)` (or a result in collect mode), never a Rust stack overflow.
#[test]
fn reviver_on_100000_nested_arrays_under_1_mib_ends_in_the_memory_budget() {
    on_bounded_stack(
        test_path!("reviver_on_100000_nested_arrays_under_1_mib_ends_in_the_memory_budget"),
        nesting_stack(reviver_depth()),
        || {
            let mut small = roomy_limits();
            small.memory_bytes = 1 << 20;
            let outcome = parse_nested(small, reviver_depth(), IDENTITY_REVIVER);
            if cfg!(feature = "collect-every-safe-point") {
                assert!(
                    matches!(outcome, Ok(()) | Err(Termination::MemoryBudget)),
                    "{outcome:?}"
                );
            } else {
                assert_eq!(outcome, Err(Termination::MemoryBudget));
            }
        },
    )
    .unwrap();
}

/// `#sec-json.rawjson` and `#sec-json.israwjson`: a raw JSON object has a null prototype, one
/// own property `rawJSON` (the text, enumerable, neither writable nor configurable) and is
/// frozen: a strict assignment of a new property throws a `TypeError`. A text that is not one
/// JSON primitive value, or that starts or ends with white space, throws a `SyntaxError`.
/// `isRawJSON` is true for it only. The three functions follow `stringify` and
/// @@toStringTag on `JSON`, in the order of their clauses.
#[test]
fn raw_json_is_frozen_has_a_null_prototype_and_refuses_what_is_not_a_primitive() {
    check_all(&[
        (
            "var r = JSON.rawJSON(1); var d = Object.getOwnPropertyDescriptor(r, 'rawJSON'); \
             [Object.getPrototypeOf(r) === null, typeof d.value, d.value, d.writable, \
              d.enumerable, d.configurable, Object.getOwnPropertyNames(r).join()].join();",
            "\"true,string,1,false,true,false,rawJSON\"",
        ),
        (
            "[JSON.rawJSON(null).rawJSON, JSON.rawJSON(true).rawJSON, \
              JSON.rawJSON('\"a\"').rawJSON, JSON.rawJSON(-1.5).rawJSON].join();",
            "\"null,true,\"a\",-1.5\"",
        ),
        (
            "[JSON.isRawJSON(JSON.rawJSON(1)), JSON.isRawJSON({}), JSON.isRawJSON(1), \
              JSON.isRawJSON(), JSON.isRawJSON({ rawJSON: '1' })].join();",
            "\"true,false,false,false,false\"",
        ),
        (
            "[JSON.isRawJSON.length, JSON.isRawJSON.name, JSON.rawJSON.length, \
              JSON.rawJSON.name].join();",
            "\"1,isRawJSON,1,rawJSON\"",
        ),
        (
            "Object.getOwnPropertyNames(JSON).join();",
            "\"stringify,isRawJSON,parse,rawJSON\"",
        ),
        (
            "var r = JSON.rawJSON(1); \
             (function () { 'use strict'; try { r.x = 1; return 'assigned'; } \
               catch (e) { return e instanceof TypeError; } }());",
            "true",
        ),
        (
            "var r = JSON.rawJSON(1); \
             (function () { 'use strict'; try { r.rawJSON = '2'; return 'assigned'; } \
               catch (e) { return e instanceof TypeError && r.rawJSON === '1'; } }());",
            "true",
        ),
    ]);
    let refused = [
        "''",
        "'[]'",
        "'{}'",
        "'[1]'",
        "' 1'",
        "'1 '",
        "'\\t1'",
        "'\"a\" '",
        "'+1'",
        "'01'",
        "'-'",
        "'tru'",
        "'undefined'",
        "'1x'",
        "'\"a'",
        "'nul'",
        "{}",
        "[]",
        "undefined",
    ];
    for text in refused {
        let source = format!(
            "try {{ JSON.rawJSON({text}); 'made'; }} \
             catch (e) {{ e instanceof SyntaxError && e.constructor === SyntaxError; }}"
        );
        let (sloppy, strict) = run_both(&source);
        assert_eq!(sloppy, Outcome::Normal("true".to_string()), "{text}");
        assert_eq!(
            strict,
            Outcome::Normal("true".to_string()),
            "strict: {text}"
        );
    }
}

/// `#sec-serializejsonproperty` step 4.a: a raw JSON object is written as its text, as a value,
/// as an element, as a member, from `toJSON` and from the replacer, whatever the gap.
#[test]
fn stringify_writes_raw_json_as_its_text() {
    check_all(&[
        ("JSON.stringify(JSON.rawJSON('1e1000'));", "\"1e1000\""),
        (
            "JSON.stringify({ a: JSON.rawJSON('-0'), b: [JSON.rawJSON('\"x\"'), \
              JSON.rawJSON('null')] });",
            "\"{\"a\":-0,\"b\":[\"x\",null]}\"",
        ),
        (
            "JSON.stringify({ toJSON: function () { return JSON.rawJSON('12345678901234567890'); } });",
            "\"12345678901234567890\"",
        ),
        (
            "JSON.stringify([1], function (k, v) { return k === '0' ? JSON.rawJSON('true') : v; });",
            "\"[true]\"",
        ),
        (
            "JSON.stringify([JSON.rawJSON('1')], null, 1);",
            "\"[\n 1\n]\"",
        ),
    ]);
}

/// A reviver that makes more than a collection's worth of garbage at the value "s1" (so that the
/// default mode collects at the next safe point too) and sets this[1] to a new String equal to
/// the one parsed there; at each key it records the key, the value and the source in the
/// host's calls.
fn collecting_reviver(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let key = call.args.first().copied().unwrap_or(Value::Undefined);
    let value = call.args.get(1).copied().unwrap_or(Value::Undefined);
    let Some(&Value::Object(context)) = call.args.get(2) else {
        panic!("no context object: {:?}", call.args);
    };
    let key_text = text(cx, key);
    let shown = match value {
        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
        Value::Object(_) => "object".to_string(),
        other => format!("{other:?}"),
    };
    if shown == "s1" {
        let garbage = "g".repeat(64 * 1024);
        for _ in 0..40 {
            cx.new_string(&garbage).unwrap();
        }
        let Value::Object(holder) = call.this else {
            panic!("this {:?}", call.this);
        };
        let one = PropertyKey::String(cx.new_string("1").unwrap());
        let fresh = Value::String(cx.new_string("s2").unwrap());
        cx.set(holder, one, fresh, true).unwrap().unwrap();
    }
    let source_key = PropertyKey::String(cx.new_string("source").unwrap());
    let source = match cx.get_property(context, source_key).unwrap().unwrap() {
        Value::String(source) => cx.to_rust_string_lossy(source).unwrap(),
        _ => "-".to_string(),
    };
    cx.host_mut()
        .calls
        .push(format!("{key_text}={shown} {source}"));
    Ok(value)
}

/// Decision 4 (`docs/design/engine.md`): what the parse made and its parse records are kept
/// across every call of the reviver. The reviver makes garbage and replaces this[1] by an equal
/// new String; at the next call a collection runs (at every call with
/// `collect-every-safe-point`); the record of "s2", which nothing else holds now, is read by
/// SameValue, so the source is given; every value and key reads back afterwards.
#[test]
fn what_parse_holds_survives_a_collection_in_the_reviver() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let reviver = cx
                .new_host_function(realm_ref, "reviver", 2, collecting_reviver, false, None)
                .unwrap()
                .unwrap();
            let source = units_value(
                cx,
                &utf16("[\"s1\", \"s2\", {\"key\": \"s3\", \"other\": [\"s4\"]}]"),
            );
            let result = parse_call(cx, global, &[source, Value::Object(reviver)]).unwrap();
            let Value::Object(result) = result else {
                panic!("{result:?}");
            };
            let first = get(cx, result, "0");
            assert_eq!(text(cx, first), "s1");
            let second = get(cx, result, "1");
            assert_eq!(text(cx, second), "s2");
            let inner = get_object(cx, result, "2");
            let key_value = get(cx, inner, "key");
            assert_eq!(text(cx, key_value), "s3");
            let other = get_object(cx, inner, "other");
            let deepest = get(cx, other, "0");
            assert_eq!(text(cx, deepest), "s4");
            std::mem::take(&mut cx.host_mut().calls)
        })
        .unwrap();
    assert_eq!(
        calls,
        [
            "0=s1 \"s1\"",
            "1=s2 \"s2\"",
            "key=s3 \"s3\"",
            "0=s4 \"s4\"",
            "other=object -",
            "2=object -",
            "=object -",
        ]
    );
}

/// The Rust stack `Limits::min_stack_bytes` promises holds through the reviver
/// (`#sec-internalizejsonproperty`, the Call of its last step): `JSON.parse("[0]", f)` re-enters
/// `f` until the re-entry limit, on seven eighths of `min_stack_bytes()`, with the helpers of
/// `tests/support/stack.rs`.
#[test]
fn reviver_reaches_the_reentry_limit_on_the_promised_stack() {
    on_bounded_stack(
        test_path!("reviver_reaches_the_reentry_limit_on_the_promised_stack"),
        stack_with_room(stack_limits(BUILTINS_REENTRY)),
        || reach_through("JSON.parse(\"[0]\", f)", BUILTINS_REENTRY),
    )
    .unwrap();
}

/// What one `JSON.parse` call answers ("normal", "thrown" or the termination) and
/// `heap_bytes()` after a collection, for four calls of the same text in one agent. The first
/// call may grow the arenas; every call after it must read the same.
fn parse_readings(limits: Limits, units: &[u16], reviver: Option<&str>) -> Vec<(String, u64)> {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let reviver = reviver.map(|source| match eval(&mut agent, &realm, source) {
        swell::Outcome::Normal(root) => root,
        other => panic!("{source}: {other:?}"),
    });
    let mut readings = Vec::new();
    for _ in 0..4 {
        let kind = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let text = units_value(cx, units);
                let mut args = vec![text];
                if let Some(root) = &reviver {
                    args.push(cx.get(root).unwrap());
                }
                match parse_call(cx, global, &args) {
                    Ok(_) => "normal".to_string(),
                    Err(Abrupt::Throw(_)) => "thrown".to_string(),
                    Err(Abrupt::Terminated(termination)) => format!("{termination:?}"),
                }
            })
            .unwrap();
        assert!(matches!(
            agent.collect().unwrap(),
            swell::Outcome::Normal(_)
        ));
        readings.push((kind, agent.heap_bytes().unwrap()));
    }
    readings
}

/// The bytes `JSON.parse` charges for its open values, its records and its copy of the text
/// are given back on every way out, so the charged reading after a call and a collection is the
/// same from the second call on: (i) a deep valid text with a reviver, (ii) a deep text that
/// ends in the middle, (iii) a deep text under a memory budget that ends the parse at depth,
/// (iv) the same with a reviver (adversary pass 2 on unit growth).
#[test]
fn parse_gives_back_its_charges_on_every_way_out() {
    let deep = 2_000;
    let unclosed = vec![u16::from(b'['); deep];
    let reviver = "(function (key, value) { return value; })";
    let mut small = limits();
    small.memory_bytes = setup_charge(DEPTH) + PARSE_ROOM;
    let mut small_reviver = limits();
    small_reviver.memory_bytes = setup_charge(DEPTH) + 2 * PARSE_ROOM;
    let cases = [
        (
            "valid with reviver",
            limits(),
            nested_text(deep),
            Some(reviver),
            "normal",
        ),
        ("ends in the middle", limits(), unclosed, None, "thrown"),
        ("budget", small, nested_text(DEPTH), None, "MemoryBudget"),
        (
            "budget with reviver",
            small_reviver,
            nested_text(DEPTH),
            Some(reviver),
            "MemoryBudget",
        ),
    ];
    for (name, limits, units, reviver, expected) in cases {
        let readings = parse_readings(limits, &units, reviver);
        for (kind, _) in &readings {
            assert_eq!(kind, expected, "{name}: {readings:?}");
        }
        let settled = readings[1].1;
        for (call, (_, bytes)) in readings.iter().enumerate().skip(1) {
            assert_eq!(*bytes, settled, "{name}: call {call}: {readings:?}");
        }
    }
}
