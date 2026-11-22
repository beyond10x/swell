---
format: aep.planning-md/3
id: story:string-comparison-and-property-keys-charge-per-code-unit
kind: story
status: active
title: Comparing two Strings and turning a String into a property key charge a step per code unit past a small allowance, before the work and without a copy
summary: string_equals, IsLessThan, the default compare of sort, key_from_string with the atom table, and StringToNumber charge their code units before they read them, compare in place, and keep every step count of a String or key of up to 32 code units as it is. The atom lookup that makes nothing is written here.
relations:
- depends_on: story:own-keys-charged-before-the-list-exists
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T10:11:57Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T10:11:57Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:own-keys-charged-before-the-list-exists` (both change step
charges that the property-index probes measure, and it writes `builtins/array/mutation.rs`,
other functions) and `story:arena-capacity-and-collector-workspace-on-the-ledger` (it charges
the atom table by capacity in `crates/swell/src/heap/string.rs`, `Atoms`, and the entry charge of
`intern`, the function whose lookup this story rewrites). Batch H2. Requirements HOST-181,
HOST-182. Consumer: the browser, requirement 2 of its list (the remainder the budget batch does
not cover). Decision D2 (taken: option A, 32 code units free, then a step per code unit, charged
before the work). Stories that wait for this one: `story:string-builtins-charge-before-they-copy`
(the test-only copy count, the in-place comparison) and
`story:an-error-object-is-read-without-running-code` (the atom lookup).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`. `docs/design/engine.md:719-722` ("The step
budget"): builtins charge "per code unit for string operations".

- **String equality charges nothing.** `string_equals` (`crates/swell/src/ops/testing.rs:43-48`)
  copies both Strings whole (`string_units`, `crates/swell/src/object/string.rs:48-50`, a new
  `Vec<u16>` each) and compares them, with no step. It is the String case of SameValueNonNumber
  (`testing.rs:55`), so of SameValue (`:64`), SameValueZero (`:72`), IsStrictlyEqual (`:80`) and
  IsLooselyEqual (`:90`). Reached by `==` (`crates/swell/src/interp/dispatch.rs:844`), `===` and
  `!==` (`:852`), a `switch` case (`:879`), ValidateAndApplyPropertyDescriptor
  (`crates/swell/src/object/ordinary.rs:246`), `Object.is`
  (`crates/swell/src/builtins/object/integrity.rs:407`), `includes`, `indexOf` and `lastIndexOf`
  over String elements (`crates/swell/src/builtins/array/search.rs:363`, `:419`, a step per
  element and none per unit), and the SameValue of a `JSON.parse` reviver
  (`crates/swell/src/builtins/json/parse.rs:894`). One instruction can so compare two Strings of
  millions of code units for one step.
- **IsLessThan** (`testing.rs:133-158`) copies both Strings whole (`:154`) and charges the
  shorter length after the copies (`:155-156`): `"a" < s` copies all of `s` for one step. The
  default compare of `Array.prototype.sort` does the same (`compare_as_strings`,
  `crates/swell/src/builtins/array/mutation.rs:776-787`, its copies through `sort_string_units`,
  `:791-806`).
- **A String as a property key charges nothing.** `key_from_string`
  (`crates/swell/src/ops/conversion.rs:336-339`) copies the String, tries `array_index` (at most
  10 units) and interns it: `Heap::intern` (`crates/swell/src/heap/string.rs:136-167`) hashes
  every unit with SipHash and compares the found atom unit by unit (`:137-143`), and on a miss
  copies the units twice more. No step. Reached by every computed member access and `in`
  (`dispatch.rs:589`, `:616`, `:635`, `:833`, `:1207`), `Object.defineProperty`,
  `getOwnPropertyDescriptor`, `hasOwnProperty`, `propertyIsEnumerable`
  (`crates/swell/src/builtins/object/mod.rs:212`, `:357`, `:419`), `Object.hasOwn`
  (`crates/swell/src/builtins/object/enumeration.rs:261`), the property list of `JSON.stringify`
  (`crates/swell/src/builtins/json/mod.rs:213`, `:334`) and the host's keys
  (`crates/swell/src/host/mod.rs:347`, `:746`).
- **StringToNumber** copies the String and then charges a step per unit
  (`conversion.rs:177-179`): in proportion, after the work.
- Precedent for an allowance: the names of an environment record's slots charge a step per entry
  past `SMALL` (`crates/swell/src/env/mod.rs:84-91`).
- The property scan that 0.2.0's changelog names among its limits (`CHANGELOG.md:97-98`) is gone
  on the base: since `4bce0a1` a lookup goes through an index map once a list passes `SMALL` = 8
  entries (`crates/swell/src/object/property.rs:184`, `KeyedList::position`, `:248-263`).
  `## [Unreleased]` says nothing of it (`CHANGELOG.md:11`).

## Change

Failing tests first (Acceptance). Then (decision D2, option A):

1. **In place.** `HeapString` in `crates/swell/src/heap/string.rs` gains a comparison of two
   records and of a record with a slice of units, equality and order, Latin-1 and UTF-16 mixed,
   allocating nothing.
2. **Equality.** `string_equals` answers `true` for one handle and `false` for two lengths that
   differ, as now, with no step. Otherwise, with n the length, it charges `n - 32` steps when n is
   above 32, through `Agent::charge_steps`, before it reads a unit, and compares the two records
   in place.
3. **IsLessThan and the default compare of `sort`.** Each charges the shorter length, as today,
   before it reads a unit, and compares in place with no copy (`sort_string_units` copies only a
   value that is not a String, as today). Their step totals do not change.
4. **Keys.** `key_from_string` charges `n - 32` steps for a String of n code units above 32
   before it hashes. `intern` splits into a lookup that hashes and compares the String's record in
   place and answers the atom or none, making nothing (`Heap::atom_of`, the one owner of this
   function; `story:an-error-object-is-read-without-running-code` calls it), and the insertion,
   which copies the units only on a miss, into the atom's own storage, charged as the arena story
   charges it.
5. **StringToNumber** charges its step per unit before it copies.
6. **A test-only count of copies.** `crates/swell/src/object/string.rs` gains, under
   `#[cfg(test)]`, a thread-local high-water mark of the longest copy `string_units` made, with a
   reset; the lib test binary has it, no other build. `story:string-builtins-charge-before-they-copy`
   reads it too.
7. **Stays as it is.** Every answer of every operation; the step count of every comparison and key
   of up to 32 code units, so the step counts of today's tests and test262 runs with such
   Strings; the lookups of `KeyedList`; determinism.
8. **Out of scope, named.** Transient copies that `string_units` makes in the builtins
   (`story:string-builtins-charge-before-they-copy`); the bytes of a transient copy (the memory
   rules of the budget batch).

Design preconditions (the design author writes them before the unit starts):
`docs/design/engine.md:719-722` ("The step budget"), after "per code unit for string
operations": "(a comparison of two Strings and a String made a property key: per code unit past
32, charged before the units are read)". And the lines of HOST-181 and HOST-182 below, with their
pending lines in `conformance/host/requirements.txt`.

New requirements:

- **HOST-181** (swell:D2) With `a` and `b` two Strings of 1,000,000 code units that differ only in
  the last, defined as globals by the host, the script `a === b` spends at least 999,968 steps
  more than the script `"x" === "y"`, and with `steps_per_entry` 10,000 it answers
  `Terminated(StepBudget)`; the same holds for `a < b` (1,000,000 steps), `a == b` and
  `[a].indexOf(b)`. [slice 6]
- **HOST-182** (swell:D2) With a global object `o` and a String `k` of 1,000,000 code units
  defined by the host, and a `collect()` before the entry, the script `o[k]` spends at least
  999,968 steps more than the same script with a key of one code unit, and with
  `steps_per_entry` 10,000 it answers `Terminated(StepBudget)` with `heap_bytes()` after the
  entry equal to `heap_bytes()` before it (no atom made). [slice 6]

Writes: `crates/swell/src/ops/testing.rs` (`string_equals`, IsLessThan),
`crates/swell/src/ops/conversion.rs` (`key_from_string`, `key_from_units`, `string_to_number`),
`crates/swell/src/heap/string.rs` (the comparisons of `HeapString`, `intern` split into
`atom_of` and the insertion, and their unit tests), `crates/swell/src/object/string.rs` (the
test-only count, under `#[cfg(test)]`), `crates/swell/src/builtins/array/mutation.rs`
(`compare_as_strings` and `sort_string_units`, lines 776-806, nothing else),
`crates/swell/tests/conversion.rs`, `crates/swell/tests/property_index.rs`,
`crates/swell/tests/probes/strings/{compare.js,run.sh}` (new), and in
`conformance/host/requirements.txt` the lines of HOST-181 and HOST-182 only. Not written:
`crates/swell/src/interp/dispatch.rs` (its callers keep their calls),
`crates/swell/src/object/property.rs`.

## Acceptance

In `crates/swell/tests/conversion.rs`, both collection modes:

- `host_181_comparing_long_strings_is_charged`: as HOST-181; today `a === b` spends the steps of
  `"x" === "y"` and the budget of 10,000 is not reached.
- `comparisons_of_short_strings_spend_what_they_spend_today` (a guard): 40 pairs of Strings of 0
  to 32 code units, Latin-1 and UTF-16 mixed, under `===`, `==`, `<`, `switch`, `Object.is` and
  `sort`: `last_entry_steps()` equal to the counts the test records from the base and states as
  constants.
- `equality_answers_are_unchanged` (a guard): a generator seeded in the test makes 2,000 String
  pairs (equal, prefix, one unit apart, Latin-1 against UTF-16 of the same text): every answer of
  `===` and `<` equals a comparison of the two `to_utf16` vectors in the test.

In `crates/swell/tests/property_index.rs`, both collection modes:
`host_182_a_long_key_is_charged_before_it_is_interned` (as HOST-182; today it answers `Normal`
within 10,000 steps), and the property-index tests of the file pass unchanged.

In the lib test binary, reading the test-only count of point 6:

- `crates/swell/src/ops/testing.rs`: `is_less_than_copies_nothing`: `"a" < s` with `s` of
  1,000,000 units answers `true` and the count reads 0 (today 1,000,000);
  `string_to_number_is_charged_before_its_copy`: `+s` of 1,000,000 units with 1,000 steps left
  ends in `Terminated(StepBudget)` and the count reads 0 (today 1,000,000).
- `crates/swell/src/builtins/array/mutation.rs`: `the_default_sort_compares_in_place`: `[s,
  t].sort()` with `s` and `t` of 1,000,000 units that differ in the last, 1,000 steps left, ends
  in `Terminated(StepBudget)` and the count reads 0 (today 1,000,000).
- `crates/swell/src/heap/string.rs`: `equality_in_place_matches_to_utf16_for_every_pair_of_storage_kinds`
  and `order_in_place_matches_to_utf16_for_every_pair_of_storage_kinds`: for each of the four
  pairs of storage kinds (Latin-1 or UTF-16 on each side), 500 pairs from a generator seeded in
  the test (equal, a prefix, a difference in the first, a middle or the last unit, a unit above
  0xFF on the UTF-16 side): the answer equals `==`, respectively `cmp`, of the two `to_utf16`
  vectors; `atom_of_makes_nothing`: `atom_of` of 1,000 unit slices no atom has answers none and
  leaves `used()` and the table's length as they were, and of an interned one answers its atom.
  Today none compiles.

Each test runs under 5 s in both collection modes (the Strings of 1,000,000 units are made by the
host with `new_string_utf16` once per test, not by a script loop).

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `conversion`, `property_index`,
`builtins_array_mutation` and `builtins_array_search`, the same with `--features
collect-every-safe-point`, `slot cargo nextest run --locked -p swell --profile default --lib` in
both modes, `slot cargo clippy --workspace --all-targets --locked -- -D warnings`; at the end,
once, `slot timeout 1800 task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared with that
of `slot timeout 1800 task test262 TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{ print $3,
$4, $5 }'` over each, and `cmp` of the two outputs exits 0 (a case that ends at a budget it did
not reach before is reported with its path); and last, once, in the background, `slot timeout 1800 task
host-requirements-check` (it builds every test target of the workspace). Never `cargo test -p
swell` whole, `slot task test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except those named as guards. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest, so
nothing is shared through a `static` between tests (the copy count is thread-local and reset by
each test). Each new test is timed, and one over 5 s is named in the report with its time. A job
of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same script under the same budgets ends
the same way at the same count on every run. Nothing is added to the frame of
`Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No third-party
crate, no unsafe code.

## Cost

No step more for a String or key of up to 32 code units; a copy fewer for every comparison and
every key lookup that hits. CPU, in one pool slot, the smallest of three runs: the probes
`index-write-read` and `named-write-read` of `crates/swell/tests/probes/property-index/` at most
1.05 times the base; a new probe `crates/swell/tests/probes/strings/compare.js` (`var a = "abc"
+ @N@, b = "abc" + @N@, n = 0; for (var i = 0; i < @N@; i++) if (a === b && !(a < b)) n++; throw
new Test262Error('n=' + n);`, run by a `run.sh` in the form of the property-index one) at N =
100,000 at most 1.0 times the base (a copy fewer per comparison).

## Report

The new tests with their times, the probe numbers, the test262 comparison, this line for the
changelog's "Host interface" heading: "`HOST-181`, `HOST-182` (new): comparing long Strings and
using a long String as a property key are charged per code unit."; and two lines for its
"Changed" heading: "Comparing two Strings, and
using a String of more than 32 code units as a property key, now charge a step per code unit past
32 before the work; shorter ones cost what they did." and "The property scan named among 0.2.0's
limits is gone: since this release a property is found through an index map once its object has
more than 8 properties, in steps that do not grow with the object."
