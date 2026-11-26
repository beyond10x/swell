---
format: aep.planning-md/3
id: story:instance-elements-initialization-order
kind: story
status: archived
title: 'InitializeInstanceElements as a whole: private methods before fields, at a base construct and after super()'
summary: 'Folded in revision 1 after round 1 of the plan critics: its 448 cases went to private-fields-and-private-in (446) and public-class-fields (2); no story names it.'
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:derived-classes-and-super-call
- depends_on: story:public-class-fields
- depends_on: story:private-names-and-private-methods
scope:
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-elements-order.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-elements-order.js
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: inferred
  path: crates/swell/tests/class_elements.rs
- confidence: cited
  path: crates/swell/tests/main.rs
revision: 5
transitions:
- {from: "draft", to: "archived", at: "2026-11-26T07:08:46Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is InitializeInstanceElements as a whole (`#sec-initializeinstanceelements`: every private method and accessor of the constructor added first, then every field defined in order) and its two callers together: a base class's [[Construct]] and the SuperCall of a derived class (after BindThisValue, on the result of the parent's construct, for the constructor the SuperCall belongs to, also when the SuperCall is in an arrow function). It owns the cases that need fields and private methods at once, or fields or private methods in a derived class: a field initializer that calls a private method or reads a private accessor, a private method that reads a field, an inner class whose private names shadow the outer's, `super.x` and `this` in a field initializer of a derived class, the `this` TDZ during the initializers of a derived class (`constructor-this-tdz-during-initializers.js`), and a derived constructor that returns an object before `super()` (the private methods are not added to it, `prod-private-*-before-super-return-*`). Order 4, after `story:derived-classes-and-super-call` (the SuperCall's call of the seam), `story:public-class-fields` (the fields loop) and `story:private-names-and-private-methods` (the methods loop): those three stories each own one part of the seam and are built at the same time; this story joins them, makes the order the clause states, and is the first to run them together.

The list is `docs/plan/slice-08/story-instance-elements.cases.txt`: the 448 cases of the key `instance-elements` of `docs/plan/slice-08/scan.tsv` (a class that needs fields and private methods, or fields or private methods with heritage or `super`, and no private field or static block; no negative case). By parts: fields with private methods 415 (the generated `*-rs-private-method*`, `*-rs-private-getter*`, `*-rs-private-setter*` cases of `class/elements` with fields around them: `alt`, `usage`, `identifier` and `methods` variants), heritage or SuperCall with private methods 18, with fields 4, with both 9, SuperProperty with fields or private methods 5. By directory: `language/{statements,expressions}/class/elements` 447 and `language/expressions/class/constructor-this-tdz-during-initializers.js`. Their 896 runs fail on the plan's base run, all on `class is not supported before slice 8`. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: design change 1 of this decomposition (the section "Classes": the two lists of instance elements and the seam, the initializers as frames of the run loop) and design change 3 (the `this` slot of a derived constructor, which a field initializer of a derived class reads as its `this` after `super()`), both written by a design author before the units start. The unit reads the text that landed and follows it where it differs.

## What later stories reuse

The seam in its final form (`story:private-fields-and-private-in` adds private fields to the fields list, run in the same order; `story:eval-in-class-field-initializers` runs eval inside initializers of derived classes through it).

## Domain relations

- None new: the order of the instance elements is a property of `#sec-initializeinstanceelements`, read by this story from the clause.

## Change

Failing tests first: the tests below are written before the code and fail on the base (each part works alone; together they meet here).

- `crates/swell/src/interp/class.rs`: the seam runs the methods loop to its end before the first field initializer's frame is pushed (one walk, not two calls that can interleave with a frame); after `super()` it runs for the constructor whose SuperCall it is (the frame's function, or the function an arrow's `this` comes from), on the object `super()` returned; an abrupt completion of an initializer ends the SuperCall with it, after `this` is bound (`#sec-super-keyword-runtime-semantics-evaluation`, the order of BindThisValue and InitializeInstanceElements). A derived constructor that returns an object before `super()` gives that object as the result and adds nothing to it.
- `crates/compiler/src/class.rs`: a field initializer of a derived class reads `this` as an initializer of a base class does (the instance is its receiver; the TDZ is the constructor's, not the initializer's); a SuperProperty in an initializer reaches the home object of its placement. Each change with a compiler test (`crates/compiler/tests/sources/class-elements-order.{js,disasm}`, new) where it changes code.
- Tests, in `crates/swell/tests/class_elements.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: the order (a field initializer that calls `this.#m()` sees the method; a getter `#g` read by a field), the same with heritage after `super()` and with `super()` in an arrow called later, an initializer that throws (the instance is bound and the construct throws), a derived constructor returning an object before `super()`, the inner class shadowing an outer private method, `super.x` in an initializer of a derived class, a collection at every safe point between the methods loop and the last field; a class with 100 private methods and 100 fields constructed 1,000 times within the step budget with `steps_per_entry` small (`Termination::StepBudget`) and within a `MemoryBudget` (`heap_bytes()` back to its value after a collection); a recursion of `new` through `super()` in an initializer stopping at `frame_depth` with a RangeError.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 450 lines (engine 120, compiler 60, tests 270): one session; most of its 448 cases are expected to pass once the three stories it joins have merged, and the session's work is the order, the derived path and what the run shows.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; the initializers run as frames of the run loop, never through a reentry; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/swell/src/interp/class.rs` (the seam), `crates/compiler/src/class.rs` (an initializer's `this` and home object in a derived class), `crates/compiler/tests/sources/class-elements-order.{js,disasm}` (new), `crates/swell/tests/{class_elements,main}.rs`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax`, `crates/swell/src/object/private_names.rs` (the add, get and set as `story:private-names-and-private-methods` made them; a fault found there is reported with a failing test and fixed in the smallest change), `crates/test262`, `conformance/`, `website/`, `docs/`.

The only story of order 4. It edits the seam that `story:derived-classes-and-super-call`, `story:public-class-fields` and `story:private-names-and-private-methods` built, after all three have merged; `story:private-fields-and-private-in` and `story:eval-in-class-field-initializers` build on it in order 5.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^class_elements::/) | test(/^class_fields::/) | test(/^private_methods::/) | test(/^derived_classes::/) | test(/^classes::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-instance-elements.cases.txt" > .out/story-instance-elements.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (523 passing cases), `cat .out/story-instance-elements.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 971 cases, 1,936 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-instance-elements.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 896 lines `failing` and `passed 523 bound 523 B 523 F 448 R 448`; on the join of order 3 the unit's base run states how many of the 448 already pass, and the report names that number); (b) `awk -F'\t' '$6 ~ /is not supported before slice 8/' .out/test262-part/runs.tsv | wc -l` prints `0` (896 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private fields (`story:private-fields-and-private-in`); eval in initializers (`story:eval-in-class-field-initializers`); static elements (`story:public-class-fields`, `story:class-static-blocks`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
