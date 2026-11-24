//! The text of a date, tested without an engine: the three forms `Date.parse` reads
//! (`docs/design/engine.md` decision 6, "`Date.parse` and the String argument of `new Date`";
//! `#sec-date-time-string-format`, `#sec-expanded-years`, `#sec-todatestring`,
//! `#sec-date.prototype.toutcstring`) and the strings of `DateString`, `TimeString`,
//! `TimeZoneString`, `toUTCString` and `toISOString`.
//!
//! Where the numbers come from:
//! - the time values of the table of writers and their strings: the date, weekday and time of
//!   each second were printed by `date -u -d @<seconds> '+%a %b %d %Y %H %M %S %m'` (GNU
//!   coreutils), the milliseconds are the time value's remainder, and the padding of the year
//!   is that of `DateString` (four digits, `-` when negative) and of the Date Time String
//!   Format (four digits for 0 to 9999, else a sign and six digits);
//! - the time values of the parsed texts: `date -u -d <date>T<time>Z +%s` (`2020-01-01`
//!   1577836800, `2020-02-01` 1580515200, `2020-02-29` 1582934400, `2020-02-29T12:34:00`
//!   1582979640, `2020-02-29T12:34:56` 1582979696, `2020-03-01` 1583020800, `2021-03-01`
//!   1614556800), times 1,000, plus the milliseconds and minus the offset of the text;
//! - 8.64 × 10^15 is the end of the time value range (`#sec-time-values-and-time-range`).

use std::time::Instant;

use swell_num::date::date_fields;
use swell_num::date_text::{
    DateText, date_string, iso_string, parse_date_text, time_string, time_zone_string, utc_string,
};

const MINUTE: f64 = 60_000.0;
const DAY: f64 = 86_400_000.0;

/// The answer for `text`, read once as UTF-8 bytes and once as UTF-16 code units; the two
/// readings agree.
fn parse(text: &str) -> DateText {
    let bytes = parse_date_text(text.as_bytes());
    let units: Vec<u16> = text.encode_utf16().collect();
    let wide = parse_date_text(&units);
    assert_eq!(bytes, wide, "{text:?} read as bytes and as UTF-16");
    bytes
}

// ---------------------------------------------------------------------------------------------
// The Date Time String Format.

#[test]
fn the_date_time_string_format_reads_its_forms() {
    let cases: &[(&str, DateText)] = &[
        ("2020", DateText::Utc(1_577_836_800_000.0)),
        ("2020-02", DateText::Utc(1_580_515_200_000.0)),
        ("2020-02-29", DateText::Utc(1_582_934_400_000.0)),
        ("+002020-02-29", DateText::Utc(1_582_934_400_000.0)),
        ("1970-01-01", DateText::Utc(0.0)),
        ("+000000-01-01", DateText::Utc(-62_167_219_200_000.0)),
        ("0000", DateText::Utc(-62_167_219_200_000.0)),
        (
            "-000001-07-13T22:13:20Z",
            DateText::Utc(-62_182_000_000_000.0),
        ),
        ("-271821-04-20T00:00:00.000Z", DateText::Utc(-8.64e15)),
        ("2020T12:34", DateText::Local(1_577_882_040_000.0)),
        ("2020-02-29T12:34", DateText::Local(1_582_979_640_000.0)),
        ("2020-02-29T12:34:56", DateText::Local(1_582_979_696_000.0)),
        (
            "2020-02-29T12:34:56.789",
            DateText::Local(1_582_979_696_789.0),
        ),
        ("2020-02-29T12:34Z", DateText::Utc(1_582_979_640_000.0)),
        (
            "2020-02-29T12:34:56.789Z",
            DateText::Utc(1_582_979_696_789.0),
        ),
        (
            "2020-02-29T12:34:56+01:30",
            DateText::Utc(1_582_979_696_000.0 - 90.0 * MINUTE),
        ),
        (
            "2020-02-29T12:34:56-23:59",
            DateText::Utc(1_582_979_696_000.0 + 1_439.0 * MINUTE),
        ),
        (
            "2020-02-29T12:34:56.789+23:59",
            DateText::Utc(1_582_979_696_789.0 - 1_439.0 * MINUTE),
        ),
        ("2020-02-29T12:34+00:00", DateText::Utc(1_582_979_640_000.0)),
        (
            "2020-12-31T23:59:59.999Z",
            DateText::Utc(1_609_459_199_999.0),
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), *expected, "{text}");
    }
}

