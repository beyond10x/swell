---
format: aep.planning-md/3
id: review-result:adversary-close9-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit close9
relations:
- reviews: story:slice-09-conformance-close
revision: 1
---
Adversary pass 1 on the unit close9 of wave big-2, impl/close9 at 66fe379, 2026-11-27.
Red: one warning (32 cases the rule takes stay out of slice 9) and three notes. The adversary's own report follows.

unit: close9
verdict: red
cases: executed 0→23, red 0
origin: introduced 2, pre-existing 2, undecided 0
wrote-outside-worktree: <scratch>/wave2/close9/adversary-1/
needs-coordinator: yes

Pass 1 on `impl/close9` at `66fe379`, base `bc45e92`. The verdict is red because the close leaves out 32 cases that its own rule takes in (F1). Nothing else is wrong with the moves, the 99 incoming cases, the numbers or the checks.

`report.md` is not in the work directory: the tool refused to write the file, so this message is the report.

## Findings

**F1, warning, acceptance, introduced.** 32 cases (59 runs) of slice 19 meet the story's rule for coming in but stay out.
- Each failed at the start on a part of slice 9 (`Map`, `Reflect` or `Proxy is not defined`) and passes in every run on base and head. None has a `feature` or `flag` line of a later slice.
- Their only later-slice tokens (`BigInt`, the typed-array names) sit in `typeof X === 'function'` guards in `harness/deepEqual.js` (lines 241, 264-274; 31 cases) or `harness/propertyHelper.js` (lines 40, 236; `sm/Array/unscopables`). The case files hold none.
- So they need nothing of slices 12 or 13, which are not built: `BigInt` and `Float64Array` are "not defined" in the head run.
- The design's rule 1 ("needs nothing of a later slice") and the story's rule take them. The unit's extra filter, "no token in a harness file it includes", drops them. That filter is copied from the slice 4 paragraph (`engine.md:4379`); no rule orders it.
- Effect: slice 9 is short by 32 cases and 59 runs, and slice 19 keeps them. `engine.md:4478-4480` counts them among the "158 with a token".
- List: `adversary-1/missed-bigint.txt` (12 under `sm/Array/`, 7 `sm/generators/delegating-yield-*`, 4 `sm/Reflect/`, 2 each under `sm/Symbol/`, `sm/String/` and `sm/object/`, and 1 each in `sm/Map/`, `sm/Proxy/`, `sm/Function/` and `sm/lexical-environment/`).
- Reached by: the documented close workflow, through the paragraph's selection rule.

**F2, note, judgement, pre-existing (`story:slice-09-conformance-close`, Context).** 37 cases of `close-later.tsv` that need slice 8 stay counted in slice 9 (`engine.md:4427`).
- At the start, 36 failed on `class is not supported before slice 8` and 1 on `new.target is not supported before slice 8`.
- They pass now only because classes of slice 8 were merged in the same wave. Slice 9 is delivered before 8, and section 12 counts a case "in the first slice after which it can pass".
- The selection of incoming cases does the opposite: it keeps out the 4 cases that failed at the start on a part of slice 8, and the 125 with a `class`, `new.target` or `super` token.
- The story orders "passes after the stories stays in slice 9", so the unit followed its order.

**F3, note, contract-drift, introduced.** `engine.md:4478` says "3,740 with a token of slice 8 only". 108 of those 3,740 also hold a token of a later slice (`BigInt`, `ArrayBuffer`, typed arrays). The number is right only if "only" means "a token and no `feature` line".

**F4, note, mutant, pre-existing (the moves check, `taskfiles/test262.yml:229-462`).** Plant b, under item 7: removing the `moves.tsv` line of `Set/prototype/union/called-with-object.js` is caught only by the close's checks (2) and (7), which read run lines that only a close makes.
- The moves check stays green, and its count still equals the lines of the file (1985 = 1985).
- So the block can keep `dir <file> 12` with no line in the list, and no gate step after the close sees it.
- A check over the block alone could see it: it would flag a file line that sends a case to a slice delivered later than the slice the case would otherwise have.

