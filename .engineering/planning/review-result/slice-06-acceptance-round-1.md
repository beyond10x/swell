---
format: aep.planning-md/3
id: review-result:slice-06-acceptance-round-1
kind: review-result
status: active
title: acceptance critic, first five stories of slice 6, round 1
relations:
- reviews: story:let-const-and-block-scopes
- reviews: story:template-literals
- reviews: story:arrow-functions
- reviews: story:default-and-rest-parameters
- reviews: story:object-destructuring
revision: 1
---
needs-revision
story:let-const-and-block-scopes — `the_per_iteration_record_is_charged` reaches `Terminated(MemoryBudget)` on the kept closures alone, so it also passes when the per-iteration record is not charged; it needs a control run, the same loop with `var` under the same budget not terminating (or `heap_bytes()` per iteration compared between the two). — .engineering/planning/story/let-const-and-block-scopes.md:119
story:let-const-and-block-scopes — the acceptance runs neither `--test scripts`, `--test realms` nor `--test names`, which hold the existing tests of `script.rs`, `realm/mod.rs` and the record lookup this story rewrites, so a regression there passes the acceptance. — .engineering/planning/story/let-const-and-block-scopes.md:127
story:template-literals — `a_template_longer_than_string_length_ends_as_concatenation_does` is described as "a substitution that doubles a string", which can hit the limit in the substitution's own `+` and pass while the template's concatenation is unbounded; it needs each substitution under `string_length` and only the template's concatenation past it. — .engineering/planning/story/template-literals.md:77
story:template-literals — the acceptance runs `--test limits_scripts` but not `--test budgets` or `--test eval_stack`, while its Stack line says the stack tests do not change and `interp/dispatch.rs` gains an arm. — .engineering/planning/story/template-literals.md:85
story:template-literals — the new parser recursion through a substitution gets no kind in `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` (`crates/syntax/tests/sources.rs:462`, not in Writes), so the acceptance shows the nesting count (40 over 32) but not that its frames fit the parser's stack bound. — .engineering/planning/story/template-literals.md:74
story:arrow-functions — the new recursion through an arrow's concise or block body gets no kind in `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` (`crates/syntax/tests/sources.rs:462`, written only "for the arm"), so nothing in the acceptance shows nested arrows fit the parser's stack bound. — .engineering/planning/story/arrow-functions.md:93
story:default-and-rest-parameters — `rest_charges_a_step_per_element` goes through `Function.prototype.apply`, whose CreateListFromArrayLike already charges a step per element (story:function-apply-and-source-text, Change), so the 10,000-step budget ends inside `apply` and the test passes even if the rest parameter charges nothing. — .engineering/planning/story/default-and-rest-parameters.md:83
story:default-and-rest-parameters — the acceptance writes `dynamic_code.rs` and runs `--test eval` but not `--test dynamic_function`, the existing binary of CreateDynamicFunction, whose parameter text now takes defaults and rest. — .engineering/planning/story/default-and-rest-parameters.md:91
story:default-and-rest-parameters — the new recursion through a parameter initializer (`function f(a = function (b = ...) {}) {}`) gets no kind in `NESTING_KINDS` of `crates/syntax/tests/sources.rs:462`, so the acceptance does not show it fits the parser's stack bound. — .engineering/planning/story/default-and-rest-parameters.md:80
story:object-destructuring — nested binding patterns and the nested re-reading of an object literal as an ObjectAssignmentPattern get no kind in `NESTING_KINDS` of `crates/syntax/tests/sources.rs:462`, so the acceptance does not show either recursion fits the parser's stack bound. — .engineering/planning/story/object-destructuring.md:84
story:object-destructuring — the acceptance runs `--test limits_scripts` but not `--test budgets` or `--test eval_stack`, while its Stack line says the stack tests do not change and `interp/dispatch.rs` gains arms. — .engineering/planning/story/object-destructuring.md:95

**What I read:** all 5 of the 5 stories given, whole, with `aep plan artifact show`. I also read the brief, `AGENTS.md`, `story:function-apply-and-source-text`, `taskfiles/host-requirements.yml`, the refused fixtures, `crates/syntax/tests/sources.rs`, the existing test binaries and `crates/test262/tests/run.rs`. Every base number in the acceptances matches the files. That covers the failing runs of each list (493, 86, 185, 176, 821), every (b) count (610, 112, 224, 110, 114), every (c) base (2/0/16, 0, 45, 0/25/212, 130/76/93), every list count by directory, and the Context counts (`616 398`, `90`, `218`, `80 22`, `1092 61`), checked with `awk` and `grep` over `.out/test262/runs.tsv`, `lines.txt` and `docs/plan/slice-06/*.cases.txt`.

