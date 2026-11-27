---
format: aep.planning-md/3
id: review-result:slice-08-round-1-acceptance
kind: review-result
status: active
title: 'Slice 8, round 1, the acceptance critic: needs-revision, 20 findings'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:class-static-blocks
- reviews: story:private-names-and-private-methods
- reviews: story:instance-elements-initialization-order
- reviews: story:private-fields-and-private-in
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 1
---
Plan critic, acceptance, round 1 on the fifteen draft stories of `epic:slice-08-classes`
(`plan/slice-08` at `999e949e`), 2026-11-26.
Verdict: needs-revision, 20 findings. The critic's answer, as returned:

needs-revision

story:super-property-and-super-syntax — the grep in the acceptance can never print nothing: on the base it prints 8 lines, not 2, and 3 of them are in files the story may not write (`conformance/test262/later-parts.tsv:311`, `docs/design/engine.md:3567` and `:3620`, while line 91 says "Not written: … `conformance/`, … `docs/`"). The fix is to limit the grep to `crates` and name the 5 crate lines it prints on the base — .engineering/planning/story/super-property-and-super-syntax.md:97

story:class-element-syntax-and-early-errors — the grep `'"private name", 8\|private name is not supported' crates conformance …` also matches `conformance/test262/later-parts.tsv:313`, a file the story may not write (line 87), so it can never print nothing. The fix is to limit it to `crates` — .engineering/planning/story/class-element-syntax-and-early-errors.md:94

story:instance-elements-initialization-order — the acceptance passes on this story unbuilt. Line 52 expects most of the 448 cases to pass once order 3 has merged. The acceptance has no text check, and its nextest filter exits 0 without `class_elements::`. The fix is to name the number of `class_elements::` tests and, as the state before, the cases of the list (or a named test) that fail on the order-3 join — .engineering/planning/story/instance-elements-initialization-order.md:65

story:new-target-meta-property — the nextest filter also matches modules that exist on the base (`arrow_functions::`, `eval::`, …), so it exits 0 without `new_target::`. The fix is to name the number of new tests in each mode — .engineering/planning/story/new-target-meta-property.md:97

story:super-property-and-super-syntax — the same defect: the filter exits 0 without `super_property::`. The fix is to name the number of new tests — .engineering/planning/story/super-property-and-super-syntax.md:97

story:class-syntax-and-early-errors — the acceptance names no number of new tests or fixtures, so it passes with none written. The fix is to name the number of new fixtures and tests — .engineering/planning/story/class-syntax-and-early-errors.md:93

story:class-definition-evaluation — the filter exits 0 without `classes::`. The fix is to name the number of new tests — .engineering/planning/story/class-definition-evaluation.md:111

story:class-element-syntax-and-early-errors — the acceptance names no number of new fixtures or tests. The fix is to name it — .engineering/planning/story/class-element-syntax-and-early-errors.md:94

story:derived-classes-and-super-call — the filter exits 0 without `derived_classes::`. The fix is to name the number of new tests — .engineering/planning/story/derived-classes-and-super-call.md:105

story:public-class-fields — the filter exits 0 without `class_fields::`. The fix is to name the number of new tests — .engineering/planning/story/public-class-fields.md:83

story:class-static-blocks — the filter exits 0 without `class_static_blocks::`. The fix is to name the number of new tests — .engineering/planning/story/class-static-blocks.md:75

story:private-names-and-private-methods — the filter exits 0 without `private_methods::`. The fix is to name the number of new tests — .engineering/planning/story/private-names-and-private-methods.md:103

story:private-fields-and-private-in — the filter exits 0 without `private_fields::`. The fix is to name the number of new tests — .engineering/planning/story/private-fields-and-private-in.md:89

story:eval-in-class-field-initializers — the filter exits 0 without `class_field_eval::`. The acceptance has no text check either, so only test262 shows the work. The fix is to name the number of new tests — .engineering/planning/story/eval-in-class-field-initializers.md:65

story:private-names-in-eval-code — the filter exits 0 without `eval_private_names::`. The acceptance has no text check either. The fix is to name the number of new tests — .engineering/planning/story/private-names-in-eval-code.md:83

docs/plan/slice-08/check.sh — the script exits 0 on an empty case list, and also when the cases of the list have no run in `<runs after>`, so check (a) of every story passes when `git show` or `part.sh` leaves the list out; check (b) then prints 0 as well. The fix is for check.sh to exit 1 on an empty list or on a list case with no run after, and to print the number of list cases, which each acceptance then states — `sh docs/plan/slice-08/check.sh docs/plan/slice-08/story-instance-elements.cases.txt runs-all.tsv /dev/null` prints `passed 0 bound 0 B 0 F 0 R 0` and exits 0

story:super-property-and-super-syntax — the filter leaves out `async_methods::` and `async_generator_methods::`, whose tests expect `super is not supported before slice 8` (`crates/swell/tests/async_methods.rs:101`, `async_generator_methods.rs:232`). The acceptance therefore passes while those tests fail. The fix is to add both modules to the filter — .engineering/planning/story/super-property-and-super-syntax.md:97

story:class-definition-evaluation — the filter leaves out `dynamic_function::`, whose tests expect `class is not supported before slice 8` (`crates/swell/tests/dynamic_function.rs:808`, `:1228`), so they fail outside the acceptance once the hold is lifted. The fix is to add the module — .engineering/planning/story/class-definition-evaluation.md:111

story:private-names-in-eval-code — the acceptance requires `grep -c "pending slice 8$"` to print `1`, while its sibling in the same order, `host-ensure-can-add-private-element`, requires `0` (line 72), so on the join one of the two fails. The fix is to drop that check here, or require `0` on the join — .engineering/planning/story/private-names-in-eval-code.md:83