## What was attacked, with results

1. **The 22 moves.**
   - Each case was read. Its only later-slice part is `1n` or `0n`; `BigInt` appears only in assertion message strings of `size-is-a-number`. The cases include no harness file beyond the defaults.
   - 22 variants with the BigInt statement taken out (and for `size-is-a-number`, the `s2.size = 0n` block) were run in both modes: all 44 runs `Passed` in slice 12. The moves hide no defect of slice 9.
   - The `moves.tsv` lines have five fields, are sorted, and their evidence is as the case holds it (the moves check is green).
2. **The 99 incoming cases.**
   - My own scan of S, B and N gives 8,504 candidates. Of these, 103 have no later `feature` line, flag or token. The unit's 99 are those 103 minus the 4 it names.
   - No case the unit took fails the rule, and the fields of `close-into-09.tsv` match the start run (message, slice, 193 runs). `proxy-enumeration` has its slice 9 token only in a string, but it does exercise the `Proxy` global.
   - The token list used is the slice-9 tokens of `later-parts.tsv`. No rule of section 12 orders a slice-9 token.
   - Missed cases: F1.
3. **The two directory lines.** The 4 cases of `set-methods/` and `sm/WeakMap/` all fit the rule. `symbols.js` has `feature symbols-as-weakmap-keys 9`. Each of the 99 matches exactly 2 dir lines and its own wins. No dir line appears twice. Base against head: exactly 22 cases go 09→12 and 99 go 19→09, so no file line catches another case.
4. **Passing for the wrong reason.**
   - Slice 9 has no case with `negative:`, and no passing run with a detail. Refusals inside `eval` are counted failures (`run.rs:401`).
   - A probe over the 328 built-in member paths named in slice 9 cases found only non-standard names undefined (`Reflect.enumerate`, `Object.ownKeys`, static `Map.getOrInsert`, and the like).
   - No slice 9 case uses an `$262` member that is not built. Nothing found.
5. **Slice-9 parts failing elsewhere.**
   - 529 failing later-slice cases name a slice 9 part by path or feature, and 660 hold a slice 9 token.
   - Every first failure is a later part: typed arrays, `Iterator` helpers, BigInt, modules, annex B accessors and `__proto__`, `$262.gc`, `re.compile`, private fields, `DisposableStack`, `WeakRef`/`FinalizationRegistry`. No wrong answer of `Proxy`, `Reflect` or a collection.
6. **Numbers.**
   - The table and the block equal my per-slice sums from N (slice 14 differs only by its excluded cases). Rows 1 to 19 sum to 44,931 and 85,794; the block sums to 53,801 and 103,360; the block equals the `expected.txt` slice lines.
   - Rows 9, 12 and 19 match before and after. The counts by directory, 87/12 by message, 866→767, 8,236/160/2/1/1/1 and class 109 / BigInt 32 / new.target 11 / super 5 / `.__proto__` 1 all match.
   - `expected.txt` equals `.out/test262/lines.txt`. Text: F3.
7. **Plants.** Copies under `adversary-1/plant/`; the moves check's own awk was run on the copies.
   - a) One strict run of an incoming case set to Failed: red in (1) (`1`) and (4) (exit 1). The moves check stays green.
   - a2) One run of an old slice 9 case set to Failed: red in (1) only.
   - b) One 9→12 line taken out of `moves.tsv`: red in (2) and (7) (exit 1). The moves check stays green, count 1985 = lines 1985 (F4).
   - c) A `close-into` line pointed at the failing `sm/extensions/weakmap.js`: red in (4) (exit 1).
   - d) Evidence `0n` on a `1n` case: the moves check is red at `moves-d.tsv:671`, "found neither in the case nor in a harness file". Checks (1) to (7) stay green.
   - e) The block line of a moved case dropped: the moves check is red, "the block gives the case slice 9, not 12".
   - f) A move added for a passing case: red in (5) (`1`) and in the moves check.
   - `test262-check` sees none of these, by design: it compares the digest and the committed lines.
