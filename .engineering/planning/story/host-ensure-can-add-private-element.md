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
- depends_on: story:host-object-overrides
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
revision: 4
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story closes the one host requirement pending for slice 8, HOST-119 (`docs/design/host-interface.md`, the paragraph `**HOST-119**`): "The engine calls `host_ensure_can_add_private_element` only for host objects with an override table, and a `Throw` from it makes the private field definition throw that value." HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`) is called by PrivateFieldAdd and PrivateMethodOrAccessorAdd before the element is added; the hook's default answers `Ok(())` (`crates/swell/src/host/hooks.rs`, `Host::host_ensure_can_add_private_element`). It adds the requirement the design names for this hook, `HOST-223` (HOST-222 is slice 9's, `story:host-object-overrides`).

Design it builds on, by name: `docs/design/engine.md`, section 3, "Private names": "What a host sees: `host_ensure_can_add_private_element` (HOST-119) before an add to a host object with an override table, and nowhere else. The requirement that the story of the hook adds is `HOST-223`", and item 2 (one add function in `object/private_names.rs` serves both adds; step 1: for a host object with an override table it calls the hook, and a `Throw` from it is the add's throw, `HOST-223`; step 2: an entry already there is a `TypeError`; step 3: the entry is appended; the add reads no [[Extensible]] and goes through no trap); `docs/design/host-interface.md` decision 6 ("A handle the host returns": every handle in a value the host returns is checked before the engine uses it; HOST-162 is its form for the hooks of slice 2) and section 15 (`HostObjectSpec<H>` with `overrides: Option<&'static Overrides<H>>`, as `story:host-object-overrides` lands it). The one sentence the design does not hold yet is the paragraph of HOST-223, which this story writes as its first commit (see Change).

Edges: `story:private-fields-and-private-in` (both kinds of add go through the one add function of `crates/swell/src/object/private_names.rs`, made by `story:private-names-and-private-methods` and extended by that story) and `story:host-object-overrides` (slice 9: the override table the hook is called for, and HOST-222 before HOST-223 in both files; slice 9 is delivered before slice 8, section 12). Order 6, built beside `story:private-names-in-eval-code`, which shares only `CHANGELOG.md` and the generated `tests/main.rs` with it.

No test262 case reaches the hook (the runner makes no host object with an override table); the story's acceptance is the tests of the two requirements and the host checks.

## What later stories reuse

Nothing of slice 8. The check of a hook's `Throw` value is the form decision 6 asks of every later hook return site.

## Domain relations

- None new: the hook's call site and the check of its answer are rules of `docs/design/host-interface.md`, decision 6 (`spec/domains/host.yaml` marks host objects and their overrides as not mapped).

## Change

