---
format: aep.planning-md/3
id: review-result:slice-08-round-2-acceptance
kind: review-result
status: active
title: 'Slice 8, round 2, acceptance: needs-revision, 3 blockers, 4 warnings and 4 notes'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:private-names-and-private-methods
- reviews: story:class-static-blocks
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-fields-and-private-in
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 1
---
Plan critic, acceptance, round 2 on the fourteen draft stories of
`epic:slice-08-classes` (`plan/slice-08` at `238e8d64`), 2026-11-26.
Verdict: needs-revision. The 20 findings of round 1 are closed; 3 blockers, 4 warnings and 4 notes are open.

needs-revision

All 20 round-1 findings are closed. The revision then opens 2 blockers and 4 warnings: acceptance commands whose stated output cannot be produced, or that cannot pass when the story is built as written. I also list 4 notes.

**Round 1 closure.**
- The two `grep`s limited to `crates` (`super-property-and-super-syntax.md:103`, `class-element-syntax-and-early-errors.md:101`) match my own runs on the tree: 5 and 2 lines on the base.
- The fifteenth story is folded: its 446 cases are on `story-private-fields.cases.txt` with 21 tests, and no story still names it.
- All 13 new-test counts match the named tests (new-target 14, super 14, classes 15, derived 18, public fields 13, static blocks 11, private methods 17, private fields 21, field eval 13, eval private 16, host 10). Fixture counts also match: 22 for class syntax, 19 for element syntax, 4 for new-target, 7 for super.
- `check.sh` exits 1 on an empty list and prints `missing` for a list case the part lacks.
- The part and (b) numbers match my reruns for all 12 list stories: `failing`, `passed … bound … B … F … R …`, and the (b) counts (64, 128, 917, 1,746, 6,484, 630, 1,033, 70, 3,912, 2,856, 164, 196). `lists.sh` ends in `slice 9302 lists 9339 incoming 37`, and the close's `53801 103360` equals the sum over `conformance/test262/expected.txt`.
- `async_methods::`, `async_generator_methods::` and `dynamic_function::` are in the filters. Only the host story moves the `pending slice 8` count: 1, then 2, then 0.
- The close's sum check is now a command with a stated output. The pinned nextest is 0.9.143 (`.github/workflows/slow.yml`), which supports `--no-tests=fail`.

Findings, one line each (artifact — reason — citation):

story:derived-classes-and-super-call — `ls crates/syntax/tests/sources/early-errors/super-call-in-*.js | wc -l` is stated as `4` (`0` on the base). `super-property-and-super-syntax` already adds `early-errors/super-call-in-object-method.js`, so it prints 1 on the unit's base and 5 after a correct build — `.engineering/planning/story/derived-classes-and-super-call.md:107`

story:class-syntax-and-early-errors — the acceptance requires `swell-syntax --test main` to pass, but `valid/class-syntax.js` holds a class while the `class` hold is on. `valid_sources_parse_in_both_modes` needs the header `parses` and panics on any `Err`, and `analyse_parsed` returns the held refusal as `Err`. The new nesting kinds contain `class`, so `admitted` in `fits_the_stack_bound` returns `None` and asserts. The story writes neither test — `.engineering/planning/story/class-syntax-and-early-errors.md:82` and `:97`

story:class-element-syntax-and-early-errors — the same defect: `valid/class-elements.js` is "held", and the nested sources `class { x = class { … } }` and `class { static { class { static { … } } } }` return the held refusal. The suite named in the acceptance cannot pass in this story's tree — `.engineering/planning/story/class-element-syntax-and-early-errors.md:86` and `:101`

story:super-property-and-super-syntax — the acceptance expects `grep -rln 'super in an arrow function is not supported before slice 8\|super in eval code …' crates | wc -l` to print `3`. The parser calls `refuse_at("super in an arrow function", 8, …)`, and `SyntaxError::refused` joins the text at run time. Only the two fixture headers carry the literal, so a correct build prints 2. Match the call form `"super in an arrow function", 8` as the other greps do — `.engineering/planning/story/super-property-and-super-syntax.md:103`

story:super-property-and-super-syntax — `refused/super-in-eval.js` cannot yield the eval-code refusal through `refused_sources_name_their_slice`, which parses as a script (`sources.rs:225`). A script `super.x` is an early error, not the refusal. The `derived-classes-and-super-call` story moves the same fixture to `valid/`, where a script parse also fails — `.engineering/planning/story/super-property-and-super-syntax.md:88` and `derived-classes-and-super-call.md:88`

story:class-syntax-and-early-errors — `refused/class-heritage.js` cannot hold one message across the stories. `Parser::held` keeps the first held construct, so with the `class` hold on, `class A extends B {}` answers `class is not supported`. After `class-definition-evaluation` lifts that hold it answers `class heritage is not supported`, and no story edits the header in between, so `refused_sources_name_their_slice` fails in one of them — `.engineering/planning/story/class-syntax-and-early-errors.md:82`

