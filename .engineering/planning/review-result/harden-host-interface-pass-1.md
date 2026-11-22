---
format: aep.planning-md/3
id: review-result:harden-host-interface-pass-1
kind: review-result
status: active
title: ess:hardening of the host interface specification, pass 1
relations:
- reviews: story:spec-gate
- reviews: story:ess-conformance-target
revision: 1
---
I ran the eight techniques in the skill's order; six could run on the pinned release, two (random sequences, caller replay) and the seeded half of determinism could not, for the reasons below. Nothing changed stays changed, and I found 26 issues, none a blocker.

**Toolchain.** `ess specify toolchain which` inside `spec/`: `reason: pin: 'requires: ess 0.96.0' in <worktree>/spec/ess-inputs.yaml` (binary `0.96.0 (cached)`). Every command ran with `ESS_TOOLCHAIN=0.96.0`, which prints `reason: override: ESS_TOOLCHAIN names it`. `<worktree>` is `$HOME/.local/state/worktree/trees/b10x/swell/swell-harden-host-interface`; my scripts are in `$HOME/.cache/scratch/408e9cc1-2772-4a97-aa1f-23a572f16276/harden-host-interface/`.

**Counts, before and after, unchanged.**
- Interpreter: 58 passed, 0 unsupported, 0 failed.
- Engine: 14 passed, 44 unsupported, 0 failed. After restoring, the rebuilt report equals the first byte for byte.

**One pool job** (`slot sh …/job.sh`, 1 slot; waited 2001 s, ran 457 s). It built the engine target once and ran 51 suites (the baseline and 50 mutants) in 26 s, so nothing was sampled.

### 1. Design review (technique 8)
- **Instrument:** `review.sh <spec dir>` checks each `HOST-<n>` against marker names and a map of 12 declared rules (`review-declared.tsv`).
- **Plant:** deleted ParseScript's `foreign-realm` branch in a copy. It produced `missing HOST-45: design host-interface.md:547 … answers Err(ForeignHandle); spec: no "- name: foreign-realm"`, exit 1. On `spec/` it reports nothing missing, exit 0.
- **Coverage:** of the 165 requirements, 109 are named by a marker and 12 by a declaration. Of the 53 left, 27 are outside what the model states (source scans, compile-time types, stack bounds, language-level results); the other 26 are in findings 3–8 and 13.
- **Marker claims, tested by validating edited copies** (`probes.sh`, `probes2.sh`):
  - Still hold: Bytes length (`ESS-TYPE-003`), the faulted refusals answering after held state (`ESS-COMMAND-005`/`004`), the foreign script (`ESS-COMMAND-004`), the settled no-op (non-exhaustive), `empty_change`.
  - No longer hold or can now be expressed: CreateRealm's two refusals (60 scenarios, 41 refusals), Link on a New module (60, 41), the load and link terminations (59, 41), an `external:` branch with a held-state guard, an `html:` ref, Integer bounds (+1 refusal).
- **Findings:** 1–13.

### 2. Spec diff in the gate (technique 7)
- **Plant:** in copies, a removed enum variant gave exit 4 with `fails --fail-on breaking-or-unknown: type/swell.host.RejectionOperation/variant-removed/handle`; an added required input gave exit 4 with `command/swell.host.Interrupt/input-added/reason`. Self-compare and HEAD against 0.2.0 both exit 0 (`spec/` is unchanged since 0.2.0).
- HEAD against 0.1.0 exits 4: `ParseScript/outcome-added/foreign-realm` and `EvaluateScript/outcome-condition-changed/interrupted` are both `unknown`.
- **Proposal, not a change:** add a task `spec-diff`. It writes `spec/` at the last release tag (`git show`) into `.out/` and runs `ess verify diff --from … --to spec --fail-on breaking-or-unknown`. Acknowledgements go in a committed `ess-diff-acknowledgements/1` file outside `spec/`, with a planted-defect script `tests/gate/spec-diff.sh`. It runs in under 1 s; CI needs the tags fetched.
- **Findings:** 14.

### 3. Mutation audit (technique 1)
- **Refused on the real model:** `ess verify conform mutate --path spec --emit …` fails with `UnsupportedPrimitive: a Binary64 … a mutation audit carries none`.
- **Proxy:** a copy with the three `Binary64` positions as `Integer`. Its suite matches: `0 added, 0 removed, 0 changed, 58 unchanged`.
- **Interpreter, synthesized only:** `80 mutant(s), 33 killed, 0 survived, 0 inconclusive, 30 stillborn, 16 unwitnessed, 1 equivalent`.
- **Interpreter, with the authored scenarios:** `43 killed, 0 survived, 6 unwitnessed`.
- **Engine** (emit, run each suite, `--collect`): `10 killed, 0 survived, 17 inconclusive, 30 stillborn, 22 unwitnessed, 1 equivalent (… 40 not scored)`.
- **Harness check:** `transition-to/Module.settle` is killed only by the authored `module-with-top-level-await`; the gate's own `a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok` also passed.
- **Mutation classes:** 33 error-swap, 11 guard-negate, 2 guard-boundary, 1 guard-connective, 1 precedence-swap (equivalent), 2 transition-to. 28 emit-drop mutants were stillborn and 28 emit-swap sites unavailable. sets-drop has only `creates_row` notes, and the other classes found no site.
- **Planted engine defects** (each restored with `restore: … equals HEAD`):
  - P1 (`begin()` no longer clears the interrupt flag) gave `13 passed, 1 failed`: `failed swell.host/authored/interrupt-ends-the-next-run` (`ESS-CF-OUTCOME — outcome swell.host.EvaluateScript/completed`).
  - P2 (`alive()` answers `Ok` for a faulted agent) gave `14 passed, 0 failed`: no verdict moved.
