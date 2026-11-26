---
format: aep.planning-md/3
id: review-result:adversary-growth-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit growth: red, 5 findings, 1 green case'
relations:
- reviews: story:buffer-growth-outside-the-charged-list-is-listed
revision: 1
---
Adversary, pass 2 (the last) on the unit growth of wave big-1 (impl/buffer-growth at 1d48bbcd), 2026-11-26.
Verdict: red. 5 findings (2 warnings, 3 notes), all introduced; 1 case added, green.

```
unit: growth (pass 2, head 1d48bbcd on impl/buffer-growth; probes taken back, the one change left in the tree is the added case)
verdict: red
cases: executed 0→1, red 0
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 9 files under <scratch>/wave/buffer-growth/adversary-2/ (listed at the end)
needs-coordinator: yes. The story promises "carries the expect, or the gate is red". Three attribute forms keep neither half of that promise: findings 1 to 3. You decide whether to accept them in writing or close them with a scan.
```

I used 3 pool jobs:
- Run 1: clippy with the gate's command (`Taskfile.yml:573`) and `--message-format=short`, probes in the library. Exit 101, log `clippy-probe.log`.
- Run 2: the same command, probes in the integration-test target. Exit 101, log `clippy-tests-probe.log`.
- Run 3: nextest on my one case. 1 passed, 0.58 s, log `nextest-json.log`.

**Findings**

1. **An inner `allow` with a reason passes wherever an inner attribute can stand.** I planted `#![allow(clippy::disallowed_methods, reason = "…")]` at the top of a module file, in an inline `mod`, in a fn body, in a block statement and in a loop body (probe A1 to A5), and in a test fn body (T4). The gate's command reports none of those calls.
   - The controls are red: an inner allow without a reason (probe line 40), a plain call (probe line 46, test probe line 19).
   - This breaks the story's lines 26 to 28 (carries the `expect`, or the gate is red) and line 127. Acceptance items 1 to 8 do not test it.
   - The unit's proposed `AGENTS.md` text says this works only "at the top of a module". That understates it.
   - Reached by any later change. Origin: introduced (the unit disclosed the gap).

2. **An `expect` of a lint group with a reason silences a listed call.** I planted `#[expect(clippy::style, reason)]`, `#[expect(clippy::all, reason)]` and `#[expect(warnings, reason)]` (probe lines 68, 74, 80). All three pass, and each is fulfilled by the call.
   - `disallowed_methods` belongs to `clippy::style`. The attribute does not name `clippy::disallowed_methods`, as story line 26 requires.
   - This one is not disclosed. Origin: introduced.

3. **An `allow` from a macro of another crate passes.** A `#[macro_export]` macro in `swell-syntax` that wraps an item in `#[allow(clippy::disallowed_methods)]`, with no reason, silences the call (probe B12, lines 132 to 137). Neither new lint reports it, because it comes from an external macro.
   - The same macro defined in the same file is red (probe line 106).
   - Origin: introduced (a gap in the new lints).

4. **An empty or blank reason passes.** `reason = ""` and `reason = " "` both pass (probe lines 86 and 92). Story line 30 says only that a reason is there. Origin: introduced.

