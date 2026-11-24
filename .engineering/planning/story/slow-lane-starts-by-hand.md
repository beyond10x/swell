---
format: aep.planning-md/3
id: story:slow-lane-starts-by-hand
kind: story
status: implemented
title: The lane slow can be started by hand, so a release tip whose push changed prose only gets its run
summary: slow.yml gets a workflow_dispatch trigger with no inputs; docs/releasing.md step 5 says how the deciding session starts the lane on a tip whose push changed no path of the lane's list; the rule that a release needs a green slow run on its commit stays.
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: .github/workflows/slow.yml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: docs/releasing.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T05:23:52Z", actor: "human:operator", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-24T05:23:52Z", actor: "human:operator", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-24T08:04:09Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":2}}, executor: "agent:coordinator"}
---
## Context

The lane `slow` (`.github/workflows/slow.yml`, the workflow named "Slow lane") makes the full
test262 run and the full counts of the heavy tests on the forge. It starts on a pull request
and on a push to `main`, and in both cases only when the change touches a path of its list
(`crates/**`, the manifests, the toolchain files, `Taskfile.yml`, the task files of the lane,
`conformance/test262/**`, `docs/design/engine.md` and the workflow file itself; 17 paths in
each of the two lists). It has no other trigger.

`docs/releasing.md`, step 5, says: "a release is not tagged on a commit whose `slow` run is
red or missing". A release whose last commit changes prose only therefore cannot be tagged:
the push of that commit starts no run of the lane, and nothing else can start one. Release
`0.8.0` met this on `270252a`, the commit that sets the heading of its changelog section to
the day of the tag (three Markdown files over `6d87989`; the push that brought `6d87989` to
`main` started the lane because it carried the gate change `a182a02`, which changes
`Taskfile.yml`). The rule of step 5 is kept as it is; the lane gets a way to run on such a
tip.

Release `0.8.0` itself does not need a run started by hand: it is tagged on the commit of
`main` that carries this change. The workflow file is a path of its own list, so the push
that brings this change to `main` starts the lane on that tip. The trigger is for the next
release whose last push changes prose only.

The workflow has two expressions. One reads the event and the ref, the concurrency group
(`group: slow-${{ github.event_name }}-${{ github.ref }}`, with `cancel-in-progress: true`);
the other reads the output of a step of the same job
(`${{ steps.toolchain.outputs.channel }}`). The workflow
has no `if:`, reads no `github.sha` and no `github.event.*`, and its checkout step names
no ref, so it checks out the commit the run was started for. A run started by hand on `main`
is in the group `slow-workflow_dispatch-refs/heads/main`: it cancels no run that a push
started and is cancelled by none, and a second run started by hand on `main` cancels the
first.

The two workflows of the documentation site (`.github/workflows/pages.yml`,
`.github/workflows/b10x-docs-site.yml`) already carry `workflow_dispatch`.

## Domain relations

- None. No noun of the engine or of the host interface is introduced.

## Change

Failing check first: on the base the first command of the acceptance prints two lines,
`  pull_request:` and `  push:`.

- `.github/workflows/slow.yml`: a third trigger under `on:`, after `push:` and its lists,
  `workflow_dispatch:` with no inputs, followed by the empty line that stands before
  `concurrency:`. Nothing else in the file changes: no line is removed, the two `paths:`
  lists, the jobs, the permissions and the concurrency group stay byte for byte.
- `Taskfile.yml`, the summary of the task `slow` (text only, no command): the sentence "The
  workflow .github/workflows/slow.yml runs it on a push to main and on a pull request that
  changes a source of the digest, the expected file or the digest file." says that the push
  and the pull request each start it only when they change a path of the workflow's list,
  and gets the third case, a run started by hand.
- `docs/releasing.md`, step 5: after the sentence "a release is not tagged on a commit whose
  `slow` run is red or missing", the case of a tip whose push changed no path of the lane's
  list: the deciding session starts the lane by hand on `main` at that tip and waits for that
  run as for one a push started, with the commands in the form of the commands of that step:
  `$gates gh -- workflow run slow.yml --ref main`, then `$gates gh -- run list --workflow
  slow.yml --branch main --event workflow_dispatch --limit 5 --json
  databaseId,headSha,event,status`, then `run watch <run-id> --exit-status` on the run of that
  list whose `headSha` is the tip. The text says that a run started by hand takes some seconds
  to appear in the list and that a run with another `headSha` is not the one to wait for; and
  the sentence, in these words, that "a run of the lane on an earlier commit does not stand in
  for it".
