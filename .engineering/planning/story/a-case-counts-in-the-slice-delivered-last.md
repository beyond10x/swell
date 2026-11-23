---
format: aep.planning-md/3
id: story:a-case-counts-in-the-slice-delivered-last
kind: story
status: implemented
title: A case counts in the slice delivered last among its slices
summary: The swell-slices block names the order of delivery (1, 2, 3, 6, 5, 7, 10, 4, 9, 8, 11 to 21); the runner, slices.awk and test262-moves-check compare slices by their place in it, so the 393 cases of slice 6 that need only slice 4 or 5 leave slice 6 and it reads every run passed.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:05:11Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T01:05:11Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-23T02:42:39Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---

## Context

`docs/design/engine.md`, section 12, counted a case in "the highest slice among its directory, its features and its flags". That is the slice after which the case can first pass only while the slices are delivered in the order of their numbers. They are not: the order is 1, 2, 3, 6, 5, 7, 10, 4, 9, then 8, 11 and 12 to 21 (`AGENTS.md`, line 36; `docs/plan/waves.md`, "The order after slice 3"). With every feature story of slice 6 merged and 136 cases moved by the first part of its close (`b14d07a`), the slice read `test262-slice 06 4300 8153 0 0 4300 3907 393 0`. The 393 failing cases (784 runs) need only slice 4 (67 by the scan of that close: `Date` 59, `Number` 5, `**` 3) or slice 5 (326: `RegExp` and the `String` methods that use it), and each carries a feature of slice 6 (`Symbol` or a well-known symbol, a computed property name, destructuring). The block gave them slice 6, the highest number, and `test262-moves-check` refused a line of `conformance/test262/moves.tsv` whose `to` was not greater than its `from`, so no line could move them, and slice 6 could not close. The same holds at every slice delivered before one of a lower number (7 and 10 before 4; 10 before 9 and 8).

## Change

- The block: one line `order delivery 1,2,3,6,5,7,10,4,9,8,11,12,13,14,15,16,17,18,19,20,21`, the first of the block. A case is counted in the slice delivered last among its longest matching `dir` prefix, its features and its flags. The excluded kinds (`exclude`, `exclude-slice`, `exclude-include`, the `x-` slices) keep their meaning.
- `crates/test262/src/slices.rs`: `read` takes the `order` line and reports as a problem line (the run exits 1; it never falls back to the numbers) a block without it, a second order line, an order line whose second word is not `delivery`, a word of the list that is not a slice, a slice named twice, and every `dir`, `feature`, `flag` or `exclude-slice` line whose slice the order does not name; `Rules::slice_of` compares by the place in the order. `Slice` loses its derived `PartialOrd`/`Ord` (a number order no reader used). Unit tests beside it, an integration test of the binary in `tests/runner.rs` with the fixture `tests/fixtures/design-no-order.md`; `design.md` and `design-unknown-kind.md` of the fixtures get an order line.
- `taskfiles/test262.yml`, `test262-moves-check`: reads the order from the block, fails on no order line, two, a malformed one or a slice named twice; a line whose `to` or `from` the order does not name, or whose `to` is not delivered after its `from`, is refused (`to <t> is not greater than from <f> in the order of delivery`); the slice it computes for a case is the one delivered last. Its other refusals stay. Its summary says so.
- `tests/gate/test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier.sh`, a planted script on a planted checkout of two cases: a line from 5 to 6 is refused, one from 6 to 5 is admitted, and the admitted line is refused against a design whose block has no order line.
- `docs/design/engine.md`, section 12: the first paragraph states the rule; a paragraph "The order of delivery" says what it is, why, and that the counts of the slices not yet delivered move when their order changes; the table, the slice 6 sentence under it, the paragraph "Slice 6's cases that need a later slice" and the output block after "Its output on this document:" carry the new counts; the listing `slices.awk` reads the order. The block gains 7 lines `dir <file> <slice>` for the cases of the 393 whose directory is of slice 3 or 6.
- `conformance/test262/moves.tsv`: 7 lines, `from` 6, kind `part`: to 5 with `RegExp`, `/a/g` and `/./` (3), to 4 with `Date` (2). No line changed or removed: no line's case is given another slice than its `to` by the new rule. No token added to `conformance/test262/later-parts.tsv`.
- `conformance/test262/expected.txt`: the lines of the full run.

Writes: `crates/test262/src/slices.rs`, `crates/test262/tests/runner.rs`, `crates/test262/tests/fixtures/design.md`, `crates/test262/tests/fixtures/design-unknown-kind.md`, `crates/test262/tests/fixtures/design-no-order.md`, `taskfiles/test262.yml`, `tests/gate/test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier.sh`, `docs/design/engine.md` (section 12 and the listing `slices.awk`), `conformance/test262/moves.tsv`, `conformance/test262/expected.txt`.

## Acceptance

- `cargo nextest run --locked -p swell-test262` passes, with the tests `slices::tests::a_case_counts_in_the_slice_delivered_last_among_its_lines`, `a_slice_of_a_lower_number_delivered_later_wins`, `a_block_without_an_order_is_a_problem`, `a_slice_named_twice_in_the_order_is_a_problem`, `a_slice_of_a_rule_line_the_order_does_not_name_is_a_problem`, `an_order_line_of_another_shape_is_a_problem` and `runner::a_block_without_an_order_of_delivery_fails_the_run` (62 tests, 55 on the base; the seven fail on the base).
- `sh tests/gate/test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier.sh` prints `test262-moves-check-refuses-a-move-to-a-slice-delivered-earlier: ok` (on the base it fails: the line from 5 to 6 is admitted).
- `slot task -d . -t taskfiles/test262.yml test262-moves-check` exits 0 and prints `test262-moves: 1774 case(s) moved, 3:78 4:325 5:285 6:694 7:60 8:183 9:22 10:86 11:4 12:21 13:16; slices 1 to 19: 44931 85794`.
- `slot --timeout 30m task test262` exits 0 and prints `test262: 22 slice lines equal the design` and `test262-slice 06 3898 7351 0 0 3898 3898 0 0`; slices 1 to 3 read `01 31 62`, `02 4259 7543`, `03 7011 13829` as before; the slice lines sum to `53801 103360`, slices 1 to 19 to `44931 85794`.
- Run by run against the run of the base: no run changes its outcome (`awk -F'\t' 'NR == FNR { o[$3 "\t" $4] = $5; next } o[$3 "\t" $4] != $5' <base runs.tsv> .out/test262/runs.tsv | wc -l` prints `0`, and both files have 85,556 lines).

## Out of scope

The engine crates; the order itself (the operator's, `docs/plan/waves.md`); `AGENTS.md`, `README.md`, `CHANGELOG.md`, `conformance/test262/expected.digest` and `conformance/slices-delivered.txt` (the coordinator); the delivery of slice 6 (`story:slice-06-conformance-close`); `taskfiles/site.yml`, which prints pending requirements by slice number and reads which table rows are slices, and compares no two slices; the planted script `tests/gate/test262-moves-check-refuses-a-line-without-evidence.sh`, whose line with `from` = `to` + 1 is refused only because the slice it picks (`to` 6, `from` 7) is delivered in that order.
