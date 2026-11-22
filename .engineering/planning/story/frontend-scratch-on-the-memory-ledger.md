---
format: aep.planning-md/3
id: story:frontend-scratch-on-the-memory-ledger
kind: story
status: active
title: The parser's tree, the compile step's scratch and the code are charged before they grow, and a parse reads the interrupt
summary: One ledger trait in swell-syntax, implemented over the agent's Accounting and interrupt flag, charges every growth of the frontend before it happens, polls the flag every 4,096 units of work, and hands the code's charge to the script record; eval and Function read the flag before their copy, and eval charges a step per code unit of its source.
relations:
- depends_on: story:interrupt-read-at-every-parse-entry
- depends_on: story:own-keys-charged-before-the-list-exists
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:27:42Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:27:42Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: `story:interrupt-read-at-every-parse-entry`, `story:own-keys-charged-before-the-list-exists`
(the test-only high-water mark of `heap/accounting.rs`),
`story:arena-capacity-and-collector-workspace-on-the-ledger` (what `store` charges and
`live_bytes()`), and the merges of `let-const-and-block-scopes` and `template-literals`. Base:
`wave/s3-1` with the gate unit (`gate-test-steps-within-budget`) and the design text merged, and
those five merged. Precondition: the design text of decisions D1, D2 and D3 (taken: option A
each), which the base holds: `engine.md` "Memory budget" (the frontend's charges, the ledger,
the transfer at install), "The step budget" (a step per code unit of eval code before its
parse) and `host-interface.md` section 12 (the reads at the start of every parse and every
4,096 units during it).

## Context

Line numbers are of `wave/s3-1` at `c108c2d`. The two language units change
`crates/syntax/src`, `crates/compiler/src`, `script.rs` and `dynamic_code.rs` before this story
starts; the unit finds each site again by its name.

- `parse_script_in` (`crates/swell/src/script.rs:210-243`) charges the UTF-16 copy of the
  source before it allocates it, for both `Source::Utf8` and `Source::Utf16`
  (`:219-227`, through `source_copy`, `:248-264`). Then `swell_syntax::parse_script` (`:228`)
  builds the tree, and `swell_compiler::compile_script` (`:235`) builds the code and its own
  scratch, with no charge: the entries of `swell-syntax` take only
  `ParseOptions { max_nesting }` (`crates/syntax/src/lib.rs:50-56`, `:99-151`), and those of
  `swell-compiler` only the tree (`crates/compiler/src/lib.rs:74-87`). The early errors and the
  scope analysis run inside the parse entry (`crates/syntax/src/lib.rs:154-167`) and allocate as
  well (the `BTreeSet<String>`s of `crates/syntax/src/scope.rs:50-58`), as do the constant maps
  of the compile step (`crates/compiler/src/lib.rs:135-142`). The comment at `script.rs:20-22`
  records that the tree is not charged.
- The code is checked against the budget after it exists: `make_script_record` credits the copy
  (`script.rs:347`), computes the code's bytes from its lengths (`code_bytes`, `:160-192`) and
  asks `Accounting::check` (`:348-351`); the atom vectors are allocated (`:354-362`) before
  `store` charges them with the record (`:383-391`).
- Eval code and the `Function` constructor go the same way: `parse_eval_code`
  (`crates/swell/src/dynamic_code.rs:239-256`) and `compile_eval` (`:219`);
  `parse_dynamic_function_code` (`:651-694`, three parses) and `compile_script` (`:615`). Both
  take their copy through `charged_source` (`:124-142`, called at `:250` and `:670`), which
  calls `source_copy`. The list `param_strings` is reserved with no charge (`:565-568`).
- `parser_nesting` bounds the depth of a parse, not its size: a flat source of n statements
  makes a tree, a scratch and a code that grow with n, none of it charged.
