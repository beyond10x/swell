---
format: aep.planning-md/3
id: review-result:copies-held-round-1-design
kind: review-result
status: active
title: Copies held across a call into script, round 1, the design critic
relations:
- reviews: story:copies-held-across-a-call-into-script-are-charged
revision: 1
---
needs-revision

**Findings**

1. **Blocker, site 18 (`Math.max`), draft lines 182, 261.** `story:math-constants-and-exact-functions` writes all of `builtins/math.rs`. It replaces `math_max`'s list with a shared `coerce_all` helper for `max`, `min` and `hypot`, which reserves with `check` and `try_reserve_exact`.
   - The two units conflict at `math.rs:47-60`.
   - After the join, `min` and `hypot` hold an uncharged list across `ToNumber`, so the rule is false in the merged tree.
   - Test 8 (draft line 342) sits after `a_loop_of_builtin_calls_collects_within_its_bound` in `object/builtin_function.rs`, the anchor the Math stories use for their twin tests.
   - Fix: drop site 18 and test 8 from this unit. Put the held `Charge` in `coerce_all`, in the Math story or a story that depends on it. Until then `Math.min` and `Math.hypot` stay under `## Not in this story` with their bound named.

2. **Blocker, the promise job queue is missing from the scan.** In `host/jobs.rs:133-139`, `reserve_entry` doubles the `VecDeque` with `try_reserve_exact`.
   - `enqueue_promise_job` (`:166-170`) charges only `promise_job_bytes` per entry.
   - Each pop credits that entry, but the capacity stays at its peak until the checkpoint ends (`:246`). That buffer is alive across every promise job.
   - Its size is script-controlled and bounded only by memory and steps, not by a depth limit like the two spare-capacity cases the draft admits (lines 491-498).
   - Fix: charge the queue's capacity through `reserve_vec` or a `ChargedVec`, or add it to `## Not in this story` with a stated bound. Its `HostMicrotask` entries are not charged at all.

3. **Warning, the rule is stated by capacity but the "already charged" list is charged by length (lines 47, 92-109, 202-203).** Line 203 says every buffer of the rule is a `ChargedVec`; the draft's own lists contradict that.
   - These plain `Vec`s grow by the standard doubling while charged per element: the flat stack (`array/iteration.rs:115`, `:510`), the `JSON.stringify` property list (`json/mod.rs:271-277`) and the `JSON.parse` records (`json/parse.rs:505-519`).
   - Their spare capacity, up to 2x, is uncharged while the reviver or replacer runs.
   - Fix: either convert them to `ChargedVec`, or add a sentence to the rule and the design text allowing "charged per element at no less than the buffer, growth bounded by doubling", with a list of these sites.

4. **Warning, the instrument cannot see a plain `Vec` (lines 202-203).** The draft states the hole but nothing closes it, and a later builtin is not held to the rule.
   - Add `disallowed-methods` for `Vec::try_reserve`, `try_reserve_exact`, `with_capacity` and `<[T]>::to_vec` to `clippy.toml`. The gate already runs clippy with `-D warnings`.
   - Each remaining site gets `#[expect(clippy::disallowed_methods, reason = "<charge or bound>")]`. The before state is the site count.
   - Alternatively, a grep over non-test code with a committed list of sites and their bounds.

5. **Warning, `reserve_vec` can refuse where the base succeeds (lines 135-138, 157-159).** Charging the doubled capacity means a builder whose needed size fits can end in `MemoryBudget` when the doubling does not. The base only checked `needed`, so this changes results the draft says stay as they are (line 205). Fix: when the doubling is refused, retry with the exact need, and test one script near the limit.

6. **Warning, `concatenate` and `Error.prototype.toString` cannot be switched with "steps unchanged" (lines 146-148).** `ops/operators.rs:134-135` and `builtins/error.rs:410-412` call `units.extend(...)`, which `ChargedVec` does not have, so they need `reserve_vec`.
   - As written they hold 2 or 3 charged copies on the `+` path, where today there are none.
   - A large `s + s` could end in `Terminated(MemoryBudget)` that did not before.
   - Fix: one `reserve_vec` of n+m, then copy from the two heap Strings in place, with the steps already charged at `operators.rs:133` and `error.rs:408`.

7. **Warning, `KeptRoom` credits by hand (lines 183-190).** `end_kept_room` credits `room.charged` by hand. A path that returns between `reserve_kept` and `end_kept_room` leaks the credit, unlike a `Charge`. Line 207 says every new charge is credited on drop. Fix: hold a `Charge` inside `KeptRoom`, and in test 9 assert `used()` before equals `used()` after on the `Throw` and `Terminated` paths.

