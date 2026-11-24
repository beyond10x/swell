# Slice 5, wave 3, with the first four stories of slice 7

The branch `wave/s5-1` over `main` at `16e0bc2` (release `0.7.0`, tagged on `e0bb1ec`), 2026-11-23 10:21 CET to
21:20 CET. Ten stories, each implemented by a sub-agent in its own worktree; the coordinator
verified, committed and merged. The source of every line below is a commit message
(`git log --first-parent 8afcf1e..wave/s5-1`). Before: [wave 2](slice-05-wave-2.md). Release
`0.8.0` was cut on this wave (`a350de3`), on the decision of the session that decides for the
operator: cut it when the flag `i` and identifiers outside ASCII are merged and the full
test262 run fits a pool job of under 15 minutes.

The wave holds stories of two slices because they were built side by side on one branch: four
of slice 5 (`epic:slice-05-regexp`), four of slice 7 (`epic:slice-07-generators`), and two
that the wave itself asked for, the cost of an index store and the test262 run in parts. The
plan of slice 7 and of slice 10 was made and accepted on the same branch
(`docs/plan/waves.md`).

## What the wave delivered

- **Slice 5.** `String.prototype.matchAll`, `RegExp.prototype[Symbol.matchAll]` with its
  iterator and `RegExp.escape`; Unicode property escapes under the flag `u`, with the
  property tables; the flag `i` and the modifiers `(?i:...)` and `(?-i:...)`; identifiers
  and group names with code points outside ASCII. Thirteen of the slice's fifteen stories
  are delivered; the flag `v` and the close are open.
- **Slice 7.** Generator functions parse with their early errors and run: `yield` moves the
  frame, boxed, into the generator object, and `next`, `return` and `throw` resume it;
  `%GeneratorFunction%` makes generators from source text; `yield*` delegates on the
  delegating frame. Four of the slice's six stories are delivered; generator methods of
  object literals and the close are open.
- **The cost of an index store.** A store to an Array at a Number index no longer makes the
  text of the index: a loop of 1,114,112 stores took 2.313 CPU seconds in a release build and
  takes 0.830. With it the full test262 run takes 598 s in one pool job (1,599 s before).
- **The full test262 run in parts**: `task test262-part TEST262_PART=<k>/<n>` per pool job,
  counted with `TEST262_FROM_PARTS` to the lines the whole run prints.

**The consumer's gap.** None of these is the first consumer's next gap, which is one `async`
function of slice 10 with two `await` on promises of the host. Slice 10 was planned and
accepted on this branch in seventeen stories, with its design text and seventeen new host
requirements (`HOST-192` to `HOST-208`), and the two stories of its first order are being
built.

## Units

| story | unit commit | merge | what it wrote |
|---|---|---|---|
| `regexp-match-all-and-escape` | `d34d73e`, `ea68f18` | `4971b9b` | `matchAll`, the RegExp String Iterator, `RegExp.escape` |
| `regexp-property-escapes-and-property-tables` | `b023531`, `bab9579` | `d917e5d` | `\p{...}` and `\P{...}` under `u`, the property tables of `swell-unicode` |
| `generator-syntax-and-early-errors` | `319598d` | `dc7da03` | `function*` and `yield` parsed, their early errors, the refusal held behind them |
| `regexp-case-insensitive-matching-and-modifiers` | `85da804`, `35f103e` | `633c0e5` | the flag `i`, the modifiers, simple case folding |
| `generator-functions-and-suspended-frames` | `2316f71` | `e07d060` | suspended frames, generator objects, `%GeneratorPrototype%` |
| `identifiers-and-group-names-outside-ascii` | `0cefd20` | `0efe342` | `ID_Start` and `ID_Continue` in the lexer and in group names |
| `an-index-store-does-not-go-through-text` | `e85fb65`, `035334e` | `3c4dd0b` | the store path of an Array at a Number index |
| `generator-function-constructor` | `ff8b604` | `e866c17` | `%GeneratorFunction%` through CreateDynamicFunction and the host hook |
| `test262-run-in-parts` | `ad4827d` | `c4405da` | `swell-test262 count --part` and `--from-parts`, the tasks |
| `yield-star-delegation` | `66e2852` | `a088318` | three instructions for `yield*`, the chain on the frame stack |

Also on the branch: `8afcf1e`, the story of wave 2 moved to `implemented`; the plan of slice 7
(`8772909`) and of slice 10 (`2a0fc79`, `89fe659`) with their review results; the design text
of slice 10 (in `2a0fc79` and `c7dd489`); `3d5afeb`, the expected file on the property escapes
and the generator syntax; `92f3d9d`, a merge of `main`.

## Numbers

