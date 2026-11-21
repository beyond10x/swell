# What the first ESS model of the host interface measured

2026-11-21. `ess 0.96.0` (`ess --version`), model format `ess/37` (`spec/system.yaml` line 1).

This is the record of one measurement: what the first ESS model of swell's host interface
(`spec/`, as merged) states, and what ESS 0.96.0 did not let it state. The decisions it fed are
in `host-interface.md`; this file decides nothing. Every number below comes from a log under
`.out/` in the worktree, from `grep`, `awk`, `wc` or `jq` over `spec/`, or from the author's
report where it says so.

## The commands and their summary lines

Run again for this record, through `slot`, from the repository root:

| Command | Exit | Last line |
|---|---|---|
| `ess --strict-requires specify validate --path spec` | 0 | `swell v1 — 2 file(s), 3 scenario(s), valid` |
| `ess --strict-requires specify validate --path spec --format json` | 0 | completeness: `unscenarioed` 33, `outside` 0, `unanswered` 18 |
| `ess verify conform synthesize --path spec --scenarios spec --out .out/host-design/suite.json` | 0 | `64 scenario(s) (3 authored), 33 refusal(s), written to .out/host-design/suite.json` |
| `ess verify conform run --path spec --suite .out/host-design/suite.json --target interpreted --report-format 2 --report-out .out/host-design/report.json` | 0 | `64 scenarios: 64 passed, 0 failed, 0 error, 0 unsupported` |

