---
format: aep.planning-md/3
id: review-result:adversary-growth-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit growth (buffer-growth-outside-the-charged-list-is-listed): 6 findings, 2 introduced'
relations:
- reviews: story:buffer-growth-outside-the-charged-list-is-listed
revision: 1
---
Adversary pass 1 of 2 on the unit growth, branch impl/buffer-growth at 3a16be13 (base f4759cfe), wave big-1.
Verdict: red. 6 findings: 2 introduced (an allow, and an expect without a reason, pass the gate), 4 found on the base too (1 warning, 3 notes); no case added.

unit: growth (head 3a16be13 on impl/buffer-growth, base f4759cfe; my temporary edits were removed, so the tree is clean)
verdict: red
cases: executed 0→0, red 0
origin: introduced 2, pre-existing 4, undecided 0
wrote-outside-worktree: 5 paths under <scratch>/wave/buffer-growth/adversary-1/ (listed in part 6)
needs-coordinator: yes. Two decisions: whether a reason that is missing, or an `allow` in place of the `expect`, is in this story's scope (findings 1 and 2); and you should know that I edited a tracked implementation file for a short time (part 1).

**1. Diff stat:** `git --no-pager diff --stat` prints nothing (exit 0), and `git status --short` is empty. As the brief allowed, I planted probes twice. Each time I added `mod adversary_probe;` to the tracked `crates/swell/src/lib.rs` and added a new file `crates/swell/src/adversary_probe.rs`. I made each edit while no pool job of mine was running and took it back right after the run. `adversary.md` says never to edit a file under attack, even briefly; I followed the brief here instead. Copies of both probes are in the scratch directory.

**2. Cases added:** none. I found no defect a test can show red against today's tree. The mechanism findings only show up when a later call is added, and the charge findings are not visible through the public interface.

**3. Suite run:** none. No case was added and the brief asks for clippy runs only. Both probe runs used the gate's own command: `cargo clippy --workspace --all-targets --locked --message-format=short -- -D warnings`. Both exited 101; the logs are `clippy-probe.log` and `clippy-probe2.log`.

**4. Findings**

1. **An `allow` passes the gate.** I planted `#[allow(clippy::disallowed_methods)]` with no reason on a fn that calls `Vec::with_capacity(n)` (probe line 86–87). The gate's command reports nothing at :87, while 20 other planted calls in the same file are each reported as `error: use of a disallowed method` (clippy-probe.log:4–24).
   - What reaches it: any later unit (story item 5) can put that attribute on a call. The call then has neither an `expect` with a reason nor a red gate.
   - Origin: introduced. Possible fix: deny `clippy::allow_attributes`, or a source-scan rule that permits only the `cfg_attr` in `lib.rs`.
2. **An `expect` without a reason passes the gate.** I planted `#[expect(clippy::disallowed_methods)]` with no `reason` on `let v = Vec::with_capacity(n);` (probe2:6–7). No error is reported. The control, the same call without the attribute, is reported at probe2:12 (clippy-probe2.log:4; exit 101).
   - This contradicts the story's line 30: "The compiler checks that a reason is there". `crates/swell/Cargo.toml:23` `[lints.clippy]` turns on no lint that requires a reason.
   - What reaches it: any later call. Origin: introduced.
3. **The JSON open-value stack stays allocated without a charge while the reviver runs script.**
   - The reservation is `builtins/json/parse.rs:396` (`parser.stack.try_reserve(1)`). Its reason says "charged by `json_parse_charge`, an `Open` per open value".
   - Each close credits that charge again (`:429`), but the `Vec<Open>` keeps its capacity: roughly the deepest nesting times `size_of::<Open>()`, and up to about twice that because `try_reserve(1)` grows by doubling.
   - `parser` is dropped only when `json_parse_with` ends, which is after `self.json_internalize(&mut walker)` (`:261`) has called the reviver.
   - What reaches it: `JSON.parse(deeplyNestedText, reviver)`. Nothing limits the depth.
   - Origin: pre-existing (the same code is at base `:392` / `:425` / `:261`). The new reason line is the part that is not true.
