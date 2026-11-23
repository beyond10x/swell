# Slice 6, wave 2: the rest of bindings and iteration, and the close

The branch `wave/s6-2` over `main` at `3cec594` (the second batch), 2026-11-22 18:21 CET to
2026-11-23. Each unit was implemented by a sub-agent in its own worktree; the coordinator
verified, committed and merged. The source of every line below is a commit message
(`git log --first-parent 3cec594..wave/s6-2`). Before: [the second batch](second-batch.md),
which holds the first five stories of the slice. Release `0.4.0` was cut from `main` inside
this wave (`20797bf`, 2026-11-22). Of this wave it holds the parser stack fix (`50cbc75`) and the
ESS pins (`3500398`) and no feature story: the stories below reached `main` after it.

## What the wave delivered

**The language of slice 6.** Scripts may use the `Symbol` builtin with twelve well-known
symbols and `Object.getOwnPropertySymbols`; computed property names, object spread and object
rest; the iterator protocol with the Array and String iterators; function declarations in
blocks and in `switch` cases, without Annex B.3.2; `for`-`of`, which closes its iterator once
on every early exit; `Array.from`, `Object.fromEntries` and `Object.groupBy`; array patterns
as bindings and as assignment targets, and spread in array literals, calls and `new`; tagged
templates with one frozen template object per site.

**The close.** Every case slice 6 counts passes in every run:
`test262-slice 06 3898 7351 0 0 3898 3898 0 0`. 143 cases that need a later slice moved there
with evidence. A case is now counted in the slice delivered last among the slices it needs,
by the order of delivery the design's block names; before, it was the slice with the highest
number, and 393 failing cases that needed only slice 4 or 5 could not leave slice 6.
`conformance/slices-delivered.txt` names slice 6.

**The host interface.** `HOST-185`: a host names `%Array.prototype.values%`, `entries`, `keys`
and `forEach` and the well-known symbol `@@iterator`, what Web IDL gives an iterable
interface. `HOST-156` has its test. 110 of the 190 requirements have a test.

**The tests' cost.** No test of the gate takes over 1.5 CPU seconds alone in the default mode
and none of the lane `slow` over 2.5 at its full count, but for the exceptions `AGENTS.md`
names. The parser's frames fit the stack bound at the full count.

## Units

| story | unit commit | merge | what |
|---|---|---|---|
| `symbol-constructor-and-well-known-symbols` | `b17b030` | `303afe8` | `Symbol`, `Symbol.for`, `keyFor`, the prototype, twelve well-known symbols, `Object.getOwnPropertySymbols` |
| `parser-frames-within-the-stack-bound-at-the-full-count` | `50cbc75` | `81b5698` | the body of a concise arrow is recorded by a function of its own, so the frame of an arrow level is 224 bytes |
| `computed-names-and-object-spread` | `801449e` | `a2a62b5` | computed property names, object spread, object rest |
| `iterator-protocol-and-array-string-iterators` | `46046e9` | `ea0d899` | the iterator protocol, `%ArrayIteratorPrototype%`, `%StringIteratorPrototype%`, `entries`, `keys`, `values` |
| `block-function-declarations` | `b68aa65` | `5769ad5` | function declarations in blocks and `switch` cases, without B.3.2 |
| `symbol-values-in-core-builtins` | `6442125` | `8e73daa` | tests of Symbol values in the builtins of slice 3 |
| `heavy-tests-far-from-the-kill` | `1b869e8` | `878893b` | the heaviest tests from 327.9 to 92.5 CPU seconds in sum |
| `the-host-names-the-iteration-intrinsics-of-web-idl` | | `1c2e53e` | `HOST-185` |
| `for-of-statement` | `4db1eab` | `2cf76a6` | `for`-`of` with IteratorClose on `break`, `continue` to an outer loop, `return` and a throw |
| `array-from-and-object-from-entries` | `6c8778d` | `19bb68b` | `Array.from`, `Object.fromEntries`, `Object.groupBy` |
| `array-destructuring-and-spread` | `ce54ecc` | `4ed9a71` | array patterns, rest elements, spread in arrays, calls and `new` |
| `tagged-templates` | `1bc7595` | `0320358` | tagged templates and `GetTemplateObject` |
| `lane-tests-under-2-5-cpu-seconds` | | `9373a70` | the lane's tests from 116.3 to 42.1 CPU seconds in sum |
| `slice-06-conformance-close` | `6c74351` | `fb28405` | 136 cases moved, `HOST-156`, the section of Annex B.3.2 |
| `a-case-counts-in-the-slice-delivered-last` | `c8f4aec` | `e394d31` | the order of delivery in the block; 7 more cases moved |
| `tagged-template-tests-under-the-lane-bound` | `1f57da8` | `1e46de5` | two tests of tagged templates take 100 evaluations and 100 sites in collect mode also at the full count: 0.51 and 1.44 CPU seconds at most (34.3 and 43.4) |

