---
format: aep.planning-md/3
id: review-result:slice-05-scope-part-b-round-1
kind: review-result
status: active
title: scope critic, the nine later stories of slice 5, round 1
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

story:slice-05-conformance-close — the epic promises "every section that depends on it carries the deviation as an omission", but the story's `sections.txt` change adds `unicode-17.0.0 open-item-C` only for `sec-names-and-keywords`. `sec-literals-regular-expression-literals` (`path ... test/language/literals/regexp/`, sections.txt:78) also depends on the version after slice 5: 14 of its cases (`named-groups/invalid-non-id-start-groupspecifier*.js` and the others on `story-identifiers.cases.txt`) are judged by `ID_Start`/`ID_Continue`. Nothing says this section was dropped, so Change (story line 50) should add `omit sec-literals-regular-expression-literals unicode-17.0.0 open-item-C` — .engineering/planning/epic/slice-05-regexp.md:39

**What I read:** 16 artifacts: the epic, the 9 drafts, the 6 accepted stories of part a (title, summary, Out of scope, and the lines on `sections.txt` and the flag getters), plus `review-result:slice-05-scope-round-1`. Commands: `aep plan artifact show` / `grep -n` on the store files, `aep plan artifact graph`, `aep plan artifact waves --kind story`. I also read `data/unicode/17.0.0/` with `SOURCE` and `LICENSE`, `conformance/test262/sections.txt`, `docs/plan/slice-05/` (scan.awk head, scan.tsv tags, the 11 case lists), and the pinned test262 `language/literals/regexp/`.

**Promises:** I took 45 from the epic (29 of Outcome, 4 open items, 5 constraints, 7 acceptance lines). 44 are traced:
- **Part b claims:** the generator list, the tables for case mapping, normalization, properties with `ID_Start`/`ID_Continue`, case folding and emoji, the flags `i` and `v`, modifiers, property escapes, set operations, properties of strings, `matchAll`, `RegExpStringIteratorPrototype`, `RegExp.escape`, the identifiers, the Annex B division (settled in the close story), acceptance 1, 2 and 4, and `expected.txt`.
- **Part a claims:** the rest of the regexp engine and methods, and acceptance 5 and 6.
- **Already in the store or the tree:** the vendored data and `SOURCE`, the emoji addresses, the `LICENSE` text, and the `Zs` white space table.
- **Named for the coordinator:** `AGENTS.md`/`README.md` numbers, and the `RangeError` and `.out/` wording.
- **Narrowed:** 1, the Unicode-version omission (the finding above).

The 11 case lists hold 2,655 lines with no duplicate. Together with the 17 cases in `close-moves-later.tsv` that makes 2,672, which equals the failing slice-5 cases in the close story's estimate. `localeCompare` and `normalize` look like reach at first, but their 11 cases are failing cases of slice 5 in `scan.tsv`, so acceptance 1 covers them. I found no other item that goes beyond the epic, and no outcome claimed twice: the HOST-165 and HOST-191 test changes and the `Cased`/`Case_Ignorable`, `ID_Start` and uppercase tables each have one writer, and the others read them.

**What I could not establish:**
- **White space:** `test/language/white-space/` (2 cases, by `Zs`) depends on the version too, but `sections.txt` has no section for it, so adding one goes past what the epic's wording asks. I did not count it.
- **Out of my lane (design or acceptance), not counted:** the v story's class set built at compile (strings, intersections) is not said to be charged through the trait (story line 77). The other stories do say it for their sets: property story line 79, i story line 65.
- **Out of my lane (acceptance), not counted:** the property-escapes story's acceptance (line 95) only asks for the two test262 wall times to be reported. It does not check the thresholds in its Change (line 83): over 10 s CPU per run, or a rise of more than half.
- I wrote no scratch file.

```findings
- file: .engineering/planning/epic/slice-05-regexp.md
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:slice-05-conformance-close narrows the epic's \"every section that depends on it carries the deviation as an omission\" to sec-names-and-keywords: sec-literals-regular-expression-literals (sections.txt:78) depends on the Unicode version after slice 5 (14 named-groups cases judged by ID_Start/ID_Continue), and its Change (line 50) adds no `omit sec-literals-regular-expression-literals unicode-17.0.0 open-item-C` and says nothing of dropping it"
```