story:slice-08-conformance-close — the sum check compares with "the lines of `.out/test262-base/runs.tsv`'s count", but no command or file gives those two numbers (the base has only a runs file). The fix is to name the base `lines.txt` file, or the awk over the runs file — .engineering/planning/story/slice-08-conformance-close.md:85

**Time bounds.** Every code story names the limits: 60 s per test, 120 s for the gate's test steps, and `SLOT_TIMEOUT=30m` on the test262 job.

The test262 part of each story fits one pool job of 30 minutes by its number of cases. The largest part is 10.8 % of the 53,801 cases of the full run:

| Story | Cases in the part |
|---|---|
| new-target | 4,792 |
| super | 4,822 |
| class-syntax | 2,936 |
| classes | 5,834 |
| element-syntax | 3,355 |
| derived | 2,902 |
| public-fields | 1,038 |
| static-blocks | 556 |
| private-methods | 2,504 |
| instance-elements | 971 |
| private-fields | 1,510 |
| eval-fields | 2,630 |
| eval-private | 2,653 |
| host hook | none |
| close | 53,801, the full run, which fits one job by your premise |

**Counts.** All the part counts, failing lines, `passed/bound` lines and (b) counts the stories state equal what I recomputed from the decomposer's base run (`runs-all.tsv`). `lists.sh` over that run prints `lists: 9302 cases of slice 8, 9302 lines in 16 lists` and exits 0.

**What I read:** all 15 stories in full (the 15 ids in your brief), the scripts and lists under `docs/plan/slice-08/`, `taskfiles/test262.yml`, the runner's usage text in `crates/test262/src/main.rs`, and `waves-section.md`. I ran the stories' greps, `lists.sh`, and `check.sh` on empty inputs; nothing was built or tested and no `slot` command ran.

**What I could not establish:**
- The story command `count` gives no `--workers`, so the part runs on 1 thread. The time of a full run at 1 thread is not measured. A part of 10.8 % fits 30 minutes if the full run on 4 threads takes 30 minutes or less.
- Whether `host-requirements-check` refuses a HOST-119 line that names a test that does not exist; I assumed it does, which is why `host-ensure-can-add-private-element` has no count finding.
- "With the list as committed on the wave branch when the wave closes" does not say when a unit can run (a) before the wave closes.
- Out of my lane: the hunks shared within each order and the edges to slice 9's stories (parallel-safety and design critics).

```findings
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 97
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the grep for 'super is not supported' matches conformance/test262/later-parts.tsv:311 and docs/design/engine.md:3567 and 3620, files the story may not write, so it can never print nothing; limit it to crates and name the 5 base lines there"
- file: .engineering/planning/story/class-element-syntax-and-early-errors.md
  line: 94
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the grep for 'private name is not supported' matches conformance/test262/later-parts.tsv:313, a file the story may not write, so it can never print nothing; limit it to crates"
- file: .engineering/planning/story/instance-elements-initialization-order.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance passes unbuilt because most of the 448 cases are expected to pass on the order-3 join, there is no text check and the filter exits 0 without class_elements::; name the count of new tests and the cases or test that fail on the order-3 join"
- file: .engineering/planning/story/new-target-meta-property.md
  line: 97
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter also matches modules on the base, so it exits 0 without new_target::; name the count of new tests"
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 97
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without super_property::; name the count of new tests"
- file: .engineering/planning/story/class-syntax-and-early-errors.md
  line: 93
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance names no count of new tests or fixtures, so it passes with none written"
- file: .engineering/planning/story/class-definition-evaluation.md
  line: 111
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without classes::; name the count of new tests"
- file: .engineering/planning/story/class-element-syntax-and-early-errors.md
  line: 94
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance names no count of new fixtures or tests, so it passes with none written"
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 105
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without derived_classes::; name the count of new tests"
- file: .engineering/planning/story/public-class-fields.md
  line: 83
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without class_fields::; name the count of new tests"
- file: .engineering/planning/story/class-static-blocks.md
  line: 75
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without class_static_blocks::; name the count of new tests"
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 103
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without private_methods::; name the count of new tests"
- file: .engineering/planning/story/private-fields-and-private-in.md
  line: 89
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without private_fields::; name the count of new tests"
- file: .engineering/planning/story/eval-in-class-field-initializers.md
  line: 65
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without class_field_eval:: and there is no text check; name the count of new tests"
- file: .engineering/planning/story/private-names-in-eval-code.md
  line: 83
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the nextest filter exits 0 without eval_private_names:: and there is no text check; name the count of new tests"
- file: docs/plan/slice-08/check.sh
  line: 36
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check.sh exits 0 on an empty list or when the list cases have no run in the runs after, so check (a) of every story passes unbuilt; exit 1 on either and print the list count for the acceptance to state"
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 97
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the filter leaves out async_methods:: and async_generator_methods::, whose tests at async_methods.rs:101 and async_generator_methods.rs:232 expect the refusal this story lifts; add both modules"
- file: .engineering/planning/story/class-definition-evaluation.md
  line: 111
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the filter leaves out dynamic_function::, whose tests at dynamic_function.rs:808 and 1228 expect the class refusal this story lifts; add the module"
- file: .engineering/planning/story/private-names-in-eval-code.md
  line: 83
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance requires pending slice 8 to count 1 while its sibling host-ensure-can-add-private-element in the same order requires 0, so one fails on the join"
- file: .engineering/planning/story/slice-08-conformance-close.md
  line: 85
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the sum check compares with the lines of the base runs count, but no command or file gives those two numbers; name the base lines file or the awk over the runs file"
```
