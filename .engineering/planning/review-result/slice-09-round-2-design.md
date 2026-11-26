---
format: aep.planning-md/3
id: review-result:slice-09-round-2-design
kind: review-result
status: active
title: 'Slice 9, round 2, the design critic: needs-revision, 6 findings (6 warnings)'
relations:
- reviews: story:keyed-entry-table
- reviews: story:reflect-namespace
- reviews: story:map-and-set
- reviews: story:proxy-objects-and-access-traps
- reviews: story:weak-map-and-weak-set
- reviews: story:set-methods
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
- reviews: story:map-upsert-and-group-by
- reviews: story:host-object-overrides
- reviews: story:slice-09-conformance-close
revision: 1
---
Plan critic, design, round 2 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `4aca865a`), 2026-11-26.
Verdict: needs-revision, 6 findings, all in mechanisms the revision added; the 8 findings of round 1 are closed. The critic's answer, as returned:

needs-revision

All 8 findings from round 1 are closed. Checking the revised set against the code under `crates/` turned up 6 new defects. All of them sit in mechanisms the revision added.

**Round-1 findings: all 8 closed**
1. Cursors of iterators that never finish: closed. The release is named at `.engineering/planning/story/keyed-entry-table.md:40` and the pass at `.engineering/planning/story/map-and-set.md:71`. New defects 1 to 3 below are in that mechanism.
2. `intern_string` / `atom_of`, with code units charged: closed at `.engineering/planning/story/keyed-entry-table.md:38`. The signatures match `crates/swell/src/heap/string.rs:289,315`.
3. The six checks as functions of (target facts, trap result): closed at `.engineering/planning/story/proxy-objects-and-access-traps.md:74`. New defect 5 below is in that line.
4. The `array.rs` fast path: closed at `.engineering/planning/story/host-object-overrides.md:108` and its Writes at `:121`.
5. Tracing the facts box: closed at `.engineering/planning/story/host-object-overrides.md:109`.
6. "Base count less 5": closed at `.engineering/planning/story/host-object-overrides.md:127`.
7. `Map.groupBy` makes and keeps its result Map first: closed at `.engineering/planning/story/map-upsert-and-group-by.md:49`. That order cannot be observed by a script, and `self.keep` exists at `from_entries.rs:137`.
8. The epic's edge to slice 8: closed. The epic has no relations left (`.engineering/planning/epic/slice-09-collections-proxy-reflect.md:1-9`).

**New findings**

story:map-and-set — The pass that releases cursors cannot be written in `heap/collect.rs` alone, because `Arena` has no way to visit its slots, `Workspace::marked` is private, and `Mark::is_marked` and `Workspace::is_marked` exist only under `#[cfg(test)]`. The body must name a method in `heap/arena.rs` that visits each live, marked record mutably and can ask whether another handle is marked, and add `heap/arena.rs` to its Writes. — `.engineering/planning/story/map-and-set.md:71`, `:83`; `crates/swell/src/heap/arena.rs:148`, `:156`, `:162`; `crates/swell/src/heap/collect.rs:188`

story:keyed-entry-table — The body charges the cursor list per doubling but credits it "by `close_cursor`". If that credit is paid per closed cursor while the list keeps its capacity, repeated open-and-close cycles drain the memory budget below what is really used. The body must say a credit is only the storage actually given back (the list freed when it holds no cursor), and that `release_cursors` frees but never reallocates, so the collector still allocates nothing. — `.engineering/planning/story/keyed-entry-table.md:40`; `crates/swell/src/object/mod.rs:141`

story:set-methods — It opens a cursor on `this` for its walk but does not close it on every exit, normal or abrupt. A cursor with no owner is released by nothing except `close_cursor`, so each `has` callback that throws leaves a cursor on a live Set for ever. The body must close the cursor before the method returns its completion, as `forEach` does, and test it with a `has` callback that throws. — `.engineering/planning/story/set-methods.md:44`; `.engineering/planning/story/keyed-entry-table.md:39`, `:40`

story:set-methods — `union`, `intersection` and `symmetricDifference` collect keys returned by the other set's `keys().next()` into a result that is built "in the order the clause fills it", that is, after the loop. Held in a bare table, those keys are reached by no root while script runs. The body must make the result Set first and keep it, as `story:map-upsert-and-group-by` now does, and add a test with a collection during the loop. — `.engineering/planning/story/set-methods.md:44`; `.engineering/planning/story/map-upsert-and-group-by.md:49`

story:proxy-objects-and-access-traps — It reads the target's descriptor and `[[IsExtensible]]` after every trap and then passes them in. ECMA-262 reads the descriptor for `has` only after a false result, and for `set` and `deleteProperty` only after a true one. It never reads `[[IsExtensible]]` for `get` or `set`, and for `deleteProperty` reads it only after a configurable descriptor. An extra read is visible to the script when the target is itself a Proxy. The body must say each fact is read at the step, and under the condition, of its clause, and each check takes only the facts read so far. — `.engineering/planning/story/proxy-objects-and-access-traps.md:74`

