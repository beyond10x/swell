---
format: aep.planning-md/3
id: story:test262-run-in-parts
kind: story
status: active
title: The full test262 run can be made in parts, each a pool job of its own
summary: swell-test262 count --part <k>/<n> runs the cases of one part (case i of the sorted list in part i mod n + 1) and writes their runs with what the count needs beside them; count --from-parts reads the files of every part, runs no case and prints, writes and compares what the whole run does, byte for byte but the milliseconds of the time lines; the tasks test262-part and TEST262_FROM_PARTS of test262, test262-expected and slow-test262 use them.
relations:
- decomposes: epic:slice-05-regexp
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T19:12:15Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T19:12:15Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---

## Context

The machine that builds swell runs every build and test as a job of a shared pool, and a job
of the pool is ended after 30 minutes at the most. The full test262 run took about 760 s in a
job until 2026-11-23 and 1,599 s on that day, when 441 generated cases of Unicode property
escapes began to run; under a higher load it does not fit in one job. The forge has no such
limit and keeps making the whole run in one job.

Rule: the count from parts never guesses an outcome. Every case of the list and every run the
whole run makes is in exactly one part's file, the part of its case, or the count is refused
(exit 2, naming the first file, case or run).

Rule: the choice of a case's part depends on nothing but the sorted list of every case of the
checkout and n: case i (from 0) belongs to part i mod n + 1, so a directory of expensive cases
is spread over the parts and 1/1 is the whole run.

Rule: the count from parts prints on standard output and standard error what the whole run
prints, byte for byte (count lines, section lines, problems, `test262-error`, `test262-budget`,
`test262-panic`, `test262-refused`, `test262-steps`, the comparison with `--expected`), and
writes the same run lines in the same order; the one difference is the milliseconds of a
`test262-time` line, a measurement, which is the sum of the times of the slice's runs in the
parts, as the whole run's line is the sum of the times of its runs.

## Change

1. `crates/test262/src/count.rs`: the runs of the count are `jobs` (case position, case, run,
   slice) and the slices of every case; what one run gave is a `Record` (outcome, detail as
   the run gave it, steps, time, refused, budget, panicked), from a run on the engine
   (`Record::of`) or from a part's file; `lines_of` makes the standard error lines, the run
   lines and each case's outcome from the jobs and their records, and `run_cases` is jobs, the
   runs on the engine, `lines_of`. `run_line` writes the six fields of a run line;
   `one_field` is public.
2. `crates/test262/src/parts.rs` (new): `Part` (`<k>/<n>`, no sign, no leading zero),
   `part_of`, `run_part` (the lines of a part's file), `read` (every `*.tsv` of a directory in
   byte order of the names, or one file) and `from_parts` (the records of every run from the
   files, with every check of the rule above, then `lines_of`).
3. The file of a part: the first line `test262-part<TAB><k>/<n><TAB><all|none><TAB><cases of
   the checkout>`; for each case of the part, in list order, `test262-case<TAB><path>`, then
   one line per run of the case: the six fields of the whole run's run line, then the steps,
   the nanoseconds, the refusal (`1` or `0`), the budget (`StepBudget`, `MemoryBudget` or `-`)
   and, for a run that panicked, its detail as the run gave it with `\`, tab, line feed and
   carriage return escaped (else empty). Cut to six fields a run line of a part is the run
   line of the whole run.
4. `crates/test262/src/main.rs`: `count --part <k>/<n> --runs <file> [--workers] [--run]`
   (no `--sections`, no `--expected`; prints `test262-part <k>/<n>: <c> case(s) of <cases>,
   <r> run(s), written to <file>` and no count line) and `count --from-parts <directory or
   file> [--sections] [--expected] [--run] [--runs]` (no `--workers`).
5. `taskfiles/test262.yml`: the task `test262-part` (`TEST262_PART=<k>/<n>` and the variables
   of the task test262 that a run reads; it writes
   `TEST262_OUT/parts/part-<k>-of-<n>.tsv` and its log beside it), and the variable
   `TEST262_FROM_PARTS` of the tasks test262 and test262-expected (slow-test262 runs test262),
   which passes `--from-parts` in place of `--workers`; without it the three tasks run the
   runner with the same arguments as before.
6. `crates/test262/tests/parts.rs` (new) and the generated `tests/main.rs`.

Measured on 2026-11-23 (release build, the pinned checkout, base `3d5afeb` with this change),
as `slot` reports the job's run time: part 1/2 ran 1,385 s with the release build of the
runner (26,901 cases, 42,788 runs; 1-minute load 60.5 at its start, 32.9 at its end), part 2/2
ran 470 s (26,900 cases, 42,768 runs; load 32.9 and 24.4), the count from the two parts ran
16 s (load 24.4 and 53.8). The merged run lines equal the whole run's of `3d5afeb` byte for
byte, in their order and sorted (85,556 lines); the count line file equals
`conformance/test262/expected.txt` byte for byte.

Writes: crates/test262/src/count.rs, crates/test262/src/parts.rs, crates/test262/src/lib.rs,
crates/test262/src/main.rs, crates/test262/tests/parts.rs, crates/test262/tests/main.rs,
taskfiles/test262.yml

## Acceptance

- Tests in `crates/test262/tests/parts.rs`, each red on `3d5afeb`:
  - `the_count_from_parts_of_the_run_tree_prints_what_the_whole_run_prints`,
    `..._of_the_refused_tree_...` and `..._of_the_counted_tree_...` (with `--sections` and
    `--expected`): the counts from 2 and from 3 parts print the whole run's standard output
    and standard error (the milliseconds of the time lines aside) and write its run lines;
  - `every_case_is_in_exactly_one_part`: for n from 1 to 5 every case is in part i mod n + 1
    and in no other; the runs of a part are runs of its cases, none in two parts;
  - `a_budget_and_a_panic_are_read_from_a_part`: the budget, steps and panic lines from a part;
  - `a_count_from_parts_refuses_a_missing_part` (and a missing run),
    `..._refuses_a_run_in_two_parts`, `..._refuses_a_run_of_an_unknown_case` (and an unknown
    case line), `..._refuses_parts_of_different_n` (and parts of another `--run`),
    `a_part_and_a_count_from_parts_refuse_a_wrong_command`.
- Unit tests in `parts.rs`: `a_part_is_k_of_n`, `an_escaped_detail_is_read_back`.
- `cargo nextest run --locked -p swell-test262`: 69 tests pass (57 before).
- Each new test under 1.5 CPU seconds, its binary run directly: the most, the run tree from
  2 and 3 parts, 0.58 CPU seconds (user 0.555, system 0.022); every other under 0.3.
- On the pinned checkout: `task test262-part TEST262_PART=1/2`, `2/2`, then
  `task test262 TEST262_FROM_PARTS=.out/test262/parts` ends with `test262: 287 lines equal
  conformance/test262/expected.txt`; the merged run lines equal the whole run's of `3d5afeb`
  after `LC_ALL=C sort` (and before); `test262-refused 33550`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`
  and `sh taskfiles/test-main.sh check` exit 0.

## Out of scope

- Making the property-escape cases cheaper in the engine (another unit).
- Removing part files of another n from `TEST262_OUT/parts/`: the count refuses them; nothing
  deletes them.
- The forge's workflows and the gate, which keep the whole run and `test262-check`.
- `GATE_VARIABLES` of the root `Taskfile.yml` and the unset line of
  `tests/gate/test262-paths-agree-when-exported-and-passed.sh` (the coordinator's files; a
  patch is in the report).
