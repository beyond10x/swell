---
format: aep.planning-md/3
id: review-result:slice-08-round-1-scope
kind: review-result
status: active
title: 'Slice 8, round 1, the scope critic: needs-revision, 3 findings'
relations:
- reviews: story:new-target-meta-property
- reviews: story:super-property-and-super-syntax
- reviews: story:class-syntax-and-early-errors
- reviews: story:class-definition-evaluation
- reviews: story:class-element-syntax-and-early-errors
- reviews: story:derived-classes-and-super-call
- reviews: story:public-class-fields
- reviews: story:class-static-blocks
- reviews: story:private-names-and-private-methods
- reviews: story:instance-elements-initialization-order
- reviews: story:private-fields-and-private-in
- reviews: story:eval-in-class-field-initializers
- reviews: story:private-names-in-eval-code
- reviews: story:host-ensure-can-add-private-element
- reviews: story:slice-08-conformance-close
revision: 1
---
Plan critic, scope, round 1 on the fifteen draft stories of `epic:slice-08-classes`
(`plan/slice-08` at `999e949e`), 2026-11-26.
Verdict: needs-revision, 3 findings. The critic's answer, as returned:

needs-revision

story:private-names-and-private-methods — The list has 4 cases that call `__lookupGetter__` or `__lookupSetter__`, which `conformance/test262/later-parts.tsv` assigns to slice 18 and which no crate defines on the base: `test/language/{statements,expressions}/class/elements/private-{getter,setter}-is-not-a-own-property.js`, line 36 of each. No slice 8 story builds them, so they belong on `close-later.tsv` moving to slice 18 (epic acceptance 4: "moved by a rule line ... none is skipped silently", `.engineering/planning/epic/slice-08-classes.md:95`). `scan.awk:77` has to read the tokens of `later-parts.tsv` (or at least the slice-18 ones), and the list (1,981 → 1,977), `close-later.tsv` and the close's "`13 5`, `all 5`" (`slice-08-conformance-close.md:46,69,85`) change to match. — `.engineering/planning/story/private-names-and-private-methods.md:69`

story:derived-classes-and-super-call — The list has 16 cases that subclass `Map`, `Set`, `WeakMap` or `WeakSet`: `{statements,expressions}/class/subclass-builtins/subclass-{Map,Set,WeakMap,WeakSet}.js` and `statements/class/subclass/builtin-objects/{Map,Set,WeakMap,WeakSet}/*.js`. Slice 9 builds those, and they are absent on the base. The story says "Subclassing a builtin ... needs nothing more than this" (line 67) and forbids writes to `crates/swell/builtins/`. The scan missed them because its L9 rule reads only text (`new Map`, `Map.`), never `extends Map` and never the features `Map`/`Set`/`WeakMap`/`WeakSet`. Fix: add those features (plus `Proxy`, `Reflect`, `Reflect.construct`) and `extends (Weak)?(Map|Set)` to `scan.awk:75`, then move the 16 to `close-slice-9.cases.txt` (38 → 54; derived 356 → 340). — `.engineering/planning/story/derived-classes-and-super-call.md:73`

story:instance-elements-initialization-order — Two of its outcomes are already claimed by stories of order 3, so each will be marked done twice:
- "The seam runs the methods loop to its end before the first field initializer" (line 48) is also in `private-names-and-private-methods.md:87` and `:99` (methods loop before the fields loop).
- "After `super()` it runs for the constructor whose SuperCall it is ... or the function an arrow's `this` comes from" (line 48) is also in `derived-classes-and-super-call.md:69` and `:89`.

Either this story's Change keeps only what is new (the combined cases, a return before `super()`, an abrupt initializer after BindThisValue), or the two earlier stories drop those sentences. — `.engineering/planning/story/instance-elements-initialization-order.md:48`

