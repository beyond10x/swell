---
format: aep.planning-md/3
id: review-result:slice-01-design-round-2
kind: review-result
status: active
title: design critic, slice 1 stories, round 2
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
story:heap-values-and-host-surface — its collector leaves "(filled later) the frame stack, realms and job queue" and gives no place where an object kind, an environment or a frame tells the mark phase which handles it holds; story:object-model-and-realms (realms, intrinsics, built-in function data), story:harness-builtins (the target, `this` and arguments of a bound function) and story:interpreter-and-script-entry-points (environments, ECMAScript function data, the frame stack, under its `collect-every-safe-point` acceptance) each create handles the collector must trace, and none of their `Writes:` lines holds `heap/collect.rs` or `object/mod.rs`, so the layout must create a trace function and a root call site per kind and per module, empty until its story fills it, as `object/mod.rs` already does for `[[Call]]` and `[[Construct]]` — .engineering/planning/story/heap-values-and-host-surface.md:124, .engineering/planning/story/heap-values-and-host-surface.md:19, .engineering/planning/story/object-model-and-realms.md:49, .engineering/planning/story/interpreter-and-script-entry-points.md:149, .engineering/planning/story/harness-builtins.md:68

**The dispatch arms you asked about:** the shape holds.
- story:object-model-and-realms owns the internal-method dispatch, and the two arms call fixed signatures in files that story:harness-builtins and story:interpreter-and-script-entry-points then fill on their own (.engineering/planning/story/object-model-and-realms.md:46-54, .engineering/planning/story/harness-builtins.md:68). Neither later story edits `object/mod.rs`, and they share no file in wave 4.
- The arms are not stand-ins. The kind data has no constructor, so nothing can reach an arm before its story lands, and reaching one anyway is a broken invariant that ends in `Terminated(EngineFault)`, never a wrong value.
- The signature is the one that `builtin_function.rs`, built in the same story, already implements, so it is tried before either later story relies on it.
- Each story can still be shown working alone: object-model without the two kinds, harness-builtins through host-function callbacks, the interpreter through scripts.
- The gap is the one in the finding: the arms cover `[[Call]]` and `[[Construct]]` but not tracing.

**Round-1 findings:** both are fixed.
- `string_length` is now checked in `join` and `JSON.stringify`, each with a test (.engineering/planning/story/harness-builtins.md, Context paragraph on `string_length`; tests `join_past_string_length_throws_range_error` and `stringify_past_string_length_throws_range_error`).
- story:host-interface-hardening now measures `min_stack_bytes` again through `call`, `map`, `toJSON` and the replacer (`min_stack_bytes_is_enough_through_builtins`). story:test262-run-path now depends on it.

**What I read:**
- The brief, the 10 stories in full, `aep plan artifact graph` and `validate` (50 artifacts, valid), plus the harness files and design sections I had read in round 1.
- **Edges:** I walked 58, inside and outside the set, not counting the `reviews` edges. There is no cycle. The waves are A; S, H; C, O; E, B; X; R, F, and no two stories in the same wave write the same file.
- **Acceptance reads:** I traced 21 to a producer in the set, and an edge records all 21, directly or through an intermediate story. They include:
  - the chain of requirement counts, 17 → 26 → 54 → 68;
  - the 280 → 283 `expected.txt` lines;
  - F's 13 passed scenarios, which rest on the `fault-injection` feature of X.

**What I could not establish:**
- Whether the collector's mark phase is meant to reach the kind data through some generic mechanism that no body names. Nothing in the 10 bodies states one: I searched them for `trace`, `roots` and `filled later`.
- Out of my lane (acceptance): story:ess-conformance-target's count of 13 passed and 48 unsupported rests on its own reading of `.out/suite.json` at this base. I did not run the `jq` command again.
- Scratch: I used `.out/critic-design/` in the store worktree and removed it before reporting.

```findings
- file: .engineering/planning/story/heap-values-and-host-surface.md
  line: 124
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its collector leaves \"(filled later) the frame stack, realms and job queue\" and gives no place where an object kind, an environment or a frame tells the mark phase which handles it holds; story:object-model-and-realms (realms, intrinsics, built-in function data), story:harness-builtins (the target, `this` and arguments of a bound function) and story:interpreter-and-script-entry-points (environments, ECMAScript function data, the frame stack, under its `collect-every-safe-point` acceptance) each create handles the collector must trace, and none of their `Writes:` lines holds `heap/collect.rs` or `object/mod.rs`, so the layout must create a trace function and a root call site per kind and per module, empty until its story fills it, as `object/mod.rs` already does for `[[Call]]` and `[[Construct]]`"
```
