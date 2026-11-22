# Slice 3, wave 3 of 3: the close. Slice 3 is delivered with 7,010 of its 7,011 cases

One wave of the [wave plan](waves.md), 2026-11-22, on the branch `wave/s3-1`. One unit,
implemented by a sub-agent on Opus in its own worktree, and two commits on the wave branch
after it. Every commit is the organization bot's. Before: [wave 1](slice-03-wave-1.md),
[wave 2](slice-03-wave-2.md); beside the waves:
[the gate and the site](slice-03-gate-and-site.md).

## Units

| unit or commit | what landed | its own check | commit |
|---|---|---|---|
| `slice-03-conformance-close` | 615 failing cases of slice 3 move to the slice they need by rule lines of the block `swell-slices`, and the 9 passing cases of the seven directories ruled on 2026-11-21 move with their directories and stay counted as passed: 624 lines join `conformance/test262/moves.tsv` (1,656 in all). `test262-moves-check` refuses a line whose target slice is not greater than its source, with a fourth plant in its script. The task `test262` and the four tasks the gate unit added read every `TEST262_` variable as a template value, with a planted script for an exported and a passed path. The four requirement lines of slice 3 (HOST-21, 120, 155, 165) name their tests. Four `omit` lines in `sections.txt`. `later-parts.tsv` gains the tokens the moves rest on. Section 12 of the design: the paragraph "Slice 3's cases that need a later slice". `expected.txt` and `expected.digest` written for the engine as it is | `3d9c1bf`: the full run exits 0 with "286 lines equal conformance/test262/expected.txt", `test262-slice 03 7011 13829 0 0 7011 7010 1 0`. `632341d`: `test262-moves-check`, `host-requirements-check` with slices 1 to 3 delivered (165 requirements: 84 test, 81 pending, none of slice 3) and `test262-check` exit 0; `gate-selftest` 38 passed, 1 failed (the stale status file of the site, [the gate and the site](slice-03-gate-and-site.md)); the planted script on exported and passed paths takes 15 to 18 s | `3d9c1bf`, `632341d` (merge `baaf69b`) |
| delivery | `conformance/slices-delivered.txt` holds 3: `host-requirements-check` now fails while a requirement is pending for slice 3 (none is). `website/data/status.json` written again by `task site-data` from the gate's files (12,120 cases passed of 44,812 run; three slices delivered), and `site-data-check` a step of `check` after `host-requirements-check`. `docs/plan/unit-invariants.md`: a pool job names its time as `slot --timeout 30m` | 165 requirements, 84 tested, 81 pending for later slices; the planted script of `site-data-check` passes again | `7aa58e0` |
| the digest | `rust-toolchain.toml` and `taskfiles/test262.yml` are sources of the digest and changed after the close wrote it; `test262-expected` ran the full run on the wave branch and wrote it again | `expected.txt` comes out unchanged, 286 lines; the digest is `02158d3cf750`. The coordinator's log: the job waited 110 s for a slot and ran 836 s (`wave-s3-1/test262-expected.log`) | `b35a4aa` |

The close unit was opened on `c108c2d`, which did not compile, and took the repair of
[wave 2](slice-03-wave-2.md) into its branch (`8e0aa7d`). It took the gate unit and the
documentation site the same way (`fbab112`), resolved the conflicts, and wrote the digest for
the engine as it is (`632341d`). The close changed no file under `crates/`.

**What works now.** Slice 3 is delivered. `conformance/test262/expected.txt`:

```
test262 total 53801 8982 7 44812 12120 32692 0
test262-slice 03 7011 13829 0 0 7011 7010 1 0
```

At the start of the slice (`main`, `6a43e18`):
`test262 total 53801 8982 7 44812 7742 37070 0` and
`test262-slice 03 7635 15074 0 0 7635 2762 4873 0`.

**The one case that fails.** The case `test/built-ins/String/prop-desc.js` fails in the
pinned harness before the engine's answer is judged; that it fails in another engine is
inferred from the harness source, not measured. The reasoning, with the lines of the harness,
is the paragraph "Slice 3's cases that need a later slice" of `docs/design/engine.md`, section
12. The case is measured again when the test262 pin moves:
`task:slice-03-harness-case-measured-at-the-next-pin` (`6becad6`).

**How the slice was closed.** With every builtin of slice 3 in place, 616 of its 7,635 cases
still failed (`3d9c1bf`). 615 need a part a later slice delivers; each has a line in
`moves.tsv` whose evidence `test262-moves-check` finds in the case. The block gains 552 lines
`dir <path> <slice>`: 11 for a directory and 541 for a single file (the design). Seven of the
11 are the directories ruled on 2026-11-21 (79 cases, 9 of them passing only on the
`TypeError` of the missing method): `Array/from/`, `Array/prototype/keys/`, `values/`,
`entries/`, `Object/fromEntries/` and `Object/groupBy/` to slice 6,
`String/prototype/normalize/` to slice 5. The 624 lines by target slice (`3d9c1bf`; the
groups in the design):

