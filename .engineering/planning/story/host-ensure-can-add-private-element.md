---
format: aep.planning-md/3
id: story:host-ensure-can-add-private-element
kind: story
status: draft
title: 'HOST-119: HostEnsureCanAddPrivateElement for host objects with an override table, and a stale handle in its Throw'
summary: No test262 case; closes HOST-119 and adds the next free requirement for the hook's Throw value (decision 6).
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:private-fields-and-private-in
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/src/host/hooks.rs
- confidence: inferred
  path: crates/swell/src/object/private_names.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/swell/tests/private_element_hook.rs
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: website/data/status.json
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story closes the one host requirement pending for slice 8, HOST-119 (`docs/design/host-interface.md`, line 1920 on the base): "The engine calls `host_ensure_can_add_private_element` only for host objects with an override table, and a `Throw` from it makes the private field definition throw that value." HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`) is called by PrivateFieldAdd and PrivateMethodOrAccessorAdd before the element is added; the hook's default answers `Ok(())` (`crates/swell/src/host/hooks.rs` line 194). Order 6, after `story:private-fields-and-private-in` (both kinds of add go through the one add function of `crates/swell/src/object/private_names.rs`), built beside `story:private-names-in-eval-code`.

It needs the override tables of a host object (`HostObjectSpec.overrides`, HOST-114, pending for slice 9 on the base; the code has no `overrides` field yet). Slice 9 is delivered before slice 8 (`order delivery 1,2,3,6,5,7,10,4,9,8,...`, section 12), so its tables exist when this story runs; the story of slice 9 that closes HOST-114 is not in the store when this decomposition is written, and the coordinator adds the edge `depends_on` to it when slice 9's stories are drafted (question Q5 of the decomposition's report).

It also takes, for this hook, the entry "Slices 2, 9 and 11: stale-handle requirement at hook return sites" of `docs/plan/waves.md`, "What later slices take over": decision 6 of `docs/design/host-interface.md` ("A handle the host returns": every handle in a value the host returns is checked before the engine uses it; "The other return sites ... get their own requirement when their slice is decomposed") applies to the `Abrupt::Throw` value of this hook, which the engine throws into the script. A new requirement states it: a `Throw` whose value holds a stale handle or a handle of another agent ends the run with `Terminated(EngineFault)`, `state()` reads `Faulted`, nothing panics (the form of HOST-160). Its number is the next free one, HOST-222 on the base (the highest is HOST-221), unless slice 9's decomposition takes it first, in which case the coordinator gives it the next.

No test262 case reaches the hook (test262 has no host object with an override table); the story's acceptance is the two requirements' tests and the host checks.

Design it builds on: decision 6 and the hook trait (section 16) of `docs/design/host-interface.md`; HOST-114's override tables as slice 9 lands them. Design change 5 of this decomposition (written by a design author into `docs/design/host-interface.md` before the unit starts, or by this unit as the first commit if the coordinator assigns it so): the new requirement's paragraph, and whether HOST-119's "only for host objects with an override table" stays (question Q4: keep it, the default, against calling the hook for every add, which `#sec-hostensurecanaddprivateelement` allows a host to observe for ordinary objects too). The unit reads the text that landed and follows it.

## What later stories reuse

Nothing of slice 8. The check of a hook's `Throw` value is the form decision 6 asks of every later hook return site.

## Domain relations

- None new: the hook's call site and the check of its answer are rules of `docs/design/host-interface.md`, decision 6.

## Change

Failing tests first: the two tests below are written before the code and fail on the base.

