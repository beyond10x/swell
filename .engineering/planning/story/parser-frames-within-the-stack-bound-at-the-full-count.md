---
format: aep.planning-md/3
id: story:parser-frames-within-the-stack-bound-at-the-full-count
kind: story
status: active
title: Every nesting kind of the parser fits the stack bound at the full count, and the gate's check fails wherever the full count does
summary: Nested arrow functions took 229.3 bytes a parser level against the stated 224, and the stack overflowed at parser_nesting 10,000; the gate's check at 2,500 passed on it. The arrow body's frame drops to 224 bytes (181.3 a level), and the gate's check at 2,500 runs on a quarter of the stack of 10,000 levels.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T18:18:03Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T18:18:03Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
A repair of the lane `slow` (step `slow-tests`, red on `main` at `3cec594`). Comes before every
feature. No host requirement changes; no constant a host reads changes.

## Context

- `ParseOptions::stack_bytes` (`crates/syntax/src/lib.rs`) is `STACK_BYTES_FIXED` (32,768) plus
  `max_nesting` × `STACK_BYTES_PER_LEVEL` (224); it is the parser's part of
  `Limits::min_stack_bytes()` (`crates/swell/src/host/limits.rs`). A host that gives the parser
  that stack must get a `SyntaxError`, never a stack overflow, at any `parser_nesting`.
- On `3cec594` the lane `slow` failed in `slow-tests` (`4 test binaries: 454 passed, 3 failed,
  321 ignored`): `sources::every_nesting_kind_fits_the_stack_bound_part_10`, `_11` and `_12`
  aborted `with parser_nesting 10000 on a stack of 2272768 bytes: signal: 6 (SIGABRT)`.
- The combinations of those parts that overflow are the three of the kind `arrow`
  (`x = () => () => ... 0;`), one per entry; every other combination of the parts fits at 10,000.
  The smallest limit at which each overflows the stated stack: 4,094 through `parse_script` and
  `parse_eval`, 3,464 through the body of `parse_dynamic_function`.
- Bytes a level, measured as the smallest thread stack that holds the deepest admitted source at
  limits 2,000 and 10,000 (the difference over 8,000 levels), in the test build (dev without debug
  information): `arrow` 229.3, every other kind at most 185.2 (`arrow_block`). One arrow nesting
  is three levels, `assignment` (160 bytes), `arrow_function` (160) and `arrow_body` (368): 688
  bytes, 5.3 a level over the bound. The fixed part through `parse_script` took 11,204 bytes.
- `git bisect` over `6a43e18..3cec594`, restricted to the commits that touch `crates/syntax/src`,
  with the red command as the test: first bad commit `b4c46f6` ("feat(lang): arrow functions"),
  which added `arrow_body` with a frame of 368 bytes and the kinds `arrow` and `arrow_block` to
  the test; `84522ad` (its parent among those commits) and `44eb3f4` pass. No later commit
  changed the three frames.
- The gate ran the check at limit 2,500 (`deep_limit`) on the stated stack and passed: 5.3 bytes
  over on 2,500 levels is 13,300 bytes, within the 21,564 bytes the fixed part leaves of
  `STACK_BYTES_FIXED`. The comment above `deep_limit` argued that a growth past the bound is at
  least 16 bytes a level; a frame grows by 16 bytes, but a nesting takes several levels (3 for
  an arrow, up to 14), so the growth a level is 16 / k.

## Change

1. **The parser.** `Parser::concise_body` (`crates/syntax/src/parser/function.rs`), not
   inlined, takes the arrow function's id and the outer strictness and records the body itself
   (`function_end`), so the list of the `ExpressionBody` no longer lives in the frame of
   `arrow_body`, which is on the recursion of nested arrows. `arrow_body`'s frame drops from 368
   to 224 bytes; an arrow nesting from 688 to 544 bytes, 181.3 a level. `STACK_BYTES_PER_LEVEL`,
   `STACK_BYTES_FIXED`, `min_stack_bytes` and `STACK_BYTES_PER_REENTRY` do not change. The doc
   comment of `STACK_BYTES_PER_LEVEL` states the new measurement.