- No parse or compile reads the interrupt flag. For `eval` and `Function` the host hook
  HostEnsureCanCompileStrings runs first (`dynamic_code.rs:199-210`, `:599-610`); after it only
  the sticky termination is checked (`:91`), so a flag the hook or another thread sets is read
  only after the parse and the compile have ended.
- Steps: the parse of eval code charges none (`dynamic_code.rs:239-256`); CreateDynamicFunction
  charges one per code unit as it copies the parts into the source String (`:583-596`), before
  the hook; `parse_script` spends none (`crates/swell/src/host/agent.rs:439-441`). `AGENTS.md`,
  "Rules" (`:60-61`): "Every loop the engine runs on behalf of a script is charged to the step
  budget"; the lexer's loop over eval code is such a loop. That rule is the source of point 6.

## Change

Failing tests first (Acceptance). Then:

1. **One ledger.** `crates/syntax/src/ledger.rs` (new) defines the trait `Ledger`:
   `charge(bytes) -> Result<(), Stop>`, `credit(bytes)`, `poll() -> Result<(), Stop>`, with
   `Stop::{OverBudget, Interrupted}`, and `Unlimited`, a ledger that charges nothing and never
   stops. `swell-compiler` takes the same trait. In `swell` one implementation, in `script.rs`,
   charges and credits the agent's `Accounting` (`crates/swell/src/heap/accounting.rs`) and
   reads the agent's `InterruptHandle`; there is no second budget, and `source_copy` stays the
   way the copy is charged.
2. **Charged before growth, by capacity.** Every `Vec`, `String`, `Box` and map entry the lexer,
   the parser, the early errors, the scope analysis and the compile step allocate is charged
   before it is allocated, in bytes of capacity. A vector grows by the frontend's own doubling
   with `try_reserve_exact`, as `KeyedList::make_room` does
   (`crates/swell/src/object/property.rs:305-358`), so a charge does not depend on the growth
   policy of the standard library. An entry of a `BTreeMap` or `BTreeSet` is charged at its
   insertion as its key and value bytes plus a share of a node, a constant the unit states with
   its derivation. A refused charge stops the step: the entry answers
   `Terminated(MemoryBudget)` through `Agent::terminate`, everything charged in it is credited,
   and no `SyntaxError` object is made.
3. **Credited when it drops.** The tree carries what it was charged (a field of
   `swell_syntax::Script`); `swell` credits it where it drops the tree (`script.rs:236`,
   `dynamic_code.rs:220`, `:616`, and each of the three parses of CreateDynamicFunction). The
   compile step credits its scratch before it returns. After a parse that answers errors or a
   termination, `heap_bytes()` reads what it read before the call plus the `SyntaxError` objects
   made.
4. **Transferred when the script is installed.** `Code` carries the capacity bytes charged for
   its vectors (a field). `make_script_record` takes that charge and the copy's as part of what
   the record owns, with no credit and no `check` between (it replaces `script.rs:347-351`);
   `code_bytes` by length goes. At install the `Compiled` record and the four atom vectors are
   charged before they are allocated, and the record goes into its arena through
   `heap::store_charged` (new, `crates/swell/src/heap/mod.rs`, beside `store`, whose signature
   stays): it stores a record whose owned bytes the caller has charged, charging only the
   arena's growth as `store` does. The record's slot holds exactly what the record owns, so the
   sweep that frees it credits exactly that.
5. **The interrupt.** `charged_source` (`dynamic_code.rs:124-142`), through which `eval` and
   `Function` take their copy, reads the flag before it calls `source_copy`: a set flag is
   cleared and the run ends in `Terminated(Interrupted)` (`Agent::check_interrupt`,
   `crates/swell/src/interp/budget.rs:33-39`). `parse_script` and `cx.parse_script` read it at
   their start (`story:interrupt-read-at-every-parse-entry`) and not again before their copy.
   During every parse and compile step, of all four entries, `Ledger::poll` is called at least
   once per 4,096 code units the lexer advances and per 4,096 nodes the early errors, the scope
   analysis and the compile step visit; the `swell` implementation reads the flag as
   `check_interrupt` does. `parse_script` then answers `Parsed::Terminated(Interrupted)`,
   `eval` and `Function` end the run in `Terminated(Interrupted)`, with everything credited.
