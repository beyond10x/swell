---
format: aep.planning-md/3
id: story:host-ensure-can-add-private-element
kind: story
status: implemented
title: 'HOST-119: HostEnsureCanAddPrivateElement for host objects with an override table, and a stale handle in its Throw'
summary: No test262 case; closes HOST-119 and adds the next free requirement for the hook's Throw value (decision 6).
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:private-fields-and-private-in
- depends_on: story:host-object-overrides
scope:
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
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T08:23:49Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T08:23:49Z", actor: "human:operator", revision: 9, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T21:14:36Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story closes the one host requirement pending for slice 8, HOST-119 (`docs/design/host-interface.md`, the paragraph `**HOST-119**`): "The engine calls `host_ensure_can_add_private_element` only for host objects with an override table, and a `Throw` from it makes the private field definition throw that value." HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`) is called by PrivateFieldAdd and PrivateMethodOrAccessorAdd before the element is added; the hook's default answers `Ok(())` (`crates/swell/src/host/hooks.rs`, `Host::host_ensure_can_add_private_element`). It adds the requirement the design names for this hook, `HOST-223` (HOST-222 is slice 9's, `story:host-object-overrides`).

Design it builds on, by name: `docs/design/engine.md`, section 3, "Private names": "What a host sees: `host_ensure_can_add_private_element` (HOST-119) before an add to a host object with an override table, and nowhere else. The requirement that the story of the hook adds is `HOST-223`", and item 2 (one add function in `object/private_names.rs` serves both adds; step 1: for a host object with an override table it calls the hook, and a `Throw` from it is the add's throw, `HOST-223`; step 2: an entry already there is a `TypeError`; step 3: the entry is appended; the add reads no [[Extensible]] and goes through no trap); `docs/design/host-interface.md` decision 6 ("A handle the host returns": every handle in a value the host returns is checked before the engine uses it; HOST-162 is its form for the hooks of slice 2) and section 15 (`HostObjectSpec<H>` with `overrides: Option<&'static Overrides<H>>`, as `story:host-object-overrides` lands it). The one sentence the design does not hold yet is the paragraph of HOST-223, which this story writes as its first commit (see Change).

Edges: `story:private-fields-and-private-in` (both kinds of add go through the one add function of `crates/swell/src/object/private_names.rs`, made by `story:private-names-and-private-methods` and extended by that story) and `story:host-object-overrides` (slice 9: the override table the hook is called for, and HOST-222 before HOST-223 in both files; slice 9 is delivered before slice 8, section 12). Order 6, built beside `story:private-names-in-eval-code`, which shares only the generated `tests/main.rs` with it.

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
- `conformance/host/requirements.txt`: `HOST-119 pending slice 8` becomes `HOST-119 test swell::private_element_hook::host_119_a_throw_from_the_hook_is_the_field_adds_throw`, and `HOST-223 pending slice 8` becomes `HOST-223 test swell::private_element_hook::host_223_a_stale_handle_in_the_throw_faults_the_agent`. Two files follow from these lines and are the coordinator's, not the unit's (the wave's coordinator writes `CHANGELOG.md`, `AGENTS.md`, `README.md` and `website/data/status.json`): the unit hands the coordinator one line of text for `### Host interface` under `## [Unreleased]` of `CHANGELOG.md` that names `HOST-223` in backticks (the new requirement, a new line of the host interface's scan), and the coordinator runs `task site-data`, which writes `website/data/status.json` from `requirements.txt`.
- Sizes: no layout changes.
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "No Python for permanent tools") state them: a test is terminated at 60 s and counts as failed; the gate fails when the wall time of its test steps passes 120 s; a test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` (user plus system); no test of this story loops (each makes at most 10 adds), so no reduced count is needed; the unit times each test before it adds it and names each one over 1 CPU second with its time. A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures, probes or waits runs without the prefix. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory or `.out/`.
- Size: about 300 lines (engine 50, tests 220, requirement and documents 30): one session.

## Requirements

- This story owns the move of the `pending slice 8` lines of `conformance/host/requirements.txt`: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` and on this story's base, `2` after its first commit, `0` after its last; every other story of slice 8 leaves the file unchanged. `slot task host-requirements-check` exits 0. `task host-surface-check` and `task site-data-check` read files the coordinator writes after the unit (`CHANGELOG.md`, `website/data/status.json`), so they are the coordinator's checks on the join, not the unit's.
- The run loop is not touched; the hook's call is in a function of its own outside `Agent::execute`; `STACK_BYTES_PER_REENTRY` is unchanged, and the stack tests pass on seven eighths of `min_stack_bytes()`.

Writes: `docs/design/host-interface.md` (the paragraph of HOST-223 only), `conformance/host/requirements.txt` (the lines of HOST-119 and HOST-223), `crates/swell/src/object/private_names.rs` (the head of the add function and `ensure_can_add_private_element`), `crates/swell/src/host/hooks.rs` (documentation only), `crates/swell/tests/private_element_hook.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `CHANGELOG.md` and `website/data/status.json` (the coordinator's), `crates/syntax`, `crates/compiler`, `crates/swell/src/interp/`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/test262/`, `docs/design/engine.md`.

