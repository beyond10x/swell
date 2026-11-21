---
format: aep.planning-md/3
id: review-result:foundation-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, foundation stories, round 1
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
revision: 2
---
needs-revision
story:workspace-and-gate — its acceptance expects `gate-selftest: 3 passed, 0 failed` from a task that runs every `tests/gate/*.sh`, but story:test262-fetch, story:unicode-data-pin and story:spec-gate add seven more scripts there in the same wave and no depends_on edge orders them, so the count is only true in its own worktree and not on the wave branch where `slot task check` runs; it needs an edge, or an acceptance that says which tree it is read in — .engineering/planning/story/workspace-and-gate.md:128, .engineering/planning/story/workspace-and-gate.md:43, docs/plan/unit-invariants.md:48

**What I read:** all 6 stories (`aep plan artifact show` for each), plus `aep plan artifact graph`, `relations`, `waves` and `validate`, `AGENTS.md`, `docs/plan/unit-invariants.md`, decisions 2, 11 and 14 of `docs/design/engine.md`, `.gitignore`, and the `.cache` link and its contents.
- **Surfaces:** 6 placed from the cited `Writes:` lines, 0 inferred, 0 that could not be placed.
- **Waves the edges allow:** wave 1 is workspace-and-gate, test262-fetch, unicode-data-pin and spec-gate. Their writes don't overlap; `tests/gate/` is shared, but each writes different files. Wave 2 is test262-counting-runner. Wave 3 is test262-expected-and-sections. The runner and the expected-and-sections story share `crates/test262/src/{main,lib}.rs`, `crates/test262/tests/fixtures/` and `taskfiles/test262.yml`, and an edge orders them.
- **What each acceptance reads, traced to who writes it:** 8 reader-to-writer pairs. 5 are covered by depends_on edges: runner after workspace-and-gate, runner after test262-fetch, expected after runner, and expected after workspace-and-gate and test262-fetch through the runner. The 3 that aren't (workspace-and-gate's script count against the three stories that add scripts) are the finding.

**What I could not establish:**
- `aep plan artifact waves` lists all 6 as unassessed, because no scope is recorded yet. I used the `Writes:` lines instead. `validate` reports valid.
- **Does any story write into `.cache`?** No, as long as the shared `.cache/test262/2a1ade303f72d3de80565c921d685222405c32ce/` exists, and it does.
  - story:test262-fetch writes there only when that directory is missing, and through the link that write would land in the shared cache.
  - Its tests use their own temporary cache under `.out/gate-tests/`, not `.cache`.
  - Its `git status --porcelain` on the shared checkout may rewrite that checkout's `.git/index`. Git does that only when it can take the lock, so it shouldn't break anything, but I didn't measure it.
  - unicode-data-pin downloads into `.out/unicode-fetch/`. The other stories only read `.cache`.
- **Out of my lane (acceptance):** story:test262-counting-runner runs `cargo test -p swell-test262` and `cargo run --locked`, which need `Cargo.lock` to list `swell-test262`. Line 145 leaves that file to the coordinator, so in practice the unit rewrites `Cargo.lock` in its own worktree without listing it in `Writes:`. That collides with nothing, since the runner is alone in its wave.
- **Out of my lane (acceptance):** story:workspace-and-gate's acceptance is `slot task check`, the whole gate, which `docs/plan/unit-invariants.md:48` reserves for the wave branch.

```findings
- file: .engineering/planning/story/workspace-and-gate.md
  line: 128
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance expects `gate-selftest: 3 passed, 0 failed` from a task that runs every `tests/gate/*.sh`, but story:test262-fetch, story:unicode-data-pin and story:spec-gate add seven more scripts there in the same wave and no depends_on edge orders them, so the count is only true in its own worktree and not on the wave branch where `slot task check` runs; it needs an edge, or an acceptance that says which tree it is read in"
```

Paths are relative to the store worktree, $HOME/.local/state/worktree/trees/b10x/swell/swell-bootstrap.
