---
format: aep.planning-md/3
id: story:private-fields-and-private-in
kind: story
status: draft
title: 'Private fields, every assignment form with a private target, and #x in'
summary: 987 cases (1,972 runs); lifts private field and private name in.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:instance-elements-initialization-order
- depends_on: story:class-static-blocks
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-private-fields.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-private-fields.js
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/object/private_names.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/swell/tests/private_fields.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-name-in.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/private-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/private-name-in.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs private fields and `#x in`: the field kind of a private element, PrivateFieldAdd (`#sec-privatefieldadd`: a TypeError when the object has the field, the case of a constructor that returns another object and stamps it), DefineField with a Private Name (`#sec-definefield`), private fields in the fields list of the seam in their order with public ones, static private fields defined on the constructor in the order of the static elements (with static blocks that read them), PrivateGet and PrivateSet of a field, every assignment form with a private reference as its target (`=`, compound, `&&=`, `||=`, `??=`, `++` and `--`, destructuring targets, `for (this.#x of ...)`), private references in optional chains (`o?.#x`, `o?.a.#x`), and `#x in o` (`#sec-relational-operators-runtime-semantics-evaluation`: a TypeError when `o` is not an object, else whether PrivateElementFind finds the name, for fields, methods and accessors). A private field may be added to a non-extensible object (the feature `nonextensible-applies-to-private` is excluded, `conformance/test262/features.txt`). It lifts the holds `private field is not supported before slice 8` and `private name in is not supported before slice 8` of `story:class-element-syntax-and-early-errors`. Order 5, after `story:instance-elements-initialization-order` (the seam in its final order, and through it the derived, field and private-method stories) and `story:class-static-blocks` (the static elements' walk), built beside `story:eval-in-class-field-initializers`.

The add goes through the one add function of `story:private-names-and-private-methods` (`crates/swell/src/object/private_names.rs`), so that `story:host-ensure-can-add-private-element` gives both kinds the hook in one place.

The list is `docs/plan/slice-08/story-private-fields.cases.txt`: the 987 cases of the key `private-fields` of `docs/plan/slice-08/scan.tsv` (every class with a private field or `#x in`, and the classes that need every part of the slice, outside eval; no negative case). By directory: `language/{statements,expressions}/class/elements` 839 (`privatefield*`, `private-field-*`, `privatename-*`, `*-private-field-*`, `static-private-*`, `private-*-in-*`), `language/statements/class/static-init-*` 4 (static blocks that read private fields), `language/identifiers/start-unicode-*-class*.js` 2 (a class with a private field per code point of a Unicode version: large sources, which the runner's time bound covers), and the private-in, compound-assignment and optional-chaining cases of `language/expressions`. Their 1,972 runs fail on the plan's base run: 1,968 on `class is not supported before slice 8` and 4 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: design change 2 of this decomposition (the section "Private names", written by a design author before the units start): the field kind of an element in [[PrivateElements]] and its charge. The unit reads the text that landed and follows it where it differs.

## What later stories reuse

The field kind and its add (`story:host-ensure-can-add-private-element` calls the hook in the add for both kinds; `story:private-names-in-eval-code` reads and writes private fields from eval code with these instructions); the `#x in` instruction.

## Domain relations

- An object → its private fields (at most one per Private Name, added once, never removed) — inferable (inferred from ECMA-262 `#sec-privatefieldadd` and `#sec-privateelement-specification-type`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax/src/parser/class.rs`: the holds `private field` and `private name in` are lifted (their lines); the documentation of `held` in `parser/mod.rs` drops their sentences and states that no construct of slice 8 holds a refusal any more; `crates/syntax/tests/sources/refused/{private-field,private-name-in}.js` move to `valid/`.
- `swell-compiler`: `crates/compiler/src/class.rs`, the arm of a private field (instance and static, its initializer in the synthetic method of its placement); `expression.rs`, a private reference as an assignment target of every form and in a destructuring pattern or a `for of` head, and `Expr::PrivateIn`; the instructions of a private field add and of `#x in` in `bytecode.rs` and `disasm.rs`. A compiler test `crates/compiler/tests/sources/class-private-fields.{js,disasm}` (new).
- `crates/swell`: the field kind in `object/private_names.rs` (add, get, set; a TypeError on a second add), the private fields in the seam's fields list and the static walk (`interp/class.rs`), each new instruction's work in a function of its own called from its arm in `interp/dispatch.rs`.
- Tests, in `crates/swell/tests/private_fields.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: a private field read, written, compound-assigned, logically assigned, incremented, destructured into, iterated into; static private fields and a static block that reads them; `#x in o` for a field, a method, an accessor, a primitive (TypeError), an object without it (false), a Proxy-free object of another class; a constructor that returns an object stamped with a private field twice (TypeError on the second add); a private field on a frozen and on a non-extensible object (added); invisibility of private fields to `Object.getOwnPropertyNames`, `JSON.stringify`, `Object.assign`, structured listing by the host interface; 10,000 instances with 100 private fields within a `MemoryBudget` (`Termination::MemoryBudget` when too small; `heap_bytes()` back to its value after a collection), a collection at every safe point between an add and a read; a loop of `this.#x++` stopped by the step budget (`Termination::StepBudget`).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 750 lines (compiler 220, object model 120, engine 100, tests 310): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after (HOST-119: `story:host-ensure-can-add-private-element`).
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; each new instruction does its work in a function of its own; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the two hold lines, the documentation of `held`), `crates/syntax/tests/sources/{refused,valid}/{private-field,private-name-in}.js` (moved); `crates/compiler/src/{class,expression,bytecode,disasm}.rs`, `crates/compiler/tests/sources/class-private-fields.{js,disasm}` (new); `crates/swell/src/object/private_names.rs`, `crates/swell/src/interp/{class,dispatch}.rs`, `crates/swell/tests/{private_fields,main}.rs`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/swell/src/host/`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with `story:eval-in-class-field-initializers`, built at the same time, each in hunks of its own: `crates/compiler/src/class.rs` (that story, [[ClassFieldInitializerName]] on the synthetic initializer method; this story, the private field arm); `crates/swell/src/interp/class.rs` (that story sets the name on the initializer's frame; this story adds private fields to the fields list and the static walk); `crates/swell/tests/main.rs` (generated). Later: `crates/swell/src/object/private_names.rs` (`story:host-ensure-can-add-private-element`, the hook's call in the add), the private instructions (`story:private-names-in-eval-code`, reached from eval code).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^private_fields::/) | test(/^private_methods::/) | test(/^class_elements::/) | test(/^class_fields::/) | test(/^class_static_blocks::/) | test(/^classes::/) | test(/^builtins_object_integrity::/) | test(/^host_objects::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'private field is not supported\|private name in is not supported\|"private field", 8\|"private name in", 8' crates` prints nothing (on the unit's base it prints the two hold lines and the fixtures' headers), and `grep -rn 'not supported before slice 8' crates` prints nothing (every refusal of slice 8 is lifted when this story and the order before it have merged). Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-private-fields.cases.txt" > .out/story-private-fields.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (523 passing cases), `cat .out/story-private-fields.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 1,510 cases, 3,012 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-private-fields.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 1,972 lines `failing` and `passed 523 bound 523 B 523 F 987 R 987`); (b) `awk -F'\t' '$6 ~ /is not supported before slice 8/' .out/test262-part/runs.tsv | wc -l` prints `0` (1,968 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private names in eval code (`story:private-names-in-eval-code`); the call of HostEnsureCanAddPrivateElement (`story:host-ensure-can-add-private-element`); `nonextensible-applies-to-private` and decorators (excluded features); `Proxy` and `Reflect` (slice 9); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
