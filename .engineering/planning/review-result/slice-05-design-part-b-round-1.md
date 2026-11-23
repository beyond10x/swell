---
format: aep.planning-md/3
id: review-result:slice-05-design-part-b-round-1
kind: review-result
status: active
title: design critic, the nine later stories of slice 5, round 1
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

story:unicode-gen-table-list-and-source-digests — its Change leaves `crates/unicode-gen/tests/check.rs` hard-coded to one table: `EQUAL` says "1 table(s)", and the changed-table test copies only the `Zs` source and `zs.rs`. Each of the five table stories runs `cargo nextest run --locked -p swell-unicode-gen --test main` in its acceptance, but none of them lists `check.rs` in its Writes. So the prefactor has to make `check.rs` take its count and its copied sources from the table list (or from `SOURCE`), or each table unit fails that run and stops on its Writes line — crates/unicode-gen/tests/check.rs:13, crates/unicode-gen/tests/check.rs:45, .engineering/planning/story/unicode-gen-table-list-and-source-digests.md:49, .engineering/planning/story/case-mapping-tables-and-string-case-methods.md:72

story:unicode-gen-table-list-and-source-digests — the sequence reader and the pair and sequence writers have no caller in this story and none of its seven tests exercises them. Each one is the half of a later story's table (pairs: story:case-mapping-tables-and-string-case-methods; sequences: story:regexp-v-flag-class-sets-and-properties-of-strings), so each should move to the story that first uses it, or this story names a test for each and the `expect(dead_code, reason = …)` mark that `-D warnings` requires — .engineering/planning/story/unicode-gen-table-list-and-source-digests.md:45, .engineering/planning/story/unicode-gen-table-list-and-source-digests.md:46, .engineering/planning/story/unicode-gen-table-list-and-source-digests.md:49, Taskfile.yml:509

story:regexp-v-flag-class-sets-and-properties-of-strings — two of its checks read state that story:identifiers-and-group-names-outside-ascii produces, and no depends_on edge runs from it to that story; today they are ordered only by accident of the step numbers. The acceptance runs `grep -rc 'not supported before slice 5' crates/regexp/src` and expects `0`, and `group name outside ASCII is not supported before slice 5` is the identifier story's refusal to remove. Its Requirements say HOST-191's test holds "for every part", which includes the identifier story's group-name part — .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:92, .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md:86, .engineering/planning/story/identifiers-and-group-names-outside-ascii.md:66, docs/design/engine.md:1290

story:normalization-tables-normalize-and-locale-compare — its acceptance expects exactly `unicode-tables: 9 table(s)`. That holds only if no other table story has landed before it, and no edge says story:regexp-property-escapes-and-property-tables, story:regexp-case-insensitive-matching-and-modifiers or story:regexp-v-flag-class-sets-and-properties-of-strings lands after it. The line should say "three more than on the base of the unit", as those three stories already word theirs — .engineering/planning/story/normalization-tables-normalize-and-locale-compare.md:72, .engineering/planning/story/regexp-property-escapes-and-property-tables.md:95

story:regexp-property-escapes-and-property-tables — `is_id_start` and `is_id_continue` are added "for the identifier story", and none of its tests exercises them. The identifier story is their only caller, and an error in them "goes back". The story should name a test of both functions, or the identifier story should own them — .engineering/planning/story/regexp-property-escapes-and-property-tables.md:78, .engineering/planning/story/regexp-property-escapes-and-property-tables.md:81, .engineering/planning/story/identifiers-and-group-names-outside-ascii.md:76

What I read: the 9 stories in full (the `.md` files in the store), the briefs of part a and part b, plan-critic-design.md and critic-rubric.md, and `crates/unicode-gen/{Cargo.toml,tests/check.rs,tests/main.rs}`, `docs/design/engine.md:1285-1304`, `AGENTS.md:462-483`, `Taskfile.yml` clippy. Commands run in the worktree: `aep plan artifact relations`, `aep plan artifact validate` (valid; one scope notice on story:a-case-counts-in-the-slice-delivered-last, not mine), `aep plan artifact waves --kind story`, `aep plan artifact graph --format dot`, and a test262 feature tally per case list. I walked 21 depends_on edges: the 15 of the set and 6 inside part a, outside the set. There is no cycle. The edge story:regexp-case-insensitive-matching-and-modifiers → story:regexp-match-all-and-escape is there for one case and adds no length to the critical path, because the case-insensitive story would otherwise still need an edge to story:identifiers-and-group-names-outside-ascii for `regexp_refusals.rs`. I traced 24 acceptance reads to a producer in the set or in part a. An edge records 21 of them, directly or through other stories. The other 3 are the two v-flag reads and normalization's count.

What I could not establish:
- How `crates/unicode-gen/tests/ucd.rs` reaches the readers of a binary-only crate. The story lists no library target and does not list `Cargo.toml`. This belongs to the acceptance lane.
- story:identifiers-and-group-names-outside-ascii says the case-insensitive and v-flag stories write `parser.rs`, `early_errors.rs` and `regexp_refusals.rs` after it has landed. No edge runs from story:regexp-case-insensitive-matching-and-modifiers to it. `crates/syntax/tests/sources/valid/regexp-literals.js` is written by three stories. Both belong to the parallel-safety lane.
- story:regexp-property-escapes-and-property-tables asks for the full test262 run timed before and after (brief point 4). Its acceptance makes the report give both times, but states the 10 s per run and one-half rise bounds only through "as Change says". This belongs to the acceptance lane.
- The epic's depends_on edge to epic:slice-04-numbers-and-dates is outside the set, and the brief says the coordinator rewrites the epic.
- I did not check each case list case by case for a use of a sibling's part. I relied on the feature tags and on `scan.awk`'s assignment.

```findings
- file: .engineering/planning/story/unicode-gen-table-list-and-source-digests.md
  line: 49
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its Change leaves crates/unicode-gen/tests/check.rs hard-coded to one table (EQUAL says 1 table(s), and the changed-table test copies only the Zs source and zs.rs), while each of the five table stories runs cargo nextest run -p swell-unicode-gen --test main and none lists check.rs in its Writes, so the prefactor has to make check.rs take its count and its copied sources from the table list (or from SOURCE)"
- file: .engineering/planning/story/unicode-gen-table-list-and-source-digests.md
  line: 46
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the sequence reader and the pair and sequence writers have no caller in this story and none of its seven tests exercises them; each is the half of a later story's table (pairs: case-mapping, sequences: v-flag), so each should move to the story that first uses it, or this story names a test for each and the expect(dead_code, reason) mark that -D warnings requires"
- file: .engineering/planning/story/regexp-v-flag-class-sets-and-properties-of-strings.md
  line: 92
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "two of its checks read state that story:identifiers-and-group-names-outside-ascii produces (the grep for 'not supported before slice 5' in crates/regexp/src printing 0 needs that story's group-name refusal gone, and the HOST-191 test 'for every part' includes its group-name part), and no depends_on edge runs from it to that story"
- file: .engineering/planning/story/normalization-tables-normalize-and-locale-compare.md
  line: 72
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance expects exactly 'unicode-tables: 9 table(s)', which holds only if no other table story has landed before it, and no edge orders the property-escape, case-insensitive or v-flag stories after it; the line should say three more than on the base of the unit"
- file: .engineering/planning/story/regexp-property-escapes-and-property-tables.md
  line: 78
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "is_id_start and is_id_continue are added for the identifier story and none of its tests exercises them, so the story should name a test of both functions or the identifier story should own them"
```