The logs are `.out/host-design/validate.log`, `validate.json`, `synthesize.log`, `conform.log`
and `report.json`. `report.json` gives `execution_status` `passed`, `conformance_status`
`inconclusive` and `coverage.knowledge` `unknown`; why the status is `inconclusive` beyond that
field is not checked. The run is against `interpreted 0.96.0` (the report's `implementation`),
ESS's own interpreter of the model: it shows the model is consistent with itself and says
nothing about an engine.

The fourth command of the author's report, `ess generate synthesize --strict-requires --path
spec --target rust --out .out/gen-rust`, was not run again; its log is `.out/gen-rust.log`
(exit 1, section "Generated Rust").

## What the model holds

| Construct | Count | Source |
|---|---|---|
| Types | 12 | `awk` over `spec/domains/host.yaml` |
| Entities | 6: Agent, Realm, Script, Module, ModuleLoad, Job | same |
| Actors | 2: Host, Engine | same |
| Errors | 6: NotHeld, EngineFault, ParseFailed, Thrown, ModuleStatusConflict, Terminated | same |
| Commands | 18: 12 issued by Host, 6 by Engine | same |
| Events | 15 | same |
| Authored scenarios | 3: `faulted-agent-refuses-every-call` (122 lines), `module-with-top-level-await` (113), `step-budget-ends-one-run` (90) | `wc -l spec/scenarios/*.yaml` |
| `UNMAPPED:` markers | 50: 49 in `host.yaml` (1,309 lines), 1 in `system.yaml` (22 lines) | `grep -rn 'UNMAPPED:' spec` |

The six Engine commands are HostLoadImportedModule, HostEnqueuePromiseJob,
HostEnqueueGenericJob, HostEnqueueTimeoutJob, HostPromiseRejectionTracker and
AsyncModuleExecutionSettled.

## The 33 refusals

From `.out/host-design/synthesize.log`, `grep '^refused:'`, counted by code:

| Code | Count | Constructs | Cause |
|---|---|---|---|
| ESS-SYNTH-003 | 14 | `faulted` of CreateRealm, ParseScript, EvaluateScript, FinishLoadingImportedModule, ParseModule, PerformMicrotaskCheckpoint and Collect (7); Agent `Faulted` refusing Collect and PerformMicrotaskCheckpoint (2); Module `EvaluatingAsync` refusing EvaluateModule (1); ModuleLoad `Finished` and `Failed` refusing FinishLoadingImportedModule (2); EvaluateScript `no-script` (1); FinishLoadingImportedModule `no-load` (1) | Synthesis finds no candidate row in the needed state. `Faulted` and `EvaluatingAsync` are reached only through `external:` branches, which synthesis does not arrange; a ModuleLoad row is created only by an Engine command (next row but one). |
| ESS-SYNTH-001 | 6 | Collect `collected`, Interrupt `set`, EvaluateModule `already-evaluated`, EvaluateScript `engine-fault`, EvaluateModule `engine-fault`, PerformMicrotaskCheckpoint `drained` | A `preserves:` or `affects:` needs a `read_your_writes` view of the row, and the model declares no view. |
| ESS-SYNTH-012 | 4 | AsyncModuleExecutionSettled in `New`, `Unlinked`, `Linked`, `Evaluated` | The command has no wrong-state answer for these states; stating the no-op for `Evaluated` makes ESS require answers for the other three (`non_exhaustive_branches`). |
| ESS-SYNTH-023 | 9 | HostLoadImportedModule `requested`; HostEnqueuePromiseJob, HostEnqueueGenericJob and HostEnqueueTimeoutJob `enqueued`; FinishLoadingImportedModule `finished`, `failed`, `no-agent`; ModuleLoad transitions `finish` and `fail` | A step the system takes to create a record is not awaited. |

The author's report gives ESS-SYNTH-003 13 and ESS-SYNTH-023 10; its own construct lists have
14 and 9, which is what the log counts. The total, 33, agrees.

The path "a module load is requested, then finished" has no scenario. An authored scenario for
it (kept outside `spec/`) was refused at validation with ESS-AUTHOR-045 ("a step the system
takes to create a record, or over rows with no subject, is not awaited in this release") and
twice ESS-AUTHOR-019 ("`load` is named before anything binds it; no earlier act captures it").

## The 18 notes

From `grep '^note' .out/host-design/synthesize.log`; the same 18 are the `unanswered` entries
of `validate.json`.

- 15 read "observes its subject only through what the declared views publish":
  LoadRequestedModules `faulted`, `already-loaded`, `loaded`, `loading`; LinkModule `faulted`,
  `threw`, `memory-budget`, `linked`, `already-linked`; EvaluateModule `faulted`; EvaluateScript
  `no-agent`; the Module transitions `load` and `link`; Module `New` and `Unlinked` refusing
  EvaluateModule.
- 2 read "declares no not-found and no `wrong_state` outcome": AsyncModuleExecutionSettled and
  HostPromiseRejectionTracker.
- 1 reads "the specification declares actors and serves no component, so enforcing a grant is
  the caller's ... and no `<command>/grant/denied` scenario is owed".

## What ESS 0.96.0 did not state

One entry per gap of the author's report, section 3. Category: (a) a construct ESS 0.96.0 does
not have; (b) a consequence of swell's interface being an in-process library and not a
component with ports; (c) closed by a decision of `host-interface.md`. The last column names
what carries the gap now.

| # | Gap | Place in the model | Category | Carried by |
|---|---|---|---|---|
| 1 | [[Status]] `linking` and `evaluating` hold only inside one call; no outcome leaves a row in them (`missing_causation`) | entity `Module` (`host.yaml:228`) | (a) | HOST-74, HOST-98 |
| 2 | Engine values: the completion and thrown values, the parse errors, the global object and `this`, the promises of load and evaluate, the rejected promise, the load error | ScriptCompleted, ScriptThrew, ParseFailed, Thrown, CreateRealm, LoadRequestedModules, EvaluateModule, HostPromiseRejectionTracker, FinishLoadingImportedModule | (b) | decisions 4 and 5; HOST-36 to HOST-38, HOST-49 to HOST-51, HOST-61, HOST-100 |
| 3 | An answer that is only a return value: ESS refuses an accepting outcome that publishes nothing (`empty_change`) | events ModuleStatusChanged, ModuleLoadEnded, MicrotaskCheckpointPerformed (`host.yaml:1289`) | (b) | decisions 5 and 13; HOST-79, HOST-97, HOST-100 |
| 4 | Hooks the engine calls inside a host call and whose answer it uses; 13 hooks and the clock, the time zone and random bytes are not stated | entity `Agent` (`host.yaml:129`) | (b) | decisions 1, 3, 16 and 17; HOST-4, HOST-26, HOST-27, HOST-83, HOST-84, HOST-116 to HOST-128 |
| 5 | `call`, `construct`, host function values, re-entrancy, host objects, roots and handles whose validity depends on a collection | end of `commands:` (`host.yaml:1208`, `1214`, `1216`) | (b) | decisions 4, 14 and 15; HOST-15, HOST-28 to HOST-30, HOST-47, HOST-103 to HOST-115 |
| 6 | Interrupt from another thread: no command acts on another command while it runs; Interrupt and the `interrupted` branches are both stated and not connected | command `Interrupt` (`host.yaml:1157`, `1161`) | (a) | decision 12; HOST-90 to HOST-93, HOST-96 |
| 7 | A budget as "at most N per entry": stated only as a number | type `Limits` (`host.yaml:40`, `43`) | (a) | decision 2; HOST-6 to HOST-10, HOST-12 to HOST-24, HOST-80 |
| 8 | Determinism of two runs | `system.yaml:16` | (a) | decision 19; HOST-133 to HOST-135 |
| 9 | An identity that names nothing: ESS requires an answer for it; the interface has no such case | error `NotHeld` (`host.yaml:402`) | (b) | ESS limit; no requirement |
| 10 | The order of answers: the subject's held state is answered before a related row, so "a faulted agent answers every call with EngineFault" is an `external:` refusal on three commands, without its tie to the agent's `Faulted` state | `faulted` of LoadRequestedModules, LinkModule, EvaluateModule (`host.yaml:694`, `825`, `882`) | (a) | decision 6; HOST-48 |
| 11 | An engine fault inside a parse or a link: a refusal changes no record but its subject, so it cannot also fault the agent | ParseScript, ParseModule, LinkModule (`host.yaml:543`, `664`, `846`) | (a) | decision 10; HOST-71 |
| 12 | EvaluateModule must answer for `New` and `Unlinked`, since an accepting `external:` branch may not carry a held-state guard; the model declared `ModuleStatusConflict` | EvaluateModule `not-linked` and the error (`host.yaml:429`, `865`, `898`) | (c) | decision 6; HOST-42, HOST-43 |
| 13 | Jobs enqueued while a checkpoint runs: a set effect selects rows as they were before the call | PerformMicrotaskCheckpoint (`host.yaml:983`, `985`) | (a) | decision 11; HOST-77, HOST-79, HOST-80 |
| 14 | The graph's move to `unlinked` when its last load finishes; the referrer of a dynamic `import()` (a script or a realm) | FinishLoadingImportedModule, `ModuleLoad` (`host.yaml:767`, `288`) | (a) for the move; (c) for the referrer | decision 9; HOST-56, HOST-60, HOST-65, HOST-66 |
| 15 | `refs:` only on commands and outcomes and only as `provider:key`; no URL; no HTML anchor cited | PerformMicrotaskCheckpoint (`host.yaml:990`) | (a) | HOST-79 cites `html:perform-a-microtask-checkpoint` |

## Constructs the model holds because ESS forced them

| Construct | Why it is there | What a conformance target does about it |
|---|---|---|
| Error `NotHeld` (15 outcomes) | ESS addresses every row by an identity and requires an answer for one no row carries (`host.yaml:402`). In the interface the agent is the receiver and a handle cannot be forged. A handle the host held past its life answers `Err(StaleHandle)` (decision 6), which is another case. | Keeps the table from scenario identity to agent and handle; answers `NotHeld` itself for an identity the table does not hold, without calling the engine. |
| Error `ModuleStatusConflict` (1 outcome) | EvaluateModule on `New` or `Unlinked` (gap 12). | Decision 6 now has the answer: `Err(HostError::ModuleStatus { found })`. The target maps it to `ModuleStatusConflict`. |
| Event `ModuleStatusChanged` (outcomes LoadRequestedModules `loaded`, LinkModule `linked`, EvaluateModule `waiting` and `evaluated`, AsyncModuleExecutionSettled `settled`) | Stands for a return value (gap 3). | Reads `module_status` before and after the call and publishes the event with the new status when it changed. |
| Event `ModuleLoadEnded` (FinishLoadingImportedModule `finished`, `failed`) | Same. | Publishes it when `finish_loading_imported_module` answers `Ok`: `Finished` for a module, `Failed` for a thrown value. |
| Event `MicrotaskCheckpointPerformed` (PerformMicrotaskCheckpoint `drained`) | Same. | Publishes it when `perform_microtask_checkpoint` answers `Normal(CheckpointReport)`. |

## Generated Rust

- On the model as merged, `.out/gen-rust.log`: exit 1, "rust target cannot emit this
  workspace", naming `HostEnqueueTimeoutJob.input.milliseconds`, `Job.fields.milliseconds.of`
  and `TimeoutJobEnqueued.fields.milliseconds`, each "this target has no qualified finite
  Binary64 codec". Nothing is generated. The model keeps `Binary64`, because the draft defines
  the delay that way.
- On a scratch copy with `Binary64` replaced by `Integer` at those three places (not kept):
  "83 capabilities: 73 generated, 8 obligation(s), 2 refused" and "10 artifact(s)". One crate,
  `swell-types`, with no dependencies: `host.rs` 2,431 lines, `behaviour.rs` 603, `actor.rs` 83,
  `primitives.rs` 32, `obligation.rs` 28, `lib.rs` 23. `cargo check --offline` (cargo 1.99.1,
  through `slot`) exits 0 with no warning.
- The 2 refused are the actor grants of `Host` and `Engine` ("generated as data, not
  enforced"). The 8 obligations (`PLAN.md`): AsyncModuleExecutionSettled and
  HostPromiseRejectionTracker (an unknown identity reaches no declared outcome), EvaluateModule
  and PerformMicrotaskCheckpoint (a subject predicate choosing between a move and an update),
  EvaluateScript (`external:` beside `when_related:`), LoadRequestedModules
  (`when_subject_state:` beside `external:`), ParseScript and ParseModule (several related rows).

What the crate is, read from its source:

- Identities are `pub struct AgentId(pub Uuid)` with `pub struct Uuid(pub String)`.
- Each entity has a storage port, `get`, `put` and `delete` of a snapshot keyed by its identity.
- Every `external:` branch is decided by the caller through `Context::external(command,
  outcome) -> bool`; new identities come from `Context::generate_…`.
- Each lifecycle is a type-level state (`Agent<S>`, sealed markers).
- There is no type for an engine value: source is `SourceText(String)`, and no field holds a
  value, an object or a promise.

Result: the generated code is not usable for an in-process library interface. The interface's
identities are handles into the agent's heap and not strings, its records live in that heap and
not behind storage ports, the engine decides the external branches by running code instead of
asking a caller, and the interface passes engine values the crate has no type for. With
`Binary64` kept, nothing is generated at all. The interface is written by hand against the
`HOST-<n>` requirements.

## The 51 markers

Brought in line with `spec/` after wave 6 of slice 1 (2026-11-21; section "Wave 6 of slice 1"
below), by `awk` over `spec/system.yaml` and `spec/domains/host.yaml`. The sections above are
the first measurement and keep its numbers. Each `UNMAPPED:` marker with its first words and
what carries it: one or several `HOST-<n>` of `host-interface.md`, `ESS limit` where nothing of
the interface is left unstated, or both where the marker names the requirement and the limit of
ESS 0.96.0 that keeps it out of the model. No marker is `open`.

| Place | First words | Carried by |
|---|---|---|
| `system.yaml:16` | determinism. Decision 9's command-line host gives the clock, the time ... | HOST-133, HOST-134, HOST-135 |
| `host.yaml:40` | the unit, width and bounds of each limit. host-interface.md decision ... | HOST-9, HOST-12, HOST-14, HOST-16, HOST-19, HOST-20, HOST-22, HOST-23, HOST-24 |
| `host.yaml:47` | what a budget is, as opposed to what it is ... | HOST-6, HOST-7, HOST-8, HOST-10, HOST-80 |
| `host.yaml:123` | the representation of source text across the interface. ParseScript and ... | HOST-35 |
| `host.yaml:135` | the host object that answers the hooks (a Rust trait ... | HOST-4, HOST-26, HOST-27, HOST-83, HOST-84, HOST-116 to HOST-128 |
| `host.yaml:186` | whether a realm ends (is released by the host, or ... | HOST-52 |
| `host.yaml:222` | whether a script may be evaluated more than once. ScriptEvaluation ... | HOST-54 |
| `host.yaml:237` | [[Status]] linking and evaluating. Both hold only inside one call ... | HOST-74, HOST-98 |
| `host.yaml:298` | the referrer of a load requested by import() is a ... | HOST-65, HOST-66 |
| `host.yaml:413` | ESS addresses every row by an identity and requires an ... | ESS limit |
| `host.yaml:438` | the List of SyntaxError objects ParseScript and ParseModule return (16.1.5, ... | HOST-18, HOST-36, HOST-137, HOST-138 |
| `host.yaml:446` | the thrown value (an engine value) of Throw(value). | HOST-37 |
| `host.yaml:452` | not part of decision 9; see EvaluateModule's `not-linked`. | HOST-43 |
| `host.yaml:482` | the length of each seed. `hash_key` is exactly 16 bytes ... | HOST-25, HOST-26, HOST-27, HOST-144, HOST-145, HOST-146 |
| `host.yaml:507` | the global object and the global `this` value the host ... | HOST-49, HOST-50, HOST-51 |
| `host.yaml:522` | what CreateRealm answers when creating the intrinsics passes the memory ... | HOST-12, HOST-41, HOST-71 |
| `host.yaml:536` | hostDefined, the value the host attaches to the Script Record ... | HOST-55 |
| `host.yaml:581` | an engine fault while parsing. It must fault the agent, ... | HOST-71 |
| `host.yaml:597` | a script of another agent than the one called, which ... | HOST-45, HOST-48; ESS limit |
| `host.yaml:677` | hostDefined, as for ParseScript. | HOST-55 |
| `host.yaml:679` | a realm of another agent than the one called, which ... | HOST-31; ESS limit |
| `host.yaml:717` | an engine fault while parsing, as for ParseScript. | HOST-157 |
| `host.yaml:729` | its result. LoadRequestedModules returns a Promise that settles when the ... | HOST-39, HOST-60, HOST-61 |
| `host.yaml:733` | hostDefined, passed on to every HostLoadImportedModule of the graph. | HOST-57 |
| `host.yaml:750` | the tie between this refusal and the agent's Faulted state, ... | HOST-48 |
| `host.yaml:818` | what the host passes. Decision 9 says "with the module's ... | HOST-57, HOST-59, HOST-62, HOST-63, HOST-64 |
| `host.yaml:823` | the error value of the throw completion (an engine value ... | HOST-31, HOST-61 |
| `host.yaml:826` | the effect on the graph. When the last pending load ... | HOST-56, HOST-60 |
| `host.yaml:872` | Link on a module whose [[Status]] is new. The draft ... | HOST-42 |
| `host.yaml:886` | the tie between this refusal and the agent's Faulted state. | HOST-48 |
| `host.yaml:908` | an engine fault while linking. The refusal that leaves the ... | HOST-157 |
| `host.yaml:925` | its result. Evaluate returns a Promise, rejected with the exception ... | HOST-38 |
| `host.yaml:929` | Evaluate on a module whose [[Status]] is new or unlinked. ... | HOST-43 |
| `host.yaml:932` | a run of Evaluate that ends with Terminated. The modules ... | HOST-70, HOST-74, HOST-48 |
| `host.yaml:950` | the tie between this refusal and the agent's Faulted state. | HOST-48 |
| `host.yaml:967` | this answer. Evaluate on a new or unlinked module breaks ... | HOST-43 |
| `host.yaml:1007` | the step on a module that is evaluated already, where ... | ESS limit |
| `host.yaml:1023` | the jobs a checkpoint runs that were enqueued while it ... | HOST-79 |
| `host.yaml:1026` | which jobs ran when the checkpoint ends with Terminated. It ... | HOST-77, HOST-80 |
| `host.yaml:1034` | the reference to HTML's "perform a microtask checkpoint". The HTML ... | HOST-79 |
| `host.yaml:1123` | running a generic or timeout job. The model has no ... | HOST-85, HOST-86, HOST-88 |
| `host.yaml:1179` | the promise, an engine value. Without it the host cannot ... | HOST-100, HOST-101, HOST-121 |
| `host.yaml:1204` | the effect. The run in progress on the agent's thread ... | HOST-90, HOST-96 |
| `host.yaml:1209` | that the engine clears the flag when it ends a ... | HOST-91, HOST-92, HOST-93 |
| `host.yaml:1229` | what a collection changes that the host sees: WeakRef targets ... | HOST-87, HOST-129, HOST-130, HOST-131 |
| `host.yaml:1260` | call(function, this, arguments) and construct (decision 9, "Entry points"), and ... | HOST-15, HOST-47, HOST-103 to HOST-107 |
| `host.yaml:1267` | host objects: their classes, brand, host data, the `trace` method, ... | HOST-108 to HOST-115 |
| `host.yaml:1270` | the root table (decision 4, "Host-held values"): a Root is ... | HOST-28, HOST-29, HOST-30 |
| `host.yaml:1297` | the completion value, an engine value. | HOST-37 |
| `host.yaml:1304` | the thrown value, an engine value "the host can read" ... | HOST-37 |
| `host.yaml:1346` | how the host reads a module's [[Status]] or a checkpoint's ... | HOST-79, HOST-97, HOST-100 |

Totals: 51 markers (50 in `host.yaml`, 1 in `system.yaml`): 47 carried by requirements, 2
`ESS limit`, 2 both, 0 `open`.

## Wave 6 of slice 1: a realm's and a script's agent

What changed in `spec/` so that the scenarios can run on the engine, and why.

- **A realm of another agent.** ParseScript has the refusal `foreign-realm`
  (`when_related: {via: input.realm_id, predicate: agent_id != input.agent_id}`, error
  `ForeignHandle`, decision 6 and HOST-45), after `faulted` as `parse_script` checks the agent
  first. The realm's agent is its `agent_id`, the `via` of Agent `realms` (Table 20,
  [[AgentSignifier]]). Synthesis in ess 0.96.0 arranges no realm whose `agent_id` equals
  `input.agent_id` (ESS-SYNTH-003 on `parsed`, and on `parse-failed` and `memory-budget`), so
  ParseScript's `parsed` and every EvaluateScript outcome (ESS-SYNTH-004: the script only
  `parsed` creates) have no synthesized scenario. Refusals 32 to 41, scenarios 61 to 55.
- **A script of another agent** stays a marker (EvaluateScript): a guard through the script's
  `realm_id` beside the guards through `input.agent_id` is refused (ESS-COMMAND-004, "a command
  reads one related row"), and a guard on a stored agent of the script answers at precedence
  step 3, before `no-agent` and `faulted` (step 4), where HOST-48 answers Faulted first.
- **ParseModule** keeps the realm of another agent as a marker: the same guard there leaves
  28 scenarios and 65 refusals, against 55 and 41, as `parsed` takes the module lifecycle's
  scenarios with it.
- **Authored scenarios**, each parse and run that is to succeed made on the agent that owns
  the realm and the script:
  `interrupt-ends-the-next-run` (new; HOST-92, HOST-91), `script-runs-on-its-realms-agent`
  (new; `foreign-realm`, `parsed`, `no-agent`, `completed`), `step-budget-ends-one-run` (its
  second run is a second script, `1 + 1;`, that completes), `faulted-agent-refuses-every-call`
  (sources end with a semicolon). The example of the source text is `1 + 1;`.
- **Markers**: ParseModule's and LinkModule's engine fault name HOST-157 (was HOST-71); the
  Interrupt marker states decision 12's answer; the `interrupted` branch reads "the host set the
  interrupt handle before the run or sets it while the script runs".
- **The four outcomes the guard left without a scenario** have authored ones, each on the
  realm's own agent: `parse-error-leaves-the-agent-parsing` (`var 1;` answers `parse-failed`,
  HOST-36, then `1 + 1;` parses), `thrown-run-leaves-the-agent-running` (`throw 1;` answers
  `threw`, HOST-37, then a second script completes) and `memory-budget-ends-one-entry`
  (ParseScript and EvaluateScript `memory-budget`, each followed by a parse or run that
  succeeds; HOST-10, HOST-70; the parse's answer has no requirement of its own and is reached
  only by forcing it). Every outcome of ParseScript and EvaluateScript has a scenario but
  EvaluateScript `no-script`, which had none before either (ESS-SYNTH-003). Scenarios 58
  (8 authored), refusals 41, markers 51.

## What a conformance target needs from the interface

The model's scenarios are to run against the hand-written engine through a conformance target
in a workspace of its own, as `$HOME/beyond10x/surf/conformance/` does: its own `Cargo.toml`
with `[workspace]`, depending on `ess-conformance`, `ess-primitives` and `ess-domain` at tag
`0.96.0` and on the system's crates by path, and a type implementing `ConformanceTarget`
(`identity`, `begin_scenario`, `establish_entity`, `end_scenario`, `execute_command`,
`query_view`, `observe_events`, `observe_invocations`, `configure_external_outcome`,
`redeliver_event`; `src/target.rs`). surf's target also depends on surf's generated crates;
swell's would not, since nothing is generated (section "Generated Rust").

What the suite asks, counted with `jq` over `.out/host-design/suite.json` (64 scenarios): 370
`execute_command`, all of the 12 Host commands and none of the 6 Engine commands; 246
`capture_instance`; 63 `configure_external_outcome` over 24 command and outcome pairs (the most
frequent LoadRequestedModules `loaded` 27, EvaluateModule `waiting` 7); 4 `await_outcome`; no
`query_view` and no `establish_entity`.

From the interface, such a target needs:

- **Handles it can keep in a table** for the scenario identities it captures: `Agent`, `Realm`,
  `Script`, `Module`, `LoadRequest` and `Root` (decisions 4 and 9).
- **A host it writes itself**: a `Host` whose hooks record each call, so that the target can
  publish ModuleLoadRequested, PromiseJobEnqueued, GenericJobEnqueued, TimeoutJobEnqueued and
  PromiseRejectionTracked, and can answer a load by calling `finish_loading_imported_module`
  (decision 16).
- **Inputs that force each external branch**: source text that throws, `while (true) {}` with a
  small `steps_per_entry`, an allocation loop with a small `memory_bytes`, a module with a
  top-level `await`, a host function that calls `interrupt` (decisions 2, 12 and 14), and for
  `engine-fault` an override that breaks an invariant (HOST-113) or the fault injection of a test
  build (HOST-71).
- **The read surface of decision 13**: `state`, `module_status` and the `CheckpointReport`, to
  publish the three return-value events and the moves of Agent and Module the outcomes name.
- **The result types of decision 5 and the errors of decision 6**, to map each answer to an
  outcome or an error of the model.
- **Seeds of the lengths decision 3 takes**: 145 of the suite's 148 CreateAgent calls pass
  one-byte `hash_seed` and `random_seed` values (the model declares `Bytes` of any length), which
  `Seeds { hash_key: [u8; 16], random_seed: [u8; 32] }` does not accept as they are.

## Conclusion

The ESS model checks well the part it states: the lifecycles of agent, realm, script, module,
module load and job, and the rule that a faulted agent answers every later call with a fault,
which its authored scenario holds and a planted wrong expectation failed (author's report,
section 1). Of the 50 places ESS 0.96.0 could not state, 48 are interface behaviour that
`host-interface.md` now carries as `HOST-<n>` requirements and 2 are ESS limits. The model also
holds five constructs only because ESS required them, and its generated Rust is not usable for
an in-process library. So the ESS model stays as the checked statement of the part it states
well (the lifecycles of agent, realm, script, module, module load and job, and the
faulted-agent rule); everything else is carried by the `HOST-<n>` requirements. Its scenarios
are to run against the hand-written engine through a conformance target in a workspace of its
own, which needs from the interface what the section above lists.