8. **Host requirements.** HOST-25, 41, 112, 113, 114, 214 and 222 each name a test that exists in `crates/swell/tests/{determinism,host_overrides}.rs`, under `#[test]`, not ignored. `host_overrides` is built when `test_modules` is unset. Each test holds its sentence. The 7 `[slice 9]` requirements of `host-interface.md` all have test lines. HOST-119 (`[slice 8]`) is the only other requirement that names overrides. Nothing found.
9. **Left over.**
   - `git grep "before slice 9"` matches only story files under `.engineering/`. No refusal names slice 9.
   - For the coordinator: dated records that state slice 9 before the close are `AGENTS.md:564` and `website/data/status.json:170` (both covered in `for-coordinator.md`), and `docs/plan/wave-big-2.md:181` and `docs/plan/waves.md:509` (not covered there).
   - Section 12, item 9, still reads `Set 381, Proxy 308, Map 202`; slice 9 now counts 360, 305 and 201 there. The unit left this on purpose.

## Cases added (uncommitted, ignored path)

- `.out/adversary/cases/test/built-ins/{Map/prototype/getOrInsertComputed/this-not-object-throw.js,Set/prototype/<7 methods>/{called-with-object,require-internal-slot,size-is-a-number}.js}`: the 22 variants. All passed in both modes.
- `.out/adversary/cases/test/built-ins/Reflect/zz-adversary-probe.js`: the member probe. It fails by design, listing only non-standard names.
- Run lines: `.out/adversary/runs.tsv`. No test was added to `crates/`.

## Commands and results

- `sh adversary-1/join.sh <S> <B> <N>`: 44,812 cases; base against head, 22 go `09->12` and 99 go `19->09`; `outcome diffs 0 detail diffs 0`.
- `sh adversary-1/rule.sh <checkout> block.txt later-parts.tsv join.tsv`: 8,504 candidates; 103 clean, of which 99 are the unit's.
- One pool job, the variants and the probe: `slot --timeout 30m --fetch .out/adversary/runs.tsv --fetch .out/adversary/lines.txt sh -c 'rm -rf .cache/test262/2a1ade…ce && task test262-fetch && … part.sh … && cp -R adv-close9-tmp/test/. … && cargo run --locked --release -q -p swell-test262 -- count …'`. It printed `count exit 0` and `test262-refused 0`, and the job exited 0. The temporary `adv-close9-tmp/` was removed afterwards; `git status --short` printed nothing.
- `sh adversary-1/plants.sh adversary-1`: results under item 7, in `adversary-1/plants.out`.

## Not attacked

- The BigInt statements themselves on slice 12: there is no BigInt yet.
- Whether each of the 1,864 passing slice 9 cases reaches its asserted path, beyond the probe and the refusal and negative checks.
- The 4,496 and 3,740 slice 8 candidates case by case: they were checked by token, and none has its slice 8 token only in a harness file.

```findings
- file: docs/design/engine.md
  line: 4478
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 32 cases (59 runs) of slice 19 failed at the start on Map, Reflect or Proxy, pass in every run and have no later feature or flag line, yet stay out of slice 9 because their only later tokens (BigInt, typed arrays) sit in typeof-guarded lines of harness deepEqual.js or propertyHelper.js, a filter no rule of section 12 orders (list in adversary-1/missed-bigint.txt).
- file: docs/design/engine.md
  line: 4427
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: 37 cases refused at the start on class or new.target stay counted in slice 9, delivered before slice 8, because slice 8 code merged in the same wave, while the incoming selection keeps out every slice-19 case that needs slice 8 (story Context orders the first).
- file: docs/design/engine.md
  line: 4478
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The text says 3,740 cases hold a token of slice 8 only, but 108 of them also hold a token of a later slice such as BigInt or ArrayBuffer.
- file: taskfiles/test262.yml
  line: 229
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: With the moves.tsv line of a 9-to-12 move removed and its block line kept, test262-moves-check stays green and its count still equals the lines of the file, so only the close's checks (2) and (7) see a block move with no evidence.
```
