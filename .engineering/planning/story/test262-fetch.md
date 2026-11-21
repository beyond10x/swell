---
format: aep.planning-md/3
id: story:test262-fetch
kind: story
status: implemented
title: task test262-fetch brings test262 at the pinned commit and refuses a checkout that differs
summary: 'conformance/test262/PIN and taskfiles/test262-fetch.yml: fetch by commit into .cache/test262/<commit>/, check tree 70ab704, ls-tree sha256 1756c10, HEAD and a clean tree; never fetch over a checkout.'
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: conformance/test262/PIN
- confidence: cited
  path: taskfiles/test262-fetch.yml
- confidence: cited
  path: tests/gate/test262-fetch-fetches-a-missing-checkout.sh
- confidence: cited
  path: tests/gate/test262-fetch-keeps-a-checkout-that-passes.sh
- confidence: cited
  path: tests/gate/test262-fetch-refuses-a-changed-working-tree.sh
- confidence: cited
  path: tests/gate/test262-fetch-refuses-a-different-tree.sh
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:04:14Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

test262 is never vendored: it is fetched by commit into `.cache/test262/<commit>/`, which git
ignores, and checked by its tree hash and the SHA-256 of `git ls-tree -r` (design decision 11,
"Pinning and storage"; decision 1: the test262 pin and the ECMA-262 pin move together, in one
change). This story writes the pin once, in a file the fetch task and the runner both read, and
the task that brings the checkout and refuses a wrong one.

Measured on the pinned checkout `.cache/test262/2a1ade303f72d3de80565c921d685222405c32ce/`, each
with `git -C <that directory>`; the design values match:

- `log -1 --format='%H %cI %s'`: `2a1ade303f72d3de80565c921d685222405c32ce
  2026-11-20T10:52:32-05:00 [immutable-arraybuffer] TypedArray internal operations (#4551)`.
- `rev-parse '2a1ade303f72d3de80565c921d685222405c32ce^{tree}'`:
  `70ab70455f270f26e92d097d863a0b08ad020b94`.
- `ls-tree -r 2a1ade303f72d3de80565c921d685222405c32ce | sha256sum`:
  `1756c106c60a6360007cc35c6e4f2d18b23e8045551e27c525612782717cf17f`.
- `ls-tree -r HEAD | wc -l`: 57252 entries. `status --porcelain | wc -l`: 0.
  `rev-parse HEAD`: the pinned commit. `rev-parse --is-shallow-repository`: `true`.

A check of the commit object alone says nothing about the files on disk, which are what the
runner reads; so the task also requires `HEAD` to be the pinned commit and the working tree to be
clean.

The gate is put together as `story:workspace-and-gate` describes: this story writes its own
include file `taskfiles/test262-fetch.yml`, which the root `Taskfile.yml` includes as optional,
and its planted-defect tests as scripts under `tests/gate/` in the form that story gives
(run from the root with `sh`, sandbox `.out/gate-tests/<test name>/`, `<test name>: ok` and exit 0
when the defect is refused). It does not need the files of that story to be demonstrated: its task runs
alone with `task -d . -t taskfiles/test262-fetch.yml test262-fetch`.

Network: the one address this story may fetch is `https://github.com/tc39/test262`, and only when
`.cache/test262/<commit>/` is missing in the worktree. The tests fetch only from local
repositories they make (`file://`).

## Change

Failing test first: `tests/gate/test262-fetch-refuses-a-different-tree.sh` is written before the
task; it prints `FAIL` (no task) before the change and `ok` after.

- `conformance/test262/PIN`, four lines, `<key> <value>`:
  `url https://github.com/tc39/test262`,
  `commit 2a1ade303f72d3de80565c921d685222405c32ce`,
  `tree 70ab70455f270f26e92d097d863a0b08ad020b94`,
  `ls-tree-sha256 1756c106c60a6360007cc35c6e4f2d18b23e8045551e27c525612782717cf17f`.
