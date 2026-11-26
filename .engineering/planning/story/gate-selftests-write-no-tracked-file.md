---
format: aep.planning-md/3
id: story:gate-selftests-write-no-tracked-file
kind: story
status: draft
title: The gate's selftests write no tracked file
summary: site-data-check-refuses-a-percentage-rounded-up-to-100 left SITE_DATA_ENGINES at its default, so every gate run overwrote the tracked website/data/engines.json with planted counts; every site-data selftest now writes each output of the task under its own directory, and gate-selftest fails, naming the files, when a tracked file differs after its scripts from what it was before them.
revision: 2
---
## Context

The gate step `gate-selftest` runs every script of `tests/gate/` four at a time. One of them,
`site-data-check-refuses-a-percentage-rounded-up-to-100.sh`, runs `task site-data` with
`SITE_DATA_FILE` and `SITE_DATA_EXPECTED` under its own directory and leaves `SITE_DATA_ENGINES`
and `SITE_DATA_OUT` at their defaults. `site-data` also writes the compare page's data, so every
gate run overwrote the tracked `website/data/engines.json` with the planted counts (7,010 of
7,011, sources under `.out/gate-tests/...`), and its work directory `.out/site-data/engines-work`
was shared with the gate's own `site-data-check`. Nothing in the gate noticed: a later
`site-data-check` compares the regeneration with the overwritten file only when it runs after
the selftests, and a developer's tree was left with a change nobody made on purpose.
`site-data-check-refuses-a-stale-status-file.sh` already passed `SITE_DATA_ENGINES` under its own
directory.

The task `site-data` (`taskfiles/site.yml`) writes three things: `SITE_DATA_FILE` (default
`website/data/status.json`), `SITE_DATA_ENGINES` (default `website/data/engines.json`, through
`taskfiles/site-engines.sh`) and the work directory `SITE_DATA_OUT/engines-work` (default
`.out/site-data`). `site-data-check` writes `SITE_DATA_OUT/status.json`,
`SITE_DATA_OUT/engines.json` and the same work directory, and reads `SITE_DATA_FILE` and
`SITE_DATA_ENGINES`.

## Change

- Every selftest that runs `site-data` passes `SITE_DATA_OUT`, `SITE_DATA_FILE` and
  `SITE_DATA_ENGINES` under its own directory; every one that runs `site-data-check` passes
  `SITE_DATA_OUT` there. Each unsets every `SITE_DATA_` variable of `taskfiles/site.yml` first,
  so a variable of the caller's environment does not point an output back into the tree.
- `gate-selftest` (`Taskfile.yml`) records, before its scripts run, each tracked file of the git
  work tree `GATE_SELFTEST_TREE` (default `.`) that differs from its commit at `HEAD`, with the
  git digest of its content, and again after them. It fails when the records differ and prints
  `gate-selftest: the scripts changed tracked files of <tree> (compared with before they ran):
  <files>`. A change that was there before the step and stays does not count.
- `tests/gate/gate-selftest-refuses-a-script-that-writes-a-tracked-file.sh` runs `gate-selftest`
  on a git repository of its own under `.out/gate-tests/`, never on this tree.

## Acceptance

- `slot task check-runner` exits 0 and leaves `git status --short` as it was before it ran;
  `website/data/engines.json` is unchanged.
- `gate-selftest` run on the version of
  `site-data-check-refuses-a-percentage-rounded-up-to-100.sh` before this change fails and
  names `website/data/engines.json`.
- `gate-selftest-refuses-a-script-that-writes-a-tracked-file.sh` passes in under 5 s: a planted
  script that writes only an untracked file passes though another tracked file differed from
  `HEAD` before the step; one that writes a committed file fails naming that file alone; one
  that changes the file already changed before the step fails naming it.