8. **Warning, `padStart` copies before it charges (`transform.rs:365`, draft lines 143-144, 219-220).** `string_units_in(fill, 0, used)` copies up to the fill's length before `string_pad` charges anything, so the claim that callers charge each unit before they copy it is false there.
   - Fix: have `string_pad` read the filler in place through the append path (step totals unchanged), or write the exception into the rule sentence.
   - The URI functions (`global.rs:250`) are the same order, kept on purpose by the other story; the design sentence needs the same exception.

9. **Note, soundness of the shared cell.** The design is sound with no `unsafe`: a drop through `?` adds to the cell exactly once, and no credit is lost or counted twice.
   - `Accounting::check` takes `&self` and cannot fold the cell. State that it subtracts the cell, as `used()` does.
   - `Heap::collected` sets `trigger_bytes` from `live_bytes()`, which includes held charges when a collection runs inside a call. That is harmless, but say it.
   - `Charge` must follow the capacity the allocator returned. If `try_reserve_exact` gives more than asked, the instrument reads uncovered bytes.

10. **Note, one unit or two.**
    - With `Math.max` removed (finding 1), one unit holds. Every part uses the same foundation (`Charge`, `ChargedVec`, the instrument), HOST-214 needs builders, `join` and the call path together, and the type changes are compiler-driven.
    - If it is cut, the cut is: A is the foundation, readers, builder, `join`/`sort` and HOST-214; B is the `call`/`new` copy and the kept rooms, depending on A. Each is green, and the rule is whole after B.

11. **Note, design sentence (lines 213-221).** It does not name the way a buffer is charged (`Charge`, `ChargedVec`, `reserve_vec`). It does not say that a hand-charged list is allowed only if the design lists it with its bound. Add both so a later builtin, such as the slice-10 builtins, is held to them.

12. **Out of my lane.**
    - The second half of the HOST-214 test (`Throw` of a `RangeError`) answers the same on the base through the public API. Only the lib twin (test 10) sees the base's copies.
    - The counts 215/146/69 hold only on this base. The other slice-4 stories add lines to `conformance/host/requirements.txt`, so the join renumbers them.

```findings
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 182
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Site 18 rewrites math_max, which story:math-constants-and-exact-functions replaces with coerce_all for max, min and hypot (whole file), so the join conflicts and min and hypot keep an uncharged list across ToNumber; move the held Charge into coerce_all behind that story and drop test 8 from the builtin_function.rs anchor.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 491
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The microtask queue (host/jobs.rs:133-139, :246) grows by doubling with only per-entry bytes charged and stays at peak capacity while promise jobs run, so a script-sized buffer alive across calls into script is neither charged by capacity nor listed with a bound.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 92
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The flat stack, the JSON.stringify property list and the JSON.parse records are charged per element but grow by amortized doubling while alive across a callback, which contradicts "charged by capacity" and "every buffer of the rule is a ChargedVec"; convert them or state the factor-2 allowance in the rule.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 202
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Nothing closes the plain-Vec hole of the instrument; add disallowed-methods in clippy.toml for Vec reserve, with_capacity and to_vec with an #[expect] reason at each remaining site, or a committed list of sites with their bounds.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 135
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: reserve_vec charges the doubled capacity and terminates when it is refused, so a builder whose needed size fits can end in MemoryBudget where the base succeeded; fall back to the exact need before terminating and test a script near the limit.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 146
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: concatenate and Error.prototype.toString call extend on the copies, which ChargedVec lacks, and as written hold 2 to 3 charged copies on the hot path of +; reserve once for n+m through reserve_vec and copy in place with the steps already charged.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 183
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: KeptRoom credits by hand in end_kept_room, so a path that skips it loses the credit; hold a Charge in KeptRoom and assert used() before equals after on the Throw and Terminated paths in test 9.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 143
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: padStart copies the filler with string_units_in (transform.rs:365) before any step is charged, contradicting the claim that callers charge first and the design sentence; read it in place through the append path or write the exception, with the URI functions, into the rule.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 125
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Say that check subtracts the cell without folding it, that trigger_bytes reads held charges in live_bytes during a collection inside a call, and that Charge follows the capacity the allocator returned.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  line: 213
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The design sentence and the requirement line do not name Charge, ChargedVec or reserve_vec as the way a buffer is charged, nor require a hand-charged list to be listed with its bound, so a later builtin is not held to them.
- file: .engineering/planning/story/copies-held-across-a-call-into-script-are-charged.md
  category: design
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: With Math.max moved out the unit holds as one (shared foundation, one requirement, compiler-driven type changes); if cut, split at foundation, readers, builder, join/sort and HOST-214 versus the call/new copy and the kept rooms.
```

Worktree: `swell-impl-copies-held-across-a-call-into-script-are-charged`.
