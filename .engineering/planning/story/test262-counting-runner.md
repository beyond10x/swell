---
format: aep.planning-md/3
id: story:test262-counting-runner
kind: story
status: implemented
title: swell-test262 counts all 53,801 cases per directory and per slice, with no engine call
summary: 'A std-only runner with no dependency and features.txt (165 run, 34 exclude), reading exclude-slice and exclude-include: 221 directory lines, test262 total 53801 8982 7 44812 0 44812 0, 22 slice lines equal to the design, an unlisted feature fails the run.'
relations:
- decomposes: epic:foundation
- depends_on: story:workspace-and-gate
- depends_on: story:test262-fetch
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: conformance/test262/features.txt
- confidence: cited
  path: crates/test262
- confidence: cited
  path: taskfiles/test262.yml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:37:18Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Every slice of the design ends with "the gate green and the expected file raised" (section 12).
This story builds `swell-test262` (decision 11, std only), which reads every case of the pinned
test262, assigns its directory and its slice, counts it as excluded, not applicable or run, and
prints the counts; and `conformance/test262/features.txt` (decision 1), which records every
test262 feature as `run` or `exclude`. The engine runs nothing in this epic and the runner does
not call it: one function of the `count` module gives the outcome of a run, and here it marks
every run as failed. That function is the one place slice 1 changes, replacing it with the call
the specification states (a fresh agent and realm per run, the harness files, `$262`, `print`,
`negative`, `async`, modules). Nothing else of the runner calls or knows the engine, and the
crate has no dependency.

What is read (the pinned checkout is `.cache/test262/<commit>/`, the commit from
`conformance/test262/PIN` of `story:test262-fetch`):

- A case is a file `test/**/*.js` that is not `*_FIXTURE.js` (INTERPRETING.md, "Modules").
  Measured in the checkout: `find test -type f -name '*.js' | wc -l` 54095;
  `find test -type f -name '*.js' ! -name '*_FIXTURE.js' | wc -l` 53801;
  `find test -name '*_FIXTURE.js' | wc -l` 294.