2. **The gate's check finds what the full count finds.** In `crates/syntax/tests/sources.rs` the
   child of the check at the deep limit runs on `deep_stack(limit)`: `stack_bytes(limit)` less
   `STACK_BYTES_FIXED` × (10,000 − limit) / 10,000, the stack of 10,000 levels scaled by
   limit / 10,000. At 2,500 that is 592,768 − 24,576 = 568,192 bytes, a quarter of 2,272,768.
   With a fixed part of f bytes and p bytes a level the check at 10,000 overflows when
   f + 10,000 p > S, and then f + 2,500 p ≥ (f + 10,000 p) / 4 > S / 4: the gate's check
   overflows whenever the full count's does, whatever f. It is stricter by 3f/4 over 2,500
   levels (3.4 bytes a level through `parse_script`): it also fails a kind up to 1.2 bytes a level
   under the bound. The full count keeps 10,000 on the stated stack. The comment above
   `deep_limit` states this with the measurements; the argument from 16-byte steps is gone.
3. **The search's cost.** The search for the deepest admitted source bisects with
   `admitted_by_the_parser`: the source with an unexpected `)` after it, which the parser reaches
   (all nodes made, their depths checked) without the early errors and the scope analysis. Only
   the deepest admitted source goes through them; through the body of `parse_dynamic_function`,
   whose whole text holds the body a few levels deeper, the search steps down until a source
   parses. At 2,500 the 17 parts take 6.7 CPU seconds together (7.6 before), at 10,000 about 26.

Option A of the brief (the gate runs 10,000) was measured and not taken: 25.7 to 26.0 CPU
seconds together, under 30, but in three runs of cargo-nextest the slowest part took 6.8, 11.6
and 28.8 s at loads of 74 to 101 on 20 cores, over 15 s in one of three.

Writes: `crates/syntax/src/parser/function.rs`, `crates/syntax/src/lib.rs` (doc comment),
`crates/syntax/tests/sources.rs`, `CHANGELOG.md` (one line under `## [Unreleased]`,
`### Fixed`).

## Acceptance

1. Full count green: `SWELL_FULL_COUNTS=1 cargo nextest run --locked -p swell-syntax --profile
   default --test main -E 'test(/^sources::every_nesting_kind_fits_the_stack_bound/)'` prints
   `17 tests run: 17 passed, 23 skipped`; on `3cec594` it printed `17 tests run: 14 passed, 3
   failed`.
2. Gate count green: the same without `SWELL_FULL_COUNTS` prints `17 tests run: 17 passed, 23
   skipped`.
3. The gate's run is red on a planted defect, without `SWELL_FULL_COUNTS`:
   - the change of `function.rs` reverted: `17 tests run: 14 passed, 3 failed` (parts 10, 11,
     12), each `with parser_nesting 2500 on a stack of 568192 bytes: signal: 6 (SIGABRT)`; the
     full count fails the same three parts;
   - a 160-byte local in `conditional_rest` (frame 160 to 320 bytes, conditional expressions 240
     bytes a level, 16 over): `17 tests run: 11 passed, 6 failed`, the same six parts as at the
     full count;
   - a 16-byte and, in a second run, a 64-byte local in `arrow_body` and in `conditional_rest`
     (arrows 186.7 and 202.7 bytes a level, conditionals 168 and 192, all under the bound):
     `17 passed` at both counts.
4. The bytes a level of every kind are at most 181.3 in the test build, 213.3 in the profile
   `stack-debug` and 149.3 in the release profile (the probe of Context, three profiles).
5. The tests of `swell-syntax`, `swell-compiler`, `swell` (both collection modes) and
   `swell-test262` pass; `cargo clippy --workspace --all-targets --locked -- -D warnings` and
   `cargo fmt --all --check` exit 0; `task slow-tests` and `task test262` (0 lines that differ
   from `conformance/test262/expected.txt`) pass.

## Out of scope

- The resolution of the check: at 10,000 levels the full count finds a kind more than about 2.2
  bytes a level over the bound, not less (the room of the fixed part over 10,000 levels). A
  check of the bytes a level itself (the probe of Context as a test) is a story of its own.
- The other tests with a gate count (`SWELL_FULL_COUNTS`): listed in the unit's report, none
  changed here.
- The early errors of nested labels, whose cost grows with the square of the depth: the search
  now pays it once a combination; the analysis itself is unchanged.
- The constants `STACK_BYTES_PER_LEVEL`, `STACK_BYTES_FIXED`, `STACK_BYTES_PER_REENTRY`.