- No new check of the gate and no new planted defect: the count of planted defects (66) and
  every line of the gate's block stay as they are. No task of the gate reads the triggers of
  a workflow (`toolchain-check` reads every workflow for a compiler name, `runner-env-check`
  reads `check.yml`); what holds the trigger is the acceptance's first command and the
  release procedure, which fails at step 5 without it.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): this
  story adds no test and no step; a test is killed at 60 s and the gate's test steps fail over
  120 s, as before. A pool job ends after 10 minutes unless it names its time with
  `slot --timeout`, at most 30 minutes. No Python in any command.

Writes: `.github/workflows/slow.yml` (the trigger), `Taskfile.yml` (the one sentence of the
summary of the task `slow`), `docs/releasing.md` (step 5). For the coordinator, in the change
that merges this story: `AGENTS.md`, "Commands and the gate", the sentence on the lane `slow`
and a release. Not written: `.github/workflows/check.yml` (it runs on every push to `main`),
the two `paths:` lists of `slow.yml`, `taskfiles/`, `tests/gate/`, `CHANGELOG.md` (no change
for a user of the engine), every source file.

## Acceptance

`BASE` is the commit the unit starts from; `.out/` is the ignored directory of the tree.

1. The triggers: `awk '/^on:/{o=1;next} /^[^ #]/{o=0} o && /^  [a-z_]+:/'
   .github/workflows/slow.yml` prints exactly the three lines `  pull_request:`, `  push:`
   and `  workflow_dispatch:` (two on the base), and `awk '/^  workflow_dispatch:$/ { getline;
   print "[" $0 "]" }' .github/workflows/slow.yml` prints `[]` (the trigger has no inputs and
   is the last key of `on:`).
2. Nothing else of the workflow moved: `git diff --numstat "$BASE" --
   .github/workflows/slow.yml` prints a line whose second field, the removed lines, is `0` and
   whose first is `1` or `2`; `git show "$BASE:.github/workflows/slow.yml" | grep '^      - "'
   > .out/paths-base.txt`, `grep '^      - "' .github/workflows/slow.yml >
   .out/paths-after.txt` and `cmp .out/paths-base.txt .out/paths-after.txt` exits 0 (34 lines
   each); `grep -c '^      - "\.github/workflows/slow\.yml"$' .github/workflows/slow.yml`
   prints `2`, so the push that carries this change starts the lane; `grep -cE
   'github\.(event\.|sha)|^[[:space:]]*if:' .github/workflows/slow.yml` prints `0` and `grep
   -c '{{ *github\.' .github/workflows/slow.yml` prints `1`, the line of the concurrency
   group, as on the base.
3. The files: `git diff --stat "$BASE"` names `.github/workflows/slow.yml`, `Taskfile.yml` and
   `docs/releasing.md` and no other file; `git diff --numstat "$BASE" -- Taskfile.yml` shows
   lines of the summary of the task `slow` only, which the report quotes.
4. The procedure: `awk '/^5\. \*\*Publish/,/^6\. \*\*Tag/' docs/releasing.md | grep -c --
   'gh -- workflow run slow.yml --ref main'` prints `1` (`0` on the base); the same range
   piped to `grep -c 'does not stand in for it'` prints `1`; piped to `grep -c 'headSha'` it
   prints `1` or more (`0` on the base); piped to `grep -c 'run watch'` it prints `2`, the
   number on the base, or more.
5. The tasks of the gate that read the workflows pass, each one pool job: `slot task -d . -t
   taskfiles/forge.yml toolchain-check` and `slot task -d . -t taskfiles/forge.yml
   runner-env-check` exit 0, and `slot task gate-selftest` prints `gate-selftest: 66 passed, 0
   failed`. They show that the change breaks none of them; none of them shows the trigger.

On the forge, not the unit's to show: the push that carries this change starts the lane on
its tip; a run started by hand is the deciding session's at the next tip whose push changes
prose only, and until one was made, the claim that such a run checks out the tip of `main`
and runs the job rests on the reading of the workflow in Context. The deciding session
confirms at that first use that the policy of its `$gates` command admits `workflow run` and
the `--json` and `--event` options of `run list`; a refusal there is reported and not worked
around.

## Out of scope

A change of the rule of step 5 (a run on an earlier commit standing in for a missing one); a
change of the lane's path lists; inputs of the manual trigger; a manual trigger of `check`; a
gate check that reads the triggers of the workflows; the ruleset of the repository.
