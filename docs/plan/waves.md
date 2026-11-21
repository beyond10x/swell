# The wave plan

Accepted stories are implemented in waves. A wave is the stories that
`aep plan artifact waves --kind story --status active` puts at one depth of the `depends_on`
edges, with no two of them on one file. Each wave has a record beside this file
(`wave-<n>.md`): what landed, what went wrong, the gate's lines, where each finding went.

## How a wave runs

1. The wave branch `wave/<n>-<name>` is cut from `main`. Each story gets a managed worktree
   and a branch `impl/<story>` from the same commit, and a brief file that starts from
   `unit-invariants.md`.
2. One implementor per story, on Opus, the failing test first. It leaves its changes in its
   worktree and reports in the six-line form.
3. The coordinator runs each unit's own gate again in the unit's tree, reads the diff against
   the story, commits on the unit's branch and merges it into the wave branch. A finding the
   unit introduced goes back to the unit (resumed with its context); a pre-existing one becomes
   a draft in the store.
4. On the wave branch, after the last unit: the coordinator writes what is its own (the root
   `Taskfile.yml` entries, `Cargo.lock`, the gate's numbers in `AGENTS.md` and `README.md`),
   runs `slot task check` once, and writes the wave record.
5. The wave branch is merged into `main`, the stories get their evidence and move to
   `implemented`, and the session that decides for the operator gets a status.

## The plan on 2026-11-21

`epic:foundation`, seven stories, accepted after two rounds of the plan critics
(`review-result:foundation-*-round-1`, `-round-2`).

| wave | stories |
|---|---|
| 1 | `workspace-and-gate`, `test262-fetch`, `unicode-data-pin`, `spec-gate`, `host-requirements-gate` |
| 2 | `test262-counting-runner` |
| 3 | `test262-expected-and-sections` |

`host-requirements-gate` starts once `docs/design/host-interface.md` and the markers that name
its requirements are on the wave branch; the other four of wave 1 do not wait for it.

After wave 3 the gate prints the design's counts against a runner that passes nothing:
`test262 total 53801 8982 7 44812 0 44812 0` (measured by the decomposition with `awk` on the
pinned checkout; the runner has to print the same).


All three waves are delivered ([wave 1](wave-1.md), [wave 2](wave-2.md), [wave 3](wave-3.md)); wave 2 also took
`host-requirements-pending`, added on 2026-11-21.

## Slice 1, accepted on 2026-11-21

`epic:slice-01-harness`, ten stories, accepted after two rounds of the plan critics
(`review-result:slice-01-*-round-1`, `-round-2`; eleven findings, all fixed). Slice 1 is done
when its 31 test262 cases (62 runs) pass and no host requirement is pending for slice 1 (68
today).

| wave | stories | closes |
|---|---|---|
| 1 | `num-and-unicode-early-parts` | the early parts of `swell-num` and `swell-unicode` |
| 2 | `syntax-es5-and-strict`, `heap-values-and-host-surface` | the ES5 grammar with strict mode; 17 requirements |
| 3 | `bytecode-compiler`, `object-model-and-realms` | 10 requirements (`HOST-161` added in wave 3) |
| 4 | `interpreter-and-script-entry-points`, `harness-builtins` | 28 requirements; a script runs |
| 5 | `host-interface-hardening` | 15 requirements (`HOST-160` added in wave 2); none pending for slice 1 |
| 6 | `test262-run-path`, `ess-conformance-target` | the 31 cases pass; 13 of the model's 61 scenarios pass on the engine |

The first wave of slice 1 runs beside wave 3 of the foundation: the two stories write
different files. `Cargo.lock` in a wave of several units: each unit regenerates it in its own
tree, and the coordinator regenerates it on the wave branch after the merges.

Delivered: [wave 1](slice-01-wave-1.md), [wave 2](slice-01-wave-2.md), [wave 3](slice-01-wave-3.md). After wave 3: 27 of 161
host requirements have a test, 43 are pending for slice 1.

What works after which wave: after wave 4 the engine evaluates a script through
`evaluate_script` under step and memory budgets; after wave 6 `slot task check` prints the 31
cases of slice 1 as passed. The other slice epics are drafts and are decomposed when their turn
comes; slice 2 starts with the measurement of how many cases per slice include
`propertyHelper.js`.

## What later slices take over

Findings of delivered waves that belong to a slice not yet decomposed. The decomposition of
that slice reads its entries first.

- Slice 2: measure how many cases per slice include `propertyHelper.js`, and send the counts
  with a recommendation to the session that decides for the operator.
- Slice 4: `MakeDay` in `crates/num/src/date.rs` answers NaN for a balanced year above 2^53 in
  magnitude without deciding whether a Number falls on the day
  ([slice 1, wave 1](slice-01-wave-1.md)).
- Slice 5: the text of UTS #51 for 17.0 was not read, and the generated tables need the notice
  the Unicode licence asks for.
- The gate, any wave: a planted-defect script under `tests/gate/` for `unicode-tables-check`;
  a check of the ECMA-262 pin; the sort order of `conformance/host/requirements.txt`; planted
  scripts for `not a checkout` and `not a slice number`.
- The next change to `spec/`: two markers name `HOST-71` for what is now `HOST-157`, and the
  marker table of `docs/design/host-interface-model.md` predates the split of nine requirements.
- Slices 2, 9 and 11: a requirement for a stale or foreign handle returned by the host at the
  return sites of that slice (`host_get_code_for_eval`; an override's result;
  `host_get_import_meta_properties`; the other hooks whose answer holds a handle). The rule is
  decision 6 of `docs/design/host-interface.md`; `HOST-160` holds it for slice 1 only.
- Each slice: the slice number in a refusal message of `swell-syntax` is the reading of the unit
  that wrote it ([slice 1, wave 2](slice-01-wave-2.md)); check it against the design when the
  slice is decomposed.
- The gate, any wave: `host-requirements-check` builds every test of the workspace before the
  `build` step, and each older planted script of the task builds the list again.
- Slice 3: `Error.isError` (not written with the Error constructors of slice 1, wave 3).
- Slice 4: the operator `**` (the parser refuses it today; it needs the deterministic `pow`).
- Slice 5: say in the design whether a regular expression pattern is parsed inside the
  parser's recursion (its levels then add to the script's for the stack bound) or after it.
- The gate, any wave: about 15 planted scripts end a failure message with a path under `.out/`,
  which the log of a run on the forge does not show; the self-test prints the last 20 lines of
  the script's own log only.
- Slice 6: the declarative part of the global environment (`RealmGlobalEnv` in
  `crates/swell/src/realm/mod.rs` is an empty record until `let`, `const` and `class` arrive;
  `crates/swell/src/env/global.rs` says what to add).
