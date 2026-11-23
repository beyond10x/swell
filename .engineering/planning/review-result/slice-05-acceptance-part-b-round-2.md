---
format: aep.planning-md/3
id: review-result:slice-05-acceptance-part-b-round-2
kind: review-result
status: active
title: acceptance critic, second reading of five later stories of slice 5
relations:
- reviews: story:case-mapping-tables-and-string-case-methods
- reviews: story:normalization-tables-normalize-and-locale-compare
- reviews: story:regexp-property-escapes-and-property-tables
- reviews: story:identifiers-and-group-names-outside-ascii
- reviews: story:regexp-v-flag-class-sets-and-properties-of-strings
revision: 1
---
needs-revision

story:regexp-property-escapes-and-property-tables — check (T4) runs `swell-test262 count` on a copy that "holds `harness/` and those ten files alone", but `count` reads `features.txt` from the root of the checkout before it runs any case, and on a copy without that file it exits 2 and prints no `test262-steps` lines; the copy must also hold `features.txt` — crates/test262/src/main.rs:217

All seven findings of round 1 are closed. The revision opened one new defect, a warning and not a blocker:

1. **case-mapping**: closed. The "What expects the refusal" item names `builtin-string-case-ascii.js` with its new header and the four `u1000` rows (2,001 steps, the same as `toLowerCase s1000` at line 875). Writes and scope list both files, and the filter now runs `builtins_string::`. The search, run on the tree, finds nothing outside the files named.
2. **normalization**: closed. It names `builtin-string-locale-compare-ascii.js` with its header ending `|-1`, plus 14 rows: the 2 `u1000` rows at 3,008 and the 12 ASCII rows at their recorded line numbers, each total plus 2 × (n + 2). The rows and their recorded totals (3, 4, 34, 35, 36, 1,003) match the file, and decision 5 is at `docs/design/engine.md:1044`. Every hit of the acceptance's search on the tree is in `transform.rs` or a file the story writes.
3. **property-escapes**: closed. Item (4) takes the property escape out of `a_refused_pattern_of_a_literal_is_counted_as_a_refused_parse`, Writes lists `regexp_literal_limits.rs`, and the filter runs `regexp_literal_limits::`.
4. **identifiers**: closed. Same change, for the group name. Its search for `"identifier with a` on the tree finds only `lexer.rs` and the three fixtures the story removes.
5. **v-flag**: closed. Writes lists `regexp_literal_limits.rs`. The story removes the old test and adds `the_patterns_of_slice_5_in_a_literal_are_not_refused`. Its search does not match `host_191_regexp_counts_its_refusals_of_slice_5`.
6. **property-escapes timing**: closed. The acceptance now holds T1 to T4: the after wall time at most 1.5 times the before, a ceiling of 2^28 steps read from the `test262-steps` lines, and no CPU bound per run. The ten cases T4 names all exist under `generated/` in the pinned checkout. T4 itself carries the new defect above.
7. **close**: closed. It now checks all four added lines with `grep -c` printing `1`. Each prints `0` on the tree today, as stated, and the awk that checks each line is in its section prints nothing on the base.

What I read: the 5 revised stories in full, `aep plan artifact show` at `16253c3`, and the close for its `sections.txt` checks. I also re-ran each story's search with `grep` on the tree and read the runner's `count` flow (`crates/test262/src/main.rs`, lines 1 to 60 and 200 to 300).

What I could not establish: the part a tests that the searches will find once those stories land; each story asks its unit to run the search on its own base and report any extra hit. I built nothing and ran no tests. My scratch directory is removed.

```findings
- file: crates/test262/src/main.rs
  line: 217
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "check (T4) runs swell-test262 count on a copy that holds harness/ and those ten files alone, but count reads features.txt from the root of the checkout before it runs any case, and on a copy without that file it exits 2 and prints no test262-steps lines; the copy must also hold features.txt (story:regexp-property-escapes-and-property-tables, .engineering/planning/story/regexp-property-escapes-and-property-tables.md:87)"
```
