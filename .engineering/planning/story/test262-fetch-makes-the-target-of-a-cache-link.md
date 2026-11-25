---
format: aep.planning-md/3
id: story:test262-fetch-makes-the-target-of-a-cache-link
kind: story
status: draft
title: test262-fetch makes the target of a .cache link that dangles, so a worktree with the link runs on the build node
summary: The pool sends a .cache link as a link; on the node its absolute target is missing and mkdir fails; the fetch resolves the link and creates the target.
revision: 1
---
Found 2026-11-25 on the pool's build node; the fix is the one slot's maintainer recommended.

## Context

- A worktree's `.cache` is a symbolic link to the primary checkout's `.cache` (an absolute
  path under the home directory). The pool syncs a link that leaves the tree as a link, and on
  the build node the same absolute path does not exist, so the link dangles.
- `taskfiles/test262-fetch.yml:46` runs `mkdir -p "$dir"` with `$dir` under `.cache/test262/`.
  With a dangling `.cache` it fails: "mkdir: cannot create directory '.cache': File exists",
  and the fetch reports a failed fetch of the pinned commit.
- The node reaches github.com.

## Change

- `test262-fetch` resolves the link before it creates the directory: `mkdir -p --
  "$(readlink -m "$dir")"` (or the same for `.cache` first), so a dangling link's target is made
  and the pinned checkout is fetched there once; every worktree mirrored on the node then reads
  it through its link.
- A planted-defect script under `tests/gate/` makes a tree whose `.cache` is a link to a missing
  directory under `.out/gate-tests/` and shows `test262-fetch` creates the target (fetching
  from a local bare repository, no network), and that the task still refuses a checkout whose
  tree hash differs.
- `AGENTS.md` ("Worktrees"): a worktree gets the link again; the copy stopgap goes.

## Acceptance

- The planted script prints `<name>: ok`; `gate-selftest` counts one more.
- On the build node, in a new worktree with the link, `slot --timeout 30m task test262-part
  TEST262_PART=1/8` exits 0 and its log has "1 symlink leaves the tree (sent as a link)".
- `slot task check` exits 0.
