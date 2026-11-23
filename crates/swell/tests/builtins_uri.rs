//! The URI handling functions (`#sec-uri-handling-functions`): `decodeURI`
//! (`#sec-decodeuri-encodeduri`), `decodeURIComponent`
//! (`#sec-decodeuricomponent-encodeduricomponent`), `encodeURI` (`#sec-encodeuri-uri`) and
//! `encodeURIComponent` (`#sec-encodeuricomponent-uricomponent`), with Encode (`#sec-encode`),
//! Decode (`#sec-decode`) and ParseHexOctet (`#sec-parsehexoctet`); the step budget, charged a
//! step per code unit read and per code unit written, `string_length`, the memory budget and the
//! re-entry limit reached through ToString of the argument on the promised stack. The Strings
//! are made from code units through the host interface (`cx.new_string_utf16`) and read back
//! as code units (`cx.to_utf16`). Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, global, limits, object, realm, stack_with_room};
use support::script::show_thrown;
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, AgentState, Limits, Outcome as HostOutcome, Realm, Root, Termination, Value};

/// What a call of a URI function answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The code units of the String it returned.
    Units(Vec<u16>),
    /// The `name` of what it threw.
    Throw(String),
    Terminated(Termination),
}

/// An agent and a realm with roots of the four functions and of `undefined`.
struct Uri {
    agent: Agent<TestHost>,
    realm: Realm,
    functions: Vec<(&'static str, Root)>,
    undefined: Root,
}

impl Uri {
    fn new(limits: Limits) -> Uri {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        let global_root = global(&mut agent, &realm);
        let (functions, undefined) = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let functions = [
                    "decodeURI",
                    "decodeURIComponent",
                    "encodeURI",
                    "encodeURIComponent",
                ]
                .into_iter()
                .map(|name| {
                    let function = support::host::get(cx, global, name);
                    (name, cx.root(function).unwrap())
                })
                .collect();
                (functions, cx.root(Value::Undefined).unwrap())
            })
            .unwrap();
        Uri {
            agent,
            realm,
            functions,
            undefined,
        }
    }

    /// The global function `name` called from the host with the String of `units`.
    fn call(&mut self, name: &str, units: &[u16]) -> Answer {
        let arg = self
            .agent
            .with_cx(&self.realm, |cx| {
                let string = cx.new_string_utf16(units).unwrap();
                cx.root(Value::String(string)).unwrap()
            })
            .unwrap();
        let function = &self
            .functions
            .iter()
            .find(|(found, _)| *found == name)
            .unwrap()
            .1;
        let outcome = self
            .agent
            .call(&self.realm, function, &self.undefined, &[&arg])
            .unwrap();
        match outcome {
            HostOutcome::Normal(root) => self
                .agent
                .with_cx(&self.realm, |cx| match cx.get(&root).unwrap() {
                    Value::String(string) => Answer::Units(cx.to_utf16(string).unwrap()),
                    other => panic!("{name} returned {other:?}"),
                })
                .unwrap(),
            HostOutcome::Throw(root) => self
                .agent
                .with_cx(&self.realm, |cx| {
                    let thrown = cx.get(&root).unwrap();
                    Answer::Throw(show_thrown(cx, thrown))
                })
                .unwrap(),
            HostOutcome::Terminated(termination) => Answer::Terminated(termination),
        }
    }

    /// [`Uri::call`] with the code units of `text`.
    fn call_str(&mut self, name: &str, text: &str) -> Answer {
        self.call(name, &units(text))
    }

    /// The steps the last call spent.
    fn steps(&self) -> u64 {
        self.agent.last_entry_steps().unwrap()
    }
}

/// The code units of `text`.
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The answer of a String of `text`.
fn string(text: &str) -> Answer {
    Answer::Units(units(text))
}

/// A thrown URIError.
fn uri_error() -> Answer {
    Answer::Throw("URIError".to_string())
}

