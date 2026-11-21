---
format: aep.planning-md/3
id: review-result:slice-01-design-round-1
kind: review-result
status: active
title: design critic, slice 1 stories, round 1
relations:
- reviews: story:num-and-unicode-early-parts
- reviews: story:syntax-es5-and-strict
- reviews: story:heap-values-and-host-surface
- reviews: story:bytecode-compiler
- reviews: story:object-model-and-realms
- reviews: story:interpreter-and-script-entry-points
- reviews: story:harness-builtins
- reviews: story:host-interface-hardening
- reviews: story:test262-run-path
- reviews: story:ess-conformance-target
revision: 1
---
needs-revision
story:harness-builtins — `Array.prototype.join` and `JSON.stringify` build Strings at run time from other values, but the body names no `string_length` check and no test for one, while the set holds that bound only in the concatenation of story:object-model-and-realms; each of the two needs the bound and a test that reaches it, here — .engineering/planning/story/harness-builtins.md:39, .engineering/planning/story/harness-builtins.md:64, .engineering/planning/story/object-model-and-realms.md:77, docs/design/host-interface.md:152
story:host-interface-hardening — `min_stack_bytes` is measured only on re-entry through host functions, before story:harness-builtins (built alongside it) adds re-entry through `Function.prototype.call`, the `map` callback and `toJSON`/replacer, and story:test262-run-path runs its workers on exactly `min_stack_bytes()`, so this story, which depends on both, must run `min_stack_bytes_is_enough` again with re-entry through those builtins and correct the constants if it fails — .engineering/planning/story/interpreter-and-script-entry-points.md:117, .engineering/planning/story/harness-builtins.md:43, .engineering/planning/story/test262-run-path.md:73

**Shape, on the points you asked about:**
- **Crate and module layout:** it holds together. story:heap-values-and-host-surface creates every file once (`.engineering/planning/story/heap-values-and-host-surface.md:71-73`). Each later story either fills its own files or edits files the layout gives to an earlier story it depends on:
  - `host/agent.rs`, `host/mod.rs`, `crates/swell/Cargo.toml`, `requirements.txt`, `tests/calls.rs`, `tests/budgets.rs` and `tests/interrupt.rs` are edited along the chain heap → object-model → interpreter → hardening.
  - `realm/intrinsics.rs` and `realm/global_object.rs` are edited by object-model, then harness-builtins.

  The two stories built side by side, story:interpreter-and-script-entry-points and story:harness-builtins, write no file in common.
- **Interpreter versus builtins:** not a split abstraction. `Call`, the step counter and the re-entry count are in story:object-model-and-realms. The interpreter fills `[[Call]]` of ECMAScript functions, the builtins call `Call`, and each side is tested on its own. They first run together in hardening and run-path, and both depend on both.
- **Budgets and bounds:** most sit in the story that creates what they bound:
  - steps in builtin loops: harness-builtins, with its `join` test;
  - re-entry through `toJSON`: harness-builtins;
  - memory accounting: heap;
  - frame depth and parser nesting: interpreter;
  - `HOST-10`/`16`: hardening.

  The two findings above are the exceptions.

**What I read:**
- The 10 stories and `epic:slice-01-harness` in full, plus the brief.
- `aep plan artifact graph`, `validate` (46 artifacts, valid) and `waves`.
- `docs/design/host-interface.md` on the limits table, `HOST-6` to `HOST-22` and `min_stack_bytes`.
- The harness file `decimalToHexString.js`.
- **Edges:** I walked 55, inside and outside the set, not counting the `reviews` edges. There is no cycle.
- **Order:** the set runs num → heap → object-model → (interpreter ‖ builtins) → (hardening ‖ run-path), with syntax → compiler joining the interpreter. ess-conformance-target needs only the interpreter, so the set is not one long queue.
- **Acceptance reads:** I traced 19 to a producer in the set, and an edge records all 19 (5 of them through an intermediate story). One is the 281 → 284 `expected.txt` lines, which run-path reads from syntax through interpreter → compiler.

**What I could not establish:**
- Whether `lib.rs` ("re-exports only", owned by heap) re-exports by glob. If it re-exports by name, the public types later stories add (`Job`, `LoadRequest`, `ModuleRef`, `Realm`, `Script`) need edits to `lib.rs` that no story lists.
- Whether the synthesized ESS scenarios of story:ess-conformance-target evaluate scripts that need builtins. If they do, it needs an edge to story:harness-builtins.
- Out of my lane (parallel safety): none of the 10 stories records a scope, so `waves` reports all of them as unassessed.
- Out of my lane (acceptance): no test runs a builtin with an ECMAScript function as its callback (`map`, `bind`, `toJSON`) before run-path. The 31 cases reach `map` only with `String` as the callback.
- Scratch: I used `.out/critic-design/` in the store worktree and removed it before reporting.

```findings
- file: .engineering/planning/story/harness-builtins.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "`Array.prototype.join` and `JSON.stringify` build Strings at run time from other values, but the body names no `string_length` check and no test for one, while the set holds that bound only in the concatenation of story:object-model-and-realms; each of the two needs the bound and a test that reaches it, here"
- file: .engineering/planning/story/host-interface-hardening.md
  line: 29
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "`min_stack_bytes` is measured only on re-entry through host functions, before story:harness-builtins (built alongside it) adds re-entry through `Function.prototype.call`, the `map` callback and `toJSON`/replacer, and story:test262-run-path runs its workers on exactly `min_stack_bytes()`, so this story, which depends on both, must run `min_stack_bytes_is_enough` again with re-entry through those builtins and correct the constants if it fails"
```
