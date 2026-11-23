---
format: aep.planning-md/3
id: story:regexp-compiler-and-backtracking-matcher
kind: story
status: draft
title: 'swell-regexp: the compiler and the backtracking matcher with an explicit stack, every step and every growth charged'
summary: CompilePattern and the matcher of 22.2.2 with and without u, in the crate alone; (a+)+$ ends at the step budget; the flag i refused until the case tables; runs beside the literal story.
relations:
- decomposes: epic:slice-05-regexp
- depends_on: story:regexp-crate-and-pattern-parser
scope:
- confidence: cited
  path: crates/regexp/src/compile.rs
- confidence: cited
  path: crates/regexp/src/lib.rs
- confidence: cited
  path: crates/regexp/src/matcher.rs
- confidence: cited
  path: crates/regexp/src/program.rs
- confidence: cited
  path: crates/regexp/tests/budgets.rs
- confidence: cited
  path: crates/regexp/tests/main.rs
- confidence: cited
  path: crates/regexp/tests/matcher.rs
revision: 8
---
## Context

Decision 7 of `docs/design/engine.md`: a pattern is compiled to a small instruction set and run by a backtracking matcher with an explicit backtrack stack, every step charged to the step budget, because the semantics of ECMAScript (ordered alternation, captures reset per iteration, backreferences, lookahead and lookbehind) are defined by backtracking (22.2.2, `#sec-pattern-semantics`), and an explicit stack means no Rust recursion: a hostile pattern such as `(a+)+$` ends at the budget and cannot hang the host. Decision 3, "Depth bounds": the matcher is a loop with an explicit backtrack stack, bounded by the step budget. This story writes the compiler and the matcher of `swell-regexp` over the tree of `story:regexp-crate-and-pattern-parser`, in the crate alone (no heap value, decision 2), so that `story:regexp-objects-exec-and-literals` runs them from `RegExp`. It runs beside `story:regexp-literal-early-errors`, which writes `crates/syntax` and does not use the matcher.

Lifts of the consumer (`docs/plan/slice-05/consumer-regexps.tsv`): none by itself; the test `the_constructs_of_the_consumer_table_match` below matches, in the crate, one pattern per row written for it with the constructs of that row (nothing of the downloaded files is copied), so the matcher the consumer needs is shown before `RegExp` exists. No test262 case runs on the crate alone; the cases its matching lifts are on the lists of `story:regexp-objects-exec-and-literals` and `story:regexp-symbol-methods-and-string-match-search`.

Design it builds on: decision 7, decision 3 ("The step budget", "Depth bounds") and the changes of the decomposition D1 (the charging trait, and what one step of the matcher is: one instruction run or one entry taken from the backtrack stack, charged in batches of at most 4,096 before they are run) and D4 (no Rust recursion in the compiler either).

## Domain relations

- None (the program and the match state are values of the crate; no record of the engine gains a relation).

## Change

Failing tests first: the tests below are written before the code and do not compile on the base of the unit (no compile or match entry exists in the crate).

