//! The tests of `swell-test262`, run against the fixture trees under `tests/fixtures/`, which are
//! written for them (no file of test262 is copied). `tree/` is test262-shaped and counted with
//! `design.md` and `features.txt`; `unlisted/` holds cases naming a feature the list lacks.

use std::error::Error;
use std::process::Command;

use swell_test262::count::{self, Case, Outcome, Run};
use swell_test262::front_matter;
use swell_test262::slices::{self, Slice};

type TestResult = Result<(), Box<dyn Error>>;

/// What one run of the binary printed.
struct Printed {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Printed {
    fn has(&self, line: &str) -> bool {
        self.stdout.lines().any(|l| l == line)
    }

    fn meta_line(&self, path: &str) -> Option<&str> {
        self.stdout
            .lines()
            .find(|l| l.split('\t').next() == Some(path))
    }

    /// The numeric fields of the lines that start with `prefix`, keyed by their second field.
    fn rows(&self, prefix: &str) -> Vec<(String, Vec<u64>)> {
        self.stdout
            .lines()
            .filter_map(|l| {
                let mut words = l.split(' ');
                if words.next() != Some(prefix) {
                    return None;
                }
                let key = words.next()?.to_string();
                let numbers = words.map(|w| w.parse().unwrap_or(u64::MAX)).collect();
                Some((key, numbers))
            })
            .collect()
    }
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn runner(args: &[&str]) -> std::io::Result<Printed> {
    let output = Command::new(env!("CARGO_BIN_EXE_swell-test262"))
        .args(args)
        .output()?;
    Ok(Printed {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn meta(tree: &str) -> std::io::Result<Printed> {
    runner(&["meta", "--test262", &fixture(tree)])
}

fn count_with(tree: &str, design: &str, features: &str) -> std::io::Result<Printed> {
    runner(&[
        "count",
        "--test262",
        &fixture(tree),
        "--design",
        &fixture(design),
        "--features",
        &fixture(features),
    ])
}

fn count_tree() -> std::io::Result<Printed> {
    count_with("tree", "design.md", "features.txt")
}

fn fixture_rules() -> Result<slices::Rules, Box<dyn Error>> {
    let design = std::fs::read_to_string(fixture("design.md"))?;
    let (rules, problems) = slices::read(&design);
    assert_eq!(problems, Vec::<String>::new());
    Ok(rules)
}

const TREE_COUNT: &str = "\
test262 built-ins/Atomics 3 1 1 1 0 1 0
test262 built-ins/Number 3 0 0 3 0 3 0
test262 built-ins/Object 3 1 0 2 0 2 0
test262 built-ins/String 1 1 0 0 0 0 0
test262 harness 1 0 0 1 0 1 0
test262 intl402 1 1 0 0 0 0 0
test262 intl402/Locale 1 1 0 0 0 0 0
test262 language/expressions 1 0 0 1 0 1 0
test262 language/expressions/yield 2 0 0 2 0 2 0
test262 language/statements/if 1 0 0 1 0 1 0
test262 language/statements/malformed 4 0 0 4 0 4 0
test262 language/types 1 0 0 1 0 1 0
test262 total 22 5 1 16 0 16 0
test262-slice 01 1 2 0 0 1 0 1 0
test262-slice 02 6 11 0 0 6 0 6 0
test262-slice 03 2 3 0 0 2 0 2 0
test262-slice 04 1 2 0 0 1 0 1 0
test262-slice 07 3 4 0 0 3 0 3 0
test262-slice 10 1 2 0 0 1 0 1 0
test262-slice 14 4 8 1 1 2 0 2 0
test262-slice 21 3 6 3 0 0 0 0 0
test262-slice x-stage3 1 2 1 0 0 0 0 0
test262-errors 4
";

#[test]
fn front_matter_reads_inline_and_block_lists() -> TestResult {
    let printed = meta("tree")?;
    assert_eq!(printed.code, Some(0), "{}", printed.stderr);
    assert_eq!(
        printed.meta_line("test/language/expressions/yield/inline-lists.js"),
        Some(
            "test/language/expressions/yield/inline-lists.js\tonlyStrict,generated\t\
             generators,json-modules\t\t\tcompareArray.js,propertyHelper.js"
        )
    );
    assert_eq!(
        printed.meta_line("test/language/expressions/yield/block-lists.js"),
        Some(
            "test/language/expressions/yield/block-lists.js\tnoStrict\tgenerators\t\t\t\
             compareArray.js,propertyHelper.js"
        )
    );

    let read = front_matter::parse(b"/*---\nflags: [ a ,b]\nfeatures:\n  - x\n  -y \n---*/\n");
    assert_eq!(read.problem, None);
    assert_eq!(read.meta.flags, "a,b");
    assert_eq!(read.meta.features, "x,y");
    assert_eq!(read.meta.flag_list().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(read.meta.feature_list().collect::<Vec<_>>(), ["x", "y"]);
    Ok(())
}

#[test]
fn front_matter_reads_negative_phase_and_type() -> TestResult {
    let printed = meta("tree")?;
    assert_eq!(printed.code, Some(0), "{}", printed.stderr);
    assert_eq!(
        printed.meta_line("test/built-ins/Object/negative-parse.js"),
        Some("test/built-ins/Object/negative-parse.js\tmodule\t\tparse\tSyntaxError\t")
    );
    assert_eq!(
        printed.meta_line("test/built-ins/Object/negative-runtime.js"),
        Some("test/built-ins/Object/negative-runtime.js\t\t\truntime\tTest262Error\t")
    );
    Ok(())
}

#[test]
fn malformed_front_matter_is_a_counted_error() -> TestResult {
    let printed = count_tree()?;
    assert_eq!(printed.code, Some(0), "{}", printed.stderr);
    assert!(printed.has("test262-errors 4"), "{}", printed.stdout);
    assert!(printed.has("test262 language/statements/malformed 4 0 0 4 0 4 0"));
    let errors: Vec<&str> = printed
        .stderr
        .lines()
        .filter(|l| l.starts_with("test262-error "))
        .collect();
    assert_eq!(errors.len(), 4, "{}", printed.stderr);
    for (path, reason) in [
        ("no-block.js", "no /*--- block"),
        ("unterminated.js", "a /*--- block without ---*/"),
        ("unclosed-list.js", "an inline features list without ]"),
        (
            "negative-without-type.js",
            "a negative without both phase and type",
        ),
    ] {
        let line = format!("test262-error test/language/statements/malformed/{path}: {reason}");
        assert!(errors.contains(&line.as_str()), "{line} not in {errors:?}");
    }

    for text in [
        &b""[..],
        b"/*---",
        b"/*---\nnegative:\n",
        b"/*---\nincludes: [a.js\n---*/",
        b"\xff\xfe/*---\nflags:\n  -\n---*/",
        b"---*/\n/*---\n:\n-\n---*/",
    ] {
        assert!(front_matter::parse(text).problem.is_some() || text.ends_with(b"---*/"));
    }
    assert_eq!(
        front_matter::parse(b"").problem.map(|p| p.to_string()),
        Some("no /*--- block".to_string())
    );
    Ok(())
}

#[test]
fn fixture_files_are_not_cases() -> TestResult {
    let printed = meta("tree")?;
    assert_eq!(printed.code, Some(0), "{}", printed.stderr);
    assert_eq!(printed.stdout.lines().count(), 22);
    assert!(printed.stdout.lines().all(|l| l.starts_with("test/")));
    assert!(!printed.stdout.contains("_FIXTURE"));
    assert!(!printed.stdout.contains(".json"));
    assert!(!printed.stdout.contains("harness/assert.js"));
    let counted = count_tree()?;
    assert!(
        counted.has("test262 total 22 5 1 16 0 16 0"),
        "{}",
        counted.stdout
    );
    Ok(())
}

#[test]
fn directory_is_second_level_and_third_under_expressions_and_statements() -> TestResult {
    assert_eq!(
        count::directory("test/language/expressions/yield/inline-lists.js"),
        "language/expressions/yield"
    );
    assert_eq!(
        count::directory("test/language/statements/if/plain.js"),
        "language/statements/if"
    );
    assert_eq!(
        count::directory("test/language/types/number/plain.js"),
        "language/types"
    );
    assert_eq!(
        count::directory("test/built-ins/Number/prototype/plain.js"),
        "built-ins/Number"
    );
    let printed = count_tree()?;
    for line in [
        "test262 language/expressions/yield 2 0 0 2 0 2 0",
        "test262 language/statements/if 1 0 0 1 0 1 0",
        "test262 language/types 1 0 0 1 0 1 0",
        "test262 built-ins/Number 3 0 0 3 0 3 0",
    ] {
        assert!(printed.has(line), "{line} not in\n{}", printed.stdout);
    }
    let directories: Vec<String> = printed
        .rows("test262")
        .into_iter()
        .map(|(d, _)| d)
        .filter(|d| d != "total")
        .collect();
    let mut sorted = directories.clone();
    sorted.sort();
    assert_eq!(directories, sorted);
    Ok(())
}

#[test]
fn a_shallow_case_keeps_its_own_directory() -> TestResult {
    assert_eq!(count::directory("test/harness/fixture-check.js"), "harness");
    assert_eq!(count::directory("test/intl402/shallow.js"), "intl402");
    assert_eq!(
        count::directory("test/language/expressions/shallow.js"),
        "language/expressions"
    );
    assert_eq!(
        count::directory("test/language/statements/shallow.js"),
        "language/statements"
    );
    let printed = count_tree()?;
    for line in [
        "test262 harness 1 0 0 1 0 1 0",
        "test262 intl402 1 1 0 0 0 0 0",
        "test262 language/expressions 1 0 0 1 0 1 0",
    ] {
        assert!(printed.has(line), "{line} not in\n{}", printed.stdout);
    }
    Ok(())
}

#[test]
fn slice_is_the_highest_of_prefix_features_and_flags() -> TestResult {
    let rules = fixture_rules()?;
    let slice = |path: &str, features: &[&str], flags: &[&str]| {
        rules
            .slice_of(path, features.iter().copied(), flags.iter().copied())
            .to_string()
    };
    assert_eq!(slice("test/built-ins/Object/x.js", &[], &[]), "03");
    assert_eq!(
        slice("test/built-ins/Number/prototype/plain.js", &[], &[]),
        "04"
    );
    assert_eq!(
        slice(
            "test/built-ins/Number/flag-wins.js",
            &["generators"],
            &["async"]
        ),
        "10"
    );
    assert_eq!(
        slice(
            "test/built-ins/Number/feature-wins.js",
            &["generators", "Atomics"],
            &["async"]
        ),
        "14"
    );
    assert_eq!(
        slice("test/built-ins/Atomics/x.js", &["generators"], &["async"]),
        "14"
    );
    assert_eq!(slice("test/annexB/x.js", &[], &[]), "00");

    let printed = count_tree()?;
    let lines: Vec<&str> = printed
        .stdout
        .lines()
        .filter(|l| l.starts_with("test262-slice "))
        .collect();
    let expected: Vec<&str> = TREE_COUNT
        .lines()
        .filter(|l| l.starts_with("test262-slice "))
        .collect();
    assert_eq!(lines, expected);
    Ok(())
}

#[test]
fn a_case_with_an_excluded_feature_counts_as_excluded() -> TestResult {
    let rules = fixture_rules()?;
    let slice = rules.slice_of(
        "test/built-ins/Object/decorated.js",
        ["generators", "decorators"].into_iter(),
        std::iter::empty(),
    );
    assert_eq!(slice, Slice::Excluded("stage3".to_string()));
    assert_eq!(slice.to_string(), "x-stage3");
    let printed = count_tree()?;
    assert!(
        printed.has("test262 built-ins/Object 3 1 0 2 0 2 0"),
        "{}",
        printed.stdout
    );
    assert!(printed.has("test262-slice x-stage3 1 2 1 0 0 0 0 0"));
    Ok(())
}

#[test]
fn an_exclude_slice_line_excludes_every_case_of_its_slice() -> TestResult {
    let printed = count_tree()?;
    for line in [
        "test262-slice 21 3 6 3 0 0 0 0 0",
        "test262 intl402 1 1 0 0 0 0 0",
        "test262 intl402/Locale 1 1 0 0 0 0 0",
        "test262 built-ins/String 1 1 0 0 0 0 0",
    ] {
        assert!(printed.has(line), "{line} not in\n{}", printed.stdout);
    }
    Ok(())
}

#[test]
fn an_exclude_include_line_excludes_a_case() -> TestResult {
    let rules = fixture_rules()?;
    assert!(rules.excludes_include(["compareArray.js", "agentHelper.js"].into_iter()));
    assert!(!rules.excludes_include(["compareArray.js"].into_iter()));
    let printed = count_tree()?;
    // agent.js is excluded, no-agent.js (the same case without the include) is run, and
    // can-block.js is not applicable.
    assert!(
        printed.has("test262 built-ins/Atomics 3 1 1 1 0 1 0"),
        "{}",
        printed.stdout
    );
    assert!(printed.has("test262-slice 14 4 8 1 1 2 0 2 0"));
    Ok(())
}

#[test]
fn can_block_is_true_counts_as_not_applicable() -> TestResult {
    let printed = count_tree()?;
    assert!(
        printed.has("test262 built-ins/Atomics 3 1 1 1 0 1 0"),
        "{}",
        printed.stdout
    );
    assert!(printed.has("test262 total 22 5 1 16 0 16 0"));
    assert!(printed.has("test262-slice 14 4 8 1 1 2 0 2 0"));
    Ok(())
}

#[test]
fn a_case_naming_an_unlisted_feature_fails_the_run() -> TestResult {
    let printed = count_with("unlisted", "design.md", "features.txt")?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    assert!(
        printed.has("test262-unlisted-feature not-a-listed-feature 2"),
        "{}",
        printed.stdout
    );
    assert_eq!(
        printed
            .stdout
            .lines()
            .filter(|l| l.starts_with("test262-unlisted-feature "))
            .count(),
        1
    );
    assert!(
        printed.has("test262 total 3 0 0 3 0 3 0"),
        "{}",
        printed.stdout
    );
    let listed = count_tree()?;
    assert_eq!(listed.code, Some(0), "{}{}", listed.stdout, listed.stderr);
    assert!(!listed.stdout.contains("test262-unlisted-feature"));
    Ok(())
}

#[test]
fn features_file_must_list_exactly_the_declared_features() -> TestResult {
    let printed = count_with("tree", "design.md", "features-mismatch.txt")?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    for line in [
        "features.txt: json-modules declared by test262, not listed",
        "features.txt: not-declared listed, not declared by test262",
        "features.txt: decorators is run, but the swell-slices block excludes it",
    ] {
        assert!(printed.has(line), "{line} not in\n{}", printed.stdout);
    }
    assert!(
        printed.has("test262 total 22 4 1 17 0 17 0"),
        "{}",
        printed.stdout
    );
    let listed = count_tree()?;
    assert_eq!(listed.code, Some(0), "{}{}", listed.stdout, listed.stderr);
    assert!(!listed.stdout.contains("features.txt:"));
    Ok(())
}

#[test]
fn directory_lines_sum_to_the_total_line() -> TestResult {
    let printed = count_tree()?;
    assert_eq!(printed.stdout, TREE_COUNT);
    let rows = printed.rows("test262");
    let mut sum = vec![0u64; 7];
    let mut total = Vec::new();
    for (directory, numbers) in rows {
        assert_eq!(numbers.len(), 7, "{directory}");
        if directory == "total" {
            total = numbers;
        } else {
            for (s, n) in sum.iter_mut().zip(numbers) {
                *s += n;
            }
        }
    }
    assert_eq!(sum, total);
    let cases: u64 = printed
        .rows("test262-slice")
        .iter()
        .map(|(_, n)| n[0])
        .sum();
    assert_eq!(cases, total[0]);
    Ok(())
}

#[test]
fn every_run_case_is_counted_failed() -> TestResult {
    let case = Case {
        path: "test/harness/fixture-check.js".to_string(),
        meta: front_matter::parse(b"/*---\n---*/\n").meta,
    };
    for run in [Run::NonStrict, Run::Strict, Run::Module, Run::Raw] {
        assert_eq!(count::run_outcome(&case, run), Outcome::Failed);
    }
    let printed = count_tree()?;
    let rows = printed.rows("test262");
    assert!(!rows.is_empty());
    for (directory, n) in rows {
        // exist, excluded, not-applicable, run, passed, failed, budget
        assert_eq!((n[4], n[5], n[6]), (0, n[3], 0), "{directory}");
    }
    let slices = printed.rows("test262-slice");
    assert!(!slices.is_empty());
    for (slice, n) in slices {
        // cases, runs, excluded, not-applicable, run, passed, failed, budget
        assert_eq!((n[5], n[6], n[7]), (0, n[4], 0), "{slice}");
    }
    Ok(())
}

#[test]
fn an_unknown_rule_kind_fails_the_run() -> TestResult {
    let printed = count_with("tree", "design-unknown-kind.md", "features.txt")?;
    assert_eq!(
        printed.code,
        Some(1),
        "{}{}",
        printed.stdout,
        printed.stderr
    );
    assert!(
        printed.has("swell-slices: unknown kind directory: directory test/built-ins/Number/ 4"),
        "{}",
        printed.stdout
    );
    // Without the exclude-slice and exclude-include lines only decorated.js is excluded.
    assert!(
        printed.has("test262 total 22 1 1 20 0 20 0"),
        "{}",
        printed.stdout
    );
    Ok(())
}

#[test]
fn a_case_with_cr_line_ends_is_read() -> TestResult {
    // line-ends/ holds one case three times: its lines end in LF, in CR LF and in CR alone.
    let printed = meta("line-ends")?;
    assert_eq!(printed.code, Some(0), "{}", printed.stderr);
    assert_eq!(printed.stderr, "", "no case of line-ends/ is an error");
    let fields = "\tnoStrict\tgenerators\tparse\tSyntaxError\tcompareArray.js";
    for name in ["lf", "crlf", "cr"] {
        let path = format!("test/language/line-ends/{name}.js");
        assert_eq!(
            printed.meta_line(&path),
            Some(format!("{path}{fields}").as_str()),
            "{}",
            printed.stdout
        );
    }
    assert_eq!(printed.stdout.lines().count(), 3);

    let lf = b"/*---\nflags: [onlyStrict]\nincludes:\n  - a.js\n---*/\n";
    let crlf = b"/*---\r\nflags: [onlyStrict]\r\nincludes:\r\n  - a.js\r\n---*/\r\n";
    let cr = b"/*---\rflags: [onlyStrict]\rincludes:\r  - a.js\r---*/\r";
    let read = front_matter::parse(lf);
    assert_eq!(read.problem, None);
    assert_eq!(read.meta.flags, "onlyStrict");
    assert_eq!(read.meta.includes, "a.js");
    assert_eq!(front_matter::parse(crlf), read);
    assert_eq!(front_matter::parse(cr), read);

    // A block that does not end is still one, whatever its line ends.
    for text in [&b"/*---\rflags: [a]\r"[..], b"/*---\r\nflags: [a]\r\n"] {
        assert_eq!(
            front_matter::parse(text).problem,
            Some(front_matter::Problem::Unterminated)
        );
    }
    Ok(())
}