/// What Encode answers for the scalar values `scalars`, written here from its steps
/// (`#sec-encode`) and the UTF-8 encoding of `char`: a code unit of the ASCII word characters,
/// of "-.!~*'()" or of `extra` as it is, every other code point as "%" and two uppercase hex
/// digits per octet.
fn expected_encode(scalars: &[char], extra: &str) -> Vec<u16> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = Vec::with_capacity(scalars.len() * 12);
    for &scalar in scalars {
        if scalar.is_ascii_alphanumeric() || "_-.!~*'()".contains(scalar) || extra.contains(scalar)
        {
            out.push(scalar as u16);
        } else {
            let mut buffer = [0_u8; 4];
            for &octet in scalar.encode_utf8(&mut buffer).as_bytes() {
                out.push(u16::from(b'%'));
                out.push(u16::from(HEX[usize::from(octet >> 4)]));
                out.push(u16::from(HEX[usize::from(octet & 0xF)]));
            }
        }
    }
    out
}

/// The reserved set of `encodeURI` and `decodeURI` (`#sec-encodeuri-uri`,
/// `#sec-decodeuri-encodeduri`).
const RESERVED: &str = ";/?:@&=+$,#";

/// The step budget of each call of [`every_code_point_of_a_plane_round_trips`]: the most a
/// call of it needs is 1,024 code points of four octets, 2,048 code units read and 12,288
/// written, with the step of the call itself.
const ROUND_TRIP_STEPS: u64 = 16_384;

/// Encode and Decode of every Unicode scalar value of the plane `plane` (0x10000 × `plane` to
/// 0xFFFF above it; in plane 0 without the 2,048 surrogates) in Strings of 1,024 code points:
/// `encodeURIComponent` and `encodeURI` answer what [`expected_encode`] writes, and
/// `decodeURIComponent` and `decodeURI` of those answers are the String again; each call within
/// [`ROUND_TRIP_STEPS`] steps. Plane 0 has 63,488 scalar values in 62 Strings, each other plane
/// 65,536 in 64: the seventeen tests below pass all 1,112,064 scalar values, 1,086 Strings and
/// 4,344 calls. One test per plane, since one test of all of them took 17.49 s in a debug build
/// (measured; `AGENTS.md` of the folder that holds the repository, "Tests stay fast").
///
/// With the feature `collect-every-safe-point` each plane takes every [`CHUNK_STRIDE`]th String
/// of 1,024 (the file has no full count, and the lane `slow` does not run it): there the test
/// shows that the four
/// functions keep the Strings of a call across the collections of its safe points, which every
/// String of a plane shows alike; that every scalar value round trips is what the default mode
/// shows, and it keeps every String. In that mode the seventeen tests took 9.4 CPU seconds
/// together, each run alone (0.53 to 0.60 a plane, measured on 2026-11-23).
fn every_code_point_of_a_plane_round_trips(plane: u32) {
    let mut limits = limits();
    limits.steps_per_entry = ROUND_TRIP_STEPS;
    let mut uri = Uri::new(limits);
    let first = plane * 0x10000;
    let scalars: Vec<char> = (first..first + 0x10000)
        .filter_map(char::from_u32)
        .collect();
    let (count, expected_strings) = if plane == 0 {
        (0x10000 - 0x800, 62)
    } else {
        (0x10000, 64)
    };
    assert_eq!(scalars.len(), count);
    let expected_strings = usize::div_ceil(expected_strings, CHUNK_STRIDE);
    let mut strings = 0;
    for chunk in scalars.chunks(1_024).step_by(CHUNK_STRIDE) {
        strings += 1;
        let text: String = chunk.iter().collect();
        let source = units(&text);
        for (encode, decode, extra) in [
            ("encodeURIComponent", "decodeURIComponent", ""),
            ("encodeURI", "decodeURI", RESERVED),
        ] {
            let expected = expected_encode(chunk, extra);
            let encoded = uri.call(encode, &source);
            assert!(
                encoded == Answer::Units(expected.clone()),
                "{encode} of the String from {:#x}",
                u32::from(chunk[0])
            );
            assert!(
                uri.call(decode, &expected) == Answer::Units(source.clone()),
                "{decode} of the String from {:#x}",
                u32::from(chunk[0])
            );
        }
    }
    assert_eq!(strings, expected_strings);
}