5. **The JSON reason does not state the bound the code holds** (`json/parse.rs:398`).
   - `try_reserve(1)` grows the capacity to max(4, 2 × the old capacity). So the capacity is at most max(4, 2(d−1)) entries for a deepest nesting d: for d = 1 it is 4, not "at most twice".
   - The bound is a count of entries. Each entry is `size_of::<Open>()` bytes, against 2 bytes of charged text per unit, so it is not a byte bound.
   - Spare room is also uncharged while values are still open, not only "the room a close leaves".
   - Origin: introduced (the sentence is the correction's).

**Answers to the brief's items**

- **Item 1, the scan.** `source_scan.rs::code_of` blanks comments and the contents of strings, so the lint paths stay readable. A scan that closes the gap would have to:
  - read every `.rs` file under `crates/swell/src` and `crates/swell/tests` except `lib.rs`;
  - find `#`, `!`, `[` with any whitespace or newlines between them (rustfmt wraps attributes), then read to the matching `]`;
  - refuse `allow` naming `disallowed_methods` or a group that holds it (`clippy::style`, `clippy::all`, `warnings`), also inside `cfg_attr` at any depth, and also with a `$metavariable` in a `macro_rules!` body;
  - also refuse an outer `expect` of those groups;
  - read the `macro_rules!` bodies of the other workspace crates. It still could not see a macro from outside the workspace.

  It is simpler to allow-list the few inner attributes the crate needs and refuse every other one.
- **Item 3, targets.**
  - The new lints bind the library, its unit tests (my probes U1 and U2 were red) and `tests/main.rs` (T1 to T3 red). There are no benches or examples.
  - Only `faults.rs` gates test code on a feature, and it uses only `TestHost`, `agent`, `realm` and `parse`, which other files use in every build. So the dead items of `support` are the same in all three feature sets. Run 2 shows the `#![expect(dead_code)]` fulfilled in the default build, so it is fulfilled in all three.
- **Item 4, other crates.** The story binds `crates/swell` only ("Not in this story", lines 236 to 237). Nothing filed.
- **Item 5, the JSON change.**
  - (a) Nothing reads `parser.stack` after line 233.
  - (b) The open values' charges stay in `parser.charged`, are added to `*charged` before `parsed?`, and are credited in `json_parse`. My case reads the same `heap_bytes()` from the second call on in all four cases: a valid deep text with a reviver, a text that ends in the middle, the memory budget, and the budget with a reviver.
  - The uncharged capacity this change fixed cannot be seen through `heap_bytes()`.
- **Item 6, the four `expect` attributes.**
  - Each reason is true of its code. `record_atoms` has one caller (`script.rs:493`) and returns four vectors.
  - No gate step runs clippy with another profile.
  - Plain rustc does not check `clippy::` expectations: the base already had some (`math.rs:139`), and the gate's `-D warnings` and release builds passed with them.
  - `dead_code` in `support` depends on no profile, since `support` has no `cfg(debug_assertions)`.

**Cases added (uncommitted, green)**

- `crates/swell/tests/builtins_json_parse.rs`: `adversary_parse_readings` and `adversary_parse_gives_back_its_charges_on_every_way_out`.

**Files under `<scratch>/wave/buffer-growth/adversary-2/`**

- `lib_adversary_probe.rs`, `adversary_top.rs`, `tests_adversary_probe.rs`, `syntax-macro.diff`
- `clippy-probe.log`, `clippy-tests-probe.log`, `nextest-json.log`
- `support-use.sh`

```findings
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 28
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an inner #![allow(clippy::disallowed_methods, reason = "...")] in a module file, inline mod, fn body, block or loop body (and in a test fn) silences a listed call under the gate's clippy command, so the call neither carries the expect nor turns the gate red, wider than the module-top case the unit disclosed.
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 26
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an #[expect(clippy::style, reason)], #[expect(clippy::all, reason)] or #[expect(warnings, reason)] is fulfilled by a listed call and passes both new lints, so the call carries no expect naming clippy::disallowed_methods.
- file: crates/swell/Cargo.toml
  line: 28
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a macro_export macro of another workspace crate that wraps an item in #[allow(clippy::disallowed_methods)] without a reason silences the listed call inside and is reported by neither allow_attributes nor allow_attributes_without_reason.
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 30
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an expect with reason = "" or reason = " " on a listed call passes the gate's clippy command, so the checked reason can be empty.
- file: crates/swell/src/builtins/json/parse.rs
  line: 398
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: try_reserve(1) leaves a capacity of up to max(4, 2(d-1)) entries of size_of::<Open>() bytes each, so the reason's "at most twice the deepest nesting" is false at depth 1, bounds a count rather than bytes, and leaves out the spare room that is uncharged while values are still open.
```
