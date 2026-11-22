---
format: aep.planning-md/3
id: story:property-index-and-dense-elements
kind: story
status: active
title: 'Property lookup through an index map and dense array elements: no scan the step budget does not see'
summary: The index map and the dense element storage of the design, for objects, arrays and the bindings eval adds; 40,000 pushes take 16 times the time of 10,000 on the base because every lookup scans the list, and at most 8 times after; the six probes are files of the story; no run changes its outcome.
relations:
- decomposes: epic:slice-03-builtins-core
scope:
- confidence: cited
  path: crates/swell/src/env/mod.rs
- confidence: cited
  path: crates/swell/src/heap/accounting.rs
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: cited
  path: crates/swell/src/heap/map.rs
- confidence: cited
  path: crates/swell/src/heap/mod.rs
- confidence: cited
  path: crates/swell/src/object/arguments.rs
- confidence: cited
  path: crates/swell/src/object/array.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/object/ordinary.rs
- confidence: cited
  path: crates/swell/src/object/property.rs
- confidence: cited
  path: crates/swell/src/object/string.rs
- confidence: cited
  path: crates/swell/tests/probes/property-index
- confidence: cited
  path: crates/swell/tests/property_index.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T21:30:44Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T21:30:44Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
---
## Context

The design gives an object "an ordered list of (key, attributes, value) with an index map above a small size, and dense element storage for arrays with a sparse fallback" (`docs/design/engine.md`, decision 4, "Objects"). Neither is built. On the base (`309b850`) a property is found by a scan of the list (`Properties::get`, `get_mut` and `remove` in `crates/swell/src/object/property.rs`, line 139 on), an Array has no storage of its own (`Array {}` in `crates/swell/src/object/array.rs`: its elements are index properties of the same list), `Properties::keys` sorts the indices again on every call, and a binding that a non-strict direct `eval` added to a record is found by the same kind of scan (`Added::find` and the search over `names` in `crates/swell/src/env/mod.rs`, lines 96 and 120). None of these comparisons is charged as a step. This story answers two entries of `docs/plan/waves.md`, "What later slices take over": the entry that begins "The index map of the design" (the map, the lookup of a binding `eval` added, and the charge of a scan that stays: "slice 3 at the latest") and the entry that begins "The slice that brings the first allocation-heavy cases" (where the time of a run that fills its memory budget with `a.push({})` goes). The epic as the decomposition read it names neither; the epic is rewritten after the decomposition to name both.

Measured on the base with the built runner (`target/release/swell-test262`, a release build) on six probe scripts, each run as one test262 case of its own. The time is the milliseconds of the runner's `test262-time` line for the run, the steps its `test262-steps` line; each probe ran twice at each n and the smaller time is given. The scripts and the command are in Change: the unit keeps them as files, so the measurement is repeated in its own tree.

| probe | what the script does | n | time in ms | steps | time at the larger n over time at the smaller |
|---|---|---|---|---|---|
| `push-objects` | `a.push({})` n times | 10,000 / 40,000 | 202 / 3,211 | 250,136 / 1,000,136 | 15.9 |
| `push-numbers` | `a.push(i)` n times | 10,000 / 40,000 | 193 / 2,990 | 250,136 / 1,000,136 | 15.5 |
| `index-write-read` | `a[i] = i` n times, then a loop that reads `a[i]` | 10,000 / 40,000 | 230 / 3,177 | 430,154 / 1,720,155 | 13.8 |
| `named-write-read` | `o["p" + i] = i` n times, then a loop that reads it | 10,000 / 40,000 | 251 / 3,951 | 557,934 / 2,297,935 | 15.7 |
| `garbage` | `x = {}` n times | 160,000 / 640,000 | 130 / 525 | 2,880,122 / 11,520,122 | 4.0 |
| `linked-live` | `x = { next: x }` n times, all live | 160,000 / 640,000 | 199 / 823 | 3,200,126 / 12,800,126 | 4.1 |