| after | cases that pass, of 44,812 | slice 5, of 2,724 | slice 7, of 1,491 | `test262-refused` |
|---|---|---|---|---|
| release `0.7.0` | 18,155 | 1,667 | 6 | 34,929 |
| `matchAll` and `RegExp.escape` | 18,238 | 1,746 | 6 | 34,929 |
| property escapes, generator syntax | 18,935 | 2,326 | 123 | 33,550 |
| the flag `i`, identifiers, generators that run, the index store | 20,165 | 2,523 | 1,112 | 30,776 |
| `%GeneratorFunction%`, `yield*` (release `0.8.0`) | 20,229 | 2,523 | 1,168 | 30,642 |

- No run that passed on a full run of the wave fails on the next one.
- A `next` through a chain of 101 delegating generators spends 500 steps more than through
  one, 5 per level; a chain of n generators holds n frames and takes no re-entry per level.
- Two cases end at the memory budget, as before the wave.

## Decisions

- **Two slices on one branch.** The stories of slice 7 were started while slice 5 was open,
  because they write other files; each unit that landed second was joined with the branch
  and tested on the join before its merge.
- **A full test262 run per join, not per unit.** A unit runs the runner on a copy of the part
  of the checkout its change can move and compares run lines with the last full run; the
  coordinator makes the full run on the join. With the index-store change the full run fits
  one pool job again.
- **`yield*` and the later design text of slice 10 are in the release.** Both were merged
  before the release's full run started.
- **The host interface of slice 10 is announced before it exists.** `HOST-192` to `HOST-208`
  are pending for slice 10 and held by no test; `HOST-201` says that `ErrorKind` gets a
  variant and `#[non_exhaustive]` in the release that brings `Promise.any`.

## What changed for a host

No function, type or constant of the host interface was added or removed. `HOST-191`
changed (of the parts of a regular expression it names as refused, the flag `v` is left), and
`HOST-192` to `HOST-208` are new and pending (`host-surface: 0 change(s) since 0.8.0, each
named` on the release commit, whose snapshot holds them).

## Findings left open

- `conformance/test262/later-parts.tsv` quotes refusal messages of regular expressions that no
  longer exist from line 137 on; the close of slice 5 rewrites them.
- No stack test runs the entry of the parser that `%GeneratorFunction%` uses, and the module
  text of `crates/syntax/src/lib.rs` lists the dynamic-function entries without it.
- The doc comments of the parser's held refusal (`crates/syntax/src/parser/mod.rs`) and of
  `parse_script` say it holds the first `yield*`; nothing sets it since `yield*` runs. The
  field stays for slice 10.
- `HOST-200` has two halves and one test line.
- The closed list of the host's intrinsics lacks `%AsyncIteratorPrototype%` and
  `%Iterator.prototype%`.
- The findings of [wave 1](slice-05-wave-1.md) and [wave 2](slice-05-wave-2.md) stand where
  the wave did not close them.

## How the wave was checked

- **The units.** Each unit ran its story's tests in both collection modes, the tests of the
  crates it changed, clippy and fmt, and the test262 runner on the part of the checkout its
  change can move, compared run by run with the last full run. A unit that landed second
  was joined with the branch and tested on the join before its merge.
- **The full test262 run** on the code of the release (`task test262-expected`, one pool
  job) wrote `expected.txt` and its digest: `test262 total 53801 8982 7 44812 20229 24581 2`,
  `test262-refused 30642`. No run that passed on the full run before it fails.
- **The gate on the release commit** `a350de3`: `slot --timeout 30m task check-runner`, load
  26 before and 34 after on 16 cores, exit 0:
  `gate-time test-steps 94.3 of 120, cpu 312.1, load 33.84 on 16 cores` (test 28.5,
  test-collect 43.6, gate-selftest 9.4, test262-moves-check 7.2, test-doc 3.7).
  `17 test binaries: 1215 passed, 0 failed, 1 ignored`;
  `test-collect: 2 test binaries: 934 passed, 0 failed, 1 ignored`;
  `gate-selftest: 65 passed, 0 failed`;
  `host-requirements: 208 requirement(s), 208 line(s): 114 test, 94 pending, 0 omitted`;
  `host-surface: 0 change(s) since 0.8.0, each named`; 14 of the 58 scenarios pass on the
  engine. The lines are in `AGENTS.md`, "The gate's numbers".
- **The lane `slow`** on the same commit, as three pool jobs, exit 0: `slow-test262`
  (`test262: 287 lines equal conformance/test262/expected.txt`, 836 s), `slow-tests`
  (`4 test binaries: 592 passed, 0 failed, 444 ignored`; with a collection at every safe
  point `2 test binaries: 506 passed, 0 failed, 429 ignored`), and the stack profiles
  (`release` and `stack-debug`: `1 test binaries: 4 passed, 0 failed, 755 skipped` each).
- **The wait in the pool.** The gate waited 1,490 s for a slot and ran 317 s; the three jobs
  of the lane waited 1,830 s, 1,468 s and 1,469 s and ran 836 s, 188 s and 982 s.
- **Not checked here**: the forge's runs, which the publishing session watches; the stories
  move to `implemented` when they are green.
