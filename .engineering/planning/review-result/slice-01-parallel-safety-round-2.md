---
format: aep.planning-md/3
id: review-result:slice-01-parallel-safety-round-2
kind: review-result
status: active
title: parallel-safety critic, slice 1 stories, round 2
relations:
- reviews: story:num-and-unicode-early-parts
- reviews: story:syntax-es5-and-strict
- reviews: story:heap-values-and-host-surface
- reviews: story:bytecode-compiler
- reviews: story:object-model-and-realms
- reviews: story:interpreter-and-script-entry-points
- reviews: story:harness-builtins
- reviews: story:host-interface-hardening
- reviews: story:test262-run-path
- reviews: story:ess-conformance-target
revision: 1
---
needs-revision
story:object-model-and-realms — the dispatch this story writes for the bound function and the ECMAScript function only has arms for [[Call]] and [[Construct]], but in wave 4 story:harness-builtins and story:interpreter-and-script-entry-points each fill those empty kind structs with heap values the precise collector must mark (the bound target, `this` and arguments; the function's environment and realm), and no story says which file marks an object kind's fields: heap-values-and-host-surface's collector covers roots and `HostData::trace` only, and neither wave-4 story lists a `heap/` file or `object/mod.rs`, so both would add marking for their kind to the same collector code (inferred). The fix is a marking arm per kind in this dispatch that calls into each kind's file, or an edge between the two wave-4 stories naming the collector file — .engineering/planning/story/object-model-and-realms.md:44, .engineering/planning/story/heap-values-and-host-surface.md:123, .engineering/planning/story/harness-builtins.md:100, .engineering/planning/story/interpreter-and-script-entry-points.md:137

Everything else in the six waves is clean: no two stories in a wave share a file under the stated `Cargo.lock` rule, and every acceptance that reads another story's output has an edge to it.

**Waves of two, by their `Writes:` lines:**
- **Wave 2 (syntax-es5-and-strict, heap-values-and-host-surface):** no shared file. Only syntax writes `sections.txt` and `expected.txt`. Its `slot task test262` builds `swell-test262`, which doesn't depend on `swell`, so heap-values' engine changes don't touch it.
- **Wave 3 (bytecode-compiler, object-model-and-realms):** `crates/compiler/` against `crates/swell/`. No shared file.
- **Wave 4 (interpreter-and-script-entry-points, harness-builtins):** no file in common. Round 1's `object/mod.rs` collision is resolved: object-model writes both dispatch arms, and both wave-4 stories say they don't edit `object/mod.rs` (`object-model-and-realms.md:44-53`, `harness-builtins.md:70`, `interpreter-and-script-entry-points.md:37-39`). The marking finding above is the one surface still open.
- **Wave 6 (test262-run-path, ess-conformance-target):** `crates/test262/`, the design file, `sections.txt`, `expected.txt` and `taskfiles/test262.yml` against `conformance/ess/` and `taskfiles/conform-engine.yml`. No shared file. The ESS target has its own lock and leaves the root lock unchanged.
- **`Cargo.lock` under the rule:** num-and-unicode is alone in wave 1 and its lock is taken. In waves 2, 3, 4 and 6, each story that changes the lock says it regenerates it only in its own tree and the coordinator regenerates the wave branch's lock. harness-builtins, host-interface-hardening and ess-conformance-target say the root lock doesn't change.

**The shared files you asked about, and whether an edge orders each pair:**
- **`conformance/host/requirements.txt`, `crates/swell/Cargo.toml` and `crates/swell/src/host/`:** heap-values (wave 2), object-model (3), interpreter (4), hardening (5). Every pair is ordered by the chain hardening → interpreter → object-model → heap-values.
- **`sections.txt` and `expected.txt`:** syntax (wave 2) and test262-run-path (6). Ordered through run-path → interpreter → bytecode-compiler → syntax.
- **`taskfiles/host-requirements.yml`:** only heap-values writes it.
- **`realm/intrinsics.rs` and `global_object.rs`:** object-model, then harness-builtins. Ordered directly.
- **The two function-kind files:** heap-values creates them, object-model writes their stubs, then harness-builtins fills one and the interpreter the other. Ordered.
- **Shared test files** (`budgets.rs`, `interrupt.rs`, `calls.rs`, `support/host.rs`, `support/script.rs`, `compile_fail.rs`): all on the same chain.

**What I read:** the brief; the 10 stories as store files, plus `grep` over them for relations, collector surfaces and lock handling.
- **Surfaces:** 10 placed from cited `Writes:` lines, 0 placed by inference, 0 that could not be placed. The one finding rests on an inferred file.
- **What each acceptance reads, traced to who writes it:** 17 reader-to-writer pairs inside the set, all covered by depends_on edges, 2 through another story (object-model → num-and-unicode, run-path → syntax).

**What I could not establish:**
- **Undeclared surfaces with no clash (inferred):** the interpreter story presumably has to make the frame stack a collector root (heap-values' "filled later", `heap-values-and-host-surface.md:124`). It probably also has to attach the global environment record to the realm, which lives in object-model's `realm/mod.rs`. Neither file is on its `Writes:` line. No other wave-4 story writes those files, so nothing collides, but the story names neither.
- **Not checked:** whether the coordinator can merge a unit's branch without its `Cargo.lock` change. The brief states that as the rule; I didn't test it.

I made no scratch files this round.

```findings
- file: .engineering/planning/story/object-model-and-realms.md
  line: 44
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "(inferred surface) the dispatch this story writes for the bound function and the ECMAScript function only has arms for [[Call]] and [[Construct]], but in wave 4 story:harness-builtins and story:interpreter-and-script-entry-points each fill those empty kind structs with heap values the precise collector must mark (the bound target, `this` and arguments; the function's environment and realm), and no story says which file marks an object kind's fields: heap-values-and-host-surface's collector covers roots and `HostData::trace` only, and neither wave-4 story lists a `heap/` file or `object/mod.rs`, so both would add marking for their kind to the same collector code. The fix is a marking arm per kind in this dispatch that calls into each kind's file, or an edge between the two wave-4 stories naming the collector file"
```

Paths are relative to the store worktree.