story:proxy-descriptor-prototype-and-extensibility-traps — It reads all target facts after the trap and then runs the checks. In the spec, I1, I6, I17 and I18 are checked before the target is read at all. The target's prototype is read only for a target that is not extensible, and the `getOwnPropertyDescriptor` trap reads `[[IsExtensible]]` only after a descriptor that is configurable. The body must place each read and each check at the step of its clause. — `.engineering/planning/story/proxy-descriptor-prototype-and-extensibility-traps.md:45`, `:46`

**What I read:** all 10 stories in full, my round-1 record, `git diff 66f11d1b..4aca865a` (stories, `check.sh`, `lists.sh`), `revision-1.md`, `critic-brief-r2.md`, and the epic header. Commands, all read-only: `aep plan artifact relations`, `validate` (valid; 4 scope warnings on active stories outside the set) and `graph --format json`. I walked all 284 `depends_on` edges of the whole store through `tsort`: no cycle, including the edges out of the set to `exponentiation-operator` and `slice-04-conformance-close`. I traced 18 acceptance reads to a producer inside the set, and an edge covers every one of them, directly or through other stories. The orders 1 to 5 agree with the edges, and the new edge from weak to proxy costs no order.

Code checked: `heap/arena.rs`, `heap/collect.rs`, `heap/string.rs`, `heap/map.rs`, `object/mod.rs` (`Grown`, `record_grown`, `charge_bytes`, the kind lists), `object/array.rs:322-354`, `host/host_object.rs`, `builtins/object/from_entries.rs:120-175`, the `downcast_ref` precedent at `object/builtin_function.rs:846`, and `Host: 'static`. Changing `KIND_SITES` to `&[&str]` builds as written. None of the mechanisms needs `unsafe` or `std`.

**What I could not establish:**
- Whether `ObjectRecord` stays at 200 bytes once `HostObject` gains a `&'static dyn Any` and a facts box. I was not allowed to build; each story's acceptance runs that test.
- Out of my lane, for the parallel-safety critic: if the arena method of the first finding is added, `heap/arena.rs` gets a third writer (map-and-set, beside weak and host-object-overrides).
- Not raised as a finding: what `release_cursors` does with a cursor that has no owner is unstated in `keyed-entry-table.md:40`. It must keep it. The map-and-set test `for_each_visits_an_entry_added_during_the_loop_and_skips_one_deleted_before_it` should catch a wrong choice in `collect-every-safe-point` mode, but only if the callback allocates enough to reach a safe point that collects.

```findings
- file: .engineering/planning/story/map-and-set.md
  line: 71
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The pass that releases cursors cannot be written in heap/collect.rs alone, because Arena has no way to visit its slots, Workspace::marked is private, and Mark::is_marked and Workspace::is_marked exist only under cfg(test), so the body must name a heap/arena.rs method that visits each live marked record mutably and can ask whether another handle is marked, and add heap/arena.rs to its Writes."
- file: .engineering/planning/story/keyed-entry-table.md
  line: 40
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The cursor list is charged per doubling but credited by close_cursor, which, paid per closed cursor while the list keeps its capacity, drains the memory budget below real use over repeated open-and-close cycles, so the body must say a credit is only the storage actually given back (the list freed when it holds no cursor) and that release_cursors frees but never reallocates."
- file: .engineering/planning/story/set-methods.md
  line: 44
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The walk of this opens a cursor that the body does not close on every exit, and a cursor with no owner is released by nothing but close_cursor, so each has callback that throws leaves a cursor on a live Set for ever; the body must close it before the method returns its completion, as forEach does, and test it with a throwing has callback."
- file: .engineering/planning/story/set-methods.md
  line: 44
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "union, intersection and symmetricDifference collect keys returned by the other set's keys().next() into a result built after the loop, held where no root reaches them while script runs, so the body must make the result Set first and keep it, as map-upsert-and-group-by now does, and add a test with a collection during the loop."
- file: .engineering/planning/story/proxy-objects-and-access-traps.md
  line: 74
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The target's descriptor and IsExtensible are read after every trap and passed in, while ECMA-262 reads the descriptor for has only after false, for set and deleteProperty only after true, never reads IsExtensible for get or set, and for deleteProperty reads it only after a configurable descriptor, so the body must say each fact is read at the step and under the condition of its clause and each check takes only the facts read so far."
- file: .engineering/planning/story/proxy-descriptor-prototype-and-extensibility-traps.md
  line: 45
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "All target facts are read after the trap and then the checks run, while the spec checks I1, I6, I17 and I18 before any read of the target, reads the target's prototype only for a target that is not extensible, and has the getOwnPropertyDescriptor trap read IsExtensible only after a configurable descriptor, so the body must place each read and each check at the step of its clause."
```
