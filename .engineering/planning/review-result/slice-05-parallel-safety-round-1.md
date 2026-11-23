---
format: aep.planning-md/3
id: review-result:slice-05-parallel-safety-round-1
kind: review-result
status: active
title: parallel-safety critic, the six stories of the first wave of slice 5, round 1
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
story:regexp-objects-exec-and-literals — its Writes line says `story:string-replace-replace-all-and-split` "writes no file of this story", but both write the generated `crates/swell/tests/main.rs` (cited, both Writes lines, and `aep plan artifact waves --status draft` prints this collision), and no `depends_on` edge puts one after the other; the sentence should name that shared file, or an ordering edge should record it as its reason — .engineering/planning/story/regexp-objects-exec-and-literals.md:138

**What you read:** all 6 artifacts, with `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact relations`, `aep plan artifact waves` and `aep plan artifact waves --status draft`. I also read `taskfiles/site.yml` and the field list of `website/data/status.json`, the header of `taskfiles/test-main.sh`, the file list of `crates/swell/tests`, how `language.rs` finds its scripts, and the scans in `source_scan.rs`.
- Surfaces: 6 cited, 0 inferred, 0 unplaced. Every story has a cited `scope`, and each matches its `Writes:` line.
- Acceptance reads: 6 traced to a producer in the set (3 on 1, 4 on 1, 5 on 3, 5 on 4, 6 on 5, 6 on 2). A `depends_on` edge records all 6.
- Step A (1 beside 2): no shared file. Story 2 adds no package to `Cargo.lock` and changes no input of `status.json`. The inputs are the lock, the crate roots, `expected.txt`, `requirements.txt`, the design, the moves and the delivered list. The two `tests/main.rs` files are in different crates.
- Step B (3 beside 4): no shared file. Story 4 adds no dependency and no crate root, so it does not touch `Cargo.lock`, `conformance/ess/Cargo.lock` or `status.json`.
- The store's own waves give the same four steps as the plan.

**What you could not establish:**
- `story:regexp-literal-early-errors` and `story:string-replace-replace-all-and-split` both write `crates/swell/tests/main.rs` (cited). Neither body names the other, and no edge orders them. Only where the waves place them keeps them apart, plus the brief's rule that whichever lands second writes the file. That is an unease, not a finding.
- Out of my lane (acceptance): `story:regexp-literal-early-errors` makes `swell` reach `swell-regexp` through `swell-syntax`. From then on the HOST-1 and HOST-2 scans in `crates/swell/tests/source_scan.rs` also cover `crates/regexp/src`, including the files story 4 adds beside it in step B. Neither story's acceptance runs `source_scan::`.

```findings
- file: .engineering/planning/story/regexp-objects-exec-and-literals.md
  line: 138
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Writes line says story:string-replace-replace-all-and-split writes no file of this story, but both write the generated crates/swell/tests/main.rs (cited, both Writes lines; aep plan artifact waves --status draft prints the collision) and no depends_on edge orders them, so the sentence names the shared file or an ordering edge records it as its reason"
```
