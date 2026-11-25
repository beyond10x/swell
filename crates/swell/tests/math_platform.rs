//! HOST-135 (`docs/design/host-interface.md`): a script run twice with the same inputs prints the
//! same bytes, `Math` results and Number to String conversions included, on every release
//! platform. One script calls the 22 functions of `Math` whose answer IEEE 754 does not fix
//! (`#sec-function-properties-of-the-math-object`: `acos` to `tanh`, `hypot` and `pow`) and
//! `max`, `min`, `round`, `fround` and `sqrt` over a table of 60 arguments; each result is
//! written as Number::toString (`#sec-numeric-types-number-tostring`, radix 10) beside the 16 hex
//! digits of its bits, and the whole is compared byte for byte with
//! `tests/fixtures/host_135_math.txt`. The file was made on the platform of the local gate
//! (x86_64 Linux) and is held there and on the forge's (`ubuntu-latest`, x86_64); a platform
//! added later gets the same check. A line a platform differs on is a defect of `swell-num`: the
//! test prints each such line with its number. HOST-133: the table run twice in one agent and
//! once in a second agent gives the same output.
//!
//! With the feature `collect-every-safe-point` two rows of the table run once (54 calls: the row
//! of `hypot(3, 4)` and that of the first drawn argument), compared with the same lines of the
//! file: there every instruction collects, and six rows run three times took 6.8 CPU seconds.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, get, realm};
use support::script::eval;
use swell::{Agent, Outcome, Realm, Value};

/// The functions the script calls, in the order of the file, each with whether it takes a
/// second argument: the argument of the next row of the table (the row after the last is the
/// first).
const FUNCTIONS: [(&str, bool); 27] = [
    ("acos", false),
    ("acosh", false),
    ("asin", false),
    ("asinh", false),
    ("atan", false),
    ("atan2", true),
    ("atanh", false),
    ("cbrt", false),
    ("cos", false),
    ("cosh", false),
    ("exp", false),
    ("expm1", false),
    ("hypot", true),
    ("log", false),
    ("log10", false),
    ("log1p", false),
    ("log2", false),
    ("pow", true),
    ("sin", false),
    ("sinh", false),
    ("tan", false),
    ("tanh", false),
    ("max", true),
    ("min", true),
    ("round", false),
    ("fround", false),
    ("sqrt", false),
];

/// Whether every safe point collects: then the rows of `COLLECT_ROWS` run once.
const COLLECT: bool = cfg!(feature = "collect-every-safe-point");

/// The rows that run with a collection at every safe point: `hypot(3, 4)` and the first drawn
/// argument.
const COLLECT_ROWS: [usize; 2] = [7, 40];

/// Whether `row` of the table runs.
fn runs(row: usize) -> bool {
    !COLLECT || COLLECT_ROWS.contains(&row)
}

/// The first 40 arguments: the special values; the arguments of the exact cases of test262
/// (`docs/design/engine.md`, decision 6, "test262, pinned"), placed so that each pair of them is
/// a row and the next (`hypot(3, 4)`, `pow(1, -2^31)`, `pow(2, 10)`, `pow(-8, 1/3)`); the domain
/// edges of the functions; the powers of two (2^-20, 2^20, 2^52, 2^-1022 and 2^1023 after those
/// above); a subnormal and the largest Number.
const FIXED: [f64; 40] = [
    f64::NAN,
    0.0,
    -0.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    1.0,
    -2_147_483_648.0,
    3.0,
    4.0,
    2.0,
    10.0,
    100.0,
    1000.0,
    0.5,
    -0.5,
    -1.0,
    -2.0,
    8.0,
    -8.0,
    1.0 / 3.0,
    1.000_000_000_000_000_2,
    -1.000_000_000_000_000_2,
    0.999_999_999_999_999_9,
    1.1,
    -1.1,
    22.0,
    -22.0,
    710.0,
    -745.0,
    std::f64::consts::PI,
    std::f64::consts::FRAC_PI_2,
    1e22,
    9.536_743_164_062_5e-7,
    1_048_576.0,
    4_503_599_627_370_496.0,
    f64::MIN_POSITIVE,
    f64::from_bits(0x7fe0_0000_0000_0000),
    5e-324,
    f64::MAX,
    f64::MIN,
];