/// Which Strings of 1,024 code points of a plane [`every_code_point_of_a_plane_round_trips`]
/// passes: each of them in the default mode, and with the feature `collect-every-safe-point`
/// the first of each 16, 4 a plane (in plane 0 the first, from U+0000, holds the code points of
/// one and of two octets, the others those of three; in the other planes every one has four).
const CHUNK_STRIDE: usize = if cfg!(feature = "collect-every-safe-point") {
    16
} else {
    1
};

/// One test per plane, each over [`every_code_point_of_a_plane_round_trips`].
macro_rules! round_trip_planes {
    ($($name:ident $plane:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                every_code_point_of_a_plane_round_trips($plane);
            }
        )*
    };
}

round_trip_planes!(
    every_code_point_of_plane_00_round_trips_through_encode_and_decode 0,
    every_code_point_of_plane_01_round_trips_through_encode_and_decode 1,
    every_code_point_of_plane_02_round_trips_through_encode_and_decode 2,
    every_code_point_of_plane_03_round_trips_through_encode_and_decode 3,
    every_code_point_of_plane_04_round_trips_through_encode_and_decode 4,
    every_code_point_of_plane_05_round_trips_through_encode_and_decode 5,
    every_code_point_of_plane_06_round_trips_through_encode_and_decode 6,
    every_code_point_of_plane_07_round_trips_through_encode_and_decode 7,
    every_code_point_of_plane_08_round_trips_through_encode_and_decode 8,
    every_code_point_of_plane_09_round_trips_through_encode_and_decode 9,
    every_code_point_of_plane_10_round_trips_through_encode_and_decode 10,
    every_code_point_of_plane_11_round_trips_through_encode_and_decode 11,
    every_code_point_of_plane_12_round_trips_through_encode_and_decode 12,
    every_code_point_of_plane_13_round_trips_through_encode_and_decode 13,
    every_code_point_of_plane_14_round_trips_through_encode_and_decode 14,
    every_code_point_of_plane_15_round_trips_through_encode_and_decode 15,
    every_code_point_of_plane_16_round_trips_through_encode_and_decode 16,
);

/// `#sec-encodeuri-uri` and `#sec-encodeuricomponent-uricomponent`: both keep the ASCII word
/// characters and "-.!~*'()" and escape the rest of Basic Latin; `encodeURI` keeps the reserved
/// set ";/?:@&=+$,#" and `encodeURIComponent` escapes it; "%" and the space are escaped by
/// both; an octet is two uppercase hex digits; the argument goes through ToString.
#[test]
fn encode_uri_keeps_the_reserved_set_and_encode_uri_component_does_not() {
    let mut uri = Uri::new(limits());
    let kept = "ABCXYZabcxyz0189_-.!~*'()";
    assert_eq!(uri.call_str("encodeURI", kept), string(kept));
    assert_eq!(uri.call_str("encodeURIComponent", kept), string(kept));
    assert_eq!(uri.call_str("encodeURI", RESERVED), string(RESERVED));
    assert_eq!(
        uri.call_str("encodeURIComponent", RESERVED),
        string("%3B%2F%3F%3A%40%26%3D%2B%24%2C%23")
    );
    for name in ["encodeURI", "encodeURIComponent"] {
        assert_eq!(
            uri.call_str(name, "% \"<>[]\\^`{|}\u{0}\u{7f}\u{80}\u{ff}"),
            string("%25%20%22%3C%3E%5B%5D%5C%5E%60%7B%7C%7D%00%7F%C2%80%C3%BF"),
            "{name}"
        );
        assert_eq!(
            uri.call_str(
                name,
                "\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}"
            ),
            string("%DF%BF%E0%A0%80%ED%9F%BF%EE%80%80%EF%BF%BF%F0%90%80%80%F4%8F%BF%BF"),
            "{name}"
        );
        assert_eq!(uri.call_str(name, ""), string(""), "{name}");
    }
    let (sloppy, strict) = support::script::run_both(
        "[encodeURI(), encodeURIComponent(null), encodeURI(12.5), encodeURIComponent(true), \
          encodeURI({ toString: function () { return 'a b'; } })].join();",
    );
    let expected = support::script::Outcome::Normal("\"undefined,null,12.5,true,a%20b\"".into());
    assert_eq!(sloppy, expected);
    assert_eq!(strict, expected);
}