Beside them: the pins of ESS moved from 0.97.3 to 0.97.4 (`3500398`, on `main` before the
release), and `main` was merged twice (`75b5ff9`, `4701a56` with release `0.4.0`).

## Numbers

| | release 0.3.0 | release 0.4.0 | `main` at `a6f22f6` | the wave's tip |
|---|---|---|---|---|
| test262 cases that pass, of 44,812 | 12,120 | 13,575 | 14,998 | 16,412 |
| slice 6, passed of counted | 100 of 4,436 | 1,364 of 4,436 | 2,528 of 4,436 | 3,898 of 3,898 |
| runs that meet a refusal, of 85,556 | 46,581 | 41,572 | | 36,178 |
| host requirements with a test, of 190 | | 108 | 109 | 110 |
| cases moved with evidence | | 1,656 | 1,656 | 1,774 |
| lines of the expected file | | 286 | 286 | 287 |

The slice lines at the tip, where the order of delivery changed them (before: at the close's
first part, `b14d07a`):

```
test262-slice 04 1598 3156 0 0 1598 261 1337 0     (1521 3002, 254 passed)
test262-slice 05 2724 5441 0 0 2724 52 2672 0      (2398 4791, 50 passed)
test262-slice 06 3898 7351 0 0 3898 3898 0 0       (4300 8153, 3907 passed)
test262-slice 07 1491 2841 0 0 1491 6 1485 0       (1492 2843, 6 passed)
test262-slice 08 9144 18139 0 0 9144 3 9141 0      (6371 12649, 3 passed)
test262-slice 09 1779 3529 0 0 1779 131 1648 0     (1772 3515, 131 passed)
test262-slice 10 3879 7499 0 0 3879 13 3866 0      (6659 13003, 13 passed)
```

Slices 1, 2 and 3 keep their cases; the sums stay `44931 85794` for slices 1 to 19 and
`53801 103360` in total. Run by run against the base, the rule changed no outcome: 652 runs
went from slice 6 to 5, 150 from 6 to 4, 5,430 from 10 to 8 (classes with `async`), 74 from
10 to 9, 60 from 9 to 8, 2 from 5 to 4 and 2 from 7 to 4.

Where the 529 failing cases of slice 6 went, with every feature story merged
(`test262-slice 06 4436 8413 0 0 4436 3907 529 0`): 136 by a line of the close's first part
(to slice 4: 5, to 5: 4, to 7: 16, to 8: 79, to 9: 4, to 10: 8, to 12: 18, to 13: 2); 386 by
the order of delivery alone (their directory or the feature `exponentiation` is of slice 4
or 5); 7 by a line of their own with `from` 6 (5 to slice 5, 2 to slice 4). None failed on a
part of slice 6. 9 passing cases left slice 6 by their directory (7 to slice 4, 2 to 5).

## Decisions

Taken by the coordinator and reported to the operator, each with its reason.

- **D40.** The `Symbol` unit ran beside computed names. Reason: their files do not overlap.
- **D41, D42.** Units for defects found inside the wave ran from a brief, and the unit wrote
  the story, stored with its commit. Reason: the defect was measured; a planning round would
  have held the fix.