/// 20 arguments drawn by Knuth's 64-bit linear congruential generator (multiplier
/// 6364136223846793005, increment 1442695040888963407, modulus 2^64, seed 1; the design,
/// decision 6, "The tool that makes the lines", item 5), two draws an argument: the first
/// gives the sign (its top bit) and a binary exponent from -20 to 20 (its next 11 bits modulo
/// 41), the second the 52 mantissa bits (its top 52).
fn drawn() -> [f64; 20] {
    let mut state: u64 = 1;
    let mut next = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    std::array::from_fn(|_| {
        let high = next();
        let low = next();
        let sign = high & (1 << 63);
        let biased = (high >> 52 & 0x7ff) % 41 + 1003;
        f64::from_bits(sign | biased << 52 | low >> 12)
    })
}

/// The 60 arguments of the table.
fn arguments() -> Vec<f64> {
    let all: Vec<f64> = FIXED.into_iter().chain(drawn()).collect();
    assert_eq!(all.len(), 60);
    all
}

/// `x` as a literal of the script: the shortest digits that name it, with -0, the infinities
/// and NaN spelled out.
fn literal(x: f64) -> String {
    if x.is_nan() {
        "NaN".to_string()
    } else if x == 0.0 && x.is_sign_negative() {
        "-0".to_string()
    } else if x == f64::INFINITY {
        "Infinity".to_string()
    } else if x == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        swell_num::to_string::number_to_string(x)
    }
}

/// The script: every function of `FUNCTIONS` over the rows of the table that run, each result
/// pushed in that order onto the array the script evaluates to.
fn script(arguments: &[f64]) -> String {
    let a: Vec<String> = arguments.iter().map(|&x| literal(x)).collect();
    let rows: Vec<String> = (0..arguments.len())
        .filter(|&row| runs(row))
        .map(|row| row.to_string())
        .collect();
    let f: Vec<String> = FUNCTIONS
        .iter()
        .map(|(name, _)| format!("Math.{name}"))
        .collect();
    let pair: Vec<&str> = FUNCTIONS
        .iter()
        .map(|&(_, two)| if two { "true" } else { "false" })
        .collect();
    format!(
        "var a = [{}];\n\
         var rows = [{}];\n\
         var f = [{}];\n\
         var pair = [{}];\n\
         var r = [];\n\
         for (var i = 0; i < f.length; i++) {{\n\
           for (var k = 0; k < rows.length; k++) {{\n\
             var j = rows[k];\n\
             r.push(pair[i] ? f[i](a[j], a[(j + 1) % a.length]) : f[i](a[j]));\n\
           }}\n\
         }}\n\
         r;",
        a.join(", "),
        rows.join(", "),
        f.join(", "),
        pair.join(", ")
    )
}

/// The bits a line gives for `x`: a NaN as 7ff8000000000000, since a NaN's payload is not
/// observable by a script.
fn bits(x: f64) -> u64 {
    if x.is_nan() {
        f64::NAN.to_bits()
    } else {
        x.to_bits()
    }
}

/// The two lines that start the file.
const HEADER: [&str; 2] = [
    "# HOST-135: Math over a fixed table; one line per call: the function, the bits of its argument or arguments, Number::toString of the result and its bits (a NaN as 7ff8000000000000).",
    "# Written by crates/swell/tests/math_platform.rs and compared with it byte for byte; a line that differs on a platform is a defect of swell-num.",
];