/// `#sec-decode`: `decodeURI` keeps an escape of a code unit of its reserved set as written,
/// upper or lower case, and decodes every other escape; `decodeURIComponent` decodes all;
/// ParseHexOctet (`#sec-parsehexoctet`) reads upper and lower case hex digits; text that is
/// not an escape is kept.
#[test]
fn decode_uri_keeps_the_escapes_of_its_reserved_set() {
    let mut uri = Uri::new(limits());
    let escaped = "%3B%2F%3F%3A%40%26%3D%2B%24%2C%23";
    assert_eq!(uri.call_str("decodeURI", escaped), string(escaped));
    assert_eq!(
        uri.call_str("decodeURIComponent", escaped),
        string(RESERVED)
    );
    assert_eq!(uri.call_str("decodeURI", "%3b%2f%2B"), string("%3b%2f%2B"));
    assert_eq!(
        uri.call_str("decodeURIComponent", "%3b%2f%2B"),
        string(";/+")
    );
    for name in ["decodeURI", "decodeURIComponent"] {
        assert_eq!(uri.call_str(name, "%41%20%25%7e"), string("A %~"), "{name}");
        assert_eq!(uri.call_str(name, "%2541"), string("%41"), "{name}");
        assert_eq!(
            uri.call_str(name, "a%C3%A9b%e2%82%acc%F0%9F%98%80"),
            string("a\u{e9}b\u{20ac}c\u{1f600}"),
            "{name}"
        );
        assert_eq!(uri.call_str(name, RESERVED), string(RESERVED), "{name}");
        assert_eq!(uri.call_str(name, ""), string(""), "{name}");
        // A lone surrogate that is not an escape is a code unit like any other.
        assert_eq!(
            uri.call(name, &[0xD800, 0x25, 0x34, 0x31]),
            Answer::Units(vec![0xD800, 0x41]),
            "{name}"
        );
    }
}

/// `#sec-encode`, step 6.b.ii: a lone surrogate, leading or trailing, at the start, in the
/// middle or at the end, throws a URIError in both encode functions. `#sec-decode`: a "%"
/// without two hex digits after it, a first octet with one leading 1 bit or more than four,
/// and a sequence cut short or whose continuation is not an escape throw a URIError in both
/// decode functions.
#[test]
fn lone_surrogates_and_malformed_escapes_throw_uri_error() {
    let mut uri = Uri::new(limits());
    let lone: [&[u16]; 7] = [
        &[0xD800],
        &[0xDBFF],
        &[0xDC00],
        &[0xDFFF],
        &[0x61, 0xD800, 0x62],
        &[0xDC00, 0xD800],
        &[0xD83D, 0xDE00, 0xD83D],
    ];
    for name in ["encodeURI", "encodeURIComponent"] {
        for input in lone {
            assert_eq!(uri.call(name, input), uri_error(), "{name} of {input:x?}");
        }
    }
    let malformed = [
        "%",
        "%4",
        "a%",
        "a%4",
        "%G0",
        "%0G",
        "%%41",
        "% 41",
        "%+1",
        "%-1",
        "%\u{663}\u{663}",
        "%80",
        "%BF",
        "%F8%80%80%80%80",
        "%FC%80%80%80%80%80",
        "%FE",
        "%FF",
        "%C3",
        "%C3%",
        "%C3%A",
        "%C3A9",
        "%C3%G9",
        "%E2%82",
        "%E2%82%A",
        "%F0%9F%98",
        "%F0%9F%98%",
        "%E2%82%AC%E2",
    ];
    for name in ["decodeURI", "decodeURIComponent"] {
        for input in malformed {
            assert_eq!(
                uri.call_str(name, input),
                uri_error(),
                "{name} of {input:?}"
            );
        }
    }
}

