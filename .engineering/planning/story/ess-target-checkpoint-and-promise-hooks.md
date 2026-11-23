---
format: aep.planning-md/3
id: story:ess-target-checkpoint-and-promise-hooks
kind: story
status: draft
title: The ESS target runs the microtask checkpoint and awaits the promise-job and rejection hooks
summary: The conformance target supports PerformMicrotaskCheckpoint, HostEnqueuePromiseJob and HostPromiseRejectionTracker, the model gets the four declarations the slice text leaves incomplete, two scenarios, and the gate floor moves.
relations:
- decomposes: epic:slice-10-promises-and-async
- depends_on: story:job-queue-and-microtask-checkpoint
- depends_on: story:promise-constructor-then-and-jobs
- depends_on: story:promise-rejection-tracker-and-promise-reads
scope:
- confidence: cited
  path: conformance/ess/src/target.rs
- confidence: cited
  path: conformance/ess/tests/planted_defect.rs
- confidence: cited
  path: spec/domains/host.yaml
- confidence: inferred
  path: spec/scenarios/promise-job-runs-at-the-next-checkpoint.yaml
- confidence: inferred
  path: spec/scenarios/rejection-is-tracked-then-handled.yaml
- confidence: cited
  path: taskfiles/conform-engine.yml
revision: 8
---
## Context

Slice 10 brings jobs, `Promise`, `async` functions and generators and `for await` (`epic:slice-10-promises-and-async`; `docs/design/engine.md`, section 12, item 10). The ESS conformance target (`conformance/ess/src/target.rs`, decision 10 of `docs/design/engine.md`) runs the scenarios of the model `spec/domains/host.yaml` against the host interface; five commands of the model wait for slice 10 in `later_slice` (line 272 on `397ffff`): `PerformMicrotaskCheckpoint`, `HostEnqueuePromiseJob`, `HostEnqueueGenericJob`, `HostEnqueueTimeoutJob` and `HostPromiseRejectionTracker`, mapped to "slice 10 (promises and jobs)" and answered `unsupported` where a scenario invokes, forces or awaits them (lines 338, 752 and 789). Two scenarios already invoke `PerformMicrotaskCheckpoint` (`spec/scenarios/step-budget-ends-one-run.yaml` line 96 and `faulted-agent-refuses-every-call.yaml` line 80). This story is a story of its own, not a part of the stories that bring the engine side: it writes only the target, its test, four declarations of the model that the text of the slice leaves incomplete, two scenario files and the floor of the gate, and it needs all three engine stories it reads (the checkpoint of `story:job-queue-and-microtask-checkpoint`, the hook of a promise job of `story:promise-constructor-then-and-jobs`, the tracker of `story:promise-rejection-tracker-and-promise-reads`), on which it depends. It needs no open story of slice 7.

The text this story removes: "slice 10 (promises and jobs)" of `later_slice` for the three commands it supports. The search `grep -rn 'slice 10 (promises and jobs)' conformance spec taskfiles` prints on `397ffff` line 285 of `target.rs` only; after this story it prints the line that keeps `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob`, now worded "slice 14 (generic and timeout jobs)" as `story:job-queue-and-microtask-checkpoint` answered open questions 1 and 2 of the epic, so the search prints nothing and `grep -rn 'slice 14 (generic and timeout jobs)' conformance/ess/src` prints that one line.

Design it builds on, by name: `docs/design/engine.md` decision 10 (the conformance target); `docs/design/host-interface.md` decision 11 (the checkpoint, its outcomes and its one budget: "The checkpoint is one entry", line 1062, merged at `843f178`), the hooks of a promise job and of the tracker; the model `spec/domains/host.yaml`: `PerformMicrotaskCheckpoint` (line 1031: `no-agent`, `faulted`, `step-budget`, `memory-budget`, `drained` and the other terminations), `HostEnqueuePromiseJob` (line 1096: `enqueued`, the event `PromiseJobEnqueued`), `HostPromiseRejectionTracker` (line 1182: `tracked`, the event `PromiseRejectionTracked`), and the system actor `swell.host.Engine` that issues the two hooks (lines 397 to 407). No sentence is missing from the design.

## What later stories reuse

The arms of the target for the checkpoint and the two hooks, which the conformance close reads through `slot task conform-engine`, and which slice 11 extends for the job of a module.

## Domain relations

