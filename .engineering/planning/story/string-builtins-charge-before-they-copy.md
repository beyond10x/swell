---
format: aep.planning-md/3
id: story:string-builtins-charge-before-they-copy
kind: story
status: implemented
title: A builtin charges the steps for a String's code units before it copies them, copies no more units than it uses, and the replacer list of JSON.stringify compares in place
summary: Builtins that copy a whole String their result does not use, or copy before they charge, read only what they use and charge first; the replacer list of JSON.stringify keeps no copies and compares its items with the charged in-place equality; QuoteJSONString reads its String in bounded ranges; the text of JSON.parse is charged before its copy. Step totals stay, except the replacer list's and a JSON text's that fails before its end, which are the stated changes.
relations:
- depends_on: story:string-comparison-and-property-keys-charge-per-code-unit
- depends_on: story:own-keys-charged-before-the-list-exists
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T11:02:18Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T11:02:18Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:52Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: `story:string-comparison-and-property-keys-charge-per-code-unit` (the test-only copy
count in `crates/swell/src/object/string.rs`, which this story writes after it, and the charged
in-place `string_equals` the replacer list calls), the budget batch's
`story:own-keys-charged-before-the-list-exists` (it writes `crates/swell/src/builtins/json/mod.rs`
at lines 542 and 950; this story writes other lines of it) and the budget batch's
`story:arena-capacity-and-collector-workspace-on-the-ledger` (it edits the budgets of the
`#[cfg(test)]` module of `crates/swell/src/builtins/string/mod.rs`, line 467, where this story adds
tests). Batch H3. No requirement of `host-interface.md`. Consumer: the browser, requirement 2 of
its list (the remainder the budget batch does not cover). Decision D2 (taken: option A).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`. `string_units` copies a String's code units
into a new `Vec<u16>` (`crates/swell/src/object/string.rs:48-50`); `string_length` reads the
length without a copy (`:53-58`); `append_units` checks `string_length`, charges a step per unit,
checks the bytes, then copies (`crates/swell/src/builtins/string/mod.rs:183-203`, the charge at
`:195`); `read_units` charges and then copies a range (`crates/swell/src/builtins/string/access.rs:183-201`).

The replacer list of `JSON.stringify` (`json_property_list`,
`crates/swell/src/builtins/json/mod.rs:229-277`): each item is copied whole with no step (`:253`)
and compared, as a `Vec<u16>`, with every item already listed for one step each (`:255-261`, the
step at `:256`); n items of L code units cost about n²·L work for n² steps. The copies are used
only for that comparison: the list is read back by its `StringRef`s (`:590-593`).

Copies of a whole String the result does not use, with no step for them:

- `String.prototype.repeat` (`crates/swell/src/builtins/string/transform.rs:234`): S is copied
  (`:250`) before the count is looked at, so `s.repeat(0)` copies all of `s`.
- `padStart` and `padEnd` (`:288`): the whole fill String is copied (`:307`) and only the units
  appended are charged; `string_pad` copies S (`:324`) before the appends charge it.
- `Array.prototype.join` (`crates/swell/src/builtins/array/mod.rs:277-291`): the separator is
  copied (`:285`) whatever the length, also below 2, where it is not used.
- `JSON.stringify`: a String `space` is copied whole and then cut to 10 units (`json/mod.rs:319-323`,
  the copy at `:321`).
- `append_string` (`string/mod.rs:211-218`) copies the String (`:216`) before `append_units`
  checks the limit and charges: the copy is made also when the result would pass
  `string_length`. It serves `concat`, `String.raw`, `join`, `toLocaleString`, the name of
  `bind` and the tag of `Object.prototype.toString`.
- QuoteJSONString: a String value is copied whole (`json/mod.rs:427`) into
  `Serialized::Quoted` (`:109`) and a member's key likewise (`json_key_units`, `:352-361`, called
  at `:625`); `json_quote` (`:465-510`) then charges as it appends, 256 units at a time
  (`QUOTE_CHUNK`, `:54`, the append at `:503-506`). The text of a raw JSON object is copied whole
  (`json_raw_text`, `:445-452`, the copy at `:448`) and charged when it is appended (`:458`).

Copies charged after they are made, with the same total as a charge before: `trim`, `trimStart`,
`trimEnd` (`trim_string`, `transform.rs:357`, the copy at `:361`, the charge at `:362`), the case
methods (`string_case_method`, `:397`, `:401-402`), `localeCompare` (`:419`, the copies at
`:428-429`, the charge at `:430`), and `Error.prototype.toString` (`error_to_string_rest`,
`crates/swell/src/builtins/error.rs:219-244`: two copies and the joined result `:235-237` before
the `string_length` check `:239-241` and the charge `:242`).

The second search, for the shape of the replacer loop (a list of copied Strings, each new one
compared with every listed one) over every caller of `string_units` (41 lines, 11 of them in
tests): `json/mod.rs:253-261` is the only one. The other callers outside tests: the comparisons
and keys of `story:string-comparison-and-property-keys-charge-per-code-unit`
(`crates/swell/src/ops/testing.rs:47`, `:154`, `crates/swell/src/ops/conversion.rs:178`, `:337`,
`crates/swell/src/builtins/array/mutation.rs:780-803`); the sites of this story; string
concatenation, which charges before it copies (`crates/swell/src/ops/operators.rs:133-135`); the
URI functions (`crates/swell/src/builtins/global.rs:250`), which charge each unit in the loop
that reads it; CanonicalNumericIndexString (`conversion.rs:377-391`), dead code until typed
arrays (slice 13).

The text of `JSON.parse` and `JSON.rawJSON` (`json_text_units`,
`crates/swell/src/builtins/json/parse.rs:252-261`, called at `:204` and `:1019`) is charged to the
memory budget and copied whole, with no step. The parser then charges one step per code unit it
reads (`Parser::advance`, `:86-92`) and one per value (`:486`), batched: the unpaid steps are
charged once they reach `STEP_CHUNK` = 4,096 (`:41`, `json_pay_chunk`, `:279-284`) and the rest at
the end (`:318`). The 4,096 is a batch size, not a rate. A parse that succeeds has read every unit
(it fails unless the position reaches the text's length, `:315-317`; `JSON.rawJSON` parses with
the same `json_parse_text`, `:1044`), so its reading steps are exactly the text's length. A text
that fails at its first unit has paid about one step for a copy of all of it.

## Change

Failing tests first (Acceptance). Then:

1. **Two readers** beside `string_units` in `crates/swell/src/object/string.rs`:
   `charged_units(string) -> Completion<Vec<u16>>`, a step per code unit through
   `Agent::charge_steps`, then the copy; and `string_units_in(string, from, to) ->
   Completion<Vec<u16>>`, the units of a range with no step, for a caller that charges them as it
   appends. Both record their length in the test-only copy count of
   `story:string-comparison-and-property-keys-charge-per-code-unit`.
2. **The replacer list** holds `StringRef`s only (`State::property_list`, `:93`, read at
   `:590-593`); an item is compared with each listed one by `string_equals`, which charges past
   32 code units before it reads them (the decision D2), with the step per comparison as today.
   The memory charge of an entry loses the copy's bytes. Its step totals change: the comparison
   of two items of the same length n above 32 now costs n − 32 steps more. This is the one step
   total of the story that changes, stated.
3. **Charge before the copy.** `trim_string`, `string_case_method` and `string_locale_compare`
   call `charged_units` in place of their copy and their charge; `error_to_string_rest` checks
   the result's length against `string_length` from `string_length` of its parts, and charges
   it, before it copies either part; `append_string` checks `string_length` and charges before it
   copies, and `append_units` keeps its order for its other callers.
4. **Copy only what is used.** `repeat` answers a count of 0 or an empty S with no copy, and
   makes its first copy by `append_string`; `padStart` and `padEnd` read at most the units of the
   fill String they append, and `string_pad` appends S by `append_string`; `join` copies the
   separator only when the length is 2 or more; a String `space` is read by `string_units_in` to
   at most 10 units.
5. **QuoteJSONString reads ranges.** `Serialized::Quoted` holds the `StringRef`, kept while it
   is read; `json_quote` reads it by `string_units_in` in ranges of `QUOTE_CHUNK` units (a range
   that would end between the two units of a surrogate pair ends before the first), and appends
   as today. A member's key goes the same way. The text of a raw JSON object is appended by
   `append_string`.
6. **The text of `JSON.parse` and `JSON.rawJSON`.** `json_text_units` charges one step per code
   unit of the text, the rate the parser charges for reading it today, before the copy (after
   the memory charge, as today). `Parser::advance` moves the position and adds nothing to
   `unpaid`; the step per value (`:486`) and its batches stay. A parse that succeeds costs the
   same total as today. One that fails, or a `JSON.rawJSON` text refused by its first or last unit
   (`:1029-1034`), has paid for the whole text before it is copied. The text is not parsed in
   place.
7. **Stays as it is.** Every result; every step total outside points 2 and 6 (a charge moves
   earlier, never grows or shrinks; point 6 changes only the total of a text that fails before
   its end); every `RangeError` and `SyntaxError` and its message; determinism. Not changed,
   named: the URI functions and CanonicalNumericIndexString (Context).

Design: none changes; `docs/design/engine.md:719-722` already says "per code unit for string
operations".

Writes: `crates/swell/src/object/string.rs` (`charged_units`, `string_units_in`),
`crates/swell/src/builtins/string/transform.rs` (`string_repeat`, `string_padding_builtins_impl`,
`string_pad`, `trim_string`, `string_case_method`, `string_locale_compare`, lines 234-432, and a
new `#[cfg(test)]` module), `crates/swell/src/builtins/string/mod.rs` (`append_string`, lines
211-218, and tests in its `#[cfg(test)]` module), `crates/swell/src/builtins/array/mod.rs`
(`array_join_steps`, lines 277-291, and tests in its `#[cfg(test)]` module, line 496),
`crates/swell/src/builtins/json/mod.rs` (`State::property_list` at 93, `Serialized` at 106-111,
`json_property_list` at 229-277, the gap at 319-323, `json_key_units` at 352-361, 427, 445-459,
`json_quote` at 465-510, 590-593, 625, and a new `#[cfg(test)]` module; not lines 542 and 950),
`crates/swell/src/builtins/error.rs` (`error_to_string_rest`, lines 219-244, and a new
`#[cfg(test)]` module), `crates/swell/src/builtins/json/parse.rs` (`Parser::advance` at 86-92,
`json_text_units` at 252-261, and tests in its `#[cfg(test)]` module, line 1083),
`crates/swell/tests/builtins_json.rs`, `crates/swell/tests/builtins_string.rs`.
Not written: `crates/swell/src/builtins/global.rs`, `crates/swell/src/ops/conversion.rs`.