Shared files: with `story:private-names-in-eval-code`, built at the same time with no edge between them: `crates/swell/tests/main.rs` (generated again on the join); no other file (that story writes no file under `crates/swell/src/object/`). Written before it by stories it depends on: `object/private_names.rs` (`story:private-names-and-private-methods`, then `story:private-fields-and-private-in`'s field kind; here the head of the add function), `conformance/host/requirements.txt` and `docs/design/host-interface.md` (`story:host-object-overrides`, the lines of HOST-114 and HOST-222; here HOST-119 and HOST-223). `website/data/status.json` is written by neither this story nor `story:slice-08-conformance-close`: the coordinator writes it, as that story states.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step. `slot cargo nextest run --locked --no-tests=fail -p swell --test main -E 'test(/^private_element_hook::/)'` runs 10 tests and passes (on the base the filter matches none and the command fails); then `slot cargo nextest run --locked -p swell --test main -E 'test(/^private_element_hook::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^hooks::/) | test(/^host_objects::/) | test(/^host_overrides::/) | test(/^fuzz::/) | test(/^misuse::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; each exits 0, in this story's tree and after `story:private-names-in-eval-code` is merged first: the wave branch is merged into the unit, the one conflict this story names is fixed as it names it (`tests/main.rs`: generated again; the two stories share no other file), and the same commands run again and exit 0. `slot task clippy`, `slot task forbid-unsafe-check` and `sh taskfiles/test-main.sh check` exit 0; `slot task host-requirements-check` exits 0; `task host-surface-check` exits 1 in the unit's tree and names one name only, `HOST-223` (the line the unit hands the coordinator; it exits 0 on its base), and it and `task site-data-check` are run by the coordinator after writing `CHANGELOG.md` and `website/data/status.json`, where both exit 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from). The text: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `0` (1 on the base); `grep -c '^HOST-119 test swell::private_element_hook::host_119_' conformance/host/requirements.txt` prints `1` (0 on the base); `grep -c '^HOST-223 test swell::private_element_hook::host_223_' conformance/host/requirements.txt` prints `1` (0 on the base); `grep -c '^\*\*HOST-223\*\*' docs/design/host-interface.md` prints `1` (0 on the base); in the first commit of the story the same grep prints `1` and `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `2`. Time: every test this story adds stays within the bounds of Change, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262: no case reaches the hook; the coordinator's full run on the join passes the checks of the other stories unchanged.

## Out of scope

Override tables (slice 9, HOST-114, `story:host-object-overrides`); the other hook return sites (their slices); private elements of objects that are not host objects with an override table (no hook call, by HOST-119); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).

## As delivered

The unit `hook` of wave big-2 delivered the story on `impl/hook` (one correction after the
adversary's pass), merged into `wave/big-2`. Where the delivery differs from the text above:

- The find runs before the hook. The story puts the hook at the head of the add function;
  `add_private_element` first runs the charged find of its key. The second half of an
  accessor whose first half is there joins that element and calls no hook. Otherwise the
  order is: the hook, the duplicate `TypeError`, the room charge and the append. The hook is
  called once per element, not once per add: InitializeInstanceElements adds each
  PrivateElement once, a getter and a setter of one name are one element, and the engine adds
  them as two halves. The hook cannot change what the find read: a `HookCx` runs no code.
- The host's duty is stated and not checked: `#sec-hostensurecanaddprivateelement` requires
  the same kind of completion for every call with one object. A host that refuses an object
  after allowing it in one construction keeps the elements added before the refusal, in
  source order, each half of an accessor where it stands; nothing is taken back and the run
  does not fault. The adversary's failing case asked for the whole accessor before the
  refused element; the compiler emits each half where it stands, and the case now holds the
  behaviour stated
  (`a_throw_between_the_halves_of_an_accessor_keeps_the_getter_added_before_it`). The text
  is in the hook's documentation and in the paragraphs of HOST-119 and HOST-223.
- 18 tests under `private_element_hook::`, not 10, and two library tests in
  `object/private_names.rs`. Eight tests and one library test came with the adversary's
  pass. Five planted defects are each red: a `Throw` value that is not checked (in the
  collecting build, by the uncaught form of the two HOST-223 tests and the test of a string
  handle), the hook's call without the check that the agent still runs, the scope of the
  hook's values not cut back after the call, no join when the setter stands first, and a
  safe point inside a `HookCx` method.
- No value is kept across the hook, and no plant removes a keep: no method of `HookCx` is a
  safe point, so nothing collects while the hook runs. The library test
  `a_value_held_only_by_the_add_survives_the_hook` holds it, red with a collection planted
  right after the hook.
- A re-entrant add has no test: a `HookCx` cannot call, construct or evaluate (the
  compile-fail doctests of HOST-4).
- A host function cannot carry an override table (`new_host_function` takes none), so it
  calls no hook; `other_kinds_call_no_hook` holds it.
- The test file has 1,129 lines; the story estimated 220.
- test262: no case reaches the hook.
