# Slice 5, wave 2: a String method takes a regular expression

The branch `wave/s5-1` over `main` at `6e9c3f4` (release `0.6.0`), 2026-11-23 09:15 CET to
10:09 CET. One story, implemented by a sub-agent in its own worktree; the coordinator
verified, committed and merged. The source of every line below is a commit message
(`git log --first-parent 6e9c3f4..wave/s5-1`). Before: [wave 1](slice-05-wave-1.md). Release
`0.7.0` was cut on this wave (`1d32bfe`), on the decision "cut 0.7.0 now; it closes surf's
first gap" of the session that decides for the operator.

## What the wave delivered

`%RegExp.prototype%` has the methods of its clauses keyed by `@@match`, `@@replace`,
`@@search` and `@@split`, and `String.prototype` has `match` and `search`. `replace`,
`replaceAll` and `split` of `String` given a regular expression answer through those methods:
the replacement patterns of numbered and named groups, a function as the replacement, the
flags `g` and `y`, and a splitter made by the species constructor.

**The consumer's gap.** All twelve rows of
[`slice-05/consumer-regexps.tsv`](slice-05/consumer-regexps.tsv), the regular expressions of
the first consumer's test harness with the calls made on them, answer
(`regexp_symbol_methods::the_calls_of_the_consumer_table_answer`). This was the first gap
that consumer named for slice 5; what it needs next is one `async` function of slice 10.

## Units

| story | unit commit | merge | what it wrote |
|---|---|---|---|
| `regexp-symbol-methods-and-string-match-search` | `1d9347d`, `6066e39` | `54ddbcf` | `crates/swell/src/builtins/regexp/symbol.rs`, `crates/swell/src/builtins/string/pattern.rs`, SpeciesConstructor in `crates/swell/src/ops/objects.rs`, 17 tests and two language scripts |

Also on the branch: `504aedb`, the eleven stories of wave 1 moved to `implemented` on the
forge's run on `6e9c3f4`, release `0.6.0` in the list of releases, and the count of delivered
stories corrected (eight of fifteen in wave 1, not seven).

## Numbers

| after | cases that pass, of 44,812 | slice 5, of 2,724 | `test262-refused` |
|---|---|---|---|
| release `0.6.0` | 17,775 | 1,327 | 35,308 |
| the symbol methods | 18,155 | 1,667 | 34,929 |

- Steps of a method beside the steps of its matches, the same at every count from 10 to 400
  matches in both collection modes: 11 per match of `@@replace` with `g`, 7 per match of
  `@@match` with `g`, 7 per piece of `@@split`.
- `HOST-187` reads 449,821 bytes (the ceiling is 506,112).
- Two cases end at a budget (one before): see below.

## Decisions

- **`%RegExp.prototype%` is a regular expression to IsRegExp.** Its `@@match` is callable, so
  step 3 of IsRegExp answers true: `"/a/".startsWith(RegExp.prototype)` throws the
  `TypeError`. A test of wave 1 expected `false` and was changed with the reason in its doc
  comment.
- **The two refusing methods are gone**, with their four rows in the sweep of every function
  with a Symbol. The design says what is true now (`docs/design/engine.md`, decision 7 and
  the third form of section 12; `docs/design/host-interface.md`, "A count of refusals" and "A
  refusal raised by `RegExp`"); the implementing unit wrote those sentences on the
  coordinator's word, and no requirement's text changed.
- **Smaller counts with a collection at every safe point**: the two stack tests re-enter 25
  levels there (50 took 2.7 to 3.2 CPU seconds), and the step-budget test uses a budget of
  5,000 on 10,000 units (50,000 on 100,000 in the default mode, where the full size was
  killed at 60 s in collect mode).

## What changed for a host

Nothing: no name of the host interface was added, removed or changed
(`host-surface: 0 change(s) since 0.6.0`).

## Test262 cases that changed against the direction

`staging/sm/String/replace-math.js` (slice 19) ended on a counted refusal and now ends at the
memory budget in both runs: it builds a String of 2^36 code units with `replace`, and the
memory budget of the runner is reached before `string_length`. It also needs `Math.pow` of
slice 4. No case that passed fails.

## Findings left open

- `built-ins/RegExp/prototype/Symbol.replace/coerce-lastindex.js` fails on `Math.pow`
  (`slice-05/close-moves-later.tsv` moves it at the close).
- The findings of [wave 1](slice-05-wave-1.md) stand.

## How the wave was checked

- **The unit.** The coordinator joined the unit with the wave branch at `504aedb` and ran, in
  one pool job, the tests of `swell-syntax`, `swell-regexp`, `swell-compiler` and
  `swell-test262` (151 passed), the story's filter of `swell` in both collection modes (142
  passed in each), `host-surface-check`, `host-requirements-check` and the full test262 run
  (`task test262-expected`), which wrote `expected.txt` and its digest.
- **The gate on the release commit** `1d32bfe`: `slot --timeout 30m task check-runner`, load
  60 before and 65 after on 16 cores. Every check passed and the run exited 201 on its last
  line alone: `gate-time test-steps 194.7 of 120, cpu 331.3, load 65.16 on 16 cores` (test
  51.9, test-collect 78.9, gate-selftest 26.7, test262-moves-check 21.1, test-doc 9.3). By
  the standing rule the forge's run judges the budget.
  `17 test binaries: 1104 passed, 0 failed, 1 ignored`;
  `test-collect: 2 test binaries: 864 passed, 0 failed, 1 ignored`;
  `gate-selftest: 65 passed, 0 failed`; `test262 total 53801 8982 7 44812 18155 26655 2`. The
  lines are in `AGENTS.md`, "The gate's numbers".
- **The lane `slow`** on the same commit, as three pool jobs, exit 0: `slow-test262`
  (`test262: 287 lines equal conformance/test262/expected.txt`), `slow-tests`
  (`4 test binaries: 577 passed, 0 failed, 385 ignored`; with a collection at every safe
  point `2 test binaries: 491 passed, 0 failed, 374 ignored`), and the stack profiles
  (`release` and `stack-debug`: `1 test binaries: 4 passed, 0 failed, 692 skipped` each).
- **Not checked here**: the forge's runs, which the publishing session watches; the story
  moves to `implemented` when they are green.
