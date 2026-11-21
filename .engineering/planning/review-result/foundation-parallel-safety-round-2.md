---
format: aep.planning-md/3
id: review-result:foundation-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, foundation stories, round 2
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
- reviews: story:host-requirements-gate
revision: 1
---
approve

No two stories in this set collide if they run at the same time, and every acceptance that depends on another story's output has a depends_on edge to it.

- **Wave 1** is workspace-and-gate, test262-fetch, unicode-data-pin, spec-gate and host-requirements-gate. Their `Writes:` lines share no file. The only shared directory is `tests/gate/`, and the 21 scripts written there all have different names.
- **Task names** don't clash between the root `Taskfile.yml` and the five include files (`validate`, `suite`, `conform`, `host-requirements-check`, `test262-fetch`, `test262`, `unicode-data-check`).
- **Wave 2** is test262-counting-runner alone, so it is safe for it to regenerate `Cargo.lock`. **Wave 3** is test262-expected-and-sections, which changes the runner's `crates/test262/src/{main,lib}.rs`, `crates/test262/tests/fixtures/` and `taskfiles/test262.yml`; its edge to the runner orders that.

**What I read:** all 7 stories (`aep plan artifact show` for each), plus `aep plan artifact graph`, `waves` and `validate`, `git log` and the `spec/` folder of the store worktree.
- **Surfaces:** 7 placed from the cited `Writes:` lines, 0 inferred, 0 that could not be placed.
- **What each acceptance reads, traced to who writes it:** 5 reader-to-writer pairs, all covered by depends_on edges:
  - runner after workspace-and-gate (the root `Taskfile.yml` and `lock-check`)
  - runner after test262-fetch (`PIN` and `taskfiles/test262-fetch.yml`)
  - expected-and-sections after the runner
  - expected-and-sections after workspace-and-gate and test262-fetch, through the runner
- **Round-1 finding:** resolved. workspace-and-gate's acceptance is now run "in its own worktree" and names its five script lines instead of a `gate-selftest` total (`.engineering/planning/story/workspace-and-gate.md`, Acceptance).

**What I could not establish:**
- `aep plan artifact waves` still reports all 7 as unassessed because no scope is recorded. I used the `Writes:` lines instead. `validate` reports valid.
- **Does any story write into `.cache`?** No, as long as the shared `.cache/test262/2a1ade303f72d3de80565c921d685222405c32ce/` exists, and it does.
  - test262-fetch writes it only when it is missing, and through the link that write would land in the shared cache.
  - Its tests use their own temporary cache under `.out/gate-tests/`, not `.cache`.
  - Its `git status --porcelain` may rewrite that checkout's `.git/index`. Git does that only when it can take the lock, so it shouldn't break anything, but I didn't measure it.
  - unicode-data-pin downloads into `.out/unicode-fetch/`. Every other story only reads `.cache`.
- **Possible clash ruled out:** the runner adds a second package to `Cargo.lock`, and workspace-and-gate's clippy and build scripts copy that lock with only `crates/swell`. I tested it on a scratch workspace outside the repository: a lock listing an extra local package not in the workspace passed `cargo build --workspace --locked` (exit 0) and the lock was left unchanged. So the two scripts keep passing after the runner lands.
- spec-gate and host-requirements-gate both read `spec/` and neither writes it. `docs/design/host-interface.md` and the second round of `spec/` come from outside the set, so I didn't judge them.

```findings
[]
```

Paths are relative to the store worktree.
