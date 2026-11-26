---
format: aep.planning-md/3
id: review-result:growth-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 2 of the unit growth: findings 1 to 5 of pass 2 closed'
relations:
- reviews: story:buffer-growth-outside-the-charged-list-is-listed
revision: 1
---
Coordinator's read of correction 2 of the unit growth of wave big-1
(`impl/buffer-growth`, `08d26ffa..058eba11`, 3 files, 631 lines added, 1 removed), 2026-11-26.
The adversary's budget of two passes is used; this read stands in for a third pass.

## What was read

The whole diff of `crates/swell/src/builtins/json/parse.rs` (the reason at the reservation
of the stack of open values) and of `crates/swell/tests/builtins_json_parse.rs` (one test,
81 lines), and in `crates/swell/tests/source_scan.rs` the functions `blanked`, `skip_blanks`,
`closing`, `attribute_spans`, `attributes`, `lint_levels`, `lint_list`,
`growth_attribute_faults` and the head of `growth_faults_of` (about 330 of its 549 added
lines); the three tests over the tree and the planted cases by the unit's report.

## The five findings of pass 2

1. An inner `allow` with a reason, in any position: refused. `attribute_spans` takes `#`, an
   optional `!` and `[` with any blanks or line ends between them; `lint_levels` reads every
   `allow(` and `expect(` inside the attribute at any depth, so one inside `cfg_attr` is
   read; `growth_attribute_faults` refuses an `allow` whose list names
   `clippy::disallowed_methods` unless it is the one attribute of `src/lib.rs`, which
   `growth_faults_of` hands out once.
2. An `expect` or `allow` of `clippy::style`, `clippy::all` or `warnings`: refused, with or
   without a reason (`GROUPS`).
3. An `allow` from a macro of another crate of the workspace: the test
   `no_other_crate_names_the_listed_lint` refuses any attribute in `crates/*/src` of the
   other crates that names the lint. The engine takes no crate from outside the repository,
   so no other macro reaches it.
4. An empty or blank reason on an `expect` of the lint: refused (`reason.trim().is_empty()`,
   the reason read from the source's own text at the offsets of the blanked text).
5. The reason at `builtins/json/parse.rs` states the bound: a capacity of at most
   max(4, 2(d − 1)) entries for a deepest nesting d, the spare room uncharged while the parse
   runs, no script during the parse, freed before the reviver. The unit read
   `grow_amortized` of the pinned toolchain's source for the numbers.

## What the read found

- No defect in the lines read.
- `warn`, `deny` and `forbid` of the lint are not read by the scan. None silences a listed
  call under the gate's `-D warnings`, so none is a way around the rule.
- The scan reads text: an attribute built by a macro from pieces (the lint path passed in as
  a macro argument from another file) is not found. The planted cases hold the form the
  adversary showed (the path written in the macro body). Accepted: such a macro would be
  written against the rule on purpose, and the reviewer of that change sees it.
- The unit says it made one edit of `source_scan.rs` while a pool job of the tree ran, and
  that the job had synced before the edit. The counts of that job (1,333 before) are a
  "before" reading only; the "after" readings come from later jobs.

## Verdict

The correction closes findings 1 to 5 of pass 2. The claim (the gate's clippy command exits
0; the tests of `source_scan` and `builtins_json_parse` pass, 29) is run again by the
coordinator on `058eba11` before the merge and recorded as verification.

```findings
[]
```
