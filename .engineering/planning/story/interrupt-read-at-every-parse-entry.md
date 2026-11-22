---
format: aep.planning-md/3
id: story:interrupt-read-at-every-parse-entry
kind: story
status: implemented
title: A pending interrupt ends parse_script and cx.parse_script before the source is copied
summary: Both host parse entries read the interrupt flag before any frontend work, as every other outermost entry does; the flag is consumed there instead of surviving the parse.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:10:12Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:10:12Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:50Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing; starts on the base at once. Base: `wave/s3-1` with the gate unit
(`gate-test-steps-within-budget`) and the design text merged. Precondition: the design sentences
of `order.md` items 4 and 5 (decision D3, taken: option A), which the base holds: `engine.md`
("Interrupting": "or at the next read of a parse") and `host-interface.md` section 12 ("at the
start of every parse (`parse_script`, `cx.parse_script`, `eval`, the `Function` constructor)").
Stories that wait for this one: `story:values-read-from-a-root-live-for-the-scope`,
`story:frontend-scratch-on-the-memory-ledger`.

## Context

Line numbers are of `wave/s3-1` at `c108c2d`.

- `Agent::parse_script` (`crates/swell/src/host/agent.rs:443-486`) is an outermost entry: it
  resets the termination and the step counter (`:451-452`), runs a safe point (`:453`) and then
  `parse_script_in` (`:457-459`), which copies the source, parses, compiles and makes the record
  (`crates/swell/src/script.rs:210-243`). It reads no interrupt flag; its comment says so
  (`agent.rs:439-441`). Every other outermost entry goes through `outermost_entry`
  (`agent.rs:406-426`), whose `begin` (`:542-552`, called at `:417`) clears a set flag and ends
  the entry in `Terminated(Interrupted)` before it spends a step.
- `Cx::parse_script` (`crates/swell/src/host/mod.rs:752-787`) checks only the sticky termination
  (`:765`) and parses (`:769`); its comment says it reads no interrupt (`:754-755`).
- So a flag set before `parse_script` survives it: the whole parse and compile run to their end
  (work bounded only by the length of the source and the memory budget), and the next
  `evaluate_script` answers `Terminated(Interrupted)` instead.
- The check exists once: `Agent::check_interrupt` (`crates/swell/src/interp/budget.rs:33-39`)
  clears a set flag and terminates the run.

## Change

Failing tests first (Acceptance). Then:

- `Agent::parse_script`: after the safe point and before `parse_script_in`, a set flag is
  cleared and the call answers `Ok(Parsed::Terminated(Termination::Interrupted))`. The order is
  that of `outermost_entry` (safe point, then the flag). Nothing is copied or charged, no
  `SyntaxError` object is made, `refusals()` does not move, `last_entry_steps()` reads 0.
- `Cx::parse_script`: after `running()` and before `parse_script_in`, the same read through
  `Agent::check_interrupt`: the flag is cleared, the run is terminated with `Interrupted`
  (sticky, decision 10), and the call answers `Ok(Parsed::Terminated(Interrupted))`. Inside a
  host function the run then ends as HOST-96 says; inside `with_cx` every later call of the
  closure answers the termination until the closure returns.
- The comments at `agent.rs:439-441` and `host/mod.rs:754-755` say the call reads the flag at its
  start.
- No new mechanism: the flag, `check_interrupt` and `Parsed::Terminated` exist.
- Stays as it is: `parse_script` spends no step; the step counter of bytecode; HOST-92 (a flag
  set after the parse still ends the next `evaluate_script`); a run without an interrupt is
  unchanged.
- This story owns the read at the start of `parse_script` and `cx.parse_script`.
  `story:frontend-scratch-on-the-memory-ledger` owns the read before the copy of `eval` and
  `Function` and the polls during every parse and compile step.

Requirements (section 12, `[slice 6]`): HOST-166 and HOST-167, as the design text words them.
Their lines in `conformance/host/requirements.txt` are the ones the design text added,
`HOST-166 pending slice 6` and `HOST-167 pending slice 6`; this story changes only those two
lines (Acceptance).

Writes: `crates/swell/src/host/agent.rs` (`parse_script` and its comment, lines 436-486, nothing
else), `crates/swell/src/host/mod.rs` (`Cx::parse_script` and its comment, lines 752-787, nothing
else), `crates/swell/tests/interrupt.rs`, `conformance/host/requirements.txt` (the lines of
HOST-166 and HOST-167). Running units: no file of `let-const-and-block-scopes` or
`template-literals`. Shared with stories that start beside it or after it, other hunks:
`host/agent.rs` with `story:arena-capacity-and-collector-workspace-on-the-ledger` (`live_bytes`
beside `heap_bytes`, lines 210-214), `host/mod.rs` with
`story:values-read-from-a-root-live-for-the-scope`, which waits for this one; the lines of
`requirements.txt` next to those of stories 3, 4 and 5. The unit that lands second gets the wave
branch merged into its tree and resolves the overlap (`AGENTS.md`, "How the work is run").

## Acceptance

In `crates/swell/tests/interrupt.rs`, with the limits of `tests/support/host.rs` and `r` a source
the parser refuses (its message read by `swell_syntax::message_kind` as a refusal, as HOST-164
takes it):

- `host_166_a_pending_interrupt_ends_parse_script_before_the_parse`: `interrupt()`, then
  `parse_script` of `r`, once as `Source::Utf8` and once as `Source::Utf16`. Today it answers
  `Parsed::Errors`, `refusals()` moves by 1 and `is_set()` still reads true. After: it answers
  `Parsed::Terminated(Interrupted)`, `is_set()` reads false, `refusals()`, `heap_bytes()` and
  `last_entry_steps()` read as before (0 steps).
- `a_flag_consumed_by_parse_script_does_not_end_the_next_entry`: `interrupt()`, `parse_script`
  of `1` (answers `Terminated(Interrupted)`), `parse_script` of `1` again and `evaluate_script`
  of it answer `Parsed` and `Normal(1)`. Today the first parse answers `Parsed` and the
  evaluation `Terminated(Interrupted)`.
- `host_167_an_interrupt_set_in_a_host_function_ends_its_cx_parse_script`: a host function that
  calls `interrupt()` on the handle the host keeps and then `cx.parse_script` of `r`; the script
  `f(); 1`. Today `cx.parse_script` answers `Parsed::Errors` and `refusals()` moves by 1. After:
  `Parsed::Terminated(Interrupted)`, `refusals()` unchanged, the outermost entry
  `Terminated(Interrupted)`. The same inside `with_cx`: `interrupt()`, then `cx.parse_script`
  answers `Parsed::Terminated(Interrupted)` and a following `cx.new_string` in the closure
  answers `Err(Abrupt::Terminated(Interrupted))`.
- HOST-90 to HOST-96 (`host_90_...` to `host_96_...` of the same file) pass unchanged.
- `conformance/host/requirements.txt`: the lines `HOST-166 pending slice 6` and `HOST-167
  pending slice 6` become `HOST-166 test swell::interrupt::host_166_a_pending_interrupt_ends_parse_script_before_the_parse`
  and `HOST-167 test swell::interrupt::host_167_an_interrupt_set_in_a_host_function_ends_its_cx_parse_script`;
  `slot task host-requirements-check` then prints two pending lines fewer than on the base.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test interrupt`,
the same two with `--features collect-every-safe-point`, `slot cargo clippy --workspace
--all-targets --locked -- -D warnings`, `slot task conform` and `slot task conform-engine` (the
counts of the base), and at the end, once, `slot task test262 TEST262_OUT=.out/test262-unit`,
whose `runs.tsv` is compared with that of `slot task test262 TEST262_OUT=.out/test262-base` run
on the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0
(no pass lost in the directories named here). Never `cargo test -p swell` whole, `slot task
test-collect`, `slot task test` or `slot task check`.

Directories: the test262 runner sets the flag itself in `$262.createRealm`
(`crates/test262/src/host.rs:268-272`) and parses through `cx.parse_script` in
`$262.evalScript`; the cases that use either keep their passes. The unit counts them
(`grep -rlE 'createRealm|evalScript' .cache/test262/<pin>/test | wc -l`) and names the count.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (a counting build
counts within one test's process). Each new test is timed, and one over 5 s is named in the
report with its time. A job of the build pool asks for one slot. Determinism: the same script
under the same budgets ends the same way at the same count on every run. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Cost

Bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell --profile
default --test scripts`, built first with `--no-run` and measured with `/usr/bin/time -f '%U %S'`
inside the pool job, three runs on the base and three on the unit, the median of each, rise by
at most 2 %. Why 2 %: the change is one load of an `AtomicBool` per parse entry, against a copy,
a parse and a compile of the source behind it; 2 % is the noise of three runs, not a cost of
the code. Past the bound the unit reports the three numbers of each side and stops before
merging.

## Report

The unit's report gives the new tests with their times, the count of test262 cases that use
`createRealm` or `evalScript`, the Cost numbers, and this sentence for the changelog: "A pending
interrupt now ends `parse_script` and `cx.parse_script` before the source is copied: they answer
`Parsed::Terminated(Interrupted)` and clear the flag, as every other outermost entry does."