4. **Charge after the reservation (operand stack).** In `interp/mod.rs:258-261`, `try_reserve_exact` runs before the charge at `:262`. The reason itself says "charged by the next statement". If the budget refuses, the grown stack stays uncharged until the run ends. No script runs in between. Reached by every growth of the operand stack. Origin: pre-existing.
5. **Charge after the reservation (environments).** In `env/mod.rs:364`, the reservation is preceded only by `accounting.check` (`:358`); the charge comes afterwards, in `store_environment` (`:368`). The reason says exactly this. No code runs in between. Origin: pre-existing.
6. **Charge after the reservation (strings).** `heap/string.rs:242` (`room`) is preceded by `check` (`:257`, `:276`); the charge comes afterwards, in `new_string_record` (`:269`, `:288`). The `new_atom_key` caller does charge first (`:374`). Origin: pre-existing.

**5. What I attacked and could not break**
- **Probe forms:** every form the brief listed was flagged, in the same run (clippy-probe.log:4–24):
  - path call, type alias, full `alloc::` path, qualified `<Vec<u8>>::reserve`;
  - `Box<Vec>`, `&mut Vec`, `Rc::make_mut`, a generic `DerefMut` receiver;
  - inside a crate macro (reported at the macro body, `:14`), inside a closure;
  - a fn used as a value (`let f: fn(..) = Vec::with_capacity`, `.map(String::with_capacity)`);
  - `alloc::collections::vec_deque::VecDeque`, `alloc::string::String`, `to_owned` / `to_string` plus `reserve`, `Cow::to_mut`.
- **Forms that passed unflagged:**
  - The `#[cfg(feature = "fault-injection")]` call: the gate builds no features, and the story's "Not in this story" says so.
  - `extend_from_within`, `collect`: growth without a reservation, also out of scope.
  - A `const fn` cannot call these methods.
- **Other crates:** the story excludes them ("Not in this story"), so this is not a defect.
- **`cfg_attr` of `lib.rs`:** it applies only under `cfg(test)`. No feature sets that, and `cli` and `test262` depend on `swell` without features.
- **`clippy.toml` files:** both hold the same 4 `allow-*-in-tests` keys, and the root file holds nothing else.
- **Mutant:** a stale `expect` reads `error: this lint expectation is unfulfilled` under the gate's exact command (`-D warnings`), in the lib target and the lib-test target (clippy-probe.log:25, :28). The gate step (`Taskfile.yml:573`) is the command I ran; the unit's control used `-p swell`.
- **Reasons:** I read all 51 `expect` attributes in `src`, which cover 54 sites (`arguments.rs`, `object/mod.rs` `listed_indices` and `promise.rs` each hold two calls in one statement), and the 4 in `tests`. Apart from findings 3–6, the named charge comes first and covers at least the bytes reserved.

**6. Paths written outside the worktree**
- <scratch>/wave/buffer-growth/adversary-1/src-diff.txt
- <scratch>/wave/buffer-growth/adversary-1/adversary_probe.rs
- <scratch>/wave/buffer-growth/adversary-1/adversary_probe2.rs
- <scratch>/wave/buffer-growth/adversary-1/clippy-probe.log
- <scratch>/wave/buffer-growth/adversary-1/clippy-probe2.log

```findings
- file: crates/swell/Cargo.toml
  line: 23
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a listed call under a plain #[allow(clippy::disallowed_methods)] with no reason passes the gate's clippy command, so a later call can carry neither an expect with a reason nor a red gate
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 30
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the story says the compiler checks that a reason is there, but an #[expect(clippy::disallowed_methods)] with no reason on a planted Vec::with_capacity passes the gate's clippy command
- file: crates/swell/src/builtins/json/parse.rs
  line: 396
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the reason names the json_parse_charge charge, but each close credits it (:429) while the Vec<Open> keeps its capacity alive and uncharged through the reviver calls of json_internalize (:261)
- file: crates/swell/src/interp/mod.rs
  line: 258
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the operand stack is reserved before its charge at :262, which the reason itself names as the next statement, so on refusal the grown stack stays uncharged until the run ends
- file: crates/swell/src/env/mod.rs
  line: 364
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the environment slots are reserved after only a budget check, and their charge is made afterwards by store_environment at :368
- file: crates/swell/src/heap/string.rs
  line: 242
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: room() reserves after only a check in new_string_from_str and new_string_from_units, and the charge comes afterwards in new_string_record at :269 and :288
```