- **Findings:** 15–22.

### 4. Guard analysis (technique 6)
- `guards.jq` over the compiled IR: 18 commands, 89 points, 9 gaps, 4 overlaps, 0 dead branches.
- **Plants, in copies:**
  - A gap (Link's `already-linked` without `Evaluated`): validate passes, the analysis prints `new gap: swell.host.LinkModule at Evaluated`.
  - A dead branch (Collect's `faulted` also taking `Alive`): validate passes, the analysis prints `new dead guard: swell.host.Collect collected`.
  - An input-guard overlap: the compiler refuses it itself (`ESS-COMMAND-005`), so that class is covered.
- The four gaps of AsyncModuleExecutionSettled are the marked ESS limit, and that limit still holds.
- **Findings:** 23–25.

### 5. Random sequences and determinism (techniques 2 and 4)
- **Random sequences and the seeded determinism run: not run.** The conformance target is built on the Rust `ess-conformance` crate, which has no explorer; only the generated Go and TypeScript packages carry `explore.ts`/`Explore`. Running them would need an explorer over a Rust `ConformanceTarget`, or a process bridge from the generated TypeScript package to this target (finding 26).
- **Determinism source scan:** a clock read planted in a copy (`std::time::Instant::now()` in `evaluate_script`) made `scan.sh` exit 1, naming `host/agent.rs:492`. On the real sources the only hit is `heap/map.rs`, a `HashMap` keyed by `hash_key` with no iteration outside tests: injected, so no finding.
- Two engine runs gave byte-equal reports. Nothing was planted against that check, so it is not evidence.

### 6. Metamorphic relations and caller replay (techniques 5 and 3)
**Not run.** Relations the design promises, and whether each could run today:
- **A refused parse changes nothing but `refusals()` (HOST-164):** not through ESS (the target answers it unsupported); possible today as a Rust test over the interface.
- **A faulted agent answers nothing afterwards (HOST-48):** possible today on the interpreter. On the engine only if the fault is raised through Collect (finding 19).
- **These three, possible today on the engine as paired authored scenarios:**
  - a failed parse changes nothing;
  - a run that ends on a budget or an interrupt leaves later answers as they would be without it (HOST-70);
  - a Collect between two calls changes no later answer.
- **Interrupt then `clear()` equals no interrupt (HOST-93):** needs a Clear command (finding 6) or a Rust test.

**Caller replay:** `crates/test262` calls `Agent::new`, `create_realm`, `parse_script`, `evaluate_script`, `with_cx`, `global_object`, `interrupt_handle`, `refusals` and `last_entry_steps`, plus nested `cx.parse_script`, `evaluate_script`, `new_host_function` and `new_host_object`. The model has commands for only five of these. A replay needs a recorder in `crates/test262/src/host.rs` and `run.rs` (a change that would stay) that turns each call into a model command, replayed as authored scenarios on the interpreter. I expect that caller never to trigger the NotHeld, `foreign-realm` or `faulted` answers.

### Cleanup
`git status --short` in the worktree prints nothing. I ran `cargo clean --manifest-path conformance/ess/Cargo.toml`, deleted the spec copies (the probe scripts recreate them), and no process I started is still running.

```findings
- {file: spec/domains/host.yaml, line: 522, category: spec-gap, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "The CreateRealm marker says decision 9 gives create_realm no Outcome, but decision 5 with HOST-12 and HOST-41 answers Terminated(MemoryBudget) and Throw, and both validate on 0.96.0 as external refusals."}
- {file: spec/domains/host.yaml, line: 872, category: spec-gap, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "LinkModule answers nothing for a New module while HOST-42 answers ModuleStatus with found New, and a when_subject_state New refusal validates on 0.96.0."}
- {file: spec/domains/host.yaml, line: 735, category: spec-gap, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "LoadRequestedModules and FinishLoadingImportedModule declare no Terminated answer and LinkModule none for StepBudget, though decision 10, HOST-75 and HOST-76 state them, and both refusals validate on 0.96.0."}
- {file: spec/domains/host.yaml, line: 875, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "The module and load commands neither declare nor mark the ForeignHandle answer HOST-31 and HOST-153 give a module or load of another agent."}
- {file: spec/domains/host.yaml, line: 1031, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "queue_microtask and the hooks host_prepare_to_run_job and host_clean_up_after_job (decision 11, HOST-78, HOST-82) are neither declared nor named by a marker."}
- {file: spec/domains/host.yaml, line: 1214, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "InterruptHandle::clear (decision 12, HOST-93) has no command although it is a store like Interrupt and expressible as one."}
- {file: spec/domains/host.yaml, line: 492, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "CreateAgent declares no answer for CreateError::AgentKeysExhausted (decision 2) and no marker names it."}
- {file: spec/domains/host.yaml, line: 572, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "A parse refused for a later slice (HOST-164) is neither declared, since parse-failed names grammar and nesting only, nor marked."}
- {file: docs/design/host-interface.md, line: 946, category: spec-gap, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "Decision 12 reads the interrupt flag at the start of every outermost entry, yet decision 5 lets collect answer only EngineFault and parse_script reads no flag, so which entry points read it is unclear."}
- {file: spec/domains/host.yaml, line: 222, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "The markers at lines 222, 186, 123 and 1229 say decision 9 does not decide, but decisions 8, 7, 4 and 11 now decide script re-evaluation, realm end, source encoding and cleanup jobs."}
- {file: spec/domains/host.yaml, line: 967, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "The ESS reasons of the markers at lines 967, 1034 and 40 no longer hold on 0.96.0, and the marker at line 929 says no branch answers what not-linked answers."}
- {file: spec/domains/host.yaml, line: 147, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "The hook marker lists HostEventSet, which decision 16 says is not a hook, and HostEnqueueFinalizationRegistryCleanupJob, which returns nothing, among hooks that answer."}
- {file: spec/system.yaml, line: 20, category: spec-gap, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "Marker carried-by lists omit HOST-159, HOST-149 to HOST-151, HOST-155, HOST-156 and HOST-158 that the design assigns them, and the design's list (b) names five contradictions none of which stands."}
- {file: spec/domains/host.yaml, line: 648, category: ess-limit, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "A prose edit of an external cause is classified unknown for callers and readers, so a diff gate fails every wording fix until it is acknowledged."}
- {file: spec/domains/host.yaml, line: 1160, category: ess-limit, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "ess 0.96.0 refuses the mutation audit for a model that reaches a Binary64, so the audit ran on an Integer proxy whose suite changed none of 58 scenarios."}
- {file: spec/scenarios, category: missing-scenario, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "ParseModule faulted and EvaluateScript no-script remain unwitnessed in the interpreter audit although an authored scenario can reach each."}
- {file: spec/domains/host.yaml, line: 830, category: ess-limit, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "FinishLoadingImportedModule faulted, no-agent and no-load cannot be witnessed because a ModuleLoad is created only by a system-issued command synthesis does not await."}
- {file: spec/domains/host.yaml, line: 668, category: ess-limit, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "No emit mutant is live here (28 emit-drop stillborn, 28 emit-swap unavailable even for ScriptCompleted and ScriptThrew with identical fields), so no mutant checks which event an outcome publishes."}
- {file: spec/scenarios/faulted-agent-refuses-every-call.yaml, category: missing-scenario, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "With alive() answering Ok for a faulted agent the engine suite stays at 14 passed, because the faulted-agent rule is checked only by this scenario, which faults through EvaluateScript and is unsupported on the engine."}
- {file: spec/scenarios/step-budget-ends-one-run.yaml, category: missing-scenario, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "EvaluateScript step-budget has no evidence on the engine because its only scenario is unsupported for its last step, a slice-10 PerformMicrotaskCheckpoint."}
- {file: spec/scenarios/memory-budget-ends-one-entry.yaml, category: missing-scenario, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "EvaluateScript memory-budget has no evidence on the engine because its only scenario first forces ParseScript memory-budget, which the target cannot reach."}
- {file: conformance/ess/src/target.rs, line: 192, category: ess-limit, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "Five of the 14 engine passes are NotHeld answers the adapter gives without calling the engine, so the engine itself is asked in nine."}
- {file: spec/scenarios, category: missing-scenario, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "ParseScript's faulted-before-foreign-realm order (HOST-48) is unpinned: swapping the branches changes no scenario and precedence-swap has no site among related-row guards."}
- {file: spec/domains/host.yaml, line: 853, category: false-positive, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "The guard analysis flags FinishLoadingImportedModule as unanswered on a Finished or Failed load, which HOST-62 makes unreachable because the token is moved."}
- {file: spec/domains/host.yaml, line: 561, category: false-positive, severity: note, verdict: CONFIRMED, origin: pre-existing, message: "ParseScript and ParseModule answer no-realm before faulted, the reverse of decision 5's order, which is harmless because NotHeld never reaches the engine."}
- {file: conformance/ess/Cargo.toml, line: 14, category: ess-limit, severity: warning, verdict: CONFIRMED, origin: pre-existing, message: "ess-conformance 0.96.0 has no explorer for a Rust ConformanceTarget, so random sequences and seeded determinism cannot drive the engine target."}
```
