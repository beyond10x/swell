---
format: aep.planning-md/3
id: story:pattern-parser-work-linear-in-its-pattern
kind: story
status: implemented
title: The work of the pattern parser is linear in the pattern it is charged for
summary: Every loop of ParsePattern and its early errors was read and none breaks the rule; a test now holds the parse linear over 19 pattern families at two sizes through the polls and charges of a counting budget and a doc-hidden count of the visits of the group-name check, and the design names the one bound that is not linear, the union-find of the duplicate-name check, at most n log n.
relations:
- decomposes: epic:slice-05-regexp
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T05:07:46Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T05:07:46Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-23T08:12:42Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---

## Context

A pattern parsed at run time costs one step per code unit of the pattern, charged by
RegExpInitialize before ParsePattern; the parser and the compiler charge no step of their own
(`docs/design/engine.md`, decision 7, "What one step of the matcher is"; decision 3, "The step
budget"). So one step of the caller stands for a bounded amount of the parser's work only when
that work is linear in the pattern's code units.

Rule: every loop of `crates/regexp/src/parser.rs`, `early.rs`, `ast.rs`, `flags.rs` and
`error.rs` does a bounded amount of work per code unit of the pattern, summed over one parse,
the named exceptions aside: the sort of a class's ranges (in the compile step), the group-name
map (n log n comparisons for n names, log n per lookup), and the union-find of the duplicate
check (at most n log n level visits for n levels). The parse runs at most twice per pattern
(without `u`, once more with [NamedCaptureGroups] when the first found a group name), never once
per `\k`.

Read on `05f4af3`, every loop is bounded so:

- `count_groups`: one pass over the pattern, once per call of ParsePattern.
- `Parser::run`: each iteration reads at least one code unit or answers an error.
- `decimal`: one iteration per digit; a run of digits is scanned at most twice, from the `{`
  before it (by the quantify step and by the main loop at the same `{`) or once from the `\`.
- `unicode_escape` (`\u{...}`), `legacy_octal` (at most 3), `hex4` (4), `modifiers` (one per
  letter), `group_name` (one per character, pushed once into the name text), `class` (one per
  class atom; `class_range` at most 3 items): each per code unit read.
- `terms_list`, `alternatives_list`: each node id moves once from its stack into the lists.
- `Meter::grow_by`: the doubling loop runs log(size) times per growth; `reserve_to` doubles, so
  the copies are amortized constant per element.
- `Names::declare`: one map lookup per declaration, the key copied once per new name;
  `Names::find`: the parent chain, compressed at each search; `check_references`: one lookup per
  reference (`has`); `release_map`: one pass over the map.
- `quantifier_in_order`, `significant`: one pass over the digits of a `{n,m}`, once per
  quantifier.
- `Flags::parse`: linear in the flags (8 letters per unit); `error.rs`: constant per error.

## Change

1. `parse_name_visits` (`#[doc(hidden)]`, beside `search_slot_visits`, for the tests of the cost
   model): the parse of `parse_pattern`, the tree released to the budget, answering the count of
   visits of the group-name check: each name declared or looked up and each level the union-find
   walked, summed over the parses that finished. `Names` counts them; `Names::release` adds them
   to the meter.
2. `Names::has` takes `&mut self` for the count; the parser's `name_text` is a free function so
   `check_references` borrows the text and the names apart. No tree, message, offset or order of
   errors changes.
3. The exception list of "What one step of the matcher is" in the design names the duplicate
   check: "the check of a duplicate group name, a union-find over the levels of the parse with
   path compression, at most n log n level visits for n levels". The module text of `early.rs`
   says the same in place of "near linear".

No loop of the parser broke the rule, so no loop is changed. Measured with a release build, one
parse of each family at 100,000 and at 400,000 code units, CPU seconds (the shell's `times`)
of the probe less its build of the pattern, 20 parses per run for the ratio: before the change
every ratio lies between 2.1 and 5.6, after between 3.3 and 4.7; the largest single parse at
400,000 code units took 0.027 s before and 0.031 s after, the build of its pattern included
(machine load 75 to 113 on 20 cores).

Writes: crates/regexp/src/parser.rs, crates/regexp/src/early.rs, crates/regexp/src/budget.rs,
crates/regexp/src/lib.rs, crates/regexp/tests/limits.rs, docs/design/engine.md

## Acceptance

- `the_parse_is_linear_in_the_pattern` in `crates/regexp/tests/limits.rs`: 19 families (a name
  in two alternatives; names declared again past a deep closed nesting; a group and its own
  `\k`; `\k<g>` repeated with and without `u`; a 500-character name referenced repeatedly;
  decimal escapes with groups, past the groups and with `u`; a digit run after `\`; the ranges of
  one class; classes; braced quantifiers; an open `{n,` with a digit run; braces that start no
  quantifier; nesting; nesting of every kind of level; alternatives; lookarounds and modifier
  groups), each at about 12,000 code units and at 4 times that: the polls, the bytes charged and
  the name visits each grow between 3 and 5 times (within 2 for the polls), and a family has name
  visits exactly when it has group names. 0.342 CPU seconds, its binary run directly.
- Green on the base but for `parse_name_visits`, which the base lacks (a compile error). Two
  mutants turn it red: `Names::find` without its path compression ("names declared again past
  a deep closed nesting: name visits 184040 at 11989 code units and 2934368 at 47941"), and a
  count of the groups made again per decimal escape ("groups and decimal escapes, without u:
  polls 5633 at 12000 code units and 90035 at 48000").
- `cargo nextest run --locked -p swell-regexp --test main`: 44 tests pass (43 before).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`
  and `sh taskfiles/test-main.sh check` exit 0.

## Out of scope

- Union by rank in the duplicate check, which would bring its bound from n log n to near linear:
  the search answers the alternative of the nearest open level that holds a level, kept per
  level on the compressed path, and a set's representative cannot be chosen by rank without
  keeping that per member another way.
- `compile.rs`, `matcher.rs` and `program.rs`, read by `story:matcher-work-bounded-by-its-steps`.
- A loop a later change adds that the polls, the charges and the name count do not see: the test
  holds what goes through the meter and the name check.