First commit, before any code (the coordinator's decision 4 of revision 1): `docs/design/host-interface.md` gets the paragraph `**HOST-223** (swell:D6) A \`host_ensure_can_add_private_element\` is called before a private method or accessor is added to a host object with an override table, as before a private field; a \`Throw\` from it is the add's throw and the element is not added; a \`Throw\` value that holds a stale handle or a handle of another agent ends the run with \`Terminated(EngineFault)\`, no element is added, \`state()\` then reads \`Faulted\`, every entry point and read but \`state()\` and \`refusals()\` answers \`Err(HostError::Faulted)\`, and nothing panics. [slice 8]`, placed after the paragraph of HOST-222 (or, if slice 9 placed HOST-222 elsewhere, after HOST-162 in decision 6's list); `conformance/host/requirements.txt` gets the line `HOST-223 pending slice 8` after the line of HOST-222. After this commit `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `2`.

Then, failing tests first: the tests below are written before the code and fail on that commit.

- `crates/swell/src/object/private_names.rs`, the one add function (step 1 of item 2, at its head, before the duplicate check): for an object whose kind is a host object with `overrides` set, call `H::host_ensure_can_add_private_element` through `HookCx` as `code_for_eval` in `crates/swell/src/dynamic_code.rs` calls its hook (the scope truncated after, `self.running()?` first: a terminated run ignores the answer, decision 10); an `Abrupt::Throw(value)` is checked with `heap.check_value` (decision 6: a stale or foreign handle is `self.fault()`) and is then the add's throw, the element not added; an object of any other kind (ordinary, function, array, proxy, a host object with no override table) calls nothing. The call is in a function of its own, `ensure_can_add_private_element`, beside the add function.
- `crates/swell/src/host/hooks.rs`: the documentation of the hook's default names when the engine calls it (HOST-119, HOST-223).
- Tests, each in both collection modes, in `crates/swell/tests/private_element_hook.rs` (new): `host_119_a_throw_from_the_hook_is_the_field_adds_throw` (a host object with one override, returned by a base constructor `function Base() { return h; }`, given `#x` by `class C extends Base { #x = 1 }`: the hook is called once with that object, and when it throws a value `new C` throws that value and `#x in h` is `false`); `host_119_an_ok_answer_adds_the_field`; `host_223_a_throw_from_the_hook_is_the_method_adds_throw` (a private method and a private accessor the same); `host_223_a_stale_handle_in_the_throw_faults_the_agent`; `host_223_a_handle_of_another_agent_in_the_throw_faults_the_agent`; `an_ordinary_object_calls_no_hook`; `a_proxy_calls_no_hook` (a `Proxy` returned by the base constructor: the element is added to the proxy, no trap and no hook runs); `a_host_object_without_an_override_table_calls_no_hook`; `a_second_add_after_an_ok_answer_is_a_type_error_and_calls_the_hook_once_more` (the hook runs before the duplicate check, as `#sec-privatefieldadd` orders it); `an_interrupt_set_in_the_hook_ends_the_run` (the hook calls `interrupt()` and answers `Ok(())`: `Terminated(Interrupted)`, the element not added): that is 10 tests under `private_element_hook::`, none on the base (the file is new). `crates/swell/tests/main.rs` generated again.
- `conformance/host/requirements.txt`: `HOST-119 pending slice 8` becomes `HOST-119 test swell::private_element_hook::host_119_a_throw_from_the_hook_is_the_field_adds_throw`, and `HOST-223 pending slice 8` becomes `HOST-223 test swell::private_element_hook::host_223_a_stale_handle_in_the_throw_faults_the_agent`; `CHANGELOG.md`: one line under `## [Unreleased]` naming HOST-223; `website/data/status.json`: written by `task site-data`, which reads `requirements.txt`.
- Sizes: no layout changes.
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); no test of this story loops (each makes at most 10 adds), so no reduced count is needed; the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 300 lines (engine 50, tests 220, requirement and documents 30): one session.

## Requirements

- This story owns the move of the `pending slice 8` lines of `conformance/host/requirements.txt`: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and on this story's base, `2` after its first commit, `0` after its last; every other story of slice 8 leaves the file unchanged. `slot task host-requirements-check` exits 0. `slot task host-surface-check` exits 0 with the change named in `CHANGELOG.md`; `slot task site-data-check` exits 0.
- The run loop is not touched; the hook's call is in a function of its own outside `Agent::execute`; `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `docs/design/host-interface.md` (the paragraph of HOST-223 only), `conformance/host/requirements.txt` (the lines of HOST-119 and HOST-223), `crates/swell/src/object/private_names.rs` (the head of the add function and `ensure_can_add_private_element`), `crates/swell/src/host/hooks.rs` (documentation only), `crates/swell/tests/private_element_hook.rs` (new), `crates/swell/tests/main.rs` (generated), `CHANGELOG.md` (one line), `website/data/status.json` (by `task site-data`). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax`, `crates/compiler`, `crates/swell/src/interp/`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/test262/`, `docs/design/engine.md`.

Shared files: with `story:private-names-in-eval-code`, built at the same time with no edge between them: `CHANGELOG.md` (each its own line under `## [Unreleased]`: the second to merge keeps both) and `crates/swell/tests/main.rs` (generated again on the join); no other file (that story writes no file under `crates/swell/src/object/`). Written before it by stories it depends on: `object/private_names.rs` (`story:private-names-and-private-methods`, then `story:private-fields-and-private-in`'s field kind; here the head of the add function), `conformance/host/requirements.txt`, `docs/design/host-interface.md` and `website/data/status.json` (`story:host-object-overrides`, the lines of HOST-114 and HOST-222; here HOST-119 and HOST-223). After it: `story:slice-08-conformance-close` writes `website/data/status.json` again.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^private_element_hook::/)'` runs 10 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^private_element_hook::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^hooks::/) | test(/^host_objects::/) | test(/^host_overrides::/) | test(/^fuzz::/) | test(/^misuse::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; each exits 0. `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `slot task host-requirements-check`, `slot task host-surface-check` and `slot task site-data-check` exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from). The text: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `0` (1 on the base); `grep -c '^HOST-119 test swell::private_element_hook::host_119_' conformance/host/requirements.txt` prints `1` (0 on the base); `grep -c '^HOST-223 test swell::private_element_hook::host_223_' conformance/host/requirements.txt` prints `1` (0 on the base); `grep -c '^\*\*HOST-223\*\*' docs/design/host-interface.md` prints `1` (0 on the base); in the first commit of the story the same grep prints `1` and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `2`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262: no case reaches the hook; the coordinator's full run on the join passes the checks of the other stories unchanged.

## Out of scope

Override tables (slice 9, HOST-114, `story:host-object-overrides`); the other hook return sites (their slices); private elements of objects that are not host objects with an override table (no hook call, by HOST-119); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
