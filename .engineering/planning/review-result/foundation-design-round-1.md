---
format: aep.planning-md/3
id: review-result:foundation-design-round-1
kind: review-result
status: active
title: Design critic, foundation stories, round 1
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
revision: 1
---
needs-revision
story:workspace-and-gate — its crate publishes `pub enum Outcome { Unsupported }` and a free `evaluate_script(&str)`; that is one of decision 9's entry points, in a shape `spec/` has not stated and with a variant decision 9's `Outcome` does not have, its only user is story:test262-counting-runner, and slice 1 must replace it, so `crates/swell` should be a `#![forbid(unsafe_code)]` library with no public item and `runs_nothing.rs` should be dropped — .engineering/planning/story/workspace-and-gate.md:74, docs/design/engine.md:492, docs/plan/unit-invariants.md:22
story:test262-counting-runner — it calls `swell::evaluate_script` and depends on `swell` even though the answer never changes and `passed` is 0 either way, which ties the runner to the placeholder, so it should count every run case as failed with no engine call and no `swell` dependency, and leave the call to slice 1, which rewrites the run path (a fresh agent and realm per run) anyway — .engineering/planning/story/test262-counting-runner.md:73, .engineering/planning/story/test262-counting-runner.md:153, .engineering/planning/epic/slice-01-harness.md:42

**On the placeholder:** the set does not hold together with it. It is a split abstraction. One story writes an interface, its only user is the other story, and the next slice deletes it. On the way it breaks the spec-first invariant and the design's "never as a stand-in" rule (docs/design/engine.md:736). It also proves nothing that lasts: the outcome does not change whether the engine is called or not.

Nothing in the epic requires the call. The epic only asks that "none [be counted] as passed" (.engineering/planning/epic/foundation.md:44-45). A shape that does not need the placeholder:
- `crates/swell` is an empty crate with the forbid line. It is still the "engine that runs nothing", and `forbid-unsafe-check` still sees 1 crate root.
- The runner's `count` module marks run cases as failed at one function, and slice 1 replaces that function with the call `spec/` states.
- The `depends_on` edge from the runner to `story:workspace-and-gate` stays. The runner still needs the root `Cargo.toml` and the root `Taskfile.yml` include.

**What I read:**
- The 6 stories and `epic:foundation` in full, plus `epic:slice-01-harness`.
- `aep plan artifact relations`, `graph`, `validate` (valid) and `waves`.
- `AGENTS.md`, `docs/plan/unit-invariants.md`, and decisions 1, 2, 8 to 14 and section 12 of the design.
- I walked all 28 edges, including the 19 slice-epic `depends_on` edges outside the set. There is no cycle, and the chain does not serialise the set: unicode-data-pin and spec-gate stand alone.
- I traced 8 acceptance reads to a producer in the set, and an edge records all 8. The runner reads 5 from workspace-and-gate and test262-fetch; expected-and-sections reads 3 from the runner.

**What I could not establish:**
- Out of my lane (acceptance / parallel safety): the runner's `slot task test262` runs `cargo run --locked`, but `Cargo.lock` only gains `swell-test262` when the coordinator regenerates it. Either the unit writes the coordinator's file or `--locked` fails before the merge (.engineering/planning/story/test262-counting-runner.md:560, :595).
- Out of my lane (parallel safety): no story records a scope, so `aep plan artifact waves` reports all 6 as unassessed.
- Outside the set, not a defect: spec-gate depends on `spec/`, which has no artifact in the store, so no edge can record it.

```findings
- file: .engineering/planning/story/workspace-and-gate.md
  line: 74
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its crate publishes `pub enum Outcome { Unsupported }` and a free `evaluate_script(&str)`; that is one of decision 9's entry points, in a shape `spec/` has not stated and with a variant decision 9's `Outcome` does not have, its only user is story:test262-counting-runner, and slice 1 must replace it, so `crates/swell` should be a `#![forbid(unsafe_code)]` library with no public item and `runs_nothing.rs` should be dropped"
- file: .engineering/planning/story/test262-counting-runner.md
  line: 73
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "it calls `swell::evaluate_script` and depends on `swell` even though the answer never changes and `passed` is 0 either way, which ties the runner to the placeholder, so it should count every run case as failed with no engine call and no `swell` dependency, and leave the call to slice 1, which rewrites the run path (a fresh agent and realm per run) anyway"
```