- The front matter, the `/*---` ... `---*/` block, as `meta.awk` of the design ("How the counts
  were made") reads it: `flags`, `features`, `includes` as an inline list `[a, b]` or a block list
  of `- a` lines, and `negative` with `phase` and `type`. Every case has the block:
  `tr '\n' '\0' < files.txt | xargs -0 grep -L -- '/\*---' | wc -l` prints 0. Block lists:
  `xargs -0 grep -l '^flags:[[:space:]]*$'` 181 files, the same for `features:` 197,
  `includes:` 0. `module` is a flag of 843 cases by `meta.awk` and of 832 by
  `grep -l -E '^flags:.*\bmodule\b'` (the other 11 write a block list; the design says the same).
  Negatives: `awk -F'\t' '$4!="" {print $4, ($4=="runtime"?"":$5)}' meta.tsv | sort | uniq -c`
  prints 4680 `parse SyntaxError`, 34 `resolution SyntaxError`, 40 `runtime`.
- `meta.awk`, run over the checkout as the "Inventory" of the design runs it, gives 53801 lines
  whose `LC_ALL=C sort | sha256sum` is
  `c39e4bbe96533198762babfe6e13f04e27b76629fe505dbb4db423204836684a`, the same as the sorted
  `.cache/inventory/meta.tsv`. The reader of the runner is held to that output byte for byte.
- The slice rules: the block `swell-slices` of `docs/design/engine.md`, read at run time.
  Extracted with the command of the design
  (`awk '$0 == "```swell-slices" {f = 1; next} /^```$/ {f = 0} f'`) it has 253 lines
  (`awk '{print $1}' | sort | uniq -c`): 86 `dir`, 143 `feature`, 4 `flag`, 17 `exclude`, 2
  `exclude-slice` (`exclude-slice 20 temporal`, `exclude-slice 21 intl402`) and 1
  `exclude-include` (`exclude-include atomicsHelper.js agents`). The slice of a case is the
  highest among its longest matching `dir` prefix, its features and its flags; a case with a
  feature an `exclude` line names gets `x-<reason>` (`slices.awk`, which ignores the two new
  kinds). Decision 1, "Two exclusions no feature name carries": a case of a slice an
  `exclude-slice` line names, or whose `includes:` names the file of an `exclude-include` line,
  is excluded in its directory and its slice. `atomicsHelper.js` is included by 112 cases
  (`awk -F'\t' '$6 ~ /(^|,)atomicsHelper\.js(,|$)/' meta.tsv | wc -l`), the same 112 that use
  `$262.agent` (`grep -rl '\$262\.agent' test | xargs grep -L 'atomicsHelper.js' | wc -l` prints 0).
- Features: the `features.txt` of the checkout declares 199 names (`grep -vE '^\s*(#|$)'
  features.txt | awk '{print $1}' | sort -u | wc -l`); cases use 198 (the third column of
  `meta.tsv`, split at commas, `sort -u`); `export-defer` is declared and used by none. 18673
  cases name no feature. The numbers of the design match.

Settled here:

- **`features.txt`.** 199 lines, one per declared feature, sorted bytewise, no comments: `run
  <name>`, or `exclude <name> <reason> decision-1` for the features the slice block puts in an
  excluded slice (slice 20 `temporal`, slice 21 `intl402`) or excludes itself (`stage3`). The 34
  `exclude` lines are printed by
  `awk '($1 == "feature" && ($3 == 20 || $3 == 21)) || $1 == "exclude" {print $2}' rules.txt`:
  reason `temporal` for `Temporal`, `canonical-tz`; `intl402` for the 15 slice-21 features
  (`Intl-enumeration`, `intl-normative-optional`, `Intl.DateTimeFormat-datetimestyle`,
  `Intl.DateTimeFormat-dayPeriod`, `Intl.DateTimeFormat-extend-timezonename`,
  `Intl.DateTimeFormat-formatRange`, `Intl.DateTimeFormat-fractionalSecondDigits`,
  `Intl.DisplayNames`, `Intl.DisplayNames-v2`, `Intl.DurationFormat`, `Intl.ListFormat`,
  `Intl.Locale`, `Intl.NumberFormat-unified`, `Intl.RelativeTimeFormat`, `Intl.Segmenter`);
  `stage3` for the 17 of decision 1. The other 165 are `run`. A case of slices 20 and 21 is
  excluded by its slice either way; the lines keep the exclusion of each feature recorded by
  name, as decision 1 asks.
- **Counting a case.** Excluded when it names an `exclude` feature, belongs to an
  `exclude-slice` slice or includes an `exclude-include` file; otherwise not applicable when it
  has the flag `CanBlockIsTrue` (decision 1: the [[CanBlock]] of the agent is false; 7 cases,
  `CanBlockIsFalse` 2, counted with `awk -F'\t'` on the flags column); otherwise run. A run
  case has 1 run with `onlyStrict`, `noStrict`, `module` or `raw`, else 2. The outcome of each
  run comes from the one function above, which answers `failed`. A case passes when every run
  passes, so every run case is failed. `budget` is 0. The columns count cases; only the slice
  line also counts runs.
- **The directory of a case** (decision 11: the second level, the third under
  `language/expressions` and `language/statements`): the path below `test/` cut to two parts,
  three under those two; a case whose directory is shallower keeps its own directory. That
  happens to `harness` (117 cases), `intl402` (22), `staging` (4) and `language/expressions` (1,
  `test/language/expressions/tco-pos.js`), measured with `awk -F/ 'NF==3' files.txt` and the
  same for `NF==4` under the two directories. 221 directory lines.
- **The slice line** (the format this epic defines; the coordinator writes it into decision 11
  when merging): `test262-slice <slice> <cases> <runs> <excluded> <not-applicable> <run>
  <passed> <failed> <budget>`, `<slice>` as `slices.awk` prints it (`01` to `21`, `x-stage3`),
  `<runs>` the runs the cases would have, excluded or not. Its fields 2 to 4 are the lines of
  "Its output on this document" in the design, so the two can be compared by a program.
- **The order of the output:** directory lines in byte order, the `test262 total` line, slice
  lines in byte order, then `test262-errors <n>`. A case whose front matter cannot be read (no
  block, an unterminated block, an inline list without `]`, a `negative` without both `phase` and
  `type`) counts as existing, run and failed, is named on standard error as
  `test262-error <path>: <reason>`, and is counted in `test262-errors`. No input panics.

The expected lines, measured from `.cache/inventory/meta.tsv` and `slices.awk`, the 253 rule
lines extracted from the design into `rules.txt`, the 34 features above in `xf.txt`, and this
program:

```awk
# Usage: awk -f slices.awk rules.txt meta.tsv | paste - meta.tsv | awk -f count2.awk xf.txt rules.txt -
BEGIN { FS = "\t" }
FILENAME == ARGV[1] { ex[$1] = 1; next }
FILENAME == ARGV[2] { split($0, w, " ")
  if (w[1] == "exclude-slice") exs[w[2]] = 1; else if (w[1] == "exclude-include") exi[w[2]] = 1; next }
{
  s = $1; path = $2; flags = $3; r = (flags ~ /onlyStrict|noStrict|module|raw/) ? 1 : 2
  np = split(path, p, "/")
  if (p[2] == "language" && (p[3] == "expressions" || p[3] == "statements"))
    d = (np >= 5) ? p[2] "/" p[3] "/" p[4] : p[2] "/" p[3]
  else d = (np >= 4) ? p[2] "/" p[3] : p[2]
  x = (s in exs)
  m = split($6, f, ","); for (i = 1; i <= m; i++) if (f[i] in ex) x = 1
  m = split($9, g, ","); for (i = 1; i <= m; i++) if (g[i] in exi) x = 1
  E[d]++; C[s]++; R[s] += r
  if (x) { X[d]++; XS[s]++ } else if (flags ~ /(^|,)CanBlockIsTrue(,|$)/) { N[d]++; NS[s]++ } else { U[d]++; US[s]++ }
}
END {
  for (d in E) { printf "test262 %s %d %d %d %d 0 %d 0\n", d, E[d], X[d], N[d], U[d], U[d]
    e += E[d]; xt += X[d]; n += N[d]; u += U[d] }
  printf "test262 total %d %d %d %d 0 %d 0\n", e, xt, n, u, u
  for (s in C) printf "test262-slice %s %d %d %d %d %d 0 %d 0\n", s, C[s], R[s], XS[s], NS[s], US[s], US[s]
}
```

`awk -f slices.awk rules.txt meta.tsv | paste - meta.tsv | awk -f count2.awk xf.txt rules.txt - |
LC_ALL=C sort` prints:

- `test262 total 53801 8982 7 44812 0 44812 0`, and 221 directory lines that sum to it. Before
  the two new line kinds the total was `test262 total 53801 8447 7 45347 0 45347 0`; the 535 more
  excluded cases are the 423 run cases of slice 21 and the 112 `atomicsHelper.js` cases.
- The 15 directory lines that change with the new kinds:
  `test262 built-ins/Atomics 389 121 7 261 0 261 0`, `test262 intl402 22 22 0 0 0 0 0`, `test262 intl402/Array 2 2 0 0 0 0 0`, `test262 intl402/BigInt 11 11 0 0 0 0 0`, `test262 intl402/Collator 65 65 0 0 0 0 0`, `test262 intl402/Date 12 12 0 0 0 0 0`, `test262 intl402/DateTimeFormat 246 246 0 0 0 0 0`, `test262 intl402/DurationFormat 110 110 0 0 0 0 0`, `test262 intl402/Intl 66 66 0 0 0 0 0`, `test262 intl402/Number 7 7 0 0 0 0 0`, `test262 intl402/NumberFormat 251 251 0 0 0 0 0`, `test262 intl402/PluralRules 53 53 0 0 0 0 0`, `test262 intl402/RelativeTimeFormat 80 80 0 0 0 0 0`, `test262 intl402/String 19 19 0 0 0 0 0`, `test262 intl402/TypedArray 1 1 0 0 0 0 0`.
  Every one of the 3394 cases under `test/intl402/` is now excluded. Unchanged examples:
  `test262 harness 117 0 0 117 0 117 0`, `test262 built-ins/Temporal 4616 4616 0 0 0 0 0`.
- 22 slice lines whose fields 2 to 4 equal the 22 lines of the design (`diff` empty, from
  `01 82 164` to `x-stage3 2591 5008`; slices 1 to 19 sum to 44931 cases and 85794 runs, all 22
  to 53801 and 103360). Two of them change with the new kinds, 14 and 21; 20 was excluded
  whole by its features already and does not change: `test262-slice 14 598 1192 112 7 479 0 479 0`,
  `test262-slice 20 4617 9234 4617 0 0 0 0 0`,
  `test262-slice 21 1662 3324 1662 0 0 0 0 0`; also
  `test262-slice x-stage3 2591 5008 2591 0 0 0 0 0`, and slices 01 to 13 and 15 to 19 read
  `test262-slice <s> <cases> <runs> 0 0 <cases> 0 <cases> 0`.

The gate is put together as `story:workspace-and-gate` describes: this story writes its own
include file `taskfiles/test262.yml`, which the root `Taskfile.yml` includes as optional. It is
alone in its wave, so it also regenerates `Cargo.lock` (`cargo generate-lockfile --offline`),
which gains the package `swell-test262`.

## Correction after the unit's first report (2026-11-21)

One case, `test/built-ins/Function/prototype/toString/line-terminator-normalisation-CR.js`, ends
its lines with CR alone. `meta.awk` splits at LF only, reads the file as one line, finds no end
of the block and prints the case with every field empty; its `includes:
[nativeFunctionMatcher.js]` is lost. The reader of this story therefore takes CR LF, CR and LF
each as a line end, which is what a JavaScript source file may use. Measured by the unit: with
`awk -v RS='\r\n|\r|\n' -f meta.awk` exactly that one line changes, the sorted hash is
`c39e4bbe96533198762babfe6e13f04e27b76629fe505dbb4db423204836684a` (before:
`413ef567d4aa9af530424a04538d1446dfef3e502a9fa4be918b7213a5cf919a`), and no count changes (the
case has no flag and no feature). Where this story says the reader equals `meta.awk` byte for
byte, read: equals `meta.awk` run with that record separator. `test262-errors 0` stands. One
more named test: `a_case_with_cr_line_ends_is_read` (a fixture whose lines end in CR alone).

## Change

Failing tests first: the tests below are written against the fixture tree before the code and
fail (the crate does not build) before it; `slot task test262` fails before it (no task).

- `crates/test262/` (package `swell-test262`, binary `swell-test262`, `license.workspace = true`,
  no dependency; `[lints.clippy]` `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`
  denied, since the epic holds the readers to no panic either; `src/main.rs` and `src/lib.rs`
  start with `#![forbid(unsafe_code)]`). Modules: `front_matter` (the reader), `slices` (the
  `swell-slices` block reader, all six line kinds, and the slice of a case), `features` (the
  `features.txt` reader and its checks), `count` (directories, totals, slices, exclusions, and
  the one function that gives the outcome of a run, documented as the place slice 1 changes).
  Two subcommands:
  - `swell-test262 meta --test262 <checkout>`: one line per case, in the format of `meta.awk`
    (path from `test/`, flags, features, phase, type, includes; tabs; lists joined by commas).
  - `swell-test262 count --test262 <checkout> --design <file> --features <file>`: the lines
    above. It exits 1, after printing, when a case names a feature `features.txt` does not list
    (`test262-unlisted-feature <name> <cases>`, one line per name), when `features.txt` does not
    list exactly the names the `features.txt` of the checkout declares
    (`features.txt: <name> declared by test262, not listed` or `... listed, not declared by
    test262`), when a feature an `exclude` line of the block names is `run` in `features.txt`,
    or when a line of the block has an unknown kind.
- `crates/test262/tests/fixtures/`: a small test262-shaped tree written for these tests (no file
  of test262 is copied), with its own `features.txt`, `harness/` file names, and a design file
  holding a `swell-slices` block with every line kind; and a second tree with a case naming an
  unlisted feature.
- Tests in `crates/test262/tests/`, each named: `front_matter_reads_inline_and_block_lists`,
  `front_matter_reads_negative_phase_and_type`, `malformed_front_matter_is_a_counted_error`,
  `fixture_files_are_not_cases`, `directory_is_second_level_and_third_under_expressions_and_statements`,
  `a_shallow_case_keeps_its_own_directory`, `slice_is_the_highest_of_prefix_features_and_flags`,
  `a_case_with_an_excluded_feature_counts_as_excluded`,
  `an_exclude_slice_line_excludes_every_case_of_its_slice`,
  `an_exclude_include_line_excludes_a_case` (a fixture case that includes the named file is
  counted excluded in its directory and slice; the same case without that include is run),
  `can_block_is_true_counts_as_not_applicable`, `a_case_naming_an_unlisted_feature_fails_the_run`
  (runs the binary on the second tree: exit 1 and the `test262-unlisted-feature` line),
  `features_file_must_list_exactly_the_declared_features`,
  `directory_lines_sum_to_the_total_line`, `every_run_case_is_counted_failed`.
- `conformance/test262/features.txt`: the 199 lines settled above.
- `taskfiles/test262.yml`, task `test262`, `deps: [test262-fetch]`: runs `cargo run --locked
  --release -p swell-test262 -- count` on the pinned checkout, `docs/design/engine.md` and
  `conformance/test262/features.txt` into `.out/test262/lines.txt` (failing with the output when
  it exits non-zero), then checks with `awk` that the directory lines sum to the total line and
  that fields 2 to 4 of the slice lines equal the block after "Its output on this document" in
  the design, printing the total line, `test262: 221 directory lines sum to the total`,
  `test262: 22 slice lines equal the design`, and the `test262-errors` line.
- `Cargo.lock`: regenerated, two packages, none from outside the repository.

Gate task (the coordinator adds it to `check` in `Taskfile.yml`, after `test`): `- task: test262`.

Writes: `crates/test262/` (new: `Cargo.toml`, `src/`, `tests/`, `tests/fixtures/`),
`conformance/test262/features.txt`, `taskfiles/test262.yml`, `Cargo.lock`.

## Acceptance

`cargo test -p swell-test262` passes the 15 tests named in Change, among them
`a_case_naming_an_unlisted_feature_fails_the_run` and `an_exclude_include_line_excludes_a_case`;
`slot task test262` exits 0 and prints `test262 total 53801 8982 7 44812 0 44812 0`,
`test262: 221 directory lines sum to the total`, `test262: 22 slice lines equal the design` and
`test262-errors 0`, with `test262-slice 14 598 1192 112 7 479 0 479 0`,
`test262-slice 21 1662 3324 1662 0 0 0 0 0` and `test262 built-ins/Atomics 389 121 7 261 0 261 0`
in `.out/test262/lines.txt`; `cargo run --locked -p swell-test262 -- meta --test262
.cache/test262/2a1ade303f72d3de80565c921d685222405c32ce | LC_ALL=C sort | sha256sum` prints
`c39e4bbe96533198762babfe6e13f04e27b76629fe505dbb4db423204836684a`; and `slot task lock-check`
prints `Cargo.lock: 2 package(s), none from outside this repository`.

## Out of scope

Calling the engine, loading the harness files, `$262`, `print`, `negative`, `async`, modules,
budgets, worker threads and the list of the most expensive cases (decision 11): slice 1, in the
one function named above and the run path around it. `expected.txt`, `sections.txt` and the
comparison with the expected file: `story:test262-expected-and-sections`. Any edit of the design:
the coordinator writes the slice line and the directory rule into decision 11.