story:class-element-syntax-and-early-errors — the five refused fixtures (`private-method`, `private-field`, `private-name-in`, and `class-field` and `class-static-block` "now held") name element-hold messages. In this story's tree the class hold comes first, because `class-definition-evaluation` is built beside it and not merged, so each fixture answers `class is not supported` — `.engineering/planning/story/class-element-syntax-and-early-errors.md:86`

Notes (do not hold the set back):
- `private-fields-and-private-in.md:99` says `not supported before slice 8` has 4 lines on the base, "the two hold calls and two fixture headers". The hold calls do not carry the text, so it is 2. The expected `0` still differs from the base. `derived-classes-and-super-call.md:107` has the same miscount ("6 on its base", really 4).
- `eval-in-class-field-initializers.md:66`: the `ContainsArguments` grep on `lib.rs` could already pass on the unit's base. The `parse_eval` documentation that `new-target-meta-property` rewrites may name the rule, so the grep adds nothing the 13 tests do not.
- `private-names-and-private-methods.md:118`: `grep 'RECORD_BYTES: usize = 208' … | wc -l` must print `2`, but the story's own fallback sets the measured value if it is not 208, so a correct fallback build fails the grep.
- `new-target-meta-property.md:95` requires a `CHANGELOG.md` line, and `private-names-in-eval-code.md` the same, but neither acceptance greps for it.

What I read: all 14 stories in full, the round-1 acceptance record, `revision-1.md` and `revision-1-answer.md`, `check.sh`, `part.sh`, `lists.sh`, the lists and `runs-all.tsv`, and the tree sites the stories name. I reran `check.sh` and the (b) filters per part and ran the `check.sh` refusals. `design-answer.md`, `waves-section.md` and the `git diff 61f2a8e2..ed4e9ec9` text were not read in full; only the story bodies were judged.

What I could not establish:
- The time of any test.
- Whether `host-requirements-check` rejects a `test` line naming a test that does not exist.
- The sizes of the object record, `SlotValue` and the `constructed` enum.
- The exact content of the fixtures the stories name, which do not exist yet.
- How the wave sets `WAVE` before it closes.

Design and parallel-safety questions, including the `hold` semantics themselves, are out of my lane. Only their effect on whether an acceptance can pass is above.

```findings
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 107
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "ls early-errors/super-call-in-*.js is stated as 4 (0 on the base) but super-call-in-object-method.js from the super story already matches it, so it prints 1 on the base and 5 after a correct build; name the four new files or a pattern that matches only them"
- file: .engineering/planning/story/class-syntax-and-early-errors.md
  line: 97
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "swell-syntax --test main cannot pass: valid/class-syntax.js holds a class while the class hold is on (valid_sources_parse_in_both_modes needs header parses and panics on Err) and the two new nesting kinds contain class so admitted returns None in fits_the_stack_bound; neither test is in the Writes"
- file: .engineering/planning/story/class-element-syntax-and-early-errors.md
  line: 101
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "swell-syntax --test main cannot pass in this tree: valid/class-elements.js is held and the nested class sources of the two new nesting kinds return the held refusal, which valid_sources_parse_in_both_modes and fits_the_stack_bound reject"
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 103
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "grep -rln for the literal 'super in an arrow function is not supported before slice 8' is stated to print 3 but the parser calls refuse_at with the construct only, so a correct build prints 2; match the call form '\"super in an arrow function\", 8' like the other greps"
- file: .engineering/planning/story/super-property-and-super-syntax.md
  line: 88
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "refused/super-in-eval.js cannot yield the eval-code refusal through refused_sources_name_their_slice, which parses a script (sources.rs:225), and the derived story moves it to valid/ where a script parse also fails; name another test or entry for it"
- file: .engineering/planning/story/class-syntax-and-early-errors.md
  line: 82
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "refused/class-heritage.js cannot name one message across stories: Parser::held keeps the first hold so it answers class is not supported until class-definition-evaluation lifts the class hold and class heritage after it, and no story edits the header in between"
- file: .engineering/planning/story/class-element-syntax-and-early-errors.md
  line: 86
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "the five refused fixtures (private-method, private-field, private-name-in, class-field, class-static-block) name element-hold messages but in this story's tree the class hold is first, so refused_sources_name_their_slice sees class is not supported"
- file: .engineering/planning/story/private-fields-and-private-in.md
  line: 99
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'4 on its base: the two hold calls and the headers of two fixtures' is wrong since hold calls do not carry the text (SyntaxError::refused builds it), so the base prints 2; the expected 0 still differs from the base; derived-classes-and-super-call.md:107 miscounts the same way (6, really 4)"
- file: .engineering/planning/story/eval-in-class-field-initializers.md
  line: 66
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "grep -c ContainsArguments in lib.rs may already pass on the unit's base because new-target-meta-property rewrites the parse_eval documentation to name each rule; grep for contains_arguments in early.rs instead"
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 118
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the grep for RECORD_BYTES = 208 printing 2 contradicts the story's own fallback that sets the measured value when it is not 208; state the expected line for the fallback or make the layout test the check"
- file: .engineering/planning/story/new-target-meta-property.md
  line: 95
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the one CHANGELOG.md line for the parse_eval change is a requirement with no command in the Acceptance (same in private-names-in-eval-code.md)"
```