- `crates/regexp/src/program.rs` and `src/compile.rs` (new): CompilePattern (`#sec-compilepattern`) of the tree to a program, with an explicit stack and no Rust recursion (D4): instructions for a code unit or a code point, a class (sorted, disjoint ranges), `.` with and without `s`, `^` and `$` with and without `m`, `\b` and `\B`, a choice (greedy or lazy) and a jump, the start and end of a capture, the clearing of the captures of a quantified atom at each iteration (`#sec-runtime-semantics-repeatmatcher-abstract-operation`, step 4), counters for `{n,m}` with the empty check of step 2.b, a backreference (by number, and by name as the list of the groups of that name, duplicates included), the entry and exit of a lookaround with its direction, and the end of the match. Each instruction is documented with what it does to the state. The program is charged through the trait before it grows, and `Program::heap_bytes` gives what its owner credits when it drops it. Modes: without `u` the subject is read as code units; with `u` as code points (a surrogate pair is one, a lone surrogate itself; `.`, classes and lookbehind step over a pair). `\d`, `\w` and `\b` are those of `#sec-compiletocharset` without the flag `i` (ASCII); `\s` is WhiteSpace and LineTerminator (`#sec-white-space`, with `swell_unicode::is_space_separator` for `Zs`). The modifiers `m` and `s` of `(?ms-ms:...)` set the mode of their atoms.
- Refused here, a refusal of slice 5 counted by the engine (`<construct> is not supported before slice 5`, as `story:regexp-crate-and-pattern-parser` writes its refusals): a pattern with the flag `i` or a modifier `i` (`case-insensitive matching`), refused at compile; Canonicalize (`#sec-runtime-semantics-canonicalize-ch`) needs the case tables of the story of case mapping, and the story of case-insensitive matching lifts it.
- `crates/regexp/src/matcher.rs` (new): the match of a program against a subject of UTF-16 code units from one index, and the search that tries each index from a start (the loop of RegExpBuiltinExec, `#sec-regexpbuiltinexec`, steps 11 to 13, with one index or a code point per try as AdvanceStringIndex gives, and only the start index when the search is sticky), answering the captures (each a range or unset, group 0 the match) or no match. A loop over the instructions with an explicit backtrack stack of entries (the instruction to resume, the position, and the captures and counters to restore), no Rust recursion. The semantics of 22.2.2: alternatives in order; greedy and lazy quantifiers; the empty check; captures reset per iteration; a backreference to an unset group matches the empty String; a lookahead is atomic (no backtracking into it), keeps the captures of a positive one and resets those of a negative one; a lookbehind reads right to left and fills its captures so; duplicate named groups answer the group that took part in the match.
- Steps and memory: one step per instruction run and per entry taken from the backtrack stack, charged through the trait in batches of at most 4,096 before they are run, and the interrupt polled with each batch; the search charges a step per start index it tries. The backtrack stack and the captures grow by capacity, each growth charged before it is made, and every byte is credited on every way out of the match: a match, no match, and a stop.
- `crates/regexp/src/lib.rs`: the `mod` lines and the exports of compile, the program, the match and the search.
- Tests (`crates/regexp/tests/`, new files, `main.rs` generated): `matcher.rs`: a table of rows (pattern, flags, subject, start, expected captures or no match) that reaches every instruction and every point of the semantics above, with and without `u`, the forms of B.1.2 included; `the_constructs_of_the_consumer_table_match`; `sticky_search_tries_one_index_and_unicode_search_steps_over_a_pair`; `case_insensitive_matching_is_a_refusal_of_slice_5`. `budgets.rs`: `catastrophic_backtracking_ends_at_the_step_budget` (`(a+)+$` against 10,000 `a` and a `!`, with a step budget of 1,000,000: the stop over the step budget, no hang), `the_steps_are_charged_before_they_are_run` (a recording budget: a match of n steps is charged at least n and at most n + 4,096, and no instruction runs past a refused charge), `a_backtrack_stack_past_the_memory_budget_stops_the_match` (`(?:a|b)*c` against 100,000 `a` with 64 KiB), `the_stack_and_the_captures_are_credited_on_every_way_out`, `an_interrupt_stops_a_long_match`, `the_compiler_and_the_matcher_use_no_rust_recursion` (a pattern of 10,000 nested groups compiled and matched, and `a*` against 1,000,000 units, on a thread of 64 KiB).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode of the gate, under 2 s with a collection at every safe point, and under 2.5 s at its full count (`SWELL_FULL_COUNTS=1`) in either mode, measured with its test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); a language script stays under 0.5 CPU seconds per run. The unit times each test before it adds it and names in its report each one over 1 CPU second with its time; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the steps run a process per test); no test makes a network request; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`, a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m`, the most), and a script that measures or waits runs without the prefix. No Python in any tool, test or command. The budget tests are bounded by their budgets (1,000,000 steps, 64 KiB); each is timed and none is expected over 0.5 CPU seconds.
- Size: the compiler and the matcher, about 1,500 to 2,000 lines with the tests.

Writes: `crates/regexp/src/{program,compile,matcher}.rs` (new), `crates/regexp/src/lib.rs` (the `mod` lines and exports), `crates/regexp/tests/{matcher,budgets}.rs` (new), `crates/regexp/tests/main.rs` (generated). Not written: any file outside `crates/regexp` (an error of the parser found here goes back to `story:regexp-crate-and-pattern-parser` or, once that story has landed, is a draft in the store for the coordinator); `Cargo.lock` (no new dependency). Shared with the story that may run beside it, `story:regexp-literal-early-errors`: none.

## Acceptance

`cargo nextest run --locked -p swell-regexp --test main` and `cargo nextest run --locked -p swell-regexp --lib` exit 0 with every test named in Change passing (on the base of the unit `matcher.rs` and `budgets.rs` do not compile), `catastrophic_backtracking_ends_at_the_step_budget` answers the stop over the step budget, and `slot task clippy` exits 0; the report gives the CPU time of each test of `budgets.rs` measured as Change says.

## Out of scope

`RegExp`, `exec`, `lastIndex`, the match array and the step budget of an agent (`story:regexp-objects-exec-and-literals`); case-insensitive matching and modifiers with `i` (the story of case-insensitive matching); the `v` flag, its class set operations and properties of strings; property escapes; an automaton for patterns without backreferences or lookaround (rejected for now, decision 7).
