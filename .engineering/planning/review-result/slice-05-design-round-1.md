---
format: aep.planning-md/3
id: review-result:slice-05-design-round-1
kind: review-result
status: active
title: design critic, the six stories of the first wave of slice 5, round 1
relations:
- reviews: story:regexp-crate-and-pattern-parser
- reviews: story:string-replace-replace-all-and-split
- reviews: story:regexp-literal-early-errors
- reviews: story:regexp-compiler-and-backtracking-matcher
- reviews: story:regexp-objects-exec-and-literals
- reviews: story:regexp-symbol-methods-and-string-match-search
revision: 1
---
needs-revision
story:string-replace-replace-all-and-split — its list says it holds only cases "whose text has no `RegExp`", but 3 of its 92 cases (6 runs) evaluate the literal `/./g` and can only pass once story:regexp-objects-exec-and-literals evaluates literals and gives IsRegExp its slot test and `toString`; acceptance (a) therefore reads state that story produces with no edge, and the three belong on `story-objects-exec.cases.txt` — .engineering/planning/story/string-replace-replace-all-and-split.md:31, .engineering/planning/story/string-replace-replace-all-and-split.md:55, test262 `test/built-ins/String/prototype/replaceAll/getSubstitution-0x0024-0x003C.js:63`, `getSubstitution-0x0024N.js:85`, `getSubstitution-0x0024NN.js:64` (`var customRE = /./g;`), .engineering/planning/story/regexp-objects-exec-and-literals.md:125
story:regexp-compiler-and-backtracking-matcher — the body does not say what unit the start index and the capture ranges are in under `u`: it reads a "subject of UTF-16 code units from one index" and tries "a code point per try", while story:regexp-objects-exec-and-literals builds "code unit indices under u from the code point positions of the crate"; the two halves of the matcher's interface disagree, and that seam is where a wrong index under `u` would come from — .engineering/planning/story/regexp-compiler-and-backtracking-matcher.md:44, .engineering/planning/story/regexp-compiler-and-backtracking-matcher.md:46, .engineering/planning/story/regexp-objects-exec-and-literals.md:123

**What I read:** all 6 stories in full, read from their files with line numbers (the same bodies `aep plan artifact show` prints); `aep plan artifact relations`, `graph`, `validate` and `waves`; `docs/plan/slice-05/` (scan.awk, scan.tsv, the four case lists, consumer-regexps.tsv); the case texts of story 2's list and the default harness files of the pinned test262.
- **Edges:** I walked the 6 `depends_on` edges, plus `decomposes` to the epic and the epic's `depends_on` chain outside the set (slice-05 → slice-04 → slice-03). There is no cycle.
- **Acceptance reads:** I traced 9 reads to a producer in the set. 8 are recorded by an edge; the one that is not is the first finding.

**What I could not establish:**
- **The index unit:** whether the design author's D1 text fixes the matcher's index unit. That text is not written yet, so the second finding may close there.
- **Out of my lane (parallel safety):** story 2 collides with story 3 and with story 5 on the generated `crates/swell/tests/main.rs` with no edge between them (`aep plan artifact waves`, `collision:` lines). Story 3's and story 5's "Shared with" sentences do not name story 2.
- **Out of my lane (scope and acceptance):** the `rx` test in `scan.awk` only notices the word `RegExp` or a refusal detail. A case whose only regular expression is a literal, and which fails earlier on the base, is filed under `string-replace-split`. That is the cause of the first finding.
- **Unease (step charge):** story 1 writes the step-charge method of the charging trait. Its only caller is story 4's matcher, and story 4's Writes list leaves out `crates/regexp/src/budget.rs`. D1 settles the method's shape, so this is not a finding.
- **Unease (between stories 5 and 6):** after story 5 lands and before story 6, `replace` and `split` given a RegExp take the String path and silently give a wrong answer, not a refusal (story 6, line 35). The wave closes after story 6.

```findings
- file: .engineering/planning/story/string-replace-replace-all-and-split.md
  line: 31
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its list says it holds only cases \"whose text has no `RegExp`\", but 3 of its 92 cases (6 runs) evaluate the literal `/./g` and can only pass once story:regexp-objects-exec-and-literals evaluates literals and gives IsRegExp its slot test and `toString`; acceptance (a) therefore reads state that story produces with no edge, and the three belong on `story-objects-exec.cases.txt`"
- file: .engineering/planning/story/regexp-compiler-and-backtracking-matcher.md
  line: 46
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say what unit the start index and the capture ranges are in under `u`: it reads a \"subject of UTF-16 code units from one index\" and tries \"a code point per try\", while story:regexp-objects-exec-and-literals builds \"code unit indices under u from the code point positions of the crate\"; the two halves of the matcher's interface disagree, and that seam is where a wrong index under `u` would come from"
```
