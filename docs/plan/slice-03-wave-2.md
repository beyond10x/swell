# Slice 3, wave 2 of 3: the builtins of Object, Function, Array, String, JSON and the global

One wave of the [wave plan](waves.md), 2026-11-22, on the branch `wave/s3-1`. Ten units side
by side, each implemented by a sub-agent on Opus in its own worktree, each writing its group
file and its tests. Four units were based on `862c55b` (after the first unit of
[wave 1](slice-03-wave-1.md)), six on `ec1653f` (after both). A repair unit with no story
followed the tenth. Every commit is the organization bot's. Before:
[wave 1](slice-03-wave-1.md); next: [wave 3](slice-03-wave-3.md); beside the waves:
[the gate and the site](slice-03-gate-and-site.md).

## Units

In the order they merged. "On this branch alone" is the unit's own run on its base with its
change; each unit compared its run lines again with those of `main`.

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `string-transform-and-compare-methods` | Eleven methods of `String.prototype` in `builtins/string/transform.rs`: `repeat`, `padStart`, `padEnd`, `trim`, `trimStart`, `trimEnd`, `toLowerCase`, `toUpperCase`, `toLocaleLowerCase`, `toLocaleUpperCase`, `localeCompare`. `repeat` and the pads ask the host's string length limit and the memory budget for the whole result before the first copy: `"x".repeat(2**30)` ends in a `RangeError` after 8 steps (HOST-21). The case methods and `localeCompare` answer a String whose code units are all below 0x80 and refuse any other with a counted `SyntaxError` of the method's realm (HOST-165) | 23 tests in `builtins_string_transform`, four language scripts, both collection modes; no new test over 1 s; `built-ins/String` 172 to 434 cases passed, the total 7,742 to 8,009 | `741f1a8` (merge `5fe9cc6`) |
| `string-access-and-search-methods` | Sixteen builtins in `builtins/string/access.rs`: `String.fromCharCode`, `fromCodePoint`, `raw`, and on the prototype `at`, `charAt`, `charCodeAt`, `codePointAt`, `slice`, `substring`, `lastIndexOf`, `includes`, `startsWith`, `endsWith`, `isWellFormed`, `toWellFormed`, `concat`. ToAbsoluteIndex and ToClampedIndex join ToUint16 in `ops/conversion.rs`. A step per code unit read, before the copy. A defect that predates the story is fixed: `indexOf` copied its whole receiver without charging a step (red before: on 100,000 code units under a budget of 10,000 steps it answered 0) | 18 tests in `builtins_string_access`, five language scripts, five library tests, both modes; no new test over 1.4 s; `built-ins/String` 172 to 465 cases on this branch alone; 630 runs of slice 3 gained, 4 of slice 9, 8 of slice 19 | `adebc23` (merge `712a304`) |
| `global-this-uri-functions-and-is-error` | `globalThis` as an own property of the global object; `encodeURI`, `encodeURIComponent`, `decodeURI` and `decodeURIComponent` with Encode, Decode and ParseHexOctet in `builtins/global.rs`, a step per code unit read and the result charged to the memory budget; `Error.isError` on `%Error%` (HOST-155) | 4 tests for `globalThis`, 26 for the URI functions, 2 for `Error.isError`, four language scripts, two library tests, both modes. The round trip of every scalar value ran 17.5 s as one test and is 17 tests, one per plane, each under 2 s (1,112,064 values in 1,086 Strings). The four URI directories pass every listed case (41, 42, 19, 19); slice 3 2,762 to 2,890 cases on this branch alone; 256 runs of slice 3 gained, 2 of slice 9, 36 of slice 19, none lost | `1a8df3f` (merge `0a7409b`) |
| `function-apply-and-source-text` | The UTF-16 source text of a script, of eval code and of a dynamic function is kept with its compiled code (`Compiled::source`), charged two bytes per code unit, credited by the sweep that frees the code. `Function.prototype.toString` returns it when `host_has_source_text_available` answers true, a NativeFunction string otherwise (HOST-120). `Function.prototype.apply` builds its list with CreateListFromArrayLike, a step per element, each element kept and charged 32 bytes until the call returns | 13 tests in `builtins_function_source`, two language scripts, one compiler test, both modes; the step test uses 100 and 200 elements (with 1,000 and 2,000 it ran 7.2 s in collect mode); `built-ins/Function` 322 to 364 cases; 112 runs of slice 3 gained, 8 of slice 19 | `363eb33` (merge `918b7a6`) |
| `json-parse-and-raw-json` | Clause 25.5 complete. `JSON.parse` is a loop with a stack of its own, charged to the memory budget: 100,000 nested brackets end in a result and never in a Rust stack overflow. A step per code unit read and per value made. The reviver walk runs on frames of its own and gets the source text of a primitive as `context.source`. `JSON.rawJSON` and `JSON.isRawJSON`; `stringify` writes a raw JSON object's text | 14 tests in `builtins_json_parse`, three language scripts, both modes; one over 5 s: the reviver on 2,000 nested arrays, 9.9 s in collect mode (4.3 s in the default mode). A re-entry through a reviver needs 10,240 bytes, through `toJSON` or a replacer 11,248 (11,216 before). `built-ins/JSON` 48 to 114 cases, every listed case passes; 134 runs of slice 3 gained, 20 of slice 19, none lost | `36091df` (merge `af4ae41`) |
| `array-search-and-copy-methods` | Eleven builtins in `builtins/array/search.rs`: `Array.of` and on the prototype `at`, `indexOf`, `lastIndexOf`, `includes`, `slice`, `concat`, `toLocaleString`, `with`, `toReversed`, `toSpliced`. `Array.prototype[@@unscopables]` through the hook of wave 1. Every loop charges a step per index (measured: exactly 100 more steps from 100 to 200 elements) | 16 tests in `builtins_array_search`, three library tests, five language scripts, both modes; no new test over 1.6 s; `built-ins/Array` 447 to 960 cases (the story's floor 953); 1,058 runs of slice 3 gained, 21 of slice 19, none lost | `8cb8571` (merge `854b1b0`) |
| `object-integrity-and-prototype-methods` | Thirteen builtins in `builtins/object/integrity.rs`: `Object.preventExtensions`, `isExtensible`, `seal`, `isSealed`, `freeze`, `isFrozen`, `create`, `defineProperties`, `setPrototypeOf`, `is`, and on the prototype `isPrototypeOf`, `toLocaleString`, `valueOf`. `create` and `defineProperties` share ObjectDefineProperties, whose lists and room on the kept stack are charged before the loop | 25 tests in `builtins_object_integrity`, six language scripts, both modes; no new test over 5 s (the re-entry test 4.7 s in collect mode). Bytes per re-entry: `defineProperties` through a getter of a descriptor field 11,359, `Object.create` 11,744, `toLocaleString` 7,904. `built-ins/Object` 1,574 to 2,658 cases (the floor 2,626), every listed case passes; 2,282 runs of slice 3 gained, 12 of slice 4, 4 of slice 9, 46 of slice 19, none lost | `9312c56` (merge `82d154e`) |
| `array-iteration-methods` | In `builtins/array/iteration.rs`: `forEach`, `every`, `some`, `filter`, `reduce`, `reduceRight`, `find`, `findIndex`, `findLast`, `findLastIndex`, `flat`, `flatMap`. A walk charges a step per index, and a missing key a step for each prototype the lookup passes (N holes cost 2N steps); `flat` and `flatMap` flatten on an explicit stack | 33 tests in `builtins_array_iteration`, two library tests, five language scripts, both modes; no new test over 3.6 s. Each of the eleven callback methods runs at the re-entry limit on seven eighths of `min_stack_bytes()`. `built-ins/Array` 447 to 1,744 cases on this branch alone (the floor 1,710), every listed case passes; 2,584 runs of slice 3 gained, 8 of slice 6, 6 of slice 19, none lost | `438f1ba` (merge `c766223`) |
| `object-enumeration-and-copy-methods` | Six builtins in `builtins/object/enumeration.rs`: `Object.assign`, `entries`, `getOwnPropertyDescriptors`, `hasOwn`, `keys`, `values`, each a step per key and its lists charged. `Object.getOwnPropertySymbols`, the seventh of the story, is not delivered (below); the test `no_builtin_hands_a_symbol_value_to_a_script` walks every object reachable from the global object and finds no property whose value is a Symbol | 14 tests in `builtins_object_enumeration`, four language scripts, both modes; no new test over 0.4 s (the re-entry test of `assign` uses depth 7 in collect mode; it took 5.9 s at depth 100). 264 runs of slice 3 gained, none lost; `built-ins/Object` 1,704 cases on this branch alone (1,574 on `main`; the floor 1,698) | `63810d7` (merge `dbef94a`) |
| `array-mutation-and-sort-methods` | In `builtins/array/mutation.rs`: `copyWithin`, `fill`, `pop`, `reverse`, `shift`, `sort`, `splice`, `toSorted`, `unshift`. The sort is a bottom-up merge sort whose list and buffer live on the kept stack, their growth charged before it happens; a step per comparison, per element moved, and per code unit compared without a comparator | 17 tests in `builtins_array_mutation`, two library tests, four language scripts, both modes. Over 5 s in collect mode: the re-entry test 11.3 s, the step-per-comparison test 8.7 s, the comparator that throws or lies 5.4 s. Bytes per re-entry: `sort` 9,952, `toSorted` 10,063, `fill` through `valueOf` 10,880. `built-ins/Array` 447 to 635 cases on this branch alone (the floor of the story); 391 runs of slice 3 gained, 15 of slice 19, none lost | `10d303d` (merge `c108c2d`) |

The repair, after the tenth unit (below):

| unit | what landed | its own gate | commits |
|---|---|---|---|
| `join-index-helpers` (no story) | One definition each of ToAbsoluteIndex and ToClampedIndex in `ops/conversion.rs`, citing their clauses; the copies in `builtins/array/mutation.rs` and `builtins/array/search.rs` are gone and the callers of the three groups use the shared ones. No behaviour, charge or test expectation changes. Then two match arms of `search.rs` as rustfmt writes them | `cargo check` of the crate exit 0 (`d976558`); then clippy exit 0, the library tests and six test binaries of the three groups pass in both collection modes (77, 16, 17, 16, 18, 23 and 1 passed, 0 failed), `cargo fmt --all --check` exit 0 (`4760cf1`) | `d976558` (merge `296fb10`), `4760cf1` (merge `703d4b2`) |

Each unit's claim was run again by the coordinator before its commit (the merge bodies); for
`string-access-and-search-methods` and `global-this-uri-functions-and-is-error` the record is
`b7d7b67`: the test binaries in both collection modes, clippy, the run lines against `main`.
On the wave branch the coordinator ran clippy and the tests of the crate `swell` on
`0a7409b` (five units of the slice merged) and on `c766223` (ten), the second in both
collection modes, exit 0 each (`wave-s3-1/after-5-units/`, `after-10-units/`). Its directory
for `c108c2d` (`after-12-units/`) holds only the queued line of a clippy job and no result.

**What works now.** A script may freeze and seal objects, create them with a prototype and
descriptors, list their keys, values and entries, and copy them with `Object.assign`; call
`Function.prototype.apply` and read a function's source text; walk, search, copy, change and
sort arrays; read, search, pad, trim and compare Strings; parse JSON with a reviver; encode
and decode URIs; and read `globalThis`. With every builtin of slice 3 in place, 616 of its
7,635 cases still failed (the count of the close, `3d9c1bf`). The record does not have a run
of the whole wave branch between the merges: each unit commit says that `expected.txt` "is
regenerated on the wave branch", and it was not changed on the branch before the close wrote
it ([wave 3](slice-03-wave-3.md)).

**Passes of other slices that went down.** Eight cases that passed at the start of the slice
fail now, none of them in slices 1 to 3 (the close, `3d9c1bf`):

- Slice 5: `test/built-ins/Function/prototype/apply/this-not-callable.js`, then a case of
  slice 3. `363eb33`: it "passed before only because apply was missing and now ends on the
  counted refusal of a regular expression literal (slice 5)". The close moved it to slice 5.
- Slice 6, five cases: `String/prototype/padEnd/exception-symbol.js`,
  `padStart/exception-symbol.js` and `toLowerCase/this-value-tostring-throws-symbol.js` (by
  `741f1a8`: "three of slice 6 call the method with a Symbol"), and
  `slice/this-value-tostring-throws-symbol.js` and
  `substring/this-value-tostring-throws-symbol.js` (by `adebc23`: they "now fail on Symbol
  (slice and substring called on a Symbol)"). Both commits say they passed before only on the
  `TypeError` of the missing method; the close: they "get a ReferenceError until slice 6 has
  Symbol". The commits give counts and methods; the five paths are the slice 6 cases under
  `String/prototype/` ending in `symbol.js` that passed in the base run lines
  (`slice-03/base-runs.tsv`), one per method named.
- Slice 18, two cases: `test/annexB/built-ins/String/prototype/trimLeft/reference-trimStart.js`
  and `trimRight/reference-trimEnd.js`. `741f1a8`: they passed before only on the `TypeError`
  of the missing method and "compare with trimLeft and trimRight", which are of slice 18; the
  close: they "compare trimLeft and trimRight with trimStart and trimEnd, which now exist".

## What went wrong, and what was decided

- **The twelfth unit of the slice was merged without compiling the merged commit.** The merge
  of `array-mutation-and-sort-methods` (`c108c2d`) applied without a conflict and left the
  crate `swell` not compiling: that unit and `string-access-and-search-methods`, merged
  earlier, each define `Agent::to_clamped_index` (E0592, and six E0034 at its call sites in
  `builtins/array/mutation.rs`); the Array search unit carried a third copy of the arithmetic
  as free functions (`d976558`). The coordinator had merged on the unit's report and its own
  comparison of run lines (`1e6fffa`). The unit that closes the slice was opened on `c108c2d`,
  waited for its first full run in the pool and stopped at the compile error.
- **The repair** is the unit `join-index-helpers` (table above). `d976558` was committed with
  its acceptance job still queued in the pool; the job then found two match arms that
  `cargo fmt --check` refuses, fixed in `4760cf1`, after which the job ran green. The close
  unit took the repair into its branch (`8e0aa7d`).
- **The rule since `1e6fffa`** (`AGENTS.md`, "How the work is run"): after the merge of a unit
  whose base is not the head of the wave branch, `cargo check --locked --workspace
  --all-targets` runs on the merged commit in the check tree, and no unit is opened on the
  branch and nothing is reported as merged and working before it exits 0. A `cargo check` of
  the workspace in the check tree finished in 2 min 46 s after 984 s in the queue
  (`compiles/s3-1.log`; the log does not name the commit it checked).
- **`Object.getOwnPropertySymbols` leaves slice 3** (`971e273`). The unit found that it is the
  one builtin of the slice that hands a Symbol value to a script, and that ToObject of a Symbol
  faults the agent before slice 6: with a symbol taken from `%Array%`, `Object(s)`,
  `Object.keys(s)` and `Object.hasOwn(s, "x")` each ended in `Terminated(EngineFault)`
  (`63810d7`). By the design's rule (section 12) a builtin that exists is correct for every
  argument or does not exist yet: the method comes with the Symbol story of slice 6. Its two
  listed cases (`length.js`, `name.js`) move to the estimate of cases the close moves to
  slice 6.
- **Cases that left a story's list**, each found by the unit:
  - `String/prototype/localeCompare/15.5.4.9_3.js` holds text outside ASCII and ends on the
    counted refusal of slice 5; from `story-string-transform.cases.txt` (270 lines) to
    `move-estimate.tsv` (621). The decomposition counted 13 such cases; it is 14 (`39c3df4`).
  - `String/prototype/lastIndexOf/S15.5.4.8_A1_T12.js` calls `lastIndexOf` on an Array; from
    `story-string-access.cases.txt` (308 lines) to `cross-story.tsv` (204) (`ec207bc`).
  - `Object/getOwnPropertyNames/15.2.3.4-4-2.js` asserts by name that `Object` has `create`,
    `seal`, `freeze` and six more; the scan of the decomposition did not see names inside
    strings. From the Array search story's list to `cross-story.tsv` (`0ba3374`).
- **Tests made shorter in collect mode**, each unit's measurement: the iteration unit's
  step-budget and step-per-index tests use 1,000 steps and 100 indices there (40.9 s and
  11.6 s with the story's numbers), and the mutation unit's stability test sorts 11 and 17
  elements there (513 took 258 s); the default mode keeps the story's numbers. The integrity
  unit builds its fixtures of 1,000 entries from Rust: a script loop collects at every
  instruction in collect mode and took 59 s.
- **Other decisions recorded during the wave**: verification records of three units
  (`b7d7b67`); a unit's pool job asks for one slot (`c2f1861`,
  [the gate and the site](slice-03-gate-and-site.md)); surf takes swell as a git dependency on
  a release tag, and the Writes line of `story:let-const-and-block-scopes` gains two files
  (`cf29780`). Beside the wave slice 6 was decomposed into fifteen stories and accepted, and
  its design text merged (`d88710a` to `15559ca`; the wave plan, "Slice 6").

## Where each finding went

| finding | where it went |
|---|---|
| Two definitions of ToClampedIndex and a third copy of the arithmetic | `join-index-helpers` (`d976558`, `4760cf1`) |
| `Object.getOwnPropertySymbols` faults on a Symbol before slice 6 | the Symbol story of slice 6; its directory moved there at the close |
| Three listed cases that need another story or slice | `move-estimate.tsv` and `cross-story.tsv`, the lists the close read |
| Eight passes lost | `apply/this-not-callable.js` moved to slice 5 at the close; the seven of slices 6 and 18 are in their slice already and are not moved (the design, section 12) |
| The one test function that runs every language script takes 49 to 74 s in collect mode under load (`438f1ba`) | the gate unit: the language test is 13 parts (`ae58a03`) |
| The stack per re-entry through `JSON` ([wave plan](waves.md), "What later slices take over") | measured by the JSON unit: 11,248 bytes through `toJSON` or a replacer; the promise is 14 KiB and `STACK_BYTES_PER_REENTRY` is unchanged |
| `expected.txt` not regenerated between the merges | the close wrote it ([wave 3](slice-03-wave-3.md)) |

## The gate

The gate ran once for the branch, after its last unit: its lines are in
[the gate and the site, "The gate"](slice-03-gate-and-site.md#the-gate).
