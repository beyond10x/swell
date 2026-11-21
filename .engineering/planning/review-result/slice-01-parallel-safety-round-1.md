---
format: aep.planning-md/3
id: review-result:slice-01-parallel-safety-round-1
kind: review-result
status: active
title: parallel-safety critic, slice 1 stories, round 1
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
story:syntax-es5-and-strict — this story and story:num-and-unicode-early-parts are in wave 1 with no edge between them, and both have to write the root `Cargo.lock`: each acceptance runs `cargo … --locked` on a crate the lock doesn't list yet (`swell-syntax` here, three packages there), which fails until the unit rewrites the lock, yet both bodies leave the lock to the coordinator; the plan needs either an edge that names `Cargo.lock` or one owner for the lock in this wave — .engineering/planning/story/syntax-es5-and-strict.md:117, .engineering/planning/story/syntax-es5-and-strict.md:128, .engineering/planning/story/num-and-unicode-early-parts.md:93, .engineering/planning/story/num-and-unicode-early-parts.md:104
story:bytecode-compiler — this story and story:heap-values-and-host-surface are in wave 2 with no edge between them, and both have to write the root `Cargo.lock`: this one adds `swell-compiler`, and heap-values adds a `swell-num` dependency to `swell` and runs `cargo test --locked --workspace --no-run`, yet both bodies leave the lock to the coordinator; the plan needs either an edge that names `Cargo.lock` or one owner for the lock in this wave — .engineering/planning/story/bytecode-compiler.md:81, .engineering/planning/story/bytecode-compiler.md:89, .engineering/planning/story/heap-values-and-host-surface.md:130, .engineering/planning/story/heap-values-and-host-surface.md:137, .engineering/planning/story/heap-values-and-host-surface.md:158
story:harness-builtins — the story says it runs beside story:interpreter-and-script-entry-points in wave 4, but each adds an object kind with its own [[Call]] and [[Construct]] (the bound function of 10.4.1 here, the ECMAScript function of 10.2 there), and the internal-method dispatch that has to route them is `crates/swell/src/object/mod.rs`, written by story:object-model-and-realms; neither story lists that file in `Writes:` (inferred), so the plan needs either an edge that names it or arms for both kinds in object-model-and-realms' dispatch — .engineering/planning/story/harness-builtins.md:46, .engineering/planning/story/object-model-and-realms.md:67, .engineering/planning/story/harness-builtins.md:87, .engineering/planning/story/interpreter-and-script-entry-points.md:121

The lock behaviour behind the first two findings is measured, not assumed. I built a scratch workspace under `.out/critic-parallel-safety/` and removed it afterwards. Both a new workspace member missing from the lock and a new path dependency made `cargo test --locked` exit 101 with `cannot update the lock file … because --locked was passed`.

**The shared files you asked about, and whether an edge orders each pair:**
- **`conformance/host/requirements.txt`:** heap-values (wave 2), object-model-and-realms (3), interpreter-and-script-entry-points (4) and host-interface-hardening (5). The chain hardening → interpreter → object-model → heap-values orders every pair, three of them directly and the rest through it.
- **`crates/swell/Cargo.toml` and `crates/swell/src/host/`:** the same four stories (heap-values creates every `host/` file), ordered by the same chain. harness-builtins writes neither.
- **`crates/swell/tests/`:** `budgets.rs`, `interrupt.rs`, `calls.rs`, `support/host.rs`, `support/script.rs` and `src/compile_fail.rs` are each written by two or three stories, all on the same chain.
- **`crates/swell/src/realm/intrinsics.rs` and `global_object.rs`:** object-model-and-realms (3), then harness-builtins (4). Ordered directly.
- **`conformance/test262/sections.txt` and `expected.txt`:** syntax-es5-and-strict (1) and test262-run-path (5). Ordered through run-path → interpreter → bytecode-compiler → syntax.
- **`taskfiles/host-requirements.yml`:** only heap-values writes it. object-model, interpreter and hardening run its task in their acceptance, and all three come after heap-values.
- **Everything else** (`crates/test262/`, `docs/design/engine.md`, `taskfiles/test262.yml`, `conformance/ess/`, `taskfiles/conform-engine.yml`, `taskfiles/unicode-tables.yml`, the new crates) has one writer in the set.

**What I read:** the 10 stories as store files and through `aep plan artifact show`, `aep plan artifact list --kind story`, the brief, `docs/plan/waves.md`, `docs/plan/wave-1.md`, the lock rule in `docs/plan/unit-invariants.md`, the root `Cargo.lock`, and `grep` over the 10 bodies for each shared path.
- **Surfaces:** 10 placed from cited `Writes:` lines, 0 placed by inference, 0 that could not be placed. One collision rests on an inferred file: `object/mod.rs`.
- **What each acceptance reads, traced to who writes it:** 14 reader-to-writer pairs inside the set, all covered by depends_on edges, 2 of them through another story (object-model → num-and-unicode, run-path → syntax).
- **Who changes the root `Cargo.lock`:** 7 stories. Waves 1 and 2 each have two, which gives the two collisions above; waves 3, 4 and 5 have one each.

**What I could not establish:**
- **Out of my lane (acceptance):** interpreter-and-script-entry-points (wave 4) and test262-run-path (wave 5) each change the root lock alone in their wave. Both run `--locked` acceptances against a lock their bodies leave to the coordinator (`test262-run-path.md:106`). There's no collision, but their checks can't pass in their own worktrees as written.
- **Out of my lane (acceptance):** ess-conformance-target runs its task alone with `-t taskfiles/conform-engine.yml`, and that task has `deps: [suite]`, but `suite` is defined in `taskfiles/spec.yml`. Run that way, Task likely reports that the task doesn't exist; I didn't run it.
- **A wide surface with no clash:** host-interface-hardening may write "any `crates/swell/src/` file a found fault is in". No other wave-5 story writes there, so nothing collides.
- **Not covered by any `Writes:` line:** test262-run-path needs 31 cases to pass, yet lists no engine file it could fix if they don't.

```findings
- file: .engineering/planning/story/syntax-es5-and-strict.md
  line: 117
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story and story:num-and-unicode-early-parts are in wave 1 with no edge between them, and both have to write the root `Cargo.lock`: each acceptance runs `cargo … --locked` on a crate the lock doesn't list yet (`swell-syntax` here, three packages there), which fails until the unit rewrites the lock, yet both bodies leave the lock to the coordinator; the plan needs either an edge that names `Cargo.lock` or one owner for the lock in this wave"
- file: .engineering/planning/story/bytecode-compiler.md
  line: 81
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story and story:heap-values-and-host-surface are in wave 2 with no edge between them, and both have to write the root `Cargo.lock`: this one adds `swell-compiler`, and heap-values adds a `swell-num` dependency to `swell` and runs `cargo test --locked --workspace --no-run`, yet both bodies leave the lock to the coordinator; the plan needs either an edge that names `Cargo.lock` or one owner for the lock in this wave"
- file: .engineering/planning/story/harness-builtins.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "(inferred surface) the story says it runs beside story:interpreter-and-script-entry-points in wave 4, but each adds an object kind with its own [[Call]] and [[Construct]] (the bound function of 10.4.1 here, the ECMAScript function of 10.2 there), and the internal-method dispatch that has to route them is `crates/swell/src/object/mod.rs`, written by story:object-model-and-realms; neither story lists that file in `Writes:`, so the plan needs either an edge that names it or arms for both kinds in object-model-and-realms' dispatch"
```

Paths are relative to the store worktree.