| target slice | lines | the largest groups (the design) |
|---|---|---|
| 4 | 193 | `Date` 137, members of `Math` other than `max` 44 |
| 5 | 140 | `RegExp` 98, regular expression literals 13, case mapping outside ASCII 12, `normalize` 11, comparison outside ASCII 2 |
| 6 | 232 | later syntax 144, `Array.from` 25, `Symbol` 18, the Array iterator methods 18 |
| 7 | 3 | generator functions |
| 8 | 19 | `class` 18, a private name 1 |
| 9 | 16 | `Proxy` 8, `Reflect` 4 |
| 10 | 4 | async functions 3, `Promise` 1 |
| 12 | 3 | `BigInt` |
| 13 | 14 | `ArrayBuffer` and the typed arrays |

`expected.txt` holds no `test262-moves` line; the gate prints it, for the whole list of both
closes: `test262-moves: 1656 case(s) moved, 3:78 4:318 5:276 6:719 7:44 8:104 9:18 10:78 11:4
12:3 13:14` ([The gate](#the-gate)).

**The counts.** Slice 3 has 7,011 cases and 13,829 runs (7,635 and 15,074 at the start). The
slices that received cases, cases now (at the start): 4 has 1,516 (1,323), 5 has 2,394
(2,254), 6 has 4,436 (4,204), 7 has 1,476 (1,473), 8 has 6,292 (6,273), 9 has 1,768 (1,752),
10 has 6,651 (6,647), 12 has 320 (317), 13 has 3,058 (3,044).

Passed cases per slice in `expected.txt`, of the cases the slice has: 01 31 of 31, 02 4,259 of
4,259, 03 7,010 of 7,011, 04 249 of 1,516, 05 50 of 2,394, 06 100 of 4,436, 07 5 of 1,476, 08 1
of 6,292, 09 6 of 1,768, 10 10 of 6,651, 11 13 of 1,281, 12 39 of 320, 13 0 of 3,058, 14 0 of
479 run, 15 31 of 1,094, 16 0 of 82, 17 0 of 35, 18 16 of 1,155, 19 300 of 1,474.

**Passes of the start the builtins took away**, none in slices 1 to 3 (`3d9c1bf`):
`Function/prototype/apply/this-not-callable.js` ends on the counted refusal of a regular
expression literal and moves to slice 5; five cases of String methods that expect a
`TypeError` for a Symbol get a `ReferenceError` until slice 6 has `Symbol`; two annex B cases
compare `trimLeft` and `trimRight` with `trimStart` and `trimEnd`, which now exist (slice 18).
The cases by name and the commits that lowered them: [wave 2](slice-03-wave-2.md).

## What went wrong, and what was decided

- **The close was opened on a branch that did not compile** and stopped at its first full
  run ([wave 2](slice-03-wave-2.md), "What went wrong").
- **The gate unit's test262 tasks read a path two ways.** After the merge with the gate
  unit, the four tasks it added read their `TEST262_` variables through `env:`: with a path
  both exported and passed, `slow-test262` ran on the passed directory and compared the
  lines of the exported one. They read them as template values now; the planted script covers
  `test262-check` too, red on the merged file and green after (`632341d`).
- **The digest went stale on the branch.** Two of its sources changed after the close wrote
  it (`rust-toolchain.toml` by the forge unit, `taskfiles/test262.yml`); `b35a4aa` wrote it
  again with `expected.txt` unchanged.
- **A number of a commit body.** `3d9c1bf` calls its four `omit` lines "for the
  RangeError at string_length"; in `conformance/test262/sections.txt` three are
  `string-length-limit decision-3` (`sec-string-objects`, `sec-array-objects`,
  `sec-json-object`) and one is `sec-string-objects case-mapping-and-comparison-outside-ascii
  section-12-slice-5`.
- **Decided on 2026-11-22** by the session that decides for the operator: slice 3 is delivered
  with 7,010 of its 7,011 cases; no rule line for a defect of the harness, for one case
  (`6becad6`, the design, section 12).

## Where each finding went

| finding | where it went |
|---|---|
| `test/built-ins/String/prop-desc.js` | `task:slice-03-harness-case-measured-at-the-next-pin` |
| An exported variable beats the command line in a task's `env:` ([wave plan](waves.md), "What later slices take over") | done: the test262 tasks read every `TEST262_` variable as a template value (`3d9c1bf`, `632341d`) |
| The 9 passing cases of the seven directories | slices 5 and 6, counted as passed there, each with a token that ends in `; passed at the move` |
| `Object.getOwnPropertySymbols` | slice 6, with its directory |
| `website/data/status.json` stale since the site was merged | written again in `7aa58e0` ([the gate and the site](slice-03-gate-and-site.md)) |

## The gate

The gate ran once for the branch, after its last unit: its lines are in
[the gate and the site, "The gate"](slice-03-gate-and-site.md#the-gate). The lines of that run
that state slice 3:

```
host-requirements: 165 requirement(s), 165 line(s): 84 test, 81 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
test262 total 53801 8982 7 44812 12120 32692 0
test262-slice 03 7011 13829 0 0 7011 7010 1 0
test262-moves: 1656 case(s) moved, 3:78 4:318 5:276 6:719 7:44 8:104 9:18 10:78 11:4 12:3 13:14; slices 1 to 19: 44931 85794
```