- **D43.** The `Symbol` unit also built `Object.getOwnPropertySymbols`. Reason: its guard test
  needs the method, and no other story held it.
- **D44.** A cargo profile of its own for collect-mode tests was not merged. Reason: the
  rebuild cost 189 to 206 CPU seconds against 31 to 33.
- **D45.** The ESS pins moved on a branch of their own on the tip of the stack fix. Reason:
  the order came while the lane `slow` was red.
- **D46.** A unit made the heaviest tests lighter ahead of the next feature story. Reason:
  tests were killed at 60 s at a machine load of 110 to 166.
- **D47, D53.** The fuzz test of the host interface runs 400 sequences at its full count
  (2,000 before, then 1,000); the parser's nesting check runs 7,500 levels on three quarters
  of the stack; the JSON nesting checks 40,000 and 60,000. Reason: the bound of 2.5 CPU
  seconds.
- **D48.** The wave was handed over for `main` before its last stories. Reason: the first
  consumer waited for `for`-`of`.
- **D49.** `test/annexB/built-ins/Array/from/iterator-method-emulates-undefined.js` stays
  failing. Reason: it passed only on the error of the missing `Array.from` and needs
  `$262.IsHTMLDDA` (slice 18).
- **D50.** Tagged templates started beside array patterns. Reason: one shared parser file,
  joined by the second unit.
- **D51.** The bound of a lane test went from 5 to 2.5 CPU seconds. Reason: a test of 4.2
  CPU seconds was killed at a load of 165.
- **D52.** A branch was handed over with one kill by load stated. Reason: the forge's lane
  judges, and it passed there.
- **D54.** A case counts in the slice delivered last among its slices. Reason: the design's
  own sentence, "the first slice after which it can pass"; the other options left slice 6
  with 393 excepted cases or needed 393 single pins.

## What changed for a host

Additive only: `HostIntrinsic::ArrayPrototypeValues`, `ArrayPrototypeEntries`,
`ArrayPrototypeKeys`, `ArrayPrototypeForEach` and `WellKnownSymbol::Iterator`
(`host-surface: 5 change(s) since 0.4.0, each named`).

## Test262 cases that changed against the direction

One: `test/annexB/built-ins/Array/from/iterator-method-emulates-undefined.js` (slice 18)
passed in `0.4.0` and fails since `Array.from` exists (D49). Against the start of slice 6
(`6a43e18`) three more cases fail, all known losses of the close of slice 3
(`trimLeft/reference-trimStart.js`, `trimRight/reference-trimEnd.js`,
`Function/prototype/apply/this-not-callable.js`).

## Findings left open

- The gate's test steps took 139.5 s of their 120 at a machine load of 43.7 (89.4 s at 36.7):
  `gate-selftest` 46.5 s (18.8), `test-collect` 49.2 s (36.9); 301.3 CPU seconds (281.2).
- `Intrinsic::IteratorPrototype` in `crates/swell/src/realm/intrinsics.rs`: the reason of its
  `expect(dead_code)` names the story of `HOST-185`, which does not use it.
- Test comments that say "before slice 6": `builtins/array/search.rs` near lines 810 and 902,
  `builtins/array/iteration.rs` near 745.
- A plain call copies its arguments before the charge
  (`crates/swell/src/interp/dispatch.rs`, near line 1376).
- `declared.contains` in `crates/compiler/src/lib.rs` (near line 200) and the early errors of
  labels are quadratic in their count.
- No test runs the parser's stack bound in the profiles `stack-debug` and release; an arrow
  level has 10.7 bytes of room in `stack-debug`.
- `option_env!("SWELL_FULL_COUNTS")` in `heap/collect.rs`, `script.rs` and `ops/objects.rs`:
  a lane run and a gate run in one tree each rebuild `swell`.
- `.github/workflows/slow.yml` does not name every included task file in its paths.
- Two tagged-template cases need tail calls (`tco-call.js`, `tco-member.js`, slice 17);
  `test/staging/sm/regress/regress-610026.js` ends at the memory budget.