#[test]
fn hour_24_is_the_next_day() {
    assert_eq!(
        parse("2020-02-28T24:00"),
        DateText::Local(1_582_934_400_000.0)
    );
    assert_eq!(
        parse("2020-02-28T24:00:00"),
        DateText::Local(1_582_934_400_000.0)
    );
    assert_eq!(
        parse("2020-02-29T24:00:00.000Z"),
        DateText::Utc(1_583_020_800_000.0)
    );
    assert_eq!(
        parse("2020-12-31T24:00Z"),
        DateText::Utc(1_609_459_200_000.0)
    );
    for text in [
        "2020-02-29T24:01",
        "2020-02-29T24:00:01",
        "2020-02-29T24:00:00.001",
        "2020-02-29T24:00:00.001Z",
        "2020-02-29T25:00",
    ] {
        assert_eq!(parse(text), DateText::Invalid, "{text}");
    }
}

#[test]
fn a_date_only_form_is_utc_and_a_date_time_without_an_offset_is_local() {
    assert!(matches!(parse("2020-02-29"), DateText::Utc(_)));
    assert!(matches!(parse("2020-02"), DateText::Utc(_)));
    assert!(matches!(parse("2020"), DateText::Utc(_)));
    assert!(matches!(parse("2020-02-29T00:00"), DateText::Local(_)));
    assert!(matches!(
        parse("2020-02-29T00:00:00.000"),
        DateText::Local(_)
    ));
    assert!(matches!(parse("2020-02-29T00:00Z"), DateText::Utc(_)));
    assert!(matches!(parse("2020-02-29T00:00-01:00"), DateText::Utc(_)));
    assert!(matches!(
        parse("Sat Feb 29 2020 00:00:00 GMT+0000"),
        DateText::Utc(_)
    ));
    assert!(matches!(
        parse("Sat, 29 Feb 2020 00:00:00 GMT"),
        DateText::Utc(_)
    ));
}

#[test]
fn the_boundary_of_the_time_range_is_read_before_the_clip() {
    assert_eq!(parse("+275760-09-13T00:00:00.000Z"), DateText::Utc(8.64e15));
    assert_eq!(
        parse("+275760-09-13T01:00:00+01:00"),
        DateText::Utc(8.64e15)
    );
    assert_eq!(parse("+275760-09-13"), DateText::Utc(8.64e15));
    assert_eq!(parse("-271821-04-20"), DateText::Utc(-8.64e15));
    assert_eq!(
        parse("-271821-04-19T23:00:00-01:00"),
        DateText::Utc(-8.64e15)
    );
    // Unclipped: the caller's TimeClip makes these NaN.
    assert_eq!(
        parse("+275760-09-13T00:00:00.001Z"),
        DateText::Utc(8.64e15 + 1.0)
    );
    assert_eq!(
        parse("-271821-04-19T23:59:59.999Z"),
        DateText::Utc(-8.64e15 - 1.0)
    );
    assert_eq!(parse("+275760-09-14"), DateText::Utc(8.64e15 + DAY));
    // The largest years of each form are read; their instants lie far outside the range (above
    // 2^53, where the Number arithmetic of MakeDate rounds), and the caller's TimeClip makes
    // them NaN.
    for text in [
        "+999999-12-31T23:59:59.999Z",
        "+999999-01-01T00:00",
        "Fri, 31 Dec 999999 23:59:59 GMT",
        "Fri Dec 31 999999 23:59:59 GMT+2359 (Zone)",
    ] {
        let (DateText::Utc(t) | DateText::Local(t)) = parse(text) else {
            panic!("{text} is not read");
        };
        assert!(t.is_finite() && t > 3e16, "{text}: {t}");
    }
    for text in [
        "-999999-01-01T00:00:00.000Z",
        "-999999-01-01",
        "Mon, 01 Jan -999999 00:00:00 GMT",
        "Mon Jan 01 -999999 00:00:00 GMT-2359",
    ] {
        let (DateText::Utc(t) | DateText::Local(t)) = parse(text) else {
            panic!("{text} is not read");
        };
        assert!(t.is_finite() && t < -3e16, "{text}: {t}");
    }
}

#[test]
fn every_range_of_the_date_time_string_format_is_held() {
    // The ends of each range are read; one past each end is not.
    let accepted = [
        "2020-01-01",
        "2020-12-01",
        "2020-01-31",
        "2020-02-29",
        "2000-02-29",
        "2021-02-28",
        "2021-04-30",
        "2021-12-31",
        "2020-01-01T00:00:00.000Z",
        "2020-01-01T23:59:59.999Z",
        "2020-01-01T00:00+23:59",
        "2020-01-01T00:00-00:00",
    ];
    for text in accepted {
        assert!(matches!(parse(text), DateText::Utc(_)), "{text}");
    }
    let rejected = [
        "2020-00-01",
        "2020-13-01",
        "2020-01-00",
        "2020-01-32",
        "2021-02-29",
        "1900-02-29",
        "2021-02-30",
        "2020-02-30",
        "2021-04-31",
        "2021-06-31",
        "2021-09-31",
        "2021-11-31",
        "2020-01-01T25:00",
        "2020-01-01T23:60",
        "2020-01-01T23:59:60",
        "2020-01-01T00:00+24:00",
        "2020-01-01T00:00+23:60",
        "2020-01-01T00:00-24:00",
    ];
    for text in rejected {
        assert_eq!(parse(text), DateText::Invalid, "{text}");
    }
}