/// `#sec-decode`, step 4.d.vii ("If octets does not contain a valid UTF-8 encoding of a
/// Unicode code point, throw a URIError exception"; RFC 3629): overlong forms, the encodings of
/// the surrogates, sequences above U+10FFFF and continuations that are not 10xxxxxx throw; the
/// edges of each length decode.
#[test]
fn overlong_and_out_of_range_utf8_throws_uri_error() {
    let mut uri = Uri::new(limits());
    let invalid = [
        "%C0%80",
        "%C0%AF",
        "%C1%BF",
        "%E0%80%80",
        "%E0%9F%BF",
        "%F0%80%80%80",
        "%F0%8F%BF%BF",
        "%ED%A0%80",
        "%ED%AF%BF",
        "%ED%B0%80",
        "%ED%BF%BF",
        "%F4%90%80%80",
        "%F4%BF%BF%BF",
        "%F5%80%80%80",
        "%F7%BF%BF%BF",
        "%C3%28",
        "%C3%C3",
        "%E2%28%A1",
        "%E2%82%28",
        "%F0%28%8C%BC",
        "%F0%90%28%BC",
        "%F0%90%8C%28",
    ];
    let valid = [
        ("%00", "\u{0}"),
        ("%7F", "\u{7f}"),
        ("%C2%80", "\u{80}"),
        ("%DF%BF", "\u{7ff}"),
        ("%E0%A0%80", "\u{800}"),
        ("%ED%9F%BF", "\u{d7ff}"),
        ("%EE%80%80", "\u{e000}"),
        ("%EF%BF%BF", "\u{ffff}"),
        ("%F0%90%80%80", "\u{10000}"),
        ("%F4%8F%BF%BF", "\u{10ffff}"),
    ];
    for name in ["decodeURI", "decodeURIComponent"] {
        for input in invalid {
            assert_eq!(uri.call_str(name, input), uri_error(), "{name} of {input}");
        }
        for (input, decoded) in valid {
            assert_eq!(
                uri.call_str(name, input),
                string(decoded),
                "{name} of {input}"
            );
        }
    }
}

/// The step budget (`docs/design/engine.md`, decision 4): each of the four functions charges a
/// step per code unit it reads and per code unit it writes, so a call costs what a call of the
/// empty String costs plus exactly that; the cost grows with the String, 1,000 code points as
/// a thousand times one.
#[test]
fn uri_functions_charge_a_step_per_code_unit() {
    let mut uri = Uri::new(limits());
    let mut base = std::collections::BTreeMap::new();
    for name in [
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
    ] {
        assert_eq!(uri.call_str(name, ""), string(""));
        base.insert(name, uri.steps());
    }
    let e_acute = "\u{e9}".repeat(1_000);
    let cases: [(&str, &str, u64); 16] = [
        ("encodeURIComponent", "a", 1 + 1),
        ("encodeURIComponent", ";", 1 + 3),
        ("encodeURI", ";", 1 + 1),
        ("encodeURIComponent", "\u{e9}", 1 + 6),
        ("encodeURI", "\u{20ac}", 1 + 9),
        ("encodeURIComponent", "\u{1f600}", 2 + 12),
        ("encodeURIComponent", "ab\u{e9}", 3 + 8),
        ("encodeURIComponent", &e_acute, 1_000 + 6_000),
        ("decodeURI", "ab", 2 + 2),
        ("decodeURIComponent", "%41", 3 + 1),
        ("decodeURI", "%3B", 3 + 3),
        ("decodeURIComponent", "%3B", 3 + 1),
        ("decodeURIComponent", "%C3%A9", 6 + 1),
        ("decodeURI", "%E2%82%AC", 9 + 1),
        ("decodeURIComponent", "%F0%9F%98%80", 12 + 2),
        ("decodeURI", "x%F0%9F%98%80y", 14 + 4),
    ];
    for (name, input, units) in cases {
        assert!(matches!(uri.call_str(name, input), Answer::Units(_)));
        assert_eq!(
            uri.steps() - base[name],
            units,
            "{name} of {} code units",
            input.encode_utf16().count()
        );
    }
}