- `taskfiles/test262-fetch.yml`, task `test262-fetch`. Variables `TEST262_URL`, `TEST262_COMMIT`, `TEST262_TREE`,
  `TEST262_LS_TREE_SHA256` default to the lines of `PIN`, and `TEST262_CACHE` to `.cache/test262`; each can be
  given on the command line (`task test262-fetch TEST262_COMMIT=<c> TEST262_CACHE=<dir> ...`). The directory is
  `$TEST262_CACHE/$TEST262_COMMIT`.
  - Missing: `git init -q`, `git -C <dir> fetch -q --depth 1 "$TEST262_URL" "$TEST262_COMMIT"`,
    `git -C <dir> checkout -q --detach FETCH_HEAD`, then the check; prints
    `test262 <commit>: fetched and checked`.
  - Present: the check only, nothing is fetched; prints `test262 <commit>: present and checked`.
  - The check, each failure printed as one line naming the directory and failing the task:
    `git rev-parse "$TEST262_COMMIT^{tree}"` equals `TEST262_TREE` (`<dir>: tree <actual>, pinned <TEST262_TREE>`);
    `git ls-tree -r "$TEST262_COMMIT" | sha256sum` equals `TEST262_LS_TREE_SHA256`
    (`<dir>: ls-tree sha256 <actual>, pinned <value>`); `git rev-parse HEAD` equals `TEST262_COMMIT`
    (`<dir>: HEAD <actual>, pinned <TEST262_COMMIT>`); `git status --porcelain` prints nothing
    (`<dir>: <n> changed path(s) in the working tree`). A directory that is not a git checkout of
    the commit fails as `<dir>: not a checkout of <TEST262_COMMIT>`. A failing directory is never fetched
    over or removed; the message ends with `remove <dir> and run task test262-fetch again`.
- `tests/gate/test262-fetch-refuses-a-different-tree.sh`: a local repository with one commit, as
  the checkout `<sandbox>/cache/<its commit>`; the task with `TEST262_CACHE=<sandbox>/cache`,
  `TEST262_COMMIT=<its commit>`, `TEST262_TREE=70ab70455f270f26e92d097d863a0b08ad020b94` and its own ls-tree sum
  must exit non-zero and print the `tree` line.
- `tests/gate/test262-fetch-refuses-a-changed-working-tree.sh`: the same checkout with every
  variable set to its own values and one file edited: non-zero, the `changed path(s)` line.
- `tests/gate/test262-fetch-keeps-a-checkout-that-passes.sh`: the same checkout with its own values
  and `TEST262_URL=file:///nonexistent`: exit 0, `present and checked`, so nothing was fetched.
- `tests/gate/test262-fetch-fetches-a-missing-checkout.sh`: an empty `TEST262_CACHE` and
  `TEST262_URL=file://<sandbox>/source`, a local repository whose tip is `TEST262_COMMIT`: exit 0,
  `fetched and checked`, and the files of the source commit on disk.

Gate task (the coordinator adds it to `check` in `Taskfile.yml`, before `build`):
`- task: test262-fetch`, which prints
`test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked` (or `fetched and checked`).

Writes: `conformance/test262/PIN`, `taskfiles/test262-fetch.yml`,
`tests/gate/test262-fetch-refuses-a-different-tree.sh`,
`tests/gate/test262-fetch-refuses-a-changed-working-tree.sh`,
`tests/gate/test262-fetch-keeps-a-checkout-that-passes.sh`,
`tests/gate/test262-fetch-fetches-a-missing-checkout.sh`.

## Acceptance

`slot task -d . -t taskfiles/test262-fetch.yml test262-fetch` exits 0 and prints
`test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked`, and each of the four
scripts, run as `sh tests/gate/<name>.sh` in the worktree of this story, exits 0 and prints its line:
`test262-fetch-refuses-a-different-tree: ok`, `test262-fetch-refuses-a-changed-working-tree: ok`,
`test262-fetch-keeps-a-checkout-that-passes: ok`, `test262-fetch-fetches-a-missing-checkout: ok`.
The `gate-selftest` total on the wave branch belongs to the coordinator.

## Out of scope

Reading or counting any case (`story:test262-counting-runner`). The ECMA-262 pin: no task fetches
or checks `.cache/ecma262/`, and the epic asks for none. Moving the pin. Committing anything of
test262.
