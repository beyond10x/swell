---
format: aep.planning-md/3
id: review-result:slice-05-acceptance-part-b-round-1
kind: review-result
status: active
title: acceptance critic, the nine later stories of slice 5, round 1
relations:
- reviews: story:unicode-gen-table-list-and-source-digests
- reviews: story:case-mapping-tables-and-string-case-methods
- reviews: story:normalization-tables-normalize-and-locale-compare
- reviews: story:regexp-property-escapes-and-property-tables
- reviews: story:identifiers-and-group-names-outside-ascii
- reviews: story:regexp-match-all-and-escape
- reviews: story:regexp-case-insensitive-matching-and-modifiers
- reviews: story:regexp-v-flag-class-sets-and-properties-of-strings
- reviews: story:slice-05-conformance-close
revision: 1
---
needs-revision

story:case-mapping-tables-and-string-case-methods — Change and Writes name neither of two tests that expect the refusal this story removes: `crates/swell/tests/language/builtin-string-case-ascii.js` expects `SyntaxError` from `"é".toUpperCase()`, so the acceptance's `language::` run fails, and `builtins_string::the_step_totals_are_unchanged` records `throw SyntaxError` for the four case methods on `u1000`, which the acceptance's filter never runs — crates/swell/tests/builtins_string.rs:905
story:normalization-tables-normalize-and-locale-compare — Change and Writes name neither of two tests that expect the `localeCompare` refusal: `crates/swell/tests/language/builtin-string-locale-compare-ascii.js` expects `SyntaxError`, so the acceptance's `language::` run fails, and `builtins_string::the_step_totals_are_unchanged` records `throw SyntaxError` for `localeCompare u1000` and `localeCompare this u1000`, which the acceptance's filter never runs — crates/swell/tests/builtins_string.rs:909
story:regexp-property-escapes-and-property-tables — Change does not name `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` of `crates/swell/tests/regexp_literal_limits.rs`, where a property escape in a literal moves `refusals()` by 1, and the acceptance's filter leaves out `regexp_literal_limits::`, so the acceptance passes while that test fails — .engineering/planning/story/regexp-literal-early-errors.md:72
story:identifiers-and-group-names-outside-ascii — Change does not name `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` of `crates/swell/tests/regexp_literal_limits.rs`, where a group name outside ASCII moves `refusals()` by 1, and the acceptance's filter leaves out `regexp_literal_limits::`, so the acceptance passes while that test fails — .engineering/planning/story/regexp-literal-early-errors.md:72
story:regexp-v-flag-class-sets-and-properties-of-strings — the acceptance runs `regexp_literal_limits::`, whose `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse` expects the flag `v` to be refused, but Change and Writes do not name `crates/swell/tests/regexp_literal_limits.rs`, so the acceptance cannot pass inside the story's Writes — .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:92
story:regexp-property-escapes-and-property-tables — the acceptance only asks the report to give the two wall times of the test262 run, not that the run stays within the bounds in Change (at most half again as long, no run over 10 s of CPU); the runner cannot show the per-run CPU bound, since it prints steps per run (`test262-steps`) and milliseconds per slice (`test262-time`) — .engineering/planning/story/regexp-property-escapes-and-property-tables.md:83
story:slice-05-conformance-close — the acceptance checks only that lines 23 and 76 of `sections.txt` are gone (`grep -c ... prints 0`), not that the new line 76 (`unicode-17.0.0 open-item-C`) and the two `parser-nesting-limit decision-3` lines were added, so it reads the same whether or not they were written — .engineering/planning/story/slice-05-conformance-close.md:59

What I read: all 9 ids, whole bodies, with `aep plan artifact show`. I also ran `aep plan artifact kinds`, `lifecycle story` and `waves --kind story`. In the worktree I read the part a bodies, `taskfiles/test262.yml`, `host-requirements.yml`, `unicode-tables.yml`, `crates/test262/src/count.rs` and `crates/unicode-gen/src/main.rs`, plus these files: `SOURCE`, `zs.rs`, `sections.txt`, `moves.tsv`, `expected.txt`, the list files of `docs/plan/slice-05/` and `.out/decompose-slice-05/runs.tsv`. I re-ran the "(a)" check of each story and the Annex B checks of the close on that `runs.tsv`. Every stated base count matches: 24, 22, 1176, 210, 166, 176, 370, 36 and 0.

What I could not establish:
- I ran no build or test. A test that expects a refusal is known only from grep and from the part a bodies; the `regexp_objects.rs` tests of part a are not written yet, so I cannot tell whether one of them uses `i` or `v`.
- I assumed the `.out/decompose-slice-05/runs.tsv` I used is the run of `e444aaf`; nothing in the file says so.
- Out of my lane (parallel safety or design): normalization's hard-coded `9 table(s)` holds only if no other table story lands before it. That is the case in the store's order, where it comes in wave 5 and property escapes in wave 11.

```findings
- file: crates/swell/tests/builtins_string.rs
  line: 905
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Change and Writes name neither of two tests that expect the refusal this story removes: crates/swell/tests/language/builtin-string-case-ascii.js expects SyntaxError from \"é\".toUpperCase(), so the acceptance's language:: run fails, and builtins_string::the_step_totals_are_unchanged records throw SyntaxError for the four case methods on u1000, which the acceptance's filter never runs"
- file: crates/swell/tests/builtins_string.rs
  line: 909
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Change and Writes name neither of two tests that expect the localeCompare refusal: crates/swell/tests/language/builtin-string-locale-compare-ascii.js expects SyntaxError, so the acceptance's language:: run fails, and builtins_string::the_step_totals_are_unchanged records throw SyntaxError for localeCompare u1000 and localeCompare this u1000, which the acceptance's filter never runs"
- file: .engineering/planning/story/regexp-literal-early-errors.md
  line: 72
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Change does not name a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse of crates/swell/tests/regexp_literal_limits.rs, where a property escape in a literal moves refusals() by 1, and the acceptance's filter leaves out regexp_literal_limits::, so the acceptance passes while that test fails (story:regexp-property-escapes-and-property-tables)"
- file: .engineering/planning/story/regexp-literal-early-errors.md
  line: 72
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Change does not name a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse of crates/swell/tests/regexp_literal_limits.rs, where a group name outside ASCII moves refusals() by 1, and the acceptance's filter leaves out regexp_literal_limits::, so the acceptance passes while that test fails (story:identifiers-and-group-names-outside-ascii)"
- file: .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md
  line: 92
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs regexp_literal_limits::, whose a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse expects the flag v to be refused, but Change and Writes do not name crates/swell/tests/regexp_literal_limits.rs, so the acceptance cannot pass inside the story's Writes"
- file: .engineering/planning/story/regexp-property-escapes-and-property-tables.md
  line: 83
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance only asks the report to give the two wall times of the test262 run, not that the run stays within the bounds in Change (at most half again as long, no run over 10 s of CPU); the runner cannot show the per-run CPU bound, since it prints steps per run (test262-steps) and milliseconds per slice (test262-time)"
- file: .engineering/planning/story/slice-05-conformance-close.md
  line: 59
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance checks only that lines 23 and 76 of sections.txt are gone (grep -c ... prints 0), not that the new line 76 (unicode-17.0.0 open-item-C) and the two parser-nesting-limit decision-3 lines were added, so it reads the same whether or not they were written"
```