**What I could not establish:**
- Whether the coordinator removes cases from the `*.cases.txt` files themselves. Each (a) says "less the ones the wave record takes off it", but the command reads only the list file, and no story lists that file in Writes.
- Whether `<!-- x` is refused through `Function` as well as through `eval`. That is the probe that replaces `let x;` in `host_164_refusals_counts_every_refusal_caught_or_not`.
- Out of my lane (parallel-safety): story 1 says it writes `crates/test262/tests/run.rs` "line 607 only", but the string it changes spans lines 606 and 607.
- Out of my lane (design): story 5 makes a CoverInitializedName "an early error elsewhere". Read literally, that gives a valid `for ({ x = 1 } of ...)` head a SyntaxError instead of the `for-of statement` refusal.

Scratch directory `.out/critic-acceptance/` removed.

```findings
- file: .engineering/planning/story/let-const-and-block-scopes.md
  line: 119
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`the_per_iteration_record_is_charged` reaches `Terminated(MemoryBudget)` on the kept closures alone, so it also passes when the per-iteration record is not charged; it needs a control run, the same loop with `var` under the same budget not terminating (or `heap_bytes()` per iteration compared between the two)."
- file: .engineering/planning/story/let-const-and-block-scopes.md
  line: 127
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs neither `--test scripts`, `--test realms` nor `--test names`, which hold the existing tests of `script.rs`, `realm/mod.rs` and the record lookup this story rewrites, so a regression there passes the acceptance."
- file: .engineering/planning/story/template-literals.md
  line: 77
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`a_template_longer_than_string_length_ends_as_concatenation_does` is described as \"a substitution that doubles a string\", which can hit the limit in the substitution's own `+` and pass while the template's concatenation is unbounded; it needs each substitution under `string_length` and only the template's concatenation past it."
- file: .engineering/planning/story/template-literals.md
  line: 85
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs `--test limits_scripts` but not `--test budgets` or `--test eval_stack`, while its Stack line says the stack tests do not change and `interp/dispatch.rs` gains an arm."
- file: .engineering/planning/story/template-literals.md
  line: 74
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new parser recursion through a substitution gets no kind in `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` (`crates/syntax/tests/sources.rs:462`, not in Writes), so the acceptance shows the nesting count (40 over 32) but not that its frames fit the parser's stack bound."
- file: .engineering/planning/story/arrow-functions.md
  line: 93
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new recursion through an arrow's concise or block body gets no kind in `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` (`crates/syntax/tests/sources.rs:462`, written only \"for the arm\"), so nothing in the acceptance shows nested arrows fit the parser's stack bound."
- file: .engineering/planning/story/default-and-rest-parameters.md
  line: 83
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`rest_charges_a_step_per_element` goes through `Function.prototype.apply`, whose CreateListFromArrayLike already charges a step per element (story:function-apply-and-source-text, Change), so the 10,000-step budget ends inside `apply` and the test passes even if the rest parameter charges nothing."
- file: .engineering/planning/story/default-and-rest-parameters.md
  line: 91
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance writes `dynamic_code.rs` and runs `--test eval` but not `--test dynamic_function`, the existing binary of CreateDynamicFunction, whose parameter text now takes defaults and rest."
- file: .engineering/planning/story/default-and-rest-parameters.md
  line: 80
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new recursion through a parameter initializer (`function f(a = function (b = ...) {}) {}`) gets no kind in `NESTING_KINDS` of `crates/syntax/tests/sources.rs:462`, so the acceptance does not show it fits the parser's stack bound."
- file: .engineering/planning/story/object-destructuring.md
  line: 84
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "nested binding patterns and the nested re-reading of an object literal as an ObjectAssignmentPattern get no kind in `NESTING_KINDS` of `crates/syntax/tests/sources.rs:462`, so the acceptance does not show either recursion fits the parser's stack bound."
- file: .engineering/planning/story/object-destructuring.md
  line: 95
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs `--test limits_scripts` but not `--test budgets` or `--test eval_stack`, while its Stack line says the stack tests do not change and `interp/dispatch.rs` gains arms."
```
