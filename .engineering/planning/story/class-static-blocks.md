---
format: aep.planning-md/3
id: story:class-static-blocks
kind: story
status: draft
title: Static blocks, run in the order of the static elements
summary: 33 cases (66 runs); lifts class static block.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
scope:
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-static-block.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-static-block.js
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: inferred
  path: crates/swell/tests/class_static_blocks.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-static-block.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-static-block.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs static blocks: ClassStaticBlockDefinitionEvaluation (`#sec-runtime-semantics-classstaticblockdefinitionevaluation`: a function with [[HomeObject]] the constructor), called with the constructor as `this` during the class's definition, in the order of the static elements with the static fields (`#sec-runtime-semantics-classdefinitionevaluation`, the loop over `staticElements`); its own var scope, `new.target` undefined, SuperProperty through the constructor, `await` reserved, an abrupt completion ending the class's definition. It lifts the hold `class static block is not supported before slice 8` of `story:class-element-syntax-and-early-errors`. Order 3, after `story:class-definition-evaluation` and `story:class-element-syntax-and-early-errors`, built beside `story:derived-classes-and-super-call`, `story:public-class-fields` and `story:private-names-and-private-methods`.

The list is `docs/plan/slice-08/story-static-blocks.cases.txt`: the 33 cases of the key `static-blocks` of `docs/plan/slice-08/scan.tsv` (a class with a static block and no heritage, field, private name or SuperCall; no negative case). By directory: the `static-init-await-binding-valid.js` cases of `language/statements/{variable,try,let,function,const,...}` and `language/expressions/object/method-definition` (`await` as a binding name inside a function in a static block), `language/statements/class/static-init-*` (the block's statements, `this`, scope, `arguments` of an enclosing function, the order with other static elements) and others. Their 66 runs fail on the plan's base run, all on `class is not supported before slice 8`. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`). Static blocks that read private names are on the list of `story:private-fields-and-private-in`, those with fields on that list or `story:public-class-fields`'s by the closure of `docs/plan/slice-08/scan.awk`.

Design it builds on: design change 1 of this decomposition (the section "Classes", written by a design author before the units start): a static block compiles to a synthetic method of the class with home object the constructor, run as a frame of the run loop in the list of static elements. The unit reads the text that landed and follows it where it differs.

## What later stories reuse

The static block's synthetic method and its place in the list of static elements (`story:private-fields-and-private-in` runs static blocks that read private fields; the order of static fields and blocks is one list, shared with `story:public-class-fields`).

## Domain relations

- A static block → the class constructor that is its home object and its `this` (one) — inferable (inferred from ECMA-262 `#sec-runtime-semantics-classstaticblockdefinitionevaluation`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax/src/parser/class.rs`: the hold `class static block` is lifted (its line); the documentation of `held` in `parser/mod.rs` drops its sentence; `crates/syntax/tests/sources/refused/class-static-block.js` moves to `valid/`.
- `swell-compiler`, `crates/compiler/src/class.rs`: the arm of a static block (a function template whose body is the block, a var scope of its own, `this` the constructor, the home object the constructor), placed in the list of static elements; `resolve.rs`: the block's scope (`var` local to the block, lexical declarations, functions declared in it). A compiler test `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new).
- `crates/swell/src/interp/class.rs`: the block run in the list of static elements as a frame of the run loop (its completion discarded, an abrupt one ending the class's definition).
- Tests, in `crates/swell/tests/class_static_blocks.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: a block's `this`, its `var` not visible outside, `super.x` in it, `new.target` undefined, two blocks and their order, a throw from a block (the class binding stays in its TDZ for a declaration), `await` as a binding name in a nested function; a block that loops stopped by the step budget (`Termination::StepBudget`) and by an interrupt (`Termination::Interrupted`); a recursion that defines classes with static blocks stopping at `frame_depth` with a RangeError.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 350 lines (compiler 100, engine 60, tests 190): one session, the smallest of the slice; it is kept apart because it lifts its own hold and shares no hunk with the field arm but the list of static elements.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; the block runs as a frame of the run loop, never through a reentry; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold line, the documentation of `held`), `crates/syntax/tests/sources/{refused,valid}/class-static-block.js` (moved); `crates/compiler/src/{class,resolve}.rs`, `crates/compiler/tests/sources/class-static-block.{js,disasm}` (new); `crates/swell/src/interp/class.rs`, `crates/swell/tests/{class_static_blocks,main}.rs`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/swell/src/object/`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 3, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` and the documentation of `held` (one hold line and one sentence each); `crates/compiler/src/class.rs` (each its own element arm); `crates/compiler/src/resolve.rs` (`story:derived-classes-and-super-call`, the `this` of a derived constructor; this story, the block's scope); `crates/swell/src/interp/class.rs` (`story:public-class-fields` the static fields in the list of static elements, this story the static blocks in the same walk, one arm each; the seam's loops are the other stories'); `crates/swell/tests/main.rs` (generated).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^class_static_blocks::/) | test(/^classes::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'class static block is not supported\|"class static block", 8' crates` prints nothing (on the unit's base it prints the hold line and the fixture's header). Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-static-blocks.cases.txt" > .out/story-static-blocks.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of the integrity, key and entry functions of `Object` and `JSON.stringify`), `cat .out/story-static-blocks.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 556 cases, 1,106 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-static-blocks.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 66 lines `failing` and `passed 523 bound 523 B 523 F 33 R 33`); (b) `awk -F'\t' '$6 ~ /class (static block )?is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (66 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Static blocks that read private names (`story:private-fields-and-private-in`); static fields (`story:public-class-fields`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