6. **Steps (decision D2).** PerformEval charges one step per code unit of the eval String before
   its copy (`Agent::charge_steps`, `agent.rs:559-570`, which reads the flag first), as
   CreateDynamicFunction charges one per unit it copies; CreateDynamicFunction charges nothing
   more; `parse_script` spends none.
7. **Stays as it is.** `ParseOptions` keeps its one field (`crates/swell/src/host/limits.rs:61-70`
   builds it in a `const fn`). The entries without a ledger (`parse_script`, `parse_eval`,
   `parse_dynamic_function`, `compile_script`, `compile_eval`) stay, as the charged entries
   called with `Unlimited`, so `crates/syntax/tests/sources.rs` and
   `crates/compiler/tests/compile.rs` need no change. The ledger lives in the parser's state,
   never in a recursive frame: `STACK_BYTES_PER_LEVEL` (224, `crates/syntax/src/lib.rs:64`)
   holds. The step counter of bytecode, the trees and codes produced. No test of
   `crates/swell/tests/eval.rs` asserts a step count (its one read of `last_entry_steps()`
   expects `Err(Faulted)`), so point 6 moves no expected number there.
8. `crates/swell/src/heap/accounting.rs` gains, under `#[cfg(test)]` beside the high-water mark,
   a countdown of polls: when it reaches 0 the `swell` ledger sets the flag before it reads it.
   Per agent, read only by the lib tests.

Requirements (`[slice 6]`): HOST-168 (section 12) and HOST-169 (section 2), as the design text
words them. Their lines in `conformance/host/requirements.txt` are the ones the design text
added, `HOST-168 pending slice 6` and `HOST-169 pending slice 6`; this story changes only those
two lines (Acceptance).

Writes: `crates/syntax/src/ledger.rs` (new), `crates/syntax/src/{lib,ast,lexer,token,error,early,scope}.rs`,
`crates/syntax/src/parser/{mod,expression,statement,function}.rs`,
`crates/compiler/src/{lib,bytecode,expression,statement,function,resolve}.rs`, the files the two
language units add under `crates/syntax/src` and `crates/compiler/src`,
`crates/swell/src/{script,dynamic_code}.rs`, `crates/swell/src/heap/mod.rs` (`store_charged`
only), `crates/swell/src/heap/accounting.rs` (the test-only countdown),
`crates/swell/tests/frontend_ledger.rs` (new), `conformance/host/requirements.txt` (the lines
of HOST-168 and HOST-169). Not written: `crates/syntax/tests/sources.rs`,
`crates/compiler/tests/compile.rs`, `crates/swell/src/host/limits.rs`,
`crates/swell/src/host/{agent,mod}.rs`. Running units: every file above under `crates/syntax`,
`crates/compiler` and `crates/swell/src/{script,dynamic_code}.rs` is written by
`let-const-and-block-scopes` or `template-literals` or both; this story starts from the tree
after both have merged. No story of this set runs beside it on those files.

## Acceptance

In `crates/swell/tests/frontend_ledger.rs` (one new binary). The flat source is
`function unused() {`, then 20,000 lines `x = 1;`, then `}`.

- `eval_and_function_of_a_flat_source_end_in_memory_budget`: the host defines the flat source
  as a global String `src` (and its body as `body`) through `with_cx`; the scripts `eval(src)`
  and `Function(body)` with `memory_bytes` = `heap_bytes()` after that + R + 4 KiB, R the rise
  of `live_bytes()` over a successful call on a probe agent (what the records made own). Today
  `Normal`; after `Terminated(MemoryBudget)`, and after `collect()` `heap_bytes()` reads as
  before the call.