#[test]
fn every_rejection_of_the_design_is_invalid() {
    let rejected = [
        // The empty text, a lone sign, a missing element.
        "",
        "+",
        "-",
        "2",
        "202",
        "2020-",
        "2020-1",
        "2020-01-",
        "2020-01-1",
        "2020-01-01T",
        "2020-01-01T1",
        "2020-01-01T12",
        "2020-01-01T12:",
        "2020-01-01T12:3",
        "2020-01-01T12:34:",
        "2020-01-01T12:34:5",
        "2020-01-01T12:34:56.",
        "2020-01-01T12:34:56.7",
        "2020-01-01T12:34:56.78",
        "2020-01-01T12:34+",
        "2020-01-01T12:34+01",
        "2020-01-01T12:34+01:",
        "2020-01-01T12:34+01:0",
        // A year of the wrong length or sign.
        "20201",
        "+2020",
        "+02020-01-01",
        "-2020-01-01",
        "+0020201",
        "-000000",
        "-000000-01-01",
        "-000000-01-01T00:00Z",
        "-0000",
        // An offset after a date only.
        "2020-01-01Z",
        "2020-01-01+01:00",
        "2020Z",
        // Forms ECMA-262 does not have.
        "2020-01-01T12:34:56.7890",
        "2020-01-01T12:34:56.789ZZ",
        "2020-01-01T12:34:56,789",
        "2020-01-01T12:34+0100",
        "2020-01-01T12:34+01:00:00",
        "2020-01-01t12:34",
        "2020-01-01T12:34z",
        "2020-01-01 12:34",
        "2020/01/01",
        "20200101",
        // A space before or after, an extra code unit.
        " 2020-01-01",
        "2020-01-01 ",
        "2020-01-01T12:34Z ",
        "2020-01-01x",
        "2020-01-01T12:34:56.789Zx",
        "\u{0660}\u{0662}\u{0660}\u{0662}",
        // The forms of toString and toUTCString with an element out of place.
        "Sat Feb 29 2020 12:34:56 GMT+0100 ",
        "Sat Feb 29 2020 12:34:56 GMT+0100  (CET)",
        "Sat  Feb 29 2020 12:34:56 GMT+0100",
        "Sat Feb  29 2020 12:34:56 GMT+0100",
        "Sat Feb 29  2020 12:34:56 GMT+0100",
        "Sat Feb 29 2020  12:34:56 GMT+0100",
        "Sat Feb 29 2020 12:34:56  GMT+0100",
        "Sat, 29 Feb 2020 12:34:56  GMT",
        "Sat,  29 Feb 2020 12:34:56 GMT",
        "Sat , 29 Feb 2020 12:34:56 GMT",
        "Sat, 29 Feb 2020 12:34:56 GMT ",
        "Sat, 29 Feb 2020 12:34:56 GMTx",
        "Sat, 29 Feb 2020 12:34:56 GMT+0000",
        "Sat, 29 Feb 2020 12:34:56 GMT (UTC)",
        "Sat, 29 Feb 2020 12:34:56",
        "Sat, 29 Feb 2020 12:34 GMT",
        "Sat, 29 Feb 2020",
        "Sat Feb 29 2020 12:34:56 GMT",
        "Sat Feb 29 2020 12:34:56 UTC+0000",
        "Sat Feb 29 2020 12:34:56 gmt+0000",
        "Sat Feb 29 2020 12:34:56 GMT+00:00",
        "Sat Feb 29 2020 12:34:56 GMT+000",
        "Sat Feb 29 2020 12:34:56 GMT0000",
        "Sat Feb 29 2020 12:34:56",
        "Sat Feb 29 2020",
        "Sat Feb 29",
        "Sat",
        "Sa",
        "S",
        // The forms of toDateString and toTimeString.
        "Sat Feb 29 2020",
        "12:34:56 GMT+0100",
        "12:34:56 GMT+0100 (CET)",
        // A name not in the tables, or of the wrong case.
        "sat Feb 29 2020 12:34:56 GMT+0100",
        "SAT Feb 29 2020 12:34:56 GMT+0100",
        "Sat feb 29 2020 12:34:56 GMT+0100",
        "Sat FEB 29 2020 12:34:56 GMT+0100",
        "Sat Fbr 29 2020 12:34:56 GMT+0100",
        "Sax Feb 29 2020 12:34:56 GMT+0100",
        "Sat, 29 feb 2020 12:34:56 GMT",
        "Saturday, 29 Feb 2020 12:34:56 GMT",
        "Sat February 29 2020 12:34:56 GMT+0100",
        // DD, Y, HH, mm, ss and the offset out of their ranges or lengths.
        "Sat Feb 30 2020 12:34:56 GMT+0100",
        "Sun Feb 29 2021 12:34:56 GMT+0100",
        "Sat Feb 00 2020 12:34:56 GMT+0100",
        "Sat Feb 9 2020 12:34:56 GMT+0100",
        "Sat Feb 029 2020 12:34:56 GMT+0100",
        "Sat Feb 29 202 12:34:56 GMT+0100",
        "Sat Feb 29 2020202 12:34:56 GMT+0100",
        "Sat Feb 29 +2020 12:34:56 GMT+0100",
        "Sat Feb 29 -0000 12:34:56 GMT+0100",
        "Sat Feb 29 -00000 12:34:56 GMT+0100",
        "Sat Feb 29 -000000 12:34:56 GMT+0100",
        "Sat, 01 Jan -0000 00:00:00 GMT",
        "Sat Feb 29 - 2020 12:34:56 GMT+0100",
        "Sat Feb 29 2020 24:00:00 GMT+0100",
        "Sat Feb 29 2020 23:60:00 GMT+0100",
        "Sat Feb 29 2020 23:59:60 GMT+0100",
        "Sat Feb 29 2020 1:34:56 GMT+0100",
        "Sat Feb 29 2020 12:34:56.789 GMT+0100",
        "Sat Feb 29 2020 12:34:56 GMT+2400",
        "Sat Feb 29 2020 12:34:56 GMT+0160",
        "Sat Feb 29 2020 12:34:56 GMT-2400",
        "Sat Feb 29 2020 12:34:56 GMT+01000",
        "Sat, 29 Feb 2020 24:00:00 GMT",
        "Sat, 30 Feb 2020 00:00:00 GMT",
        "Sat, 29 Feb 02020202 00:00:00 GMT",
        // The name of the toString form, wrong.
        "Sat Feb 29 2020 12:34:56 GMT+0100 ()",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (CET",
        "Sat Feb 29 2020 12:34:56 GMT+0100 CET)",
        "Sat Feb 29 2020 12:34:56 GMT+0100(CET)",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (CET) ",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (CET)x",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (C(E)T)",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (CE)T)",
        "Sat Feb 29 2020 12:34:56 GMT+0100 ((CET))",
        "Sat Feb 29 2020 12:34:56 GMT+0100 (CET)(CET)",
        // A letter at the start that begins no form.
        "T12:34",
        "Z",
        "x2020-01-01",
    ];
    for text in rejected {
        assert_eq!(parse(text), DateText::Invalid, "{text:?}");
    }
}

#[test]
fn a_code_unit_outside_ascii_is_invalid_where_the_grammar_names_ascii() {
    // U+FF10 FULLWIDTH DIGIT ZERO and U+0130, whose low byte is an ASCII digit or letter, and a
    // lone surrogate.
    for units in [
        [0xFF12_u16, 0x30, 0x32, 0x30].as_slice(),
        [0x0132_u16, 0x30, 0x32, 0x30].as_slice(),
        [0x32_u16, 0x30, 0x32, 0x0130].as_slice(),
        [0xD800_u16].as_slice(),
        [0x53_u16, 0x61, 0x0174, 0x2C].as_slice(),
    ] {
        assert_eq!(parse_date_text(units), DateText::Invalid, "{units:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// The forms of toString and toUTCString.

#[test]
fn the_to_string_form_reads_its_fields_at_its_offset() {
    let cases: &[(&str, DateText)] = &[
        ("Thu Jan 01 1970 00:00:00 GMT+0000", DateText::Utc(0.0)),
        (
            "Sat Feb 29 2020 13:34:56 GMT+0100",
            DateText::Utc(1_582_979_696_000.0),
        ),
        (
            "Sat Feb 29 2020 11:04:56 GMT-0130",
            DateText::Utc(1_582_979_696_000.0),
        ),
        (
            "Sat Feb 29 2020 12:34:56 GMT-0000",
            DateText::Utc(1_582_979_696_000.0),
        ),
        (
            "Sun Mar 01 2020 12:33:56 GMT+2359",
            DateText::Utc(1_582_979_696_000.0),
        ),
        (
            "Sat Jan 01 -10000 00:00:00 GMT+0000",
            DateText::Utc(-377_736_739_200_000.0),
        ),
        (
            "Sat Feb 03 -0100 11:33:20 GMT+0000",
            DateText::Utc(-65_320_000_000_000.0),
        ),
        (
            "Sat Jan 01 0000 00:00:00 GMT+0000",
            DateText::Utc(-62_167_219_200_000.0),
        ),
        (
            "Sat Sep 13 275760 00:00:00 GMT+0000",
            DateText::Utc(8.64e15),
        ),
        (
            "Tue Apr 20 -271821 00:00:00 GMT+0000",
            DateText::Utc(-8.64e15),
        ),
        (
            "Sat Jan 01 02020 00:00:00 GMT+0000",
            DateText::Utc(1_577_836_800_000.0),
        ),
        (
            "Tue Dec 31 2024 23:59:59 GMT+0000",
            DateText::Utc(1_735_689_599_000.0),
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), *expected, "{text}");
    }
}

#[test]
fn the_to_utc_string_form_is_read_as_utc() {
    let cases: &[(&str, DateText)] = &[
        ("Thu, 01 Jan 1970 00:00:00 GMT", DateText::Utc(0.0)),
        (
            "Sat, 29 Feb 2020 12:34:56 GMT",
            DateText::Utc(1_582_979_696_000.0),
        ),
        ("Tue, 20 Apr -271821 00:00:00 GMT", DateText::Utc(-8.64e15)),
        ("Sat, 13 Sep 275760 00:00:00 GMT", DateText::Utc(8.64e15)),
        (
            "Fri, 31 Dec 9999 23:59:59 GMT",
            DateText::Utc(253_402_300_799_000.0),
        ),
        (
            "Sat, 01 Jan 10000 00:00:00 GMT",
            DateText::Utc(253_402_300_800_000.0),
        ),
        (
            "Sat, 01 Jan 0000 00:00:00 GMT",
            DateText::Utc(-62_167_219_200_000.0),
        ),
        (
            "Tue, 13 Jul -0001 22:13:20 GMT",
            DateText::Utc(-62_182_000_000_000.0),
        ),
        ("Wed, 31 Dec 1969 23:59:59 GMT", DateText::Utc(-1_000.0)),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text), *expected, "{text}");
    }
}

#[test]
fn the_weekday_is_read_and_not_checked() {
    for weekday in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"] {
        let text = format!("{weekday} Jan 01 1970 00:00:00 GMT+0000");
        assert_eq!(parse(&text), DateText::Utc(0.0), "{text}");
        let text = format!("{weekday}, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(parse(&text), DateText::Utc(0.0), "{text}");
    }
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut previous = f64::NEG_INFINITY;
    for month in months {
        let text = format!("Thu, 01 {month} 1970 00:00:00 GMT");
        let DateText::Utc(t) = parse(&text) else {
            panic!("{text} is not read");
        };
        assert!(t > previous, "{text}");
        previous = t;
    }
    assert_eq!(previous, 334.0 * DAY);
}

#[test]
fn the_name_in_parentheses_is_ignored() {
    let at = DateText::Utc(1_582_979_696_000.0);
    for text in [
        "Sat Feb 29 2020 13:34:56 GMT+0100 (Central European Standard Time)",
        "Sat Feb 29 2020 13:34:56 GMT+0100 (CET)",
        "Sat Feb 29 2020 13:34:56 GMT+0100 (x)",
        "Sat Feb 29 2020 13:34:56 GMT+0100 ( )",
        "Sat Feb 29 2020 13:34:56 GMT+0100 (Mitteleurop\u{e4}ische Normalzeit)",
        "Sat Feb 29 2020 13:34:56 GMT+0100 (\u{1F30D} GMT+0100 2020-01-01T00:00Z)",
    ] {
        assert_eq!(parse(text), at, "{text}");
    }
    // A name of any length is read in one pass.
    let mut long = String::from("Sat Feb 29 2020 13:34:56 GMT+0100 (");
    long.push_str(&"n".repeat(100_000));
    long.push(')');
    assert_eq!(parse(&long), at);
    // A lone surrogate inside the name is a code unit like any other.
    let mut units: Vec<u16> = "Sat Feb 29 2020 13:34:56 GMT+0100 ("
        .encode_utf16()
        .collect();
    units.extend([0xD800, 0x41, 0xDFFF, 0x29]);
    assert_eq!(parse_date_text(&units), at);
}

// ---------------------------------------------------------------------------------------------
// The writers.

/// A time value, then the strings of `toUTCString`, `DateString`, `TimeString` and
/// `toISOString` for it (see the module header for where they come from).
const TABLE: [(f64, &str, &str, &str, &str); 24] = [
    (
        0.0,
        "Thu, 01 Jan 1970 00:00:00 GMT",
        "Thu Jan 01 1970",
        "00:00:00",
        "1970-01-01T00:00:00.000Z",
    ),
    (
        -1.0,
        "Wed, 31 Dec 1969 23:59:59 GMT",
        "Wed Dec 31 1969",
        "23:59:59",
        "1969-12-31T23:59:59.999Z",
    ),
    (
        8_640_000_000_000_000.0,
        "Sat, 13 Sep 275760 00:00:00 GMT",
        "Sat Sep 13 275760",
        "00:00:00",
        "+275760-09-13T00:00:00.000Z",
    ),
    (
        -8_640_000_000_000_000.0,
        "Tue, 20 Apr -271821 00:00:00 GMT",
        "Tue Apr 20 -271821",
        "00:00:00",
        "-271821-04-20T00:00:00.000Z",
    ),
    (
        -62_167_219_200_000.0,
        "Sat, 01 Jan 0000 00:00:00 GMT",
        "Sat Jan 01 0000",
        "00:00:00",
        "0000-01-01T00:00:00.000Z",
    ),
    (
        -62_182_000_000_000.0,
        "Tue, 13 Jul -0001 22:13:20 GMT",
        "Tue Jul 13 -0001",
        "22:13:20",
        "-000001-07-13T22:13:20.000Z",
    ),
    (
        253_402_300_799_999.0,
        "Fri, 31 Dec 9999 23:59:59 GMT",
        "Fri Dec 31 9999",
        "23:59:59",
        "9999-12-31T23:59:59.999Z",
    ),
    (
        253_402_300_800_000.0,
        "Sat, 01 Jan 10000 00:00:00 GMT",
        "Sat Jan 01 10000",
        "00:00:00",
        "+010000-01-01T00:00:00.000Z",
    ),
    (
        951_827_696_789.0,
        "Tue, 29 Feb 2000 12:34:56 GMT",
        "Tue Feb 29 2000",
        "12:34:56",
        "2000-02-29T12:34:56.789Z",
    ),
    (
        -2_203_891_200_000.0,
        "Thu, 01 Mar 1900 00:00:00 GMT",
        "Thu Mar 01 1900",
        "00:00:00",
        "1900-03-01T00:00:00.000Z",
    ),
    (
        -14_182_940_000.0,
        "Sun, 20 Jul 1969 20:17:40 GMT",
        "Sun Jul 20 1969",
        "20:17:40",
        "1969-07-20T20:17:40.000Z",
    ),
    (
        1_735_689_599_000.0,
        "Tue, 31 Dec 2024 23:59:59 GMT",
        "Tue Dec 31 2024",
        "23:59:59",
        "2024-12-31T23:59:59.000Z",
    ),
    (
        -11_670_912_000_000.0,
        "Wed, 01 Mar 1600 00:00:00 GMT",
        "Wed Mar 01 1600",
        "00:00:00",
        "1600-03-01T00:00:00.000Z",
    ),
    (
        -62_135_596_799_999.0,
        "Mon, 01 Jan 0001 00:00:00 GMT",
        "Mon Jan 01 0001",
        "00:00:00",
        "0001-01-01T00:00:00.001Z",
    ),
    (
        8_639_999_999_999_999.0,
        "Fri, 12 Sep 275760 23:59:59 GMT",
        "Fri Sep 12 275760",
        "23:59:59",
        "+275760-09-12T23:59:59.999Z",
    ),
    (
        -8_639_999_999_999_999.0,
        "Tue, 20 Apr -271821 00:00:00 GMT",
        "Tue Apr 20 -271821",
        "00:00:00",
        "-271821-04-20T00:00:00.001Z",
    ),
    (
        999.0,
        "Thu, 01 Jan 1970 00:00:00 GMT",
        "Thu Jan 01 1970",
        "00:00:00",
        "1970-01-01T00:00:00.999Z",
    ),
    (
        2_147_483_647_000.0,
        "Tue, 19 Jan 2038 03:14:07 GMT",
        "Tue Jan 19 2038",
        "03:14:07",
        "2038-01-19T03:14:07.000Z",
    ),
    (
        1_234_567_890_123.0,
        "Fri, 13 Feb 2009 23:31:30 GMT",
        "Fri Feb 13 2009",
        "23:31:30",
        "2009-02-13T23:31:30.123Z",
    ),
    (
        -1_234_567_890_123.0,
        "Tue, 18 Nov 1930 00:28:29 GMT",
        "Tue Nov 18 1930",
        "00:28:29",
        "1930-11-18T00:28:29.877Z",
    ),
    (
        3_093_527_980_800_000.0,
        "Sat, 01 Jan 100000 00:00:00 GMT",
        "Sat Jan 01 100000",
        "00:00:00",
        "+100000-01-01T00:00:00.000Z",
    ),
    (
        -377_705_116_800_000.0,
        "Mon, 01 Jan -9999 00:00:00 GMT",
        "Mon Jan 01 -9999",
        "00:00:00",
        "-009999-01-01T00:00:00.000Z",
    ),
    (
        -65_319_999_999_123.0,
        "Sat, 03 Feb -0100 11:33:20 GMT",
        "Sat Feb 03 -0100",
        "11:33:20",
        "-000100-02-03T11:33:20.877Z",
    ),
    (
        -377_736_739_199_877.0,
        "Sat, 01 Jan -10000 00:00:00 GMT",
        "Sat Jan 01 -10000",
        "00:00:00",
        "-010000-01-01T00:00:00.123Z",
    ),
];

#[test]
fn the_writers_match_a_table_of_24_time_values() {
    for (t, utc, date, time, iso) in TABLE {
        let fields = date_fields(t).expect("a time value of the table");
        assert_eq!(utc_string(&fields), utc, "utc_string of {t}");
        assert_eq!(date_string(&fields), date, "date_string of {t}");
        assert_eq!(time_string(&fields), time, "time_string of {t}");
        assert_eq!(iso_string(&fields), iso, "iso_string of {t}");
    }
}

#[test]
fn time_zone_string_writes_the_sign_hours_and_minutes_of_the_offset() {
    let cases: [(i64, &str); 9] = [
        (0, "GMT+0000"),
        (3_600_000, "GMT+0100"),
        (-19_800_000, "GMT-0530"),
        (86_399_000, "GMT+2359"),
        (-86_399_000, "GMT-2359"),
        (59_999, "GMT+0000"),
        (-60_000, "GMT-0001"),
        (-1, "GMT-0000"),
        (20_700_000, "GMT+0545"),
    ];
    for (offset, text) in cases {
        assert_eq!(time_zone_string(offset), text, "{offset}");
    }
    // HourFromTime and MinuteFromTime of the magnitude, for any i64.
    assert_eq!(time_zone_string(i64::MIN), "GMT-0712");
    assert_eq!(time_zone_string(i64::MAX), "GMT+0712");
}

#[test]
fn each_text_written_reads_back() {
    for (t, ..) in TABLE {
        let fields = date_fields(t).expect("a time value of the table");
        let seconds = t - t.rem_euclid(1_000.0);
        assert_eq!(parse(&utc_string(&fields)), DateText::Utc(seconds), "{t}");
        let text = format!(
            "{} {} {}",
            date_string(&fields),
            time_string(&fields),
            time_zone_string(0)
        );
        assert_eq!(parse(&text), DateText::Utc(seconds), "{text}");
        assert_eq!(parse(&iso_string(&fields)), DateText::Utc(t), "{t}");
    }
}

#[test]
fn the_to_string_text_reads_back_at_every_offset_it_writes() {
    let t = 1_582_979_696_789.0;
    let seconds = 1_582_979_696_000.0;
    let text_at = |offset: i64| {
        let local = date_fields(t + offset as f64).expect("a local time");
        format!(
            "{} {} {} (Zone)",
            date_string(&local),
            time_string(&local),
            time_zone_string(offset)
        )
    };
    // A whole-minute offset reads back to the second.
    for offset in [
        -86_340_000_i64,
        -50_400_000,
        -19_800_000,
        -60_000,
        0,
        60_000,
        20_700_000,
        50_400_000,
        86_340_000,
    ] {
        let text = text_at(offset);
        assert_eq!(parse(&text), DateText::Utc(seconds), "{text}");
    }
    // The seconds of an offset are not written (`TimeZoneString` writes hours and minutes), so
    // an offset of 23:59:59 reads back 59 seconds off, toward the offset's sign.
    let text = text_at(86_399_000);
    assert_eq!(text, "Sun Mar 01 2020 12:34:55 GMT+2359 (Zone)");
    assert_eq!(parse(&text), DateText::Utc(seconds + 59_000.0), "{text}");
    let text = text_at(-86_399_000);
    assert_eq!(text, "Fri Feb 28 2020 12:34:57 GMT-2359 (Zone)");
    assert_eq!(parse(&text), DateText::Utc(seconds - 59_000.0), "{text}");
}

/// 4,000 time values spread over the whole range by the multiplicative step 2^40 + 7 (times a
/// prime, so that consecutive values differ in every field): each is written by the four
/// writers and read back.
#[test]
fn a_walk_over_4000_time_values_reads_back() {
    const STEP: i128 = (1 << 40) + 7;
    const LOW: i128 = -8_640_000_000_000_000;
    const SPAN: i128 = 17_280_000_000_000_001;
    let start = Instant::now();
    let mut walked = 0;
    for k in 0..4_000_i128 {
        let ms = LOW + (k * STEP * 1_000_003).rem_euclid(SPAN);
        let t = ms as f64;
        let fields = date_fields(t).expect("a time value in the range");
        let seconds = t - t.rem_euclid(1_000.0);
        let utc = utc_string(&fields);
        assert!(utc.len() <= 32, "{utc}");
        assert_eq!(parse(&utc), DateText::Utc(seconds), "{utc}");
        let iso = iso_string(&fields);
        assert_eq!(parse(&iso), DateText::Utc(t), "{iso}");
        let text = format!(
            "{} {} {}",
            date_string(&fields),
            time_string(&fields),
            time_zone_string(0)
        );
        assert_eq!(parse(&text), DateText::Utc(seconds), "{text}");
        walked += 1;
    }
    eprintln!(
        "date_text walk: {walked} time values in {:?}",
        start.elapsed()
    );
    assert_eq!(walked, 4_000);
}

/// A step of xorshift64 (Marsaglia), the fixed pseudo-random source of the texts below.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn random_texts_are_read_without_a_panic() {
    const ALPHABET: &[u16] = &[
        b'0' as u16,
        b'1' as u16,
        b'2' as u16,
        b'3' as u16,
        b'5' as u16,
        b'9' as u16,
        b'+' as u16,
        b'-' as u16,
        b':' as u16,
        b'.' as u16,
        b'T' as u16,
        b'Z' as u16,
        b' ' as u16,
        b',' as u16,
        b'(' as u16,
        b')' as u16,
        b'G' as u16,
        b'M' as u16,
        b'S' as u16,
        b'a' as u16,
        b't' as u16,
        b'F' as u16,
        b'e' as u16,
        b'b' as u16,
        0x00,
        0x7F,
        0xFF,
        0x0130,
        0xD800,
        0xFF10,
    ];
    let seeds = [
        "2020-02-29T12:34:56.789+01:30",
        "+275760-09-13T00:00:00.000Z",
        "-271821-04-20T00:00",
        "Sat Feb 29 2020 13:34:56 GMT+0100 (CET)",
        "Tue, 20 Apr -271821 00:00:00 GMT",
    ];
    let start = Instant::now();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let mut read = 0_u32;
    let mut valid = 0_u32;
    let mut text: Vec<u16> = Vec::with_capacity(64);
    for round in 0..60_000_u32 {
        text.clear();
        if round % 2 == 0 {
            // A text of the alphabet, 0 to 40 code units.
            let length = next(&mut state) % 41;
            for _ in 0..length {
                let index = (next(&mut state) % ALPHABET.len() as u64) as usize;
                text.push(ALPHABET[index]);
            }
        } else {
            // A form that is read, with one to three code units replaced, dropped or added.
            let seed = seeds[(next(&mut state) % seeds.len() as u64) as usize];
            text.extend(seed.encode_utf16());
            for _ in 0..1 + next(&mut state) % 3 {
                let at = (next(&mut state) % (text.len() as u64 + 1)) as usize;
                let unit = ALPHABET[(next(&mut state) % ALPHABET.len() as u64) as usize];
                match next(&mut state) % 3 {
                    0 if at < text.len() => text[at] = unit,
                    1 if at < text.len() => {
                        text.remove(at);
                    }
                    _ => text.insert(at, unit),
                }
            }
        }
        let answer = parse_date_text(&text);
        match answer {
            DateText::Invalid => {}
            DateText::Utc(t) | DateText::Local(t) => {
                assert!(t.is_finite() && t.trunc() == t, "{text:?}: {t}");
                valid += 1;
            }
        }
        if text.iter().all(|&unit| unit < 0x100) {
            let bytes: Vec<u8> = text.iter().map(|&unit| unit as u8).collect();
            assert_eq!(parse_date_text(&bytes), answer, "{text:?}");
        }
        read += 1;
    }
    eprintln!(
        "date_text random texts: {read} read, {valid} of them a time value, in {:?}",
        start.elapsed()
    );
    assert_eq!(read, 60_000);
}