- Target scenario → agent: the scenario invokes `PerformMicrotaskCheckpoint` on an agent it holds and awaits the hooks the engine issues — declared (`spec/domains/host.yaml:1031`, the command and its input `agent_id`; `:403` and `:406`, the commands the system actor issues).

## Change

Failing tests first: the scenario files and the test below are written before the code and fail on the base (the target answers `unsupported` for the three commands).

- `conformance/ess/src/target.rs`: an arm of `invoke` for `swell.host.PerformMicrotaskCheckpoint` (line 330 on `397ffff`) that calls `Agent::perform_microtask_checkpoint` on the agent of the scenario and answers the outcome of the model for each answer (`Normal` → `drained`, `Terminated(StepBudget)` → `step-budget`, `Terminated(MemoryBudget)` → `memory-budget`, `Terminated(Interrupted)` → `interrupted`, `Err(Faulted)` → `faulted`, an agent the scenario does not hold → `no-agent`), and the forced answers of the model for it in `force` (line 745) as for `EvaluateScript`; `ConformanceHost` (line 93) overrides `host_enqueue_promise_job` and `host_promise_rejection_tracker` to record each call, with its realm or operation, in the scenario (through the host data of its `HookCx`, never a `static`); `await_outcome` (line 785) answers `enqueued` and `tracked` from those records in their order, and `unsupported` with its reason for a command no record answers; `later_slice` keeps the commands of slice 11 and maps `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob` to "slice 14 (generic and timeout jobs)".

- `conformance/ess/tests/planted_defect.rs`: `a_target_whose_checkpoint_runs_no_job_fails_a_scenario`, in the form of `a_target_whose_interrupt_does_nothing_fails_a_scenario` (line 49): a target whose checkpoint answers `Normal` without running a job fails the new scenario of the promise job. The crate keeps its one test binary.

- `spec/scenarios/promise-job-runs-at-the-next-checkpoint.yaml` (new): an agent, a realm, a script `Promise.resolve().then(function () {})` evaluated (`completed`), the awaited `HostEnqueuePromiseJob` (`enqueued`, `PromiseJobEnqueued` with the realm), then `PerformMicrotaskCheckpoint` (`drained`). `spec/scenarios/rejection-is-tracked-then-handled.yaml` (new): a script `var p = Promise.reject(1);` (the awaited `HostPromiseRejectionTracker` with `Reject`), then a script `p.catch(function () {});` (the awaited one with `Handle`). Each is in the form `ess-scenario/6` of the files beside it, with the head comment that names the requirements it carries (HOST-78 and HOST-83; HOST-121). The coordinator has each new or changed scenario file admitted before it is committed.

- `spec/domains/host.yaml`, the four declarations the text of the slice (merged at `843f178` and changed at `7abd7e0`; `docs/design/engine.md` decision 3, "The microtask checkpoint is one entry", and `docs/design/host-interface.md` decision 11, "The checkpoint is one entry") leaves incomplete, named by the design author's report: `PerformMicrotaskCheckpoint` (line 1031), its outcome `step-budget` (its `external:` "the checkpoint's promise jobs together spend the steps of the call" becomes the entries it takes off, a step each, and what they run), its outcome `interrupted` ("while a promise job runs" becomes the three places of `engine.md` line 1007 and HOST-198: at the start of the checkpoint, before an entry is taken off, which then stays queued, and while an entry runs; line 1065 of the model) and its `naming.summary` ("Run the agent's promise jobs until its queue is empty" leaves out host microtasks: every entry of the queue, promise jobs and host microtasks, under one step budget); and the lifecycle of `swell.host.Job` (line 364: `Enqueued` and `Ran` only) gets a terminal state for a job that will not run to its end: one that started and was ended by a termination ("not run again"), or one `Agent::clear_microtasks` dropped (HOST-200); a job whose step of taking off was refused was not taken off and stays `Enqueued` (changed at `7abd7e0`, HOST-199). The transition the `step-budget`, `memory-budget` and `interrupted` outcomes of the checkpoint name for the job that started goes in where the model can state it, with an `UNMAPPED` comment naming HOST-77 where it cannot; the model has no command for `clear_microtasks`, and this story declares none: an `UNMAPPED` comment names HOST-200, and the report names the command as a gap of the model for the design author. The comment of the checkpoint (lines 1025 to 1030) and the `UNMAPPED` comment above it are reworded to match. `slot task validate` (the validation of the model, `taskfiles/spec.yml`) exits 0 after the change, and the report quotes what it prints for the change. `story:job-queue-and-microtask-checkpoint` names the gap of the lifecycle and leaves it to this story; it is not folded into that story, since that story writes no file of `spec/` or `conformance/ess` and the target needs the promise job and the tracker of two later stories. the floor of passed scenarios (14 on `397ffff`, its `desc`, `summary` and check) raised to the count the unit measures, and `ESS_UNSUPPORTED_MAX` (44) lowered to the count it measures; the report quotes the summary line `<n> scenarios: <p> passed, <f> failed, <e> error, <u> unsupported` before and after.

- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. The planted-defect test stays under 1.5 CPU seconds alone, measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell; the step `conform-engine` is timed before and after and both times are reported (a rise over 5 s is fixed in this change). The suite is built once by `task suite` and read by every test; no test makes a network request or a model call; the unit runs the binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot` (`slot task suite`, `slot task conform-engine`); a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures or waits runs without the prefix. No Python in any tool, test or command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.

- Size: about 400 lines (the target 200, the test 60, the two scenarios 120, the task 20): one session.

## Requirements

None of `conformance/host/requirements.txt` moves (the tests of the engine stories carry HOST-78, HOST-83 and HOST-121; the scenarios name them in their head comments).

Writes: `conformance/ess/src/target.rs`, `conformance/ess/tests/planted_defect.rs` (one test), `spec/domains/host.yaml` (the four declarations of Change), `spec/scenarios/promise-job-runs-at-the-next-checkpoint.yaml` and `spec/scenarios/rejection-is-tracked-then-handled.yaml` (new), `taskfiles/conform-engine.yml` (the floor and `ESS_UNSUPPORTED_MAX`). Checked against the code on `397ffff`: the matches over a command name end in a wildcard arm (`other =>`, `_ =>`), so the new arms break no match; the printers are the summary line of the runner and the ess-conformance-report of `.out/conform-engine.json`, neither recorded in the tree; the list the gate reads is the floor and the maximum of `taskfiles/conform-engine.yml`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `spec/system.yaml`, the scenario files that exist (`step-budget-ends-one-run.yaml` and `faulted-agent-refuses-every-call.yaml` pass further with no edit, which the report shows), `crates/` (every engine file), `conformance/host/`, `docs/design/`. The only story of its wave that writes these files (`story:async-generators-and-their-request-queue`, the other story of its wave, writes none of them). Shared with open stories of other epics, none ordered against it, each in parts of its own: `conformance/ess/src/target.rs`, `spec/scenarios/` (new files of other names) and `taskfiles/conform-engine.yml` (the floor) with `story:host-interface-scenarios-with-engine-evidence` (draft) and `story:host-interface-spec-closes-design-gaps` (draft), which also writes `spec/domains/host.yaml` (declarations of other commands; this story, the checkpoint and the lifecycle of `Job`); where both raise the floor, the coordinator sets it on the join to the count the join measures.

## Acceptance

`slot task suite` exits 0, then `slot task conform-engine` exits 0 and its summary line shows both new scenarios passed, `step-budget-ends-one-run` and `faulted-agent-refuses-every-call` passed, more passed and fewer unsupported than on its base (the report quotes both lines), and its planted-defect check names `a_target_whose_checkpoint_runs_no_job_fails_a_scenario` passed. `grep -rn 'slice 10 (promises and jobs)' conformance spec taskfiles` prints nothing (line 285 of `target.rs` on `397ffff`). `git diff --quiet "$BASE" -- crates spec/system.yaml` exits 0, `slot task validate` exits 0 (`BASE` the join the unit starts from). `slot task clippy` exits 0. Time: the new test stays under 1.5 CPU seconds alone and the step `conform-engine` rises by at most 5 s, as the unit measured them; the report lists both, and a time over its bound fails this acceptance. Scenario files: the coordinator has each new or changed scenario file admitted before it is committed.

test262: no case moves; the story writes no file the runner reads.

## Out of scope

Generic, timeout and cleanup jobs and `run_job` (slices 14 and 16); the job of a module (slice 11); the reads of a promise in the model (the promise is an engine value the model does not hold: its comment at `spec/domains/host.yaml:1179`); any engine file.
