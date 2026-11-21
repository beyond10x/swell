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
| 5 | `host-interface-hardening`, `test262-run-path` (planned in wave 6, moved here) | 15 requirements (`HOST-160` added in wave 2); none pending for slice 1; the 31 cases pass |
| 6 | `ess-conformance-target`, and the change of `spec/` it asked for | 14 of the model's 58 scenarios pass on the engine (planned: 13 of 61) |

The first wave of slice 1 runs beside wave 3 of the foundation: the two stories write
different files. `Cargo.lock` in a wave of several units: each unit regenerates it in its own
tree, and the coordinator regenerates it on the wave branch after the merges.

Delivered: [wave 1](slice-01-wave-1.md), [wave 2](slice-01-wave-2.md), [wave 3](slice-01-wave-3.md), [wave 4](slice-01-wave-4.md), [wave 5](slice-01-wave-5.md), [wave 6](slice-01-wave-6.md). Slice 1 is
delivered: 70 of 164 host requirements have a test, none is pending for slice 1, the gate
counts 3,969 test262 cases as passed, the 31 of slice 1 among them, and 14 of the model's 58
scenarios pass on the engine.

What works after which wave: after wave 4 the engine evaluates a script through
`evaluate_script` under step and memory budgets; after wave 5 `slot task check` prints the 31
cases of slice 1 as passed; after wave 6 it runs the model's scenarios against the engine.
The other slice epics are drafts and are decomposed when their turn comes.

## Slice 2, `language-core`

`epic:slice-02-language-core`: the rest of the ES5-level language (`eval`, `with`, `arguments`,
labels, automatic semicolon insertion) and the global `Function`. Decomposed on 2026-11-21
into six stories (accepted, `active`), read by the four plan critics in two rounds (`review-result:slice-02-*`: 13
findings, all applied). Measured on `main` before the slice
(`test262-slice 02 5245 9173 0 0 5245 2125 3120 0`): 2,125 of its 5,245 cases pass. The first
cause of the 3,120 failing cases, by a throwaway copy of the runner that prints one line per
run: automatic semicolon insertion 411, `\u` escapes 352, `eval` 305, `Number` 201, `let` and
`const` 295, `Boolean` 164, `arguments` 128, `with` 115, labels 86, and smaller groups. About
1,000 cases need syntax or builtins of a later slice and move by rule lines in wave 4; about
4,270 stay. The slice is done when every run of what stays passes.

| wave | stories | closes |
|---|---|---|
| 1 | `test262-run-causes`, `syntax-language-core`, `number-boolean-and-global-functions-early` | one line per run with its cause, and `$262.evalScript`; the grammar of slice 2 (semicolon insertion, labels, `??`, `?.`, logical assignment, plain methods and shorthand properties); `Number`, `Boolean`, `isNaN`, `isFinite` |
| 2 | `with-arguments-and-dynamic-names` | `with` and the `arguments` object; names looked up at run time where the design says |
| 3 | `eval-and-function-constructor` | `eval` and `Function` through one entry, the two hooks, the count of refusals; 10 requirements |
| 4 | `slice-02-conformance-close` | the cases that need a later slice move, each with its evidence in a list the gate checks; every run of the slice passes |

Delivered: [wave 1](slice-02-wave-1.md), [wave 2](slice-02-wave-2.md). After wave 2 slice 2 passes 3,743 of its 5,245 cases, the gate
counts 6,262 cases as passed, and 44,212 of the 85,556 runs end at a refusal. The base run lines
for the next wave are the run lines of this wave on its branch after its close.

Open, to decide before wave 4 with the run lines: `Array.prototype.toString` is missing (slice
3), so an array converts through `Object.prototype.toString` and answers `[object Array]`;
`isNaN([])` is `true`. Count the runs of slice 2 that fail on it and take it early if they do.