The time grows with the square of n while the steps grow with n, with numbers as with objects; allocation and collection are linear. So the finding recorded as the time of allocation is the scan of the property list: `a.push` finds the key among n. The step budget does not bound the time of such a run (`AGENTS.md`, "Rules": every loop the engine runs for a script is charged): at the runner's 2^30 steps a script may push 43 million elements, and 40,000 already take 3.2 s. No run of the base ends by a budget (`awk -F'\t' '$5 == "Budget"' .out/test262-base/runs.tsv | wc -l` prints `0` on the base), and no case of slice 3 loops over a large array today, because the array methods are missing; `Array.prototype.sort/stability-2048-elements.js`, the URI cases (fourteen of the runner's twenty `test262-steps` lines of the base, 3,013,941 steps each, `.out/test262-base/count.log`) and `test/staging/sm/JSON/parse-mega-huge-array.js` (16,777,758 steps) are the ones that will.

## Domain relations

- Object → Property, one-to-many, the object owns its properties and they are freed with it — inferable (inferred from `crates/swell/src/object/mod.rs:74`, the field `properties` of `ObjectRecord`; no ess/1 document declares this relation).
- Declarative Environment Record → binding added by eval code, one-to-many, the record owns them — inferable (inferred from `crates/swell/src/env/mod.rs:112`, `Added`; no ess/1 document declares this relation).

## Change

Failing tests first: the tests below are written before the code; the counting tests fail on the base (a lookup visits every entry before the key) and `memory_budget_of_64_mib_ends_a_push_loop` does not end.

