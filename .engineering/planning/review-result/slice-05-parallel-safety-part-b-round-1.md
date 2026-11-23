---
format: aep.planning-md/3
id: review-result:slice-05-parallel-safety-part-b-round-1
kind: review-result
status: active
title: parallel-safety critic, the nine later stories of slice 5, round 1
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

story:regexp-v-flag-class-sets-and-properties-of-strings — Its acceptance reads the removal of the `group name outside ASCII` refusal that story:identifiers-and-group-names-outside-ascii makes, and no depends_on edge runs from it to that story. Three lines depend on that removal: the `grep -rc 'not supported before slice 5' crates/regexp/src` that must print `0`, "the refusal test goes (no refusal remains)", and HOST-191 holding "for every part". The two stories also share `crates/regexp/src/parser.rs`, `crates/regexp/tests/early_errors.rs`, the generated `crates/regexp/tests/main.rs`, `crates/swell/tests/regexp_refusals.rs` and `crates/syntax/tests/sources/valid/regexp-literals.js`. The fix is that edge, or merging the two stories. — .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:92, .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:78, .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:9, .engineering/planning/story/identifiers-and-group-names-outside-ascii.md:66

story:normalization-tables-normalize-and-locale-compare — Its acceptance expects exactly `unicode-tables: 9 table(s)`, which holds only if story:regexp-property-escapes-and-property-tables, story:regexp-case-insensitive-matching-and-modifiers and story:regexp-v-flag-class-sets-and-properties-of-strings have not landed yet. It also shares `crates/unicode/src/lib.rs`, `crates/unicode/src/tables/mod.rs` and `crates/unicode-gen/src/tables.rs` with all three, plus `crates/unicode/tests/main.rs` with the property story (cited, scope of all four). No depends_on edge orders it against them, and none of the four bodies names the pair. Either remedy works: an ordering edge (property escapes depends_on normalization, naming the shared files), or a count relative to the unit's base plus a sentence naming the shared files. — .engineering/planning/story/normalization-tables-normalize-and-locale-compare.md:72, .engineering/planning/story/normalization-tables-normalize-and-locale-compare.md:68, .engineering/planning/story/regexp-property-escapes-and-property-tables.md:91

story:identifiers-and-group-names-outside-ascii — Its Writes line says story:regexp-case-insensitive-matching-and-modifiers writes `crates/regexp/src/parser.rs` and `early_errors.rs` after this story lands, but that story's Writes name neither file. The files the two really share (the `host_191_regexp_counts_its_refusals_of_slice_5` test in `regexp_refusals.rs`, and the generated `crates/regexp/tests/main.rs`) are kept in the stated order by no depends_on edge from that story to this one. — .engineering/planning/story/identifiers-and-group-names-outside-ascii.md:76, .engineering/planning/story/regexp-case-insensitive-matching-and-modifiers.md:77, .engineering/planning/story/regexp-case-insensitive-matching-and-modifiers.md:9

**What I read:** all 9 draft stories in full, the Writes lines and scope of the 6 active part-a stories, both briefs, the role procedure and the rubric. Commands: `aep plan artifact show`, `aep plan artifact waves --kind story` (all, `--status draft`, `--status active`), `aep plan artifact relations`, `aep plan artifact graph --format dot`. In the tree: `taskfiles/test-main.sh`, `taskfiles/site.yml`, `crates/syntax/tests/sources.rs` (fixtures are read with `read_dir`, so removing one touches no other file), `crates/*/Cargo.toml`, decision 7's list of refusals in `docs/design/engine.md`, `docs/plan/slice-05/scan.awk`, and the harness files `regExpUtils.js` and `nativeFunctionMatcher.js`.

**Surfaces:** 9 cited, 0 inferred, 0 not placeable. I worked out the dependency order of all 15 stories and checked every pair that no edge orders, not only the pairs that share a step.

**Acceptance reads:** 14 traced to a producer in the set, 12 of them recorded by a depends_on edge. The 2 without an edge: the v-flag story reading what the identifiers story removes, and the normalization story's fixed table count. Another 3 reads go to part-a producers (property escapes on objects-exec, identifiers and match-all on symbol-methods), and all 3 have edges. I also grepped the case lists for features of stories they are not ordered against (matchAll, `.match(`/`.replace(`, `RegExp.escape`, `\p{`, group names outside ASCII, `.normalize(`, RegExp use in the case and normalize lists) and found nothing.

**What I could not establish:**
- Whether units start strictly by the eight steps or as soon as their dependencies are done. Today the step order happens to put the normalization story before property escapes, and identifiers before the v-flag story. Nothing records either order, and the normalization story still waits on an open design sentence (`localeCompare` outside ASCII), which could delay it.
- Out of my lane (scope/acceptance): story:slice-05-conformance-close writes `moves.tsv`, section 12 of `engine.md` and `expected.txt`. All three are inputs of `site-data` (`taskfiles/site.yml`, lines 24-35), yet its Writes do not name `website/data/status.json`. I worked this out from the task's description; it collides with nothing, because that story runs alone in its step.
- No build was run, as the brief says.

```findings
- file: .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md
  line: 92
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance (grep of 'not supported before slice 5' over crates/regexp/src printing 0, the refusal test of early_errors.rs removed as 'no refusal remains', HOST-191 'for every part') reads the removal of the group-name refusal that story:identifiers-and-group-names-outside-ascii makes, and it shares parser.rs, early_errors.rs, the generated crates/regexp/tests/main.rs, regexp_refusals.rs and valid/regexp-literals.js with it, yet no depends_on edge runs from it to that story"
- file: .engineering/planning/story/normalization-tables-normalize-and-locale-compare.md
  line: 72
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance's fixed 'unicode-tables: 9 table(s)' holds only while story:regexp-property-escapes-and-property-tables, story:regexp-case-insensitive-matching-and-modifiers and story:regexp-v-flag-class-sets-and-properties-of-strings have not landed, and it shares crates/unicode/src/lib.rs, crates/unicode/src/tables/mod.rs and crates/unicode-gen/src/tables.rs (with the property story also crates/unicode/tests/main.rs) with those three, which no depends_on edge orders against it and no body of the four names"
- file: .engineering/planning/story/identifiers-and-group-names-outside-ascii.md
  line: 76
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Writes line says story:regexp-case-insensitive-matching-and-modifiers writes crates/regexp/src/parser.rs and early_errors.rs after it has landed, which that story's Writes do not name, and the order it states for the files they do share (the host_191 test of regexp_refusals.rs, the generated crates/regexp/tests/main.rs) is held by no depends_on edge from that story to this one"
```