In wave 1 the three stories share no file but `conformance/test262/expected.txt`, which the
coordinator regenerates on the wave branch after the merges. The design changes the stories
wait for are in the design since wave 6 of slice 1 (`20c1f35`): the parts taken before their
slice, `Function` in slice 2, name resolution under `with` and direct `eval`, the omission
rule, `HOST-162` to `HOST-164`.

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
- The next change to `spec/`: the seed of the scenarios. The pinned secret scanner with its
  default rules reports the present value of `hash_key` (the bytes 0 to 15, in base64) and
  not a seed of 16 zero bytes, probed by the session that decides for the operator on
  2026-11-21; the gates run the scanner with their own configuration, so a staged file and a
  bot commit are the test. If it holds, change the seed in all scenario files in one wave
  and tell that session, which then removes the ten exceptions.
- The next change to `spec/`: where `spec/domains/host.yaml` (the notes at `ParseScript` and
  `EvaluateScript`) and `docs/design/host-interface-model.md` call the refused synthesis of
  `parsed` (`ESS-SYNTH-003`, `ESS-SYNTH-004`) "an ESS limit", write "a defect of ESS, issue 801
  of beyond10x/ess": ruled a defect and filed by the session that decides for the operator on
  2026-11-21 (an owner-link guard beside an owner-existence guard is not witnessed). Check
  that `host-requirements-check` still accepts the marker text. When ESS fixes it, the
  synthesized scenarios return and the authored ones that replace them are reviewed.
  `ESS-COMMAND-004` (a command reads one related row) is a documented limit and stays.
- Done in wave 6 of slice 1: the two markers that named `HOST-71` name `HOST-157`, and the
  marker table of `docs/design/host-interface-model.md` is regenerated.
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
- Slice 3: a way for a host function to create a realm. The runner's `$262.createRealm` makes
  its realms before the case runs and runs the case again when they do not suffice
  (`crates/test262/src/host.rs`, [slice 1, wave 5](slice-01-wave-5.md)); the cross-realm cases
  of the builtins start in slice 3. A design change before the code.
- Slice 2, at its decomposition: the 23 `propertyhelper-*` cases of `test/harness` move from
  slice 3 to slice 2 with the global `Function` (`AGENTS.md`, "How the work is run").
- Slice 4: the operator `**` (the parser refuses it today; it needs the deterministic `pow`).
- The next change of the host interface design: the data of a host function can be written
  (`new_host_function`) and not read back. `Cx::host_data` reads host objects only, and
  `HOST-108` covers only those. The runner found it ([slice 2, wave 1](slice-02-wave-1.md)):
  with the realm index as the data of `$262.evalScript`, every run ended in
  `Terminated(EngineFault)`; it compares the callee with roots it keeps instead. Decide
  whether a host function reads its data (and through which method) or the parameter goes.
- After wave 2 of slice 2, before wave 3: the frame of the run loop. `Agent::execute` is 19,216
  of the 31,314 bytes one re-entry takes in a debug build, and every instruction adds its
  locals to it ([slice 2, wave 1](slice-02-wave-1.md)). A unit moves the arms of the dispatch
  into functions of their own and measures the bytes per re-entry before and after; the
  constant `STACK_BYTES_PER_REENTRY` (37 KiB) follows the measurement.
- The slice that brings the first allocation-heavy cases (slice 3 at the latest): a run that
  fills its memory budget with `a.push({})` takes 0.5 s for 1 MiB, 1.2 s for 2 MiB and 4.1 s
  for 4 MiB in a debug build, and did not finish 64 MiB in about nine minutes
  ([slice 1, wave 6](slice-01-wave-6.md)). Measure where the time goes (when a collection
  starts, how an array grows) before the builtins of slice 3 depend on it.
- Slice 5: say in the design whether a regular expression pattern is parsed inside the
  parser's recursion (its levels then add to the script's for the stack bound) or after it.
- The gate, any wave: about 15 planted scripts end a failure message with a path under `.out/`,
  which the log of a run on the forge does not show; the self-test prints the last 20 lines of
  the script's own log only.
- Slice 6: the declarative part of the global environment (`RealmGlobalEnv` in
  `crates/swell/src/realm/mod.rs` is an empty record until `let`, `const` and `class` arrive;
  `crates/swell/src/env/global.rs` says what to add).
