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

**Delivered**, 2026-11-21: [wave 1](slice-02-wave-1.md), [wave 2](slice-02-wave-2.md),
[wave 3](slice-02-wave-3.md), [wave 4](slice-02-wave-4.md), and between waves 2 and 3
[the run loop's frame](slice-02-run-loop-frame.md). All 4,259 cases of slice 2 pass in all 7,543
runs; 1,008 cases moved to the slices they need, each with evidence the gate checks; 80 of
164 host requirements have a test and none is pending for slices 1 and 2; the gate counts
7,742 cases as passed, and 46,493 of the 85,556 runs meet a refusal.

`Array.prototype.toString` was decided at the boundary of wave 3 by the deciding session:
slice 2 took it early and complete (11 cases failed on it, one of them of slice 2).

In wave 1 the three stories share no file but `conformance/test262/expected.txt`, which the
coordinator regenerates on the wave branch after the merges. The design changes the stories
wait for are in the design since wave 6 of slice 1 (`20c1f35`): the parts taken before their
slice, `Function` in slice 2, name resolution under `with` and direct `eval`, the omission
rule, `HOST-162` to `HOST-164`.

## Slice 3, `builtins-core`

`epic:slice-03-builtins-core`: `Object`, `Function`, `Array`, `String`, `Error`, `JSON`,
`globalThis` and the URI functions, and the object model of decision 4 (an index map above a
small size, dense elements of arrays). Decomposed on 2026-11-21 into thirteen stories
(accepted, `active`), read by the four plan critics in two rounds
(`review-result:slice-03-*`: 21 findings in round 1, 3 in round 2, all applied). Measured on
`main` after slice 2 (`test262-slice 03 7635 15074 0 0 7635 2762 4873 0`): 2,762 of its 7,635
cases pass. The 4,873 failing cases are on lists the stories read, under
[`slice-03/`](slice-03/): 4,050 on the ten lists of the builtin stories
(`story-<key>.cases.txt`), 203 that need the builtin of two stories (`cross-story.tsv`), 620
that are estimated to need a later slice (`move-estimate.tsv`).

| wave | stories | closes |
|---|---|---|
| 1 | `builtin-group-modules`, `property-index-and-dense-elements` | one file per group of builtins, and the operations two stories of wave 2 need, each with its tests; the object model, so that a lookup's time does not grow with the object |
| 2 | `object-integrity-and-prototype-methods`, `object-enumeration-and-copy-methods`, `function-apply-and-source-text`, `global-this-uri-functions-and-is-error`, `json-parse-and-raw-json`, `array-iteration-methods`, `array-search-and-copy-methods`, `array-mutation-and-sort-methods`, `string-access-and-search-methods`, `string-transform-and-compare-methods` | the builtins, ten units side by side, each writing its group file and its tests |
| 3 | `slice-03-conformance-close` | the cases that need a later slice move with their evidence; four requirement lines and four `omit` lines; every run of the slice passes |

**Delivered**, 2026-11-22: [wave 1](slice-03-wave-1.md), [wave 2](slice-03-wave-2.md),
[wave 3](slice-03-wave-3.md), and beside them the units on the gate and the documentation
site with the three gate runs of the branch ([the gate and the site](slice-03-gate-and-site.md)).
7,010 of the slice's 7,011 cases pass; row 3 of the table was the plan, and the design,
section 12, states the one case that does not pass and why the slice is delivered with it.

`aep plan artifact waves` prints four waves for this set: it puts the two stories of wave 1
one after the other because both record `crates/swell/src/object/mod.rs`. They run side by
side by decision: each story names its lines of that file. The ten stories of wave 2 share
mark lines in four files (`ops/objects.rs`, `ops/testing.rs`, `object/mod.rs`,
`builtins/string/mod.rs`); each story says which, and the recorded scope leaves them out.

Before wave 1, `story:gate-test-steps-within-budget`: the gate's test steps took 284 s on
`29f674c` (`test` 84 s, `test-collect` 110 s, `test262` 90 s) against a budget of 120 s set on
2026-11-21. A unit of slice 3 runs the tests of the code it touched and not the gate.

The design changes wave 2 waits for are in the design (`8d5828d`): a String method that needs
the tables of slice 5 refuses a code unit outside ASCII with a counted refusal (`HOST-165`),
and a function's source text lives as long as the function and is charged to the memory
budget.

Decided by the deciding session on 2026-11-21: the seven directories of builtins that need
iterators or the normalization tables (`Array/from`, `Object/fromEntries`, `Object/groupBy`,
`Array/prototype/keys`, `values`, `entries`, `String/prototype/normalize`; 79 cases) move
whole to slices 6 and 5 at the close, the 9 included that pass today on the `TypeError` of the
missing method. Each of the 9 has its own line of evidence that says it passed at the move,
`expected.txt` keeps them as passing, and `test262-moves-check` is not loosened for it.

## The order after slice 3

The consumer of swell is the browser surf. A rule of the organization of 2026-11-21 ("Goal
first") orders the waves by the consumer's first gap. surf's first use of a script engine,
named by the deciding session: the testharness.js tests of its HTML parser (101 files of
web-platform-tests at the commit surf pins; none runs today).

Measured on 2026-11-21 with the parser of `main` (`6a43e18`): `resources/testharness.js`,
`resources/testharnessreport.js`, `html/syntax/parsing/html5lib_url.html` and the three
scripts it loads. Each script was parsed, the first refused construct rewritten into text
slice 2 accepts, and parsed again until it was accepted, so every refused construct is
counted and not only the first.

| slice | refused constructs | what |
|---|---|---|
| 6 | 191 | `const` 75, arrow functions 41, `let` 23, template literals 22, `for`-`of` 12, default parameters 9, destructuring 6, rest parameters 2, spread 1 |
| 5 | 12 | regular expression literals (parsed; evaluating one is refused) |
| 10 | 3 | one async function and two `await`, all in `testharness.js` |
| 7, 8, 11 to 19 | 0 | |

`testharnessreport.js` and two of the three loaded scripts are accepted as they are. Builtins
the scripts name that swell lacks after slice 3, by a count of identifiers (an estimate:
references, not calls): `Promise` 13 (slice 10), `Math` and `Date` 8 (slice 4), `Map` and
`Set` 4 (slice 9), `RegExp` 2 (slice 5); no `Symbol`, typed array, `WeakMap`, `Proxy` or
`Reflect`.

The order that follows: **6, 5, 7, 10, 4, 9**, then the design's order for the rest. Slice 7
has no use of its own in these scripts and comes before 10 because async functions need its
suspended frames (the design, section 12). The design's slice table and its counts do not
change; only the order of delivery does, and it is decided again at each boundary.

How surf takes swell, as the deciding session said on 2026-11-22: as a git dependency on a
release tag (surf's `epic:scripts-in-a-page`, a draft there). Seven requirements surf needs
from the host interface come as a list when that epic is accepted; until then nothing of the
host interface is ordered by surf.

Not measured: the inline and linked scripts of the 101 test files themselves (the listing of
the repository at that commit was cut off at 61,341 entries), and which paths of the harness
run. Objects of the DOM and of the host are surf's side.

## Slice 6, `bindings-and-iteration`

**Delivered on 2026-11-23.** In the second batch ([the record](second-batch.md), 2026-11-22):
template literals, `let` and `const`, arrow functions, default and rest parameters and object
destructuring, the first five of its stories, with the stories on resource accounting and on
the host interface that the first consumer asked for; 1,364 of the slice's 4,436 cases passed
then (`test262-slice 06 4436 8413 0 0 4436 1364 3072 0`, release `0.4.0`). In wave 2
([the record](slice-06-wave-2.md), 2026-11-22 and 23): `Symbol`, computed names and object
spread, the iterator protocol, block function declarations, `for`-`of`, `Array.from`, array
destructuring and spread, tagged templates, and the close. Every case the slice counts
passes in every run (`test262-slice 06 3898 7351 0 0 3898 3898 0 0`). The close moved 143
cases to later slices with evidence, and it added the order of delivery to the block of the
design (`story:a-case-counts-in-the-slice-delivered-last`): a case is counted in the slice
delivered last among the slices it needs, so 402 cases that need slice 4 or 5 beside a part
of slice 6 are counted there.

**The order after slice 6**: 5, 7, 10, 4, 9, unchanged. Measured on 2026-11-22 by the
acceptance of `story:array-destructuring-and-spread`: the consumer's two scripts parse with
no refusal of slice 6; the regular expression literals of slice 5 (12) and one `async`
function with two `await` of slice 10 remain.

`epic:slice-06-bindings-and-iteration`, next after slice 3 by the order above. Decomposed on
2026-11-22 into fifteen stories in two parts (accepted, `active`), each part read once by the
four plan critics (`review-result:slice-06-*`: 15 findings on the first five stories, 13 on
the other ten, all applied; a story without a blocker finding is accepted after one
revision). Measured on `main` (`6a43e18`): 4,204 cases and 7,952 runs, 94 cases pass. The
lists the stories read are under [`slice-06/`](slice-06/); about 657 cases need a later
slice and move at the close (`move-estimate.tsv`).

| order | stories | starts after | lifts of the consumer's 191 constructs |
|---|---|---|---|
| 1 | `let-const-and-block-scopes`, `template-literals` (side by side, seven shared files) | `function-apply-and-source-text` of slice 3, which writes the same parser and compiler files | `const` 75, `let` 23; template literals 22 |
| 2 | `arrow-functions` | 1 | arrow functions 41 |
| 3 | `default-and-rest-parameters` | 2 | default parameters 9, rest 2 |
| 4 | `object-destructuring` | 3 | 3 of the 6 patterns |
| 5 | `computed-names-and-object-spread` | 4 | none |
| 6 | `block-function-declarations`, `symbol-constructor-and-well-known-symbols` | 5; the Symbol story also after slice 3's close | none |
| 7 | `iterator-protocol-and-array-string-iterators` | the Symbol story | none |
| 8 | `for-of-statement`, `symbol-values-in-core-builtins` | 7 and `block-function-declarations` | `for`-`of` 10 of 12 |
| 9 | `array-destructuring-and-spread`, `array-from-and-object-from-entries` | 8 | the last 2 `for`-`of` heads, array patterns 3, spread 1: both scripts parse with no refusal of slice 6 |
| 10 | `tagged-templates` | 9 | none |
| 11 | `slice-06-conformance-close` | 10 and slice 3's close | none |

The chain is long because the stories write the same files of the parser and the compiler;
each edge says which. The design text they build on is in decisions 3 and 4 (`0458ebd`,
`aec8a36`).

## Slice 5, `regexp`

`epic:slice-05-regexp`, next after slice 6 by the order above. Decomposed on 2026-11-23 into
fifteen stories (accepted, `active`), read by the four plan critics in two parts
(`review-result:slice-05-*`: 9 findings on the first six stories, none a blocker; 16 on the
other nine, 6 of them blockers, all applied, and the six stories with a blocker read a second
time). Measured on release `0.5.0`: 2,724 cases and 5,441 runs, 52 cases pass
(`test262-slice 05 2724 5441 0 0 2724 52 2672 0`). The lists the stories read are under
[`slice-05/`](slice-05/): the scan of the failing cases by what they need, the case list of
eleven stories, the table of the consumer's twelve regular expressions, and the two lists of
the close. The design text they build on is in decisions 2, 5, 7 and 8 and in section 12
(`eacc828` and its second part): how `swell-regexp` charges its work, one step of the matcher,
where a RegExp object keeps its program, a literal compiled at each evaluation, no recursion
in the pattern parser and compiler, the counted refusals of `RegExp` (`HOST-191`), the order
`localeCompare` gives outside ASCII, and a case that comes into a slice delivered before the
slice of its directory.

| order | stories | what it brings | lifts of the consumer's 12 regular expressions |
|---|---|---|---|
| 1 | `regexp-crate-and-pattern-parser`, `string-replace-replace-all-and-split`, `unicode-gen-table-list-and-source-digests` | the crate `swell-regexp` with the pattern parser; `replace`, `replaceAll` and `split` for a String pattern; the generator's table list | none; the 8 `split` calls with a String |
| 2 | `regexp-compiler-and-backtracking-matcher`, `regexp-literal-early-errors`, `case-mapping-tables-and-string-case-methods` | the matcher under budgets; a literal checked at parse time (`HOST-142`); the case methods outside ASCII | all 12 pass the early errors |
| 3 | `regexp-objects-exec-and-literals`, `normalization-tables-normalize-and-locale-compare` | `RegExp`, `exec`, `test`, the literal evaluated (`HOST-143`, `HOST-191`); `normalize` and `localeCompare` | all 12 evaluate; the `test` call and both `new RegExp` calls answer |
| 4 | `regexp-symbol-methods-and-string-match-search`, `regexp-property-escapes-and-property-tables` | `Symbol.match`, `replace`, `search`, `split` and the String methods with a RegExp; `\p{...}` | the other 11 calls: the harness's regular expressions all run |
| 5 | `identifiers-and-group-names-outside-ascii`, `regexp-match-all-and-escape` | `ID_Start` and `ID_Continue` in identifiers and group names; `matchAll`, `RegExp.escape` | none |
| 6 | `regexp-case-insensitive-matching-and-modifiers` | the flag `i` and modifiers | none |
| 7 | `regexp-v-flag-class-sets-and-properties-of-strings` | the flag `v` | none |
| 8 | `slice-05-conformance-close` | 18 cases moved to later slices, 18 Annex B grammar cases taken into slice 5 | none |

Ahead of them, by the organization's rule that a test step over its budget is fixed before the
next feature: `story:gate-test-steps-cost-half` (the gate's test steps cost about 300 CPU
seconds and passed their 120 s of wall time in one of five runs on 2026-11-23).

**Delivered so far.** [Wave 1](slice-05-wave-1.md), 2026-11-23, release `0.6.0`: the orders 1
to 3 of the table, `story:gate-test-steps-cost-half` (about 270 CPU seconds; the 150 of its
goal are not met, and the floor is stated in the story), and two stories the plan did not
have, `story:matcher-work-bounded-by-its-steps` and
`story:pattern-parser-work-linear-in-its-pattern`: 1,327 of the 2,724 cases of the slice pass
(`test262-slice 05 2724 5441 0 0 2724 1327 1397 0`). [Wave 2](slice-05-wave-2.md), release
`0.7.0`: `regexp-symbol-methods-and-string-match-search` of order 4, with which all twelve
regular expressions of the consumer answer and 1,667 cases pass. [Wave 3](slice-05-wave-3.md),
release `0.8.0`: `regexp-match-all-and-escape`, `regexp-property-escapes-and-property-tables`,
`regexp-case-insensitive-matching-and-modifiers` and
`identifiers-and-group-names-outside-ascii`, with which 2,523 cases pass
(`test262-slice 05 2724 5441 0 0 2724 2523 201 0`). After it, on the branch of the first wave
of slice 10, 2026-11-24: `regexp-v-flag-class-sets-and-properties-of-strings`, with which
2,706 cases pass, and `slice-05-conformance-close`: 18 cases move to slices 4, 8 and 9 (14, 2
and 2) and 18 cases of the pattern grammar of Annex B.1.2 come in from slice 18. Slice 5 is
delivered: `test262-slice 05 2724 5441 0 0 2724 2724 0 0`.

## Slice 7, `generators`

`epic:slice-07-generators`, next after slice 5 by the order above, and planned while the last
stories of slice 5 were built: slice 10 holds the consumer's next gap (one `async` function
with two `await`) and builds on the suspended frame of this slice's second story. Decomposed
on 2026-11-23 into six stories, read by the four plan critics in two rounds
(`review-result:slice-07-round-1-*`: 15 findings with 2 blockers on a first draft of one story,
which was then cut in two; `review-result:slice-07-round-2-*`: 17 findings with 3 blockers on
the six stories, 16 applied) and by two of them a third time for the three stories that had a
blocker (`review-result:slice-07-round-3-*`: 2 findings, both applied). Measured on the code of
release `0.7.0` with `matchAll`: 1,491 cases and 2,841 runs, 6 cases pass
(`test262-slice 07 1491 2841 0 0 1491 6 1485 0`). The lists the stories read are under
[`slice-07/`](slice-07/): the scan of the 1,485 failing cases by what they need, the case list
of five stories and the two lists of the close. The design text they build on is the group of
paragraphs "Suspended frames" in decision 3 with the matching sentences of decision 4
(`eb1164e`): what of a frame moves, who holds and traces it, its charge and its credit,
`frame_depth`, steps and terminations across entries, the result of a frame and re-entry.

| order | stories | what it brings | cases of its list |
|---|---|---|---|
| 1 | `generator-syntax-and-early-errors` | `function*` and `yield` parse with their early errors; a script with a generator and no early error still ends on the counted refusal of slice 7 | 114 |
| 2 | `generator-functions-and-suspended-frames` | generators run: the frame moves, boxed, into the generator object and back; `next`, `return`, `throw`; `HOST-73` | 978 |
| 3 | `generator-function-constructor`, `yield-star-delegation` | `GeneratorFunction` from source text through the host's hook; `yield*` in the loop of the delegating frame | 21 and 49 |
| 4 | `generator-methods-in-object-literals` | generator methods of object literals and their early errors | 281 |
| 5 | `slice-07-conformance-close` | 40 cases that need `class` or `super` moved to slice 8; the 2 cases that need both `yield*` and methods; passing cases of later slices taken in by rule lines | 42 |

A decomposition of slice 10 can start when order 2 is merged. The close runs after
`slice-05-conformance-close`: both write section 12 of the design, `moves.tsv`,
`later-parts.tsv` and `expected.txt`.

**Delivered so far.** The orders 1 to 3, built on the wave branch of slice 5 ([its wave 3](slice-05-wave-3.md), release `0.8.0`): generator
syntax, generators on suspended frames, `%GeneratorFunction%` and `yield*`; 1,168 of the
1,491 cases pass (`test262-slice 07 1491 2841 0 0 1491 1168 323 0`). One case of the list of
order 1, `expressions/object/prop-def-invalid-star-prefix.js`, ends on the refusal of a
generator method and went to the list of order 4 (281 cases). After it, on the branch of the
first wave of slice 10, 2026-11-24: order 4, `generator-methods-in-object-literals`, with which
1,451 cases pass, and `slice-07-conformance-close`: the 40 cases that need `class` (36) or
`super` (4) move to slice 8, and 21 cases come in, 2 from slice 18 and 19 from slice 19. The
close was made on a base that holds code of slice 10, so a case of a later slice comes in only
when every failed run of it at the start of the slice ends on a part of slice 7, no `feature`
or `flag` line names a later slice for it and it uses no part of a later slice; 661 other cases
that pass now stay in their slice. Slice 7 is delivered:
`test262-slice 07 1472 2801 0 0 1472 1472 0 0`.

## Slice 10, `promises-and-async`

`epic:slice-10-promises-and-async`, next after slice 7 by the order above, and planned on
the day generators ran: the suspended frame it builds on is the second story of slice 7, and
the first consumer's next gap is here, one `async` function with two `await` on promises
of the host (`fetch_json` of its test harness), in an embedder that drains the queue
after each script. Decomposed on 2026-11-23 into seventeen stories. The four of the first
wave were read by the four plan critics (`review-result:slice-10-round-1-*`: 13 findings,
none a blocker, all applied) and are accepted; the other thirteen were read in two sets
(`review-result:slice-10-later-{a,b}-*`: 69 findings, one blocker) and revised on the design
text of `3648bf6`, read again (`review-result:slice-10-later-r2-*`: three warnings, applied) and accepted. Measured on the last full run before
generators ran: 3,879 cases and 7,499 runs, 13 cases pass (`test262-slice 10 3879 7499 0 0
3879 13 3866 0`); 2,748 of its cases carry the flag `async`, and none of those passes before
the runner drains the queue. The lists the stories read are under [`slice-10/`](slice-10/):
the scans of the failing cases by what they need, a case list per story, two guard lists of
passing cases, the lists of the close, and two scripts for a unit (`part.sh` copies a story's
cases into a part of the checkout for a run of the runner, `check.sh` checks the run's lines
against the base). The design text they build on is in decisions 3, 4 and 9 and in the host
interface (`1479d2f`, `90ccbf5`): the checkpoint as one outermost entry under one step budget,
what is charged for a queued job, the realm of a host microtask, the activation of an `async`
function as the holder of its suspended frame, `await` through the host's job-callback hooks,
terminations, and nine requirements (`HOST-192` to `HOST-200`), four of them asked for by the
first embedder: what `last_entry_steps()` reads after a checkpoint, where the interrupt ends
one, what stays queued, and a call that empties the queue. The later waves build on `3648bf6`:
async generators on suspended frames, a built-in async function, the Async-from-Sync Iterator,
the kind `ErrorKind::AggregateError` and the thirteenth well-known symbol, with eight more
requirements (`HOST-201` to `HOST-208`), each moved by one story.

| order | stories | what it brings |
|---|---|---|
| 1 | `job-queue-and-microtask-checkpoint`, `async-function-syntax-and-early-errors` | the queue, the checkpoint and its bound, host microtasks, the runner's handling of `async` cases; `async` and `await` parsed with their early errors, the refusal held behind them |
| 2 | `promise-constructor-then-and-jobs` | `Promise`, its resolving functions, `then`, `catch`, the reaction jobs |
| 3 | `async-functions-and-await` | `async` functions run on a suspended frame; with it the consumer's function runs to its end |
| 4 | `promise-rejection-tracker-and-promise-reads`, `async-methods-in-object-literals`, `async-function-constructor`, `for-await-and-async-from-sync-iterator`, `async-generator-syntax-and-early-errors` | the rejection tracker and the reads of a promise; async methods (needs the generator methods of slice 7); the `AsyncFunction` constructor (needs the `GeneratorFunction` constructor); `for await`; async generator syntax |
| 5 | `promise-statics-finally-and-aggregate-error`, `ess-target-checkpoint-and-promise-hooks` | the statics and `finally` (after the tracker: `HOST-201` reads `promise_state`); the executable specification's target for the checkpoint |
| 6 | `async-generators-and-their-request-queue` | async generators (needs `yield*` of slice 7 and the statics) |
| 7 | `yield-star-in-async-generators`, `async-generator-methods-in-object-literals`, `async-generator-function-constructor`, `array-from-async` | `yield*` in an async generator; its methods and constructor; `Array.fromAsync` |
| 8 | `slice-10-conformance-close` | 141 cases moved to later slices, 21 taken in; after `slice-07-conformance-close` |

**Delivered.** On the branch of the first wave of slice 10, by 2026-11-24: the orders 1 to 4 (the last of order 4 was `for await` with the Async-from-Sync Iterator), and order 5, the statics of `Promise` with `finally` and `AggregateError` and the target of the executable specification (18 of the model's 60 scenarios pass on the engine, 14 of 58 before); and of order 6 async generators with their request queue, the constructor of async generator functions, async generator methods of object literals, `yield*` in async generators and `Array.fromAsync`; and the close, with 141 cases that need a later slice moved to slices 8, 9, 12 and 13 with evidence and 21 cases taken in from slices 18 and 19. Slice 10 is delivered: all 3,759 cases pass in all 7,258 runs (`test262-slice 10 3759 7258 0 0 3759 3759 0 0`). The record of the wave is [slice-10-wave-1.md](slice-10-wave-1.md), and release `0.9.0` holds it.

## Slice 4, `numbers-and-dates`

`epic:slice-04-numbers-and-dates`, accepted on 2026-11-24 in seventeen stories after three
readings by the plan critics (`review-result:slice-04-round-3-*`). The case list and the guard
list of each story are in `docs/plan/slice-04/`. The order follows the stories' edges; stories
of one order can be built at the same time, and each names the hunks of the files it shares.

| order | stories | what it brings |
|---|---|---|
| 1 | `math-constants-and-exact-functions`, `date-object-clock-and-time-value`, `swell-num-calendar-and-local-time`, `number-format`, `swell-num-math-double-double` | the constants and exact functions of `Math`; `Date` with the host's clock and the time value (with the first, the browser's seven `Math` calls and two `Date.now()` reads); calendar and local time in `swell-num`; `toFixed`, `toExponential`, `toPrecision`, `toLocaleString`; double-double arithmetic with `hypot` |
| 2 | `date-object-constructor-now-utc-and-getters`, `swell-num-date-text`, `math-sum-precise`, `number-functions`, `swell-num-math-exp-and-log`, `swell-num-math-trigonometry` | the `Date` constructor, `Date.UTC` and the getters; date text in `swell-num`; `Math.sumPrecise`; `parseInt`, `parseFloat` and the predicates of `Number`; the exponential, logarithm and trigonometric functions in `swell-num` |
| 3 | `date-setters`, `math-functions` | the 14 setters of `Date`; the 22 functions of `Math` |
| 4 | `date-text-formats`, `exponentiation-operator`, `math-random` | the text methods of `Date` and `Date.parse`; `**` and `**=`; `Math.random` |
| 5 | `slice-04-conformance-close` | the close of the slice |

On the wave's branch before the first story: 261 of the 1,612 cases pass
(`test262-slice 04 1612 3184 0 0 1612 261 1351 0`), and 14 host requirements are pending for
the slice.

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
- The slice that completes `JSON`: the stack one re-entry takes
  is now mostly outside the run loop ([the run loop's frame](slice-02-run-loop-frame.md)):
  of 11,100 bytes on the worst path, `json_serialize_property` is 2,080, `json_stringify`
  1,632, `json_property_list` 1,584, `call_function` 1,344 and `builtin_call` 720. A unit
  that adds to a builtin on a re-entry path has 1,560 bytes per re-entry before a stack test
  fails; it then measures again and the constant follows, or the frame is taken apart.
- The next change of `taskfiles/test262.yml`: a variable that is both exported and passed on
  the command line keeps the exported value in a task's `env:` and the command line's in
  its templates (go-task 3.54.0, [slice 2, wave 4](slice-02-wave-4.md)). The task `test262`
  reads its paths through `env:`; `test262-moves-check` reads them as template values.
- The index map of the design ("Objects": an ordered list with an index map above a small
  size) is not built: a property is found by a scan of the list, and so is a binding that a
  non-strict direct `eval` added to a record ([slice 2, wave 3](slice-02-wave-3.md)); the
  comparisons of neither scan are charged as steps. The slice that builds the map (slice 3 at
  the latest) covers both, or charges the scan.
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