## Acceptance

Unit tests in the lib test binary of `swell`, each with Strings of 1,000,000 code units made by
the host with `new_string_utf16`, reading the test-only copy count (today's value in brackets):

- `crates/swell/src/builtins/string/transform.rs`: `repeat_zero_copies_nothing` (`s.repeat(0)`:
  0 [1,000,000]); `repeat_is_charged_before_its_copy` (`s.repeat(1)` with 1,000 steps left:
  `Terminated(StepBudget)` and 0 [1,000,000]); `pad_copies_only_the_fill_it_uses`
  (`"x".padStart(3, f)`: at most 2 [1,000,000]); `pad_is_charged_before_it_copies_s`
  (`s.padEnd(1000001)` with 1,000 steps left: `Terminated(StepBudget)` and 0 [1,000,000]);
  `trim_methods_are_charged_before_their_copy` (`trim`, `trimStart` and `trimEnd`, each in a
  fresh agent, with 1,000 steps left: `Terminated(StepBudget)` and 0 [1,000,000]);
  `case_methods_are_charged_before_their_copy` (`toLowerCase`, `toUpperCase`,
  `toLocaleLowerCase`, `toLocaleUpperCase` of an ASCII `s`, the same); and
  `locale_compare_is_charged_before_its_copies` (`s.localeCompare("a")`, the same).
- `crates/swell/src/builtins/array/mod.rs`: `join_of_one_element_copies_no_separator`
  (`[1].join(f)`: 0 [1,000,000]).
- `crates/swell/src/builtins/string/mod.rs`: `concat_past_string_length_copies_nothing`
  (`string_length` 1,000,000; `"a".concat(s)` throws a `RangeError`, 0 [1,000,000]).
- `crates/swell/src/builtins/json/mod.rs`: `a_string_gap_copies_ten_units`
  (`JSON.stringify([1], null, s)`: at most 10 [1,000,000]); `quoting_reads_a_string_in_ranges`
  (`JSON.stringify(s)` with 1,000 steps left: `Terminated(StepBudget)` and at most 256
  [1,000,000]); `quoting_a_key_reads_it_in_ranges` (`JSON.stringify(o)`, `o` with one property
  keyed by `s`, the same); `a_raw_text_is_charged_before_its_copy` (`JSON.stringify(r)`, `r`
  made by `JSON.rawJSON` from a JSON String literal of 1,000,000 units in an earlier entry, with
  1,000 steps left: `Terminated(StepBudget)` and 0 [1,000,000]).
- `crates/swell/src/builtins/error.rs`: `error_to_string_past_string_length_copies_nothing`
  (`string_length` 1,000,000; an Error whose `name` is `s` and `message` is `"m"`:
  `e.toString()` throws a `RangeError` and the count reads 0 [1,000,000]).
- `crates/swell/src/builtins/json/parse.rs`: `a_text_invalid_at_its_first_unit_pays_for_its_copy`
  (`steps_per_entry` 100,000; `JSON.parse(t)` with `t` the host-made String `"}"` followed by
  999,999 `"0"` units: `Terminated(StepBudget)` and the count reads 0; today it throws a
  `SyntaxError` within 100,000 steps and the count reads 1,000,000);
  `a_raw_json_text_refused_by_its_first_unit_pays_for_its_copy` (`JSON.rawJSON(t)`, the same).

In `crates/swell/tests/builtins_json.rs`, both collection modes:

- `a_replacer_list_of_long_strings_is_charged_per_code_unit`: with `steps_per_entry`
  10,000,000, a first entry makes an array of 200 Strings of 10,000 code units that differ in
  their last unit; the entry `JSON.stringify({}, list)` answers `Terminated(StepBudget)` (today
  it answers `Normal` within about 20,100 steps).
- `a_replacer_list_of_short_strings_spends_what_it_spends_today` (a guard): 50 items of 1 to 32
  units, with repeats: `last_entry_steps()` equals the count the test records from the base and
  states as a constant, and the answer is the same.

In `crates/swell/tests/builtins_string.rs`, both collection modes:
`the_step_totals_are_unchanged` (a guard): each method of points 3 to 5 on Strings of 0, 1, 31,
32, 33 and 1,000 units: `last_entry_steps()` and the answer equal the constants the test records
from the base. Its `JSON.parse` case: valid texts of 1, 4,095, 4,096, 4,097 and 20,000 units
(nested arrays, objects, Strings with escapes, numbers, white space), each with and without a
reviver, and `JSON.rawJSON` of a String literal and a number: `last_entry_steps()` equals the
base's constant (the reading steps move before the copy; the total stays).

Each test runs under 5 s in both collection modes; the replacer test today spends about 2 × 10⁸
unit comparisons, measured on the base before the change and named in the report with its time.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --lib`, `slot cargo nextest run --locked -p swell --profile default --test
<b>` for each `<b>` of `builtins_string`, `builtins_string_transform`, `builtins_string_access`,
`builtins_array`, `builtins_json`, `builtins_json_parse` and `errors`, the same with `--features
collect-every-safe-point`, `slot cargo clippy --workspace --all-targets --locked -- -D
warnings`; at the end, once, `slot timeout 1800 task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv`
is compared with that of `slot timeout 1800 task test262 TEST262_OUT=.out/test262-base` run on the base:
`awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0 (a case
that ends at a budget it did not reach before is reported with its path); and last, once, in the
background, `slot timeout 1800 task host-requirements-check` (it builds every test target of the workspace).
Never `cargo test -p swell` whole, `slot task test-collect`, `slot task test` or `slot task
check`.

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

No step more outside the replacer list; a copy fewer at every site above. CPU, in one pool slot,
the smallest of three runs: the probe `crates/swell/tests/probes/strings/compare.js` of
`story:string-comparison-and-property-keys-charge-per-code-unit` and the property-index probes
at most 1.05 times the base.

## Report

The new tests with their times, the replacer test's time on the base, the probe numbers, the
test262 comparison, and this line for the changelog's "Changed" heading (no name of the host
interface changes): "Builtins no longer copy a whole String they do not use, charge a String's
steps before they copy it, and `JSON.stringify` compares the Strings of a replacer list in place,
a step per code unit past 32. `JSON.parse` and `JSON.rawJSON` charge the step per code unit of
their text before they copy it: a text parsed to its end costs what it did, and one that fails
early has paid for its whole text."