/// The data lines of the whole table, by function and row, from the results of the rows that
/// ran (`None` for a row that did not).
fn lines(arguments: &[f64], results: &[f64]) -> Vec<Option<String>> {
    let mut results = results.iter();
    let mut lines = Vec::new();
    for (name, two) in FUNCTIONS {
        for (row, &x) in arguments.iter().enumerate() {
            if !runs(row) {
                lines.push(None);
                continue;
            }
            let args = if two {
                let y = arguments[(row + 1) % arguments.len()];
                format!("{:016x}, {:016x}", bits(x), bits(y))
            } else {
                format!("{:016x}", bits(x))
            };
            let result = *results.next().expect("a result for every call");
            let text = swell_num::to_string::number_to_string(result);
            lines.push(Some(format!("{name}({args}) {text} {:016x}", bits(result))));
        }
    }
    assert!(results.next().is_none(), "a result for no call");
    lines
}

/// Run the script in `realm` and read the Numbers of the array it evaluates to.
fn run(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Vec<f64> {
    let Outcome::Normal(root) = eval(agent, realm, source) else {
        panic!("the script of HOST-135 did not complete normally");
    };
    agent
        .with_cx(realm, |cx| {
            let Value::Object(array) = cx.get(&root).unwrap() else {
                panic!("the script of HOST-135 answered no array");
            };
            let Value::Number(length) = get(cx, array, "length") else {
                panic!("an array with no length");
            };
            (0..length as u32)
                .map(|index| match get(cx, array, &index.to_string()) {
                    Value::Number(number) => number,
                    other => panic!("result {index} is {other:?}"),
                })
                .collect()
        })
        .unwrap()
}

/// HOST-135 and HOST-133, as the module says: the output of the table equals the committed file
/// to the byte, and three runs, two in one agent and one in another, give the same results
/// (with a collection at every safe point: two rows, one run, the same lines of the file).
#[test]
fn host_135_a_script_prints_the_same_bytes_on_every_platform() {
    let arguments = arguments();
    let source = script(&arguments);
    let mut first = agent().unwrap();
    let first_realm = realm(&mut first);
    let once = run(&mut first, &first_realm, &source);
    let rows = (0..arguments.len()).filter(|&row| runs(row)).count();
    assert_eq!(once.len(), FUNCTIONS.len() * rows);
    if !COLLECT {
        let twice = run(&mut first, &first_realm, &source);
        let mut second = agent().unwrap();
        let second_realm = realm(&mut second);
        let other = run(&mut second, &second_realm, &source);
        let shown = |results: &[f64]| -> Vec<u64> { results.iter().map(|&x| bits(x)).collect() };
        assert_eq!(shown(&once), shown(&twice), "two runs in one agent");
        assert_eq!(shown(&once), shown(&other), "runs in two agents");
    }

    let produced = lines(&arguments, &once);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/host_135_math.txt"
    );
    let fixture = std::fs::read_to_string(path).unwrap_or_default();
    let mut expected = fixture.lines();
    let mut differ = 0;
    for (number, want) in HEADER.iter().enumerate() {
        let got = expected.next();
        if got != Some(want) {
            differ += 1;
            println!("host-135 line {}: {want}", number + 1);
        }
    }
    let mut count = HEADER.len();
    for line in &produced {
        count += 1;
        let want = expected.next();
        if let Some(got) = line
            && want != Some(got.as_str())
        {
            differ += 1;
            println!("host-135 line {count}: {got}");
        }
    }
    assert!(
        expected.next().is_none(),
        "the file has more lines than the table"
    );
    assert_eq!(
        differ, 0,
        "{differ} line(s) of {path} differ on this platform (printed above as this platform \
         gives them)"
    );
    if !COLLECT {
        let mut whole: Vec<&str> = HEADER.to_vec();
        whole.extend(produced.iter().flatten().map(String::as_str));
        assert_eq!(fixture, whole.join("\n") + "\n", "the file to the byte");
    }
}