- Gate counts that may pass where the full count fails: `assert_every_exit_credits`,
  `property_index`.
- The planted script `test262-moves-check-refuses-a-line-without-evidence.sh` plants a line
  with `from = to + 1`; it is refused only because the line it picks has `to` 6.
- The inventory's copy of `slices.awk` under `.cache/inventory/` holds the old listing.
- Check 2 of `story:slice-06-conformance-close` ("every case whose slice moved has a line of
  the list") no longer holds as written: the order moves 3,207 cases without a line.
- `website/docs/limits.md` states the limits of release `0.4.0` and is written again at the
  next release.

## Rule breaches by units

- `pgrep -f` once each: the `Symbol` unit and the unit of the Symbol-value tests.
- Python: `python3 --version` (the `for`-`of` unit) and `python3 -` without a script (the
  tagged-templates unit).
- The computed-names unit built a copy of the base and a probe crate in scratch with a
  `target/` of their own, and deleted them.
- The `for`-`of` unit ran `git rm --cached`; the coordinator undid it.
- The tagged-templates unit ran a `cargo check` of 2 min 32 s in the foreground.
- The coordinator handed `3cec594` over without a local run of the lane `slow`, which then
  ended red on the forge on three tests of the parser's stack bound; `AGENTS.md`, "How the
  work is run", has the rule since `280a50b`. The coordinator did not read the slowest tests
  of the lane's run on the join of tagged templates; two of its tests cost 34 and 43 CPU
  seconds in collect mode at the full count and were found by a kill at a load of 155.
- The coordinator started one regeneration outside the build pool and stopped it after 16 s.

## How the wave was checked

Each unit ran its own tests in both collection modes, `cargo clippy` and the full test262 run
where it changed what a script evaluates to; the coordinator read each diff, ran the unit's
tests again where a claim rested on them, and regenerated the expected file after each group
of merges (`task test262-expected`: `a6f22f6`, `79b98b5`, the join `b14d07a`, and `34e2b0f`
on the merged tree `e394d31`, which wrote the 287 lines the order-of-delivery unit had
measured).

On `main`: the forge's `check` and its lane `slow` were green on `c08452a` (the stack fix),
`3500398`, `20797bf` (release `0.4.0`) and `a6f22f6` (the first part of this wave).

On the tip with the slice closed (`1e46de5`), 2026-11-23, on a machine shared with other
builds:

- `slot --timeout 30m task check-runner`, twice. Every check passed in both runs: 977 tests
  in the default mode, 796 with a collection at every safe point, 13 doc tests in each
  mode, 12 conformance tests, 65 planted defects refused, `host-requirements: 190
  requirement(s), 190 line(s): 110 test, 80 pending`, `host-surface: 5 change(s) since 0.4.0,
  each named`, `test262: 287 lines of conformance/test262/expected.txt`, `test262-moves: 1774
  case(s) moved`. Both runs exited 201 on the last line alone, the wall time of the test
  steps: `gate-time test-steps 263.9 of 120, cpu 303.2, load 54.34 on 18 cores` (the load was
  153 at the start) and `gate-time test-steps 327.8 of 120, cpu 323.6, load 77.07 on 18 cores`
  (113 at the start). On `9373a70`, at a load of 37, the same steps took 89.4 s and 281.2 CPU
  seconds.
- The lane `slow`, its three parts as pool jobs of their own, each exit 0: `slow-test262`
  (`287 lines equal conformance/test262/expected.txt byte for byte`), `slow-tests` (548
  passed, and 461 with a collection at every safe point, none killed), and the stack
  profiles (4 tests passed in the release profile and 4 in `stack-debug`). As one job the
  lane does not end within the pool's 30 minutes at a load of 155: on `79b98b5` it was
  ended there, after `slow-test262` alone had taken 1,085 s.

Left open by these runs, for the first story of the next wave: the gate's test steps cost
about 300 CPU seconds and exceed their 120 s of wall time whenever the machine's load is
above about 40.
