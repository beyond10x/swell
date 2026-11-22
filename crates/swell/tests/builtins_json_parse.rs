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

mod support;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, is_error_of, limits, object, realm,
    stack_with_room, text, thrown,
};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits};
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

/// `DEPTH` "[" then `DEPTH` "]".
fn nested_text() -> Vec<u16> {
    let mut units = vec![u16::from(b'['); DEPTH];
    units.extend(std::iter::repeat_n(u16::from(b']'), DEPTH));
    units
}

/// Follow the element "0" from `value` down `DEPTH - 1` levels: each an Array of length 1, the
/// innermost of length 0.
fn check_nesting(cx: &mut Cx<'_, TestHost>, value: Value) {
    let Value::Object(mut array) = value else {
        panic!("{value:?}");
    };
    for level in 1..DEPTH {
        assert_eq!(get(cx, array, "length"), Value::Number(1.0), "{level}");
        array = get_object(cx, array, "0");
    }
    assert_eq!(get(cx, array, "length"), Value::Number(0.0));
}

/// What a parse of the nested text answers with `limits`, through `JSON.parse(text, reviver)`
/// when `reviver` is a script function; a normal result is checked level by level.
fn parse_nested(limits: Limits, reviver: Option<&str>) -> Result<(), Termination> {
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
            let text = units_value(cx, &nested_text());
            let mut args = vec![text];
            if let Some(root) = &reviver {
                args.push(cx.get(root).unwrap());
            }
            match parse_call(cx, global, &args) {
                Ok(value) => {
                    check_nesting(cx, value);
                    Ok(())
                }
                Err(Abrupt::Terminated(termination)) => Err(termination),
                Err(other) => panic!("{other:?}"),
            }
        })
        .unwrap()
}

/// The epic's acceptance 6 (`docs/design/engine.md`, decision 3, "Depth bounds"): a text of
/// 100,000 nested brackets ([`DEPTH`] in collect mode), on a thread of seven eighths of
/// `min_stack_bytes()`, ends in a result under the test limits, every level an Array, and in
/// `Terminated(MemoryBudget)` under a memory budget of 1 MiB (or a result, in collect mode,
/// where the nesting is smaller), never in a Rust stack overflow.
#[test]
fn parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget() {
    on_bounded_stack(
        "parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget",
        stack_with_room(limits()),
        || {
            assert_eq!(parse_nested(limits(), None), Ok(()));
            let mut small = limits();
            small.memory_bytes = 1 << 20;
            let outcome = parse_nested(small, None);
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

/// The reviver on 100,000 nested arrays ([`DEPTH`] in collect mode), a script function called
/// once per array, on a thread of seven eighths of `min_stack_bytes()`: the walk is a loop on
/// a stack of its own, so with steps and memory enough (512 MiB: the arrays, their records and
/// the frames of the walk pass the 64 MiB of the test limits, measured) it ends in a result,
/// each level an Array, and under a memory budget of 1 MiB in `Terminated(MemoryBudget)` (or a
/// result in collect mode), never in a Rust stack overflow.
#[test]
fn reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget() {
    on_bounded_stack(
        "reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget",
        stack_with_room(limits()),
        || {
            let reviver = Some("(function (k, v) { return v; });");
            let mut roomy = limits();
            roomy.steps_per_entry = 1 << 40;
            roomy.memory_bytes = 512 << 20;
            assert_eq!(parse_nested(roomy, reviver), Ok(()));
            let mut small = roomy;
            small.memory_bytes = 1 << 20;
            let outcome = parse_nested(small, reviver);
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
        "reviver_reaches_the_reentry_limit_on_the_promised_stack",
        stack_with_room(stack_limits(BUILTINS_REENTRY)),
        || reach_through("JSON.parse(\"[0]\", f)", BUILTINS_REENTRY),
    )
    .unwrap();
}