/// The step budget ends an encode of a long String: with `steps_per_entry` 1,000,
/// `encodeURIComponent` of 10,000 code units, which needs 70,000 steps, and `decodeURI` of
/// 2,000 ends in `Terminated(StepBudget)` without spending more than the budget; the agent
/// stays alive and the next call answers.
#[test]
fn step_budget_ends_an_encode_of_a_long_string() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let mut uri = Uri::new(limits);
    let long = vec![0xE9_u16; 10_000];
    assert_eq!(
        uri.call("encodeURIComponent", &long),
        Answer::Terminated(Termination::StepBudget)
    );
    assert!(uri.steps() <= 1_000);
    assert_eq!(
        uri.call("decodeURI", &[0x61; 2_000]),
        Answer::Terminated(Termination::StepBudget)
    );
    assert!(uri.steps() <= 1_000);
    assert_eq!(uri.agent.state(), AgentState::Alive);
    assert_eq!(uri.call_str("encodeURI", "\u{e9}"), string("%C3%A9"));
}

/// `string_length` (`docs/design/host-interface.md`, section 2): with `string_length` 100, an
/// encode whose result has exactly 100 code units answers it, and one whose result would have
/// 101 or 120 throws a RangeError; the agent answers the next call.
#[test]
fn encode_past_string_length_throws_range_error() {
    let mut limits = limits();
    limits.string_length = 100;
    let mut uri = Uri::new(limits);
    let at = format!("{}abcd", "\u{e9}".repeat(16));
    let Answer::Units(encoded) = uri.call_str("encodeURIComponent", &at) else {
        panic!("not a String");
    };
    assert_eq!(encoded.len(), 100);
    assert_eq!(
        uri.call_str("encodeURIComponent", &format!("{at}e")),
        Answer::Throw("RangeError".to_string())
    );
    assert_eq!(
        uri.call_str("encodeURI", &"\u{e9}".repeat(20)),
        Answer::Throw("RangeError".to_string())
    );
    assert_eq!(uri.call_str("encodeURI", "a"), string("a"));
}

/// The memory budget (`docs/design/engine.md`, decision 4): an encode whose result would not
/// fit `memory_bytes` ends in `Terminated(MemoryBudget)`, with `heap_bytes()` within the
/// budget after it.
#[test]
fn encode_past_the_memory_budget_terminates() {
    let mut probe = Uri::new(limits());
    probe.agent.collect().unwrap();
    let room = probe.agent.heap_bytes().unwrap() + 64 * 1024;
    let mut limits = limits();
    limits.memory_bytes = room + 40_000;
    limits.steps_per_entry = 1 << 30;
    let mut uri = Uri::new(limits);
    // 20,000 code units, 40,000 bytes; the result would be 120,000 code units.
    assert_eq!(
        uri.call("encodeURIComponent", &vec![0xE9_u16; 20_000]),
        Answer::Terminated(Termination::MemoryBudget)
    );
    assert!(uri.agent.heap_bytes().unwrap() <= limits.memory_bytes);
    assert_eq!(uri.agent.state(), AgentState::Alive);
}

/// The body of the stack test: ToString of the argument of `encodeURI` calls `f`.
fn reach_through_encode_uri() {
    reach_through("encodeURI({ toString: f })", BUILTINS_REENTRY);
}

/// `Limits::min_stack_bytes` through ToString of the argument of `encodeURI`
/// (`#sec-encodeuri-uri`, step 1): on a thread with seven eighths of `min_stack_bytes()`
/// ([`stack_with_room`]), a script reaches `reentry_depth` re-entries through
/// `encodeURI({ toString: f })`, parses 1,000 nested brackets at the deepest and ends in the
/// RangeError of the limit, without a stack overflow. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn to_string_of_the_argument_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("to_string_of_the_argument_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_encode_uri,
    )
    .unwrap();
}