- `host_168_an_interrupt_set_by_the_compile_strings_hook_ends_eval_before_its_parse`: a
  `host_ensure_can_compile_strings` that calls `interrupt()` and answers `Ok(())`; the scripts
  `try { eval(r); } catch (e) {}` and `try { Function(r); } catch (e) {}`, `r` a refused
  source. Today each answers `Normal` and `refusals()` moves by 1 for each. After, each ends in
  `Terminated(Interrupted)` and `refusals()` does not move.
- `eval_charges_a_step_per_code_unit_before_its_parse` (D2): a host global `big` of 100,000 code
  units that ends with a refused construct, `steps_per_entry` 10,000, the script `eval(big)`.
  Today `Throw` of a `SyntaxError` and `refusals()` moves by 1. After
  `Terminated(StepBudget)`, `refusals()` unchanged, `last_entry_steps()` at most 10,000.

Unit tests of the crates (the lib test binary of each):

- `crates/syntax/src/ledger.rs`: `a_parse_polls_once_per_4096_code_units` (a counting ledger, a
  source of 300,000 code units: at least 73 polls; the test is written against the new entry
  and fails on the count until the polls exist);
  `a_stop_from_poll_ends_the_parse_within_4096_code_units` (the third poll answers
  `Interrupted`: the parse answers `Stop::Interrupted` with the lexer at most 3 × 4,096 units in);
  `the_charge_covers_the_tree` (for a flat, a nested and an identifier-heavy source, the charge
  minus the credits when the parse returns equals the tree's capacity bytes summed by an
  independent walk of its arenas and node vectors: at least the walk, and not more);
  `a_refused_charge_stops_the_parse_with_everything_credited`.
- `crates/syntax/src/scope.rs`: `the_scope_sets_are_charged_per_entry`: a source declaring
  10,000 distinct names with `var` and referring to each; the counting ledger's high-water mark
  during the scope analysis is at least 20,000 × the unit's per-entry constant plus the names'
  bytes (the two sets `var_seen` and `reference_seen`); today the phase charges nothing.
- `crates/compiler/src/lib.rs`: `compile_polls_once_per_4096_nodes`,
  `the_code_carries_the_capacity_bytes_of_its_vectors`, `the_constant_maps_are_charged_per_entry`
  (10,000 distinct string literals and 10,000 distinct numbers: the high-water mark of the
  compile step is at least 20,000 × the per-entry constant plus the literals' units; today
  nothing is charged).
- `crates/swell/src/script.rs`, reading the high-water mark and the countdown of
  `heap/accounting.rs`:
  - `host_169_a_flat_source_whose_tree_does_not_fit_ends_in_memory_budget`, as `Utf8` and as
    `Utf16`, with the scripts arena holding a free slot (a parse whose script the host dropped,
    then `collect()`): P, the high-water mark minus `heap_bytes()` before, of a successful parse
    under 64 MiB; T, the tree's capacity bytes by the walk of `the_charge_covers_the_tree`; R,
    what the record of that parse owns. Three budgets, each `heap_bytes()` before + the amount:
    P answers `Parsed` (after); P − T answers `Parsed::Terminated(MemoryBudget)` (after); R + 4
    KiB answers `Parsed` today (the tree and the scratch are not charged) and
    `Parsed::Terminated(MemoryBudget)` after. After each refusal `heap_bytes()` reads as before
    the call and `parse_script` of `1` answers `Parsed` (HOST-70).
  - `the_peak_of_a_parse_covers_copy_tree_and_code`, for `Utf8`, `Utf16`, `eval` and
    `Function`: the high-water mark minus `heap_bytes()` before is at least the copy plus the
    walk of the tree. Today it is the copy alone.
  - `a_parse_leaves_only_its_record_charged`, with the scripts arena holding a free slot: after
    a successful parse `heap_bytes()` = before + what the record's slot holds (`Slot::bytes`,
    what the record owns); after one that ends by a budget, at the copy, in the tree or in the
    compile step (three budgets measured), = before.
  - `a_flag_raised_at_the_second_poll_ends_every_entry`: the countdown at 2; `parse_script` of
    the flat source (`Utf8`, `Utf16`) answers `Parsed::Terminated(Interrupted)`, `eval(src)` and
    `Function(body)` end in `Terminated(Interrupted)`; in each the countdown reads 0 and
    `heap_bytes()` reads as before the call. Today no poll happens: each answers `Parsed` or
    `Normal`.