- `crates/swell/src/object/private_names.rs`, the add function: for an object that is a host object with an override table (as slice 9's code marks it), call `host_ensure_can_add_private_element` before the element is added; a `Throw` from it is checked (decision 6) and thrown to the script, the element not added; an object of any other kind calls nothing. The call goes through the hook dispatch as other hooks do (a reentry of the host: `reentry_depth` and the interrupt reach it).
- `crates/swell/src/host/hooks.rs`: the documentation of the hook's default names when the engine calls it (HOST-119).
- Tests, in `crates/swell/tests/private_element_hook.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: `host_119_*`: a host object with an override table (one override, as slice 9's API makes it) given a private field by `class C extends Base { #x = 1 }` with a base constructor that returns the host object; the hook is called once with that object, and when it throws a value, `new C` throws that value and the object has no `#x` (a second construct with a hook that answers `Ok(())` adds it); a private method add the same; an ordinary object and a host object without an override table do not call the hook. `host_222_*` (or the number given): the hook's `Throw` value holding a stale handle and a handle of another agent each end the run with `Terminated(EngineFault)`, `state()` reads `Faulted`, every entry point but `state()` and `refusals()` answers `Err(HostError::Faulted)`, nothing panics. A hook that loops is stopped by the interrupt (`Termination::Interrupted`).
- `conformance/host/requirements.txt`: `HOST-119 pending slice 8` becomes `HOST-119 test swell::private_element_hook::host_119_<name>`, and the line of the new requirement `HOST-222 test swell::private_element_hook::host_222_<name>`; `docs/design/host-interface.md`: the new requirement's paragraph after decision 6's list (if design change 5 has not landed it), HOST-119's text as Q4 decides; `CHANGELOG.md`: one line under `## [Unreleased]` naming the new requirement (the host surface's line of each `**HOST-` paragraph changes: `host-surface-check` counts it); `website/data/status.json`: written by `task site-data`, which reads `requirements.txt`.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 300 lines (engine 60, tests 200, requirement and documents 40): one session.

## Requirements

- HOST-119 (`ecma262:sec-hostensurecanaddprivateelement`) is closed: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `0` (1 on the base), and `slot task host-requirements-check` exits 0. The new requirement (HOST-222 or the number given) is stated in `docs/design/host-interface.md` with its slice `[slice 8]` and has its test line; `slot task host-surface-check` exits 0 with one change more than before, named in `CHANGELOG.md`; `slot task site-data-check` exits 0.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping); HOST-3's fuzz test (`fuzz::`) passes with the hook in its sequences. The run loop: nothing is added to the frame of `Agent::execute`; the hook's call is a reentry of the host bounded by `reentry_depth`; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/swell/src/object/private_names.rs` (the add function), `crates/swell/src/host/hooks.rs` (documentation only), `crates/swell/tests/{private_element_hook,main}.rs`, `conformance/host/requirements.txt`, `docs/design/host-interface.md` (the new paragraph and HOST-119's text, if design change 5 did not land them), `CHANGELOG.md` (one line under `## [Unreleased]`), `website/data/status.json` (by `task site-data`). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax`, `crates/compiler`, `crates/test262`, `conformance/test262/`, `docs/design/engine.md`.

Shared with `story:private-names-in-eval-code`, built at the same time, each in hunks of its own: `crates/swell/src/object/private_names.rs` (this story, the add function; that story, if it puts its lookup there, a function of its own), `CHANGELOG.md` (each one line under `## [Unreleased]`: the second to merge keeps both) and `crates/swell/tests/main.rs` (generated). `website/data/status.json` is the coordinator's file in a close; this story writes it only by `task site-data` in its own change, and the close of slice 8 writes it again.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test main -E 'test(/^private_element_hook::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^hooks::/) | test(/^host_objects::/) | test(/^fuzz::/) | test(/^misuse::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `slot task host-requirements-check`, `slot task host-surface-check` and `slot task site-data-check` exit 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `0`; `grep -c "^HOST-119 test swell::private_element_hook::host_119_" conformance/host/requirements.txt` prints `1`; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0. Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262: no case reaches the hook; the coordinator's full run on the join passes the checks of the other stories unchanged.

## Out of scope

Override tables (slice 9, HOST-114); the other hook return sites (their slices); private elements of ordinary objects (no hook call, by HOST-119 as it stands); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
