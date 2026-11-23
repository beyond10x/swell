---
format: aep.planning-md/3
id: review-result:slice-05-acceptance-round-1
kind: review-result
status: active
title: acceptance critic, the six stories of the first wave of slice 5, round 1
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
story:regexp-crate-and-pattern-parser — the acceptance requires `cargo nextest run --locked -p swell-regexp --lib` to exit 0, but Change only adds unit tests "where a private item needs one", and nextest 0.9.143 fails when there are no tests to run (`--no-tests` defaults to fail), so a correct crate with no unit test fails its own acceptance; either name at least one unit test or drop the `--lib` command — .engineering/planning/story/regexp-crate-and-pattern-parser.md:71
story:regexp-compiler-and-backtracking-matcher — the acceptance requires `cargo nextest run --locked -p swell-regexp --lib` to exit 0, but Change names no unit test in `src/`, so the command only passes if some unit test happens to exist (nextest fails on no tests by default) — .engineering/planning/story/regexp-compiler-and-backtracking-matcher.md:57
story:regexp-symbol-methods-and-string-match-search — the acceptance clause "the command of Context over the downloaded files exits 0 and prints the three lines" reads only the committed `consumer-regexps.tsv` and the downloaded files, and it already prints those three lines and exits 0 on the base, so it cannot tell the story done from not done; it also runs in `consumer-gap/`, which is not in the tree; drop it or replace it with a check that runs the engine — .engineering/planning/story/regexp-symbol-methods-and-string-match-search.md:64
story:regexp-objects-exec-and-literals — the story claims "after this story all 12 literals evaluate", but no test or check of it evaluates a literal for the 11 rows whose call is not `test` (the scripts cover only rows where the call is `test`), so the acceptance does not show that consumer lift — .engineering/planning/story/regexp-objects-exec-and-literals.md:105
story:string-replace-replace-all-and-split — Change says the stack constant does not change for the new re-entering builtins, but the acceptance filter runs `limits_scripts::` only and not the stack tests `budgets::` and `eval_stack::` that stories 5 and 6 run for the same promise — .engineering/planning/story/string-replace-replace-all-and-split.md:55

**What I read:** all 6 of the 6 ids, whole, with `aep plan artifact show`. I also read the epic's acceptance, the rubric and procedure, `AGENTS.md` (run loop, gate), `docs/plan/unit-invariants.md`, `waves.md` ("The order after slice 3"), the form story `story:for-of-statement`, `Taskfile.yml` and the files under `taskfiles/`.

What I checked against the tree:
- The output strings and base counts the checks print: test-main 7, forbid-unsafe 8, lock 7, test binaries 7, `pending slice 5` 2, the sections line 1.
- Every "on `e444aaf`" run count, against the decomposition's base `runs.tsv`: 182, 300, 334, 1,837, 843, 816, 148. All match.
- That the test modules and helpers named by the filters and tests exist.
- I ran the story 6 consumer command on the base.

**What I could not establish:**
- I ran no build or test (the pool is blocked).
- I assumed `.out/decompose-slice-05/runs.tsv` is the run of `e444aaf`.
- Stories 1 and 4 leave `slot --timeout 30m task test262` out of their acceptance, which brief rule 5 asks for. I read that as deliberate, since no case runs on the crate alone, so I did not file it.
- Story 5's check (b) selects runs by message across all slices, not by its list or directories as rule 5 asks. Once `RegExp` exists no run can print that message, so I did not file it.
- Stories 2, 3 and 6 change what a script evaluates to but do not name `cargo nextest run --locked -p swell-test262`, which `docs/plan/unit-invariants.md` requires of such a unit. The accepted slice-6 form omits it too, so I left it as an observation.
- Out of my lane, for parallel safety: both stories of step 2 write `crates/regexp/tests/main.rs` or `crates/swell/tests/main.rs`, which brief point 3 already settles.

I put two scratch files under `.out/critic-acceptance-*.txt` instead of `.out/critic-acceptance/`; both are removed.

```findings
- file: .engineering/planning/story/regexp-crate-and-pattern-parser.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires `cargo nextest run --locked -p swell-regexp --lib` to exit 0, but Change only adds unit tests \"where a private item needs one\", and nextest 0.9.143 fails when there are no tests to run (`--no-tests` defaults to fail), so a correct crate with no unit test fails its own acceptance; either name at least one unit test or drop the `--lib` command"
- file: .engineering/planning/story/regexp-compiler-and-backtracking-matcher.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires `cargo nextest run --locked -p swell-regexp --lib` to exit 0, but Change names no unit test in `src/`, so the command only passes if some unit test happens to exist (nextest fails on no tests by default)"
- file: .engineering/planning/story/regexp-symbol-methods-and-string-match-search.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance clause \"the command of Context over the downloaded files exits 0 and prints the three lines\" reads only the committed `consumer-regexps.tsv` and the downloaded files, and it already prints those three lines and exits 0 on the base, so it cannot tell the story done from not done; it also runs in `consumer-gap/`, which is not in the tree; drop it or replace it with a check that runs the engine"
- file: .engineering/planning/story/regexp-objects-exec-and-literals.md
  line: 105
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story claims \"after this story all 12 literals evaluate\", but no test or check of it evaluates a literal for the 11 rows whose call is not `test` (the scripts cover only rows where the call is `test`), so the acceptance does not show that consumer lift"
- file: .engineering/planning/story/string-replace-replace-all-and-split.md
  line: 55
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change says the stack constant does not change for the new re-entering builtins, but the acceptance filter runs `limits_scripts::` only and not the stack tests `budgets::` and `eval_stack::` that stories 5 and 6 run for the same promise"
```