- The stack: the stack tests of HOST-17 (`limits_scripts.rs`), HOST-152 and HOST-163
  (`eval_stack.rs`) pass unchanged on seven eighths of `min_stack_bytes()`, and
  `every_nesting_kind_fits_the_stack_bound` with its parts `_part_2` to `_part_17`
  (`crates/syntax/tests/sources.rs`) pass.
- `conformance/host/requirements.txt`: the lines `HOST-168 pending slice 6` and `HOST-169
  pending slice 6` become `HOST-168 test swell::frontend_ledger::host_168_an_interrupt_set_by_the_compile_strings_hook_ends_eval_before_its_parse`
  and `HOST-169 test swell::lib::script::tests::host_169_a_flat_source_whose_tree_does_not_fit_ends_in_memory_budget`;
  `slot task host-requirements-check` then prints two pending lines fewer than on the base.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test <b>` for each
`<b>` of `frontend_ledger`, `eval`, `dynamic_function`, `eval_stack`, `limits_scripts` and
`scripts`, the same with `--features collect-every-safe-point`, `slot cargo nextest run --locked
-p swell-syntax --profile default --lib`, `slot cargo nextest run --locked -p swell-syntax
--profile default --test sources`, `slot cargo nextest run --locked -p swell-compiler --profile
default --lib`, `slot cargo clippy --workspace --all-targets --locked -- -D warnings`, `slot task
conform` and `slot task conform-engine` (the counts of the base), and at the end, once, `slot
task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared with that of `slot
task test262 TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{ print $3, $4, $5 }'`
over each, and `cmp` of the two outputs exits 0 (no pass lost in the directories named here).
Never `cargo test -p swell` whole, `slot task test-collect`, `slot task test` or `slot task
check`.

Directories: `test/language`, `test/built-ins/eval`, `test/built-ins/Function`,
`test/language/eval-code` and `test/annexB/language/eval-code`; the budget field of `test262
total` stays 0.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (a counting build
counts within one test's process). Each new test is timed, and one over 5 s is named in the
report with its time. A job of the build pool asks for one slot. Determinism: the same script
under the same budgets ends the same way at the same count on every run. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Cost

Bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell --profile
default --test language` (it parses and compiles every case of `crates/swell/tests/language/`),
built first with `--no-run` and measured with `/usr/bin/time -f '%U %S'` inside the pool job,
three runs on the base and three on the unit, the median of each, rise by at most 5 %. Why 5 %:
a vector pays one charge per doubling, amortised a constant per element against the element's
write and the allocator call the charge precedes; a map entry pays one charge against a B-tree
insertion of log n comparisons; a poll is one atomic load per 4,096 units or nodes; the steps of
eval code are one addition to the step counter per `eval`. Past the bound the
unit reports the three numbers of each side and stops before merging.

## Report

The unit's report gives the new tests with their times, the per-entry constant with its
derivation, P, T and R of `host_169`, the bytes per level of the parser before and after as
`crates/syntax/src/lib.rs:58-64` measures them, the Cost numbers, and this sentence for the
changelog: "The parser's tree, the scope analysis, the compile step's scratch and the code are
now charged to `memory_bytes` before they grow, credited when they drop, and the code's charge
passes to its script record; every parse reads the interrupt at least once per 4,096 code units
or nodes, `eval` and `Function` read it before their copy, and `eval` charges a step per code
unit of its source."