**What I read:** the epic first (32 promises listed before any story), then all 15 stories in full, `docs/plan/slice-08/{lists.sh,scan.awk,part.sh,check.sh}` and the list files, and `waves-section.md`. Against them: `engine.md` (§12 item 8, decision 3 item 1 on derived arrows, the slices block, line 5821), `host-interface.md` (HOST-119, decision 6), `requirements.txt`, `waves.md` "What later slices take over", `later-parts.tsv`, `features.txt`, `hooks.rs:194`, and the slice-10 round-1 scope review. Counting, all read-only:
- Every list equals its `scan.tsv` key exactly. 9,302 unique paths, no duplicates, no blank lines, no `other` key.
- I recomputed the slice-8 set independently: the design's `slices.awk` and block over `.cache/inventory/meta.tsv`, plus `moves.tsv`. It equals `scan.tsv`'s 9,302 paths with zero difference, and agrees with `expected.txt:230` (`9302 18455 ... 5 9297`).
- I sampled the scan's assignments. Spot checks were clean: the subclass, super and new.target directories, the 38 slice-9 cases (each really uses Proxy or Reflect), no private name on a list without private parts, no excluded feature on any story list, no Intl, Temporal, `$262.agent`, WeakRef or BigInt calls. The two misses are above.

**Promises:** 32 extracted, 32 accounted for:
- **28 traced to a story.** Among them:
  - HOST-119 and its stale-handle requirement (`host-ensure-can-add-private-element`, which follows decision 6 at `host-interface.md:677`).
  - The "Each slice" refusal check (in 4 stories).
  - The "Slice 6" class global binding (`class-definition-evaluation`).
  - The decision 3 arrow `this` slot (`derived-classes-and-super-call`).
- **3 named out by the drafter:** the epic counts, `AGENTS.md`/`README.md` numbers and the async class cases. The close hands these to the coordinator (`slice-08-conformance-close.md:54,73`).
- **1 answered by the design:** the open question on the 39 features (`engine.md:5821`).
- **1 narrowed silently:** the partition. 20 cases sit on lists whose stories cannot pass them (findings 1 and 2).

The "Slices 2, 9 and 11" stale-handle entry does not name slice 8. Taking it for this hook still traces to decision 6, so it is not reach beyond the parent. Nothing excluded (decorators, `nonextensible-applies-to-private`) is drafted.

**What I could not establish, or left out of my lane:**
- **Epic still contradicts the set.** The epic's out-of-scope line 70 ("Class cases that use `async` ... counted in slice 10") contradicts the set, which follows the design's 9,302 cases. The drafter named it; it stays open until the coordinator rewrites the epic.
- **Acceptance lane:** several existing tests assert the slice-8 refusal texts (`crates/swell/tests/async_methods.rs:101`, `async_generator_methods.rs:232`, `dynamic_function.rs:808,1228`). No story's Writes line or nextest filter names `async_methods::`, `async_generator_methods::` or (for the lifting story) `dynamic_function::`.
- **Acceptance lane:** whether `check.sh` and `lists.sh` hold up when the coordinator takes a case off a list.
- I wrote nothing. One `find` I started in the background (looking for a runs file) finished with no use.

```findings
- file: .engineering/planning/story/private-names-and-private-methods.md
  line: 69
  category: scope
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The list holds 4 cases that call __lookupGetter__ or __lookupSetter__ (slice 18 in later-parts.tsv, absent on the base): test/language/{statements,expressions}/class/elements/private-{getter,setter}-is-not-a-own-property.js; scan.awk:77 must read the slice-18 tokens of later-parts.tsv, the 4 cases move to close-later.tsv (to 18), and the counts of this story (1,981 to 1,977) and of the close (slice-08-conformance-close.md:46,69,85) follow."
- file: .engineering/planning/story/derived-classes-and-super-call.md
  line: 73
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The list holds 16 cases that subclass Map, Set, WeakMap or WeakSet (subclass-builtins/subclass-{Map,Set,WeakMap,WeakSet}.js in statements and expressions, subclass/builtin-objects/{Map,Set,WeakMap,WeakSet}/*.js), parts of slice 9 absent on the base, because scan.awk:75 matches slice-9 parts by text only and misses 'extends Map' and the features Map, Set, WeakMap, WeakSet; add those features and that pattern to the L9 rule and move the 16 to close-slice-9.cases.txt (38 to 54, derived 356 to 340)."
- file: .engineering/planning/story/instance-elements-initialization-order.md
  line: 48
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "This story claims the methods-before-fields order of the seam and the seam's call after super() for the SuperCall's constructor (arrow case included), which private-names-and-private-methods.md:87 and derived-classes-and-super-call.md:69 and :89 already claim; keep only the outcomes new to this story (combined cases, return before super(), abrupt initializer after BindThisValue) or drop the sentences from the earlier two."
```
