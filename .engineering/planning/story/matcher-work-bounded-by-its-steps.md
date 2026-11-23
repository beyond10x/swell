---
format: aep.planning-md/3
id: story:matcher-work-bounded-by-its-steps
kind: story
status: active
title: The work of the matcher is bounded by the steps it charges
summary: A try of the search wrote every capture slot of the pattern for one step, and the clearing of a quantified atom's captures and a named backreference scanned every group of their range or name for one step; a try now writes one slot, since a failed try has undone its changes entry by entry, and the clearing and the named backreference charge one step per group before they read a slot.
relations:
- decomposes: epic:slice-05-regexp
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T04:45:17Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T04:45:17Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---

## Context

The step budget bounds the work of one entry by a number, so a step stands for a bounded
amount of work (`docs/design/engine.md`, decision 3, "The step budget"; decision 7, "What one
step of the matcher is"). The number of groups of a pattern is bounded only by its length,
which the caller charges once, one step per code unit, before the parse.

Rule: every loop of the matcher and of the compile step does a bounded amount of work per step
charged (matcher), per code unit of the pattern charged by the caller (parser and compile
step; the sort of a class's ranges and of the group names aside) or per byte charged. No
number of the pattern (groups, counters, lookarounds, ranges, names) multiplies the work of a
step.

Three places of `crates/regexp/src/matcher.rs` broke it on `2f54796`:

1. `attempt`, the try at one start index, charged one step and wrote all 2 × (groups + 1)
   capture slots (`self.slots.fill(UNSET)`). One search with 20,000 groups over 200,000 start
   indices that fail at their first instruction: 400,002 steps, 1.615 CPU seconds (release
   build).
2. `Inst::Clear`, at the head of each iteration of a quantified atom that holds groups, scanned
   the two slots of every group of its range for its one step. One match of an atom with
   20,000 groups in an alternative never taken, repeated 200,000 times: 1,200,010 steps, 7.228
   CPU seconds (release build).
3. `Inst::NamedBackref` searched the groups of its name for the one that is set, for its one
   step; a name n groups have (duplicate named groups in different alternatives) is n reads
   of two slots.

## Change

1. A try writes one slot, the start of group 0. The first try finds every slot unset as the
   search's preparation made them (charged by their bytes), and a try that fails has taken
   every entry of its backtrack stack, one step each, and so restored every slot and counter it
   changed. The one way a try fails without the stack emptied, an instruction index past the
   program, takes the entries left (`unwind`), their steps charged first.
2. `Inst::Clear { first, last }` charges one step per group of its range, its instruction's
   step the first of them (`last - first` more), before it reads a slot; it finds the set slots
   of its range with one pass over the slice.
3. `Inst::NamedBackref` charges one step per group of its name, its instruction's step the
   first, before the search.
4. `search_slot_visits` (`#[doc(hidden)]`, for the tests of the cost model): the search of
   `search`, answering the count of capture slots it read in a scan or wrote (the first fill,
   charged by bytes, not counted).
5. The paragraph "What one step of the matcher is" of the design states what a try, the
   clearing of a quantified atom's captures, a named backreference and the test of a class
   cost, with the example `/(?:a|(b)(c))*/.exec("a")`: 20 steps, 18 before.

Every other loop of `matcher.rs`, `compile.rs` and `program.rs` was read and is bounded by a
step, a code unit or a byte charged: `prepare` sizes the captures, counters and registers once
per call of `search` or `match_at`, each element charged as bytes; the exit of a lookaround
charges one step per entry it takes or keeps; the test of a class is a binary search over
sorted, disjoint ranges that do not meet, at most 557,056 of them and so at most 20 probes.

Measured after (release build): the try probe 400,002 steps, 0.007 CPU seconds; the clearing
probe 4,001,020,009 steps, 6.305 CPU seconds.

Writes: crates/regexp/src/matcher.rs, crates/regexp/src/lib.rs, crates/regexp/tests/budgets.rs,
docs/design/engine.md

## Acceptance

- Tests in `crates/regexp/tests/budgets.rs`, each red on `2f54796`:
  - `a_try_does_no_work_per_group_it_does_not_charge`: `x(a)` and `x` with 2,000 groups over
    1,000 code units that fail at once spend 2,002 steps and visit 1,001 slots alike (5,005 for
    one group on the base); four patterns whose groups take part in a failed try find them
    unset in the matching one, with at most 4 slot visits per step;
  - `the_clearing_of_a_quantified_atom_is_charged_per_group_of_its_range`: `(?:a|x(y)...)*` over
    1,000 code units spends 1,999 × 1,001 steps more with 2,000 groups than with one (0 on the
    base); `/(?:a|(b)(c))*/` in "a" spends 20; a step budget that ends inside a clearing stops
    the match with exactly its steps spent;
  - `a_named_backreference_is_charged_per_group_of_its_name`: `\k<n>` of a name n groups have,
    none set, costs n steps (1 on the base).
- `cargo nextest run --locked -p swell-regexp --test main`: 43 tests pass (40 before).
- Each new test under 0.5 CPU seconds, its binary run directly.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`
  and `sh taskfiles/test-main.sh check` exit 0.
- Totals that moved: none recorded by a test; a pattern whose quantified atom holds k groups
  spends k - 1 steps more per iteration, and `\k<name>` of a name n groups have n - 1 more.

## Out of scope

- The parser (`crates/regexp/src/parser.rs`) and `early.rs`, whose loops this change did not
  read.
- A clearing that visits only the set slots of its range without a scan: it needs a record of
  the set slots per range that a nested quantifier does not multiply, larger than this change.
- `Captures::iter` and `Program::group_names`, iterated by the caller that builds the match
  array and charges it.