- `crates/swell/src/object/property.rs`: `Key` is hashable; `Properties` keeps its ordered list and, above a small size the unit states with its measurement, an index map from key to position (`KeyedMap` of `crates/swell/src/heap/map.rs`, keyed by the agent's hash key and by nothing else, `HOST-145`), kept in step by every addition and removal. For that key `crates/swell/src/heap/mod.rs` gains one field and nothing else: `Heap` keeps a `SipBuildHasher` made from the agent's hash key (`HOST-145`), because no other file of this story could reach the key, which ended in the private `Atoms::map`. A removal keeps the order of the others. `keys` answers the order of OrdinaryOwnPropertyKeys (`sec-ordinaryownpropertykeys`) without a sort per call that is not charged. A counter compiled for tests counts the entries a lookup visits.
- `crates/swell/src/object/array.rs`: the elements of an Array from index 0 on, each a data property with the attributes of an element an assignment makes ({ [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }), live in dense storage with holes; an element with other attributes, an accessor, and an index far beyond the dense part (the bound is the unit's, stated and tested) live in the list, the sparse fallback. [[GetOwnProperty]], [[DefineOwnProperty]] (`sec-array-exotic-objects-defineownproperty-p-desc`), ArraySetLength (`sec-arraysetlength`), [[Delete]] and [[OwnPropertyKeys]] of an Array answer over both, as the clauses say; nothing a script can observe changes.
- `crates/swell/src/object/mod.rs`, `crates/swell/src/object/ordinary.rs`, `crates/swell/src/object/string.rs`, `crates/swell/src/object/arguments.rs`: the callers of the list; the growth of the map and of the dense storage is charged to the memory budget before it is made (`try_reserve`, `Terminated(MemoryBudget)` over it) and credited when the object is freed, as the list's growth is (`Grown`).
- `crates/swell/src/env/mod.rs`: the bindings eval code added and the names of a record's slots are found through the same map above the same size, or each comparison of the scan is charged as a step; the unit says which and why.
- `crates/swell/src/heap/map.rs`, `crates/swell/src/heap/accounting.rs`: what `KeyedMap` lacks for it (removal, the bytes of its table). `crates/swell/src/heap/collect.rs`: the dense elements are traced, and the test that makes one object of every kind holds an Array with dense and with sparse elements.
- A scan or sort that stays anywhere on these paths charges a step per entry it visits.
- `crates/swell/tests/probes/property-index/` (new): the six probe scripts and `run.sh`, so that the measurement of Context needs nothing outside the unit's tree. Each script is one line, with `@N@` where n goes, and ends by throwing a `Test262Error` that carries its result, so the run line reads `Failed` with the result as its detail:
  - `push-objects.js`: `var a = []; for (var i = 0; i < @N@; i++) a.push({}); throw new Test262Error('n=' + a.length);`
  - `push-numbers.js`: `var a = []; for (var i = 0; i < @N@; i++) a.push(i); throw new Test262Error('n=' + a.length);`
  - `index-write-read.js`: `var a = []; for (var i = 0; i < @N@; i++) a[i] = i; var s = 0; for (var i = 0; i < @N@; i++) s += a[i]; throw new Test262Error('s=' + s);`
  - `named-write-read.js`: `var o = {}; for (var i = 0; i < @N@; i++) o['p' + i] = i; var s = 0; for (var i = 0; i < @N@; i++) s += o['p' + i]; throw new Test262Error('s=' + s);`
  - `garbage.js`: `var x; for (var i = 0; i < @N@; i++) x = {}; throw new Test262Error('done');`
  - `linked-live.js`: `var x = null; for (var i = 0; i < @N@; i++) x = {next: x}; throw new Test262Error('done');`
  - `run.sh`, run from the repository root after `cargo build --locked --release -p swell-test262`. It prints one line: the probe, n, the milliseconds, the steps, the outcome and the detail. `slot sh crates/swell/tests/probes/property-index/run.sh push-objects 40000` on the base printed `push-objects	40000	3211	1000136	Failed threw Test262Error: n=40000`. Its text, with `here` the directory of the scripts:

    ```sh
    #!/bin/sh
    # usage, from the repository root: run.sh <probe> <n>
    set -eu
    probe=$1
    n=$2
    here=$(dirname "$0")
    pin=$(awk '$1 == "commit" { print $2 }' conformance/test262/PIN)
    out=.out/probes/property-index/$probe-$n
    rm -rf "$out"
    mkdir -p "$out/test/built-ins/probe"
    ln -s "$PWD/.cache/test262/$pin/harness" "$out/harness"
    ln -s "$PWD/.cache/test262/$pin/features.txt" "$out/features.txt"
    { printf '/*---\ndescription: probe\nflags: [onlyStrict]\n---*/\n'; sed "s/@N@/$n/g" "$here/$probe.js"; } > "$out/test/built-ins/probe/p.js"
    target/release/swell-test262 count --test262 "$out" --design docs/design/engine.md --features conformance/test262/features.txt --workers 1 --runs "$out/runs.tsv" > "$out/lines.txt" 2> "$out/log.txt" || true
    awk -v p="$probe" -v n="$n" '$1 == "test262-time" { ms = $4 } $1 == "test262-steps" { steps = $2 } END { printf "%s\t%s\t%s\t%s\t", p, n, ms, steps }' "$out/log.txt"
    awk -F'\t' '{ print $5, $6 }' "$out/runs.tsv"
    ```
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and the `AGENTS.md` of the repository once `story:gate-test-steps-within-budget` has landed): the unit times each test it adds before it adds it; none runs over 5 s without a reason given in the report, and none over 60 s; no `OnceLock` is shared between test functions (the test steps run a process per test, so it would be built once per test); the unit does not run the whole gate, which runs once on the wave branch.
- Tests: in `object/property.rs`, `a_lookup_above_the_small_size_visits_a_bounded_number_of_entries` (100,000 properties, 100,000 lookups, the counter stays below the small size per lookup) and `removal_keeps_the_order_and_the_map`; in `object/array.rs`, `dense_and_sparse_elements_answer_alike` (the same sequence of defines, deletes and length changes on an Array kept dense and on one forced sparse gives the same own keys, descriptors and length) and `an_index_far_beyond_the_dense_part_does_not_allocate_it` (`a[4294967294] = 1` under a memory budget of 1 MiB answers) and `a_non_extensible_array_with_read_only_elements_refuses_every_change` (an Array made non-extensible by `prevent_extensions`, each element defined again with [[Writable]] and [[Configurable]] false, once in dense and once in sparse storage: a Set, a define, a delete and a change of `length` are refused alike and every element stays; this is the state the integrity operations of `story:builtin-group-modules` and the builtins of the next wave produce); `crates/swell/tests/property_index.rs` (new): `memory_budget_of_64_mib_ends_a_push_loop` (the script of `HOST-10` with `memory_bytes` 64 MiB ends in `Terminated(MemoryBudget)`: it did not end in nine minutes before, `docs/plan/slice-01-wave-6.md`), `own_keys_keep_their_order_past_the_small_size` (integer keys ascending, then strings and symbols in creation order, after additions and deletions on both sides of the size), `eval_added_bindings_are_found_past_the_small_size`, `the_same_seeds_give_the_same_steps_and_heap_bytes` (two agents, a script of 10,000 properties: `last_entry_steps()` and `heap_bytes()` are equal; the map's iteration order is never observed). Every test runs in both collection modes (`test-collect`).

Writes: `crates/swell/src/object/{property,array,mod,ordinary,string,arguments}.rs`, `crates/swell/src/env/mod.rs`, `crates/swell/src/heap/{map,accounting,collect}.rs`, `crates/swell/src/heap/mod.rs` (one field), `crates/swell/tests/property_index.rs` (new), `crates/swell/tests/probes/property-index/{push-objects,push-numbers,index-write-read,named-write-read,garbage,linked-live}.js` and `crates/swell/tests/probes/property-index/run.sh` (new). Shared with the wave: `crates/swell/src/object/mod.rs` with `story:builtin-group-modules`, which removes the four attribute lines 280 to 283 there (the mark of `prevent_extensions`) and nothing else; this story leaves those four lines as they are and writes what it needs elsewhere in the file. No other file of that story; `conformance/test262/expected.txt` does not change.

## Acceptance

`cargo test --locked -p swell`, `slot task test-collect` and `slot task test-faults` exit 0 and run the tests named in Change; with `slot task test262 TEST262_OUT=.out/test262-base` run on the base before the change and `slot task test262 TEST262_OUT=.out/test262-unit` after it, both exit 0 and `awk -F'\t' '{ print $3, $4, $5 }'` over the two `runs.tsv` gives the same 85,556 lines (`cmp` of the two outputs exits 0: no run changes its outcome); with `slot sh crates/swell/tests/probes/property-index/run.sh <probe> <n>` run three times for each probe at each n of the table of Context and the smallest of the three times taken, the time at 40,000 is at most 8 times the time at 10,000 for each of `push-objects`, `push-numbers`, `index-write-read` and `named-write-read`, and the time at 640,000 at most 8 times the time at 160,000 for `garbage` and `linked-live` (a time that grows with n gives 4, and 4.0 and 4.1 are measured for the last two; the scan of the base gives 13.8 to 15.9 for the first four and tends to 16; 8 is twice the one and half the other, and a ratio of two runs made one after the other holds on a loaded machine where a time in seconds does not), every run prints the result of the base (`n=`, `s=`, `done`), and the report gives the thirty-six lines with those of the base beside them; and the three stack tests (`limits_scripts.rs`, `budgets.rs`, `eval_stack.rs`) pass unchanged on seven eighths of `min_stack_bytes()`.

## Out of scope

Shapes, NaN-boxing and every other change of representation the design rejects for the first slices (decision 4); fast paths in the interpreter's instructions; any builtin; `STACK_BYTES_PER_REENTRY`; typed arrays (slice 13).
