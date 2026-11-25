---
format: aep.planning-md/3
id: story:interrupt-read-after-a-host-function-throws
kind: story
status: active
title: The interrupt flag is read after a host function returns by a throw, as after a return
summary: Call, New and their spread forms read the flag whatever the host function answered; HOST-216 (a throw with the flag set) and HOST-217 (a host function answering Terminated(Interrupted)).
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/test262/expected.digest
- confidence: cited
  path: crates/swell/src/interp/budget.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/spread.rs
- confidence: cited
  path: crates/swell/tests/interrupt.rs
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: website/data/status.json
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T03:16:25Z", actor: "human:operator", revision: 12}
- {from: "proposed", to: "active", at: "2026-11-25T03:16:25Z", actor: "human:operator", revision: 13}
---
Waits for: nothing. Base: `main` at `d49a09b` (release `0.9.0` and its numbers). Asked by the
first consumer (surf): its limit end rests on the interrupt being read after every return of a
host function. Ordered ahead of slice 4's operator `**`.

## Context

Line numbers are of `main` at `d49a09b`.

- The interrupt flag is read "right after a builtin or host function returns"
  (`crates/swell/src/interp/budget.rs:10-13`). Four sites read it only when the call returned
  `Ok`, because the call's answer is unwrapped with `?` before the read:
  - `Call(n)`: `crates/swell/src/interp/dispatch.rs:1426-1427`
    (`self.internal_call(object, this, &args)?; self.check_interrupt()?;`);
  - `New(n)`: `dispatch.rs:1558-1559` (`self.construct_value(object, &args, None)?;`);
  - a call with spread arguments: `crates/swell/src/interp/spread.rs:151-152`;
  - a construction with spread arguments: `spread.rs:208-209`.
- So a host function that sets the flag (or runs while `interrupt()` is called from another
  thread) and then answers `Err(Abrupt::Throw(v))` hands `v` to the script: a `catch` block runs,
  and straight-line code after it runs, until the next `LoopJump`, call or return reads the flag.
- A host function that answers `Err(Abrupt::Terminated(t))` is turned into the sticky
  termination at `crates/swell/src/object/builtin_function.rs:876`
  (`Err(Abrupt::Terminated(termination)) => Err(self.terminate(termination))`). No `HOST-<n>`
  states that answer for `Interrupted`; surf asked for one.

## Change

Failing tests first (Acceptance). Then:

- At each of the four sites the flag is read after the call whatever the call answered: an `Ok`
  and an `Err(Abrupt::Throw(_))` are both followed by `check_interrupt`; a set flag ends the run
  in `Terminated(Interrupted)` and the thrown value is dropped. An `Err(Abrupt::Terminated(_))`
  the call answered keeps its own reason (the termination is sticky, decision 10) and the flag
  is not read there.
- The comment at `budget.rs:10-13` says the flag is read after a builtin or host function
  returns or throws.
- `docs/design/host-interface.md`, section 12: two requirements after HOST-92 (`:1330`), each
  `[slice 1]`:
  - **HOST-216** (swell:D12) A host function that calls `interrupt()` on its agent's interrupt
    handle and then answers `Err(Abrupt::Throw(v))`, called by the script
    `try { f() } catch (e) { x = 1 }`, makes `evaluate_script` answer
    `Terminated(Interrupted)`, the global `x` stays `undefined`, and `is_set()` reads false
    after; the same holds for `new f()`, `f(...[1])` and `new f(...[1])`.
  - **HOST-217** (swell:D10) A host function that answers
    `Err(Abrupt::Terminated(Termination::Interrupted))` with the flag clear, called by the script
    `try { f() } catch (e) { x = 1 }`, makes `evaluate_script` answer
    `Terminated(Interrupted)`, the global `x` stays `undefined`, `state()` reads `Alive`, and a
    following script `1` answers `Normal(1)`.
- `conformance/host/requirements.txt`: `HOST-216 test swell::interrupt::<name>` and
  `HOST-217 test swell::interrupt::<name>`; `task site-data` regenerates
  `website/data/status.json`.
- `CHANGELOG.md`, `## [Unreleased]`, `### Host interface`: one entry that names `HOST-216` and
  `HOST-217` in backticks (the form `docs/releasing.md` states), as `host-surface-check`
  (`taskfiles/host-surface.yml:92-108`) requires, and a `### Fixed` entry: the interrupt flag is
  read after a host function's throw.
- Stays as it is: the read after an `Ok`; HOST-90, 91, 92, 198; the step counter; a run without
  an interrupt is unchanged; no new mechanism (the flag and `check_interrupt` exist).

## Acceptance

- On the base, the HOST-216 test fails (the script answers `Normal` or `x` reads `1`), each of
  its four call forms; after the change it passes. The HOST-217 test is written first and its
  result on the base is reported (it may already pass: then it is the requirement's test).
- `slot cargo nextest run --locked -p swell --test main -E 'test(/^interrupt::/) |
  test(/^termination::/) | test(/^host_objects::/)'` and the same with
  `--features collect-every-safe-point` exit 0.
- `slot cargo nextest run --locked -p swell-test262 --test main` exits 0.
- `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0;
  `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` prints 217
  requirements, 150 test; `slot --local task site-data-check` exits 0.
- The change to `crates/swell/src` moves the test262 sources digest: the full run
  (`slot --timeout 30m task test262-expected`, in parts where one job does not fit) writes
  `conformance/test262/expected.digest`, and `conformance/test262/expected.txt` has no changed
  line (`git diff --stat -- conformance/test262/expected.txt` prints nothing).
- The repository's gate `slot task check` exits 0 on the branch.
