//! Strings at the host boundary (`docs/design/host-interface.md`, decision 4; `engine.md`,
//! decision 5): the exact UTF-16 conversion of Web IDL's `DOMString`, the lossy one of its
//! `USVString`, and a `&str` taken without loss.

use crate::support;

use swell::HostError;

/// HOST-32: `to_utf16` returns every code unit, lone surrogates included.
#[test]
fn host_32_to_utf16_keeps_lone_surrogates() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let units = agent
        .with_cx(&realm, |cx| {
            let s = cx.new_string_utf16(&[0xD800, 0x61]).unwrap();
            cx.to_utf16(s).unwrap()
        })
        .unwrap();
    assert_eq!(units, vec![0xD800, 0x61]);

    // A trailing lone surrogate and a lone low surrogate as well, and the Latin-1 form.
    let units = agent
        .with_cx(&realm, |cx| {
            let a = cx.new_string_utf16(&[0x61, 0xDC00, 0x62, 0xD800]).unwrap();
            let b = cx.new_string_utf16(&[0x61, 0xFF]).unwrap();
            (cx.to_utf16(a).unwrap(), cx.to_utf16(b).unwrap())
        })
        .unwrap();
    assert_eq!(units, (vec![0x61, 0xDC00, 0x62, 0xD800], vec![0x61, 0xFF]));
}

/// HOST-33: `to_rust_string_lossy` replaces each lone surrogate with U+FFFD and keeps every
/// other code point.
#[test]
fn host_33_lossy_string_replaces_lone_surrogates() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let text = agent
        .with_cx(&realm, |cx| {
            let s = cx
                .new_string_utf16(&[0xD800, 0x61, 0xD83D, 0xDE00])
                .unwrap();
            cx.to_rust_string_lossy(s).unwrap()
        })
        .unwrap();
    assert_eq!(text, "\u{FFFD}a\u{1F600}");

    let text = agent
        .with_cx(&realm, |cx| {
            let s = cx.new_string_utf16(&[0xDE00, 0xD83D, 0xFF]).unwrap();
            cx.to_rust_string_lossy(s).unwrap()
        })
        .unwrap();
    assert_eq!(text, "\u{FFFD}\u{FFFD}\u{FF}");
}

/// HOST-34: `new_string` of a `&str` is the String of its UTF-16 encoding.
#[test]
fn host_34_new_string_is_utf16_of_the_str() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    for text in [
        "",
        "abc",
        "caf\u{E9}",
        "\u{20AC}uro",
        "a\u{1F600}b",
        "\u{FFFF}\u{10FFFF}",
    ] {
        let (units, lossy) = agent
            .with_cx(&realm, |cx| {
                let s = cx.new_string(text).unwrap();
                (cx.to_utf16(s).unwrap(), cx.to_rust_string_lossy(s).unwrap())
            })
            .unwrap();
        assert_eq!(units, text.encode_utf16().collect::<Vec<u16>>(), "{text:?}");
        assert_eq!(lossy, text);
    }
}

/// A string handle of another agent is `ForeignHandle`; one the collector freed is
/// `StaleHandle` (HOST-30 and HOST-31 are held by later stories; this is the read they use).
#[test]
fn a_string_of_another_agent_or_a_freed_one_is_refused() {
    let mut a = support::host::agent().unwrap();
    let realm_a = support::host::realm(&mut a);
    let mut b = support::host::agent().unwrap();
    let realm_b = support::host::realm(&mut b);
    let s = a
        .with_cx(&realm_a, |cx| cx.new_string("x").unwrap())
        .unwrap();
    assert_eq!(
        b.with_cx(&realm_b, |cx| cx.to_utf16(s)).unwrap(),
        Err(HostError::ForeignHandle)
    );
    assert!(matches!(a.collect(), Ok(swell::Outcome::Normal(_))));
    assert_eq!(
        a.with_cx(&realm_a, |cx| cx.to_utf16(s)).unwrap(),
        Err(HostError::StaleHandle)
    );
}
