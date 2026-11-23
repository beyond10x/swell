---
format: aep.planning-md/3
id: review-result:slice-07-round-1-scope
kind: review-result
status: active
title: 'Slice 7, round 1, the scope critic: the first story'
relations:
- reviews: story:generator-functions-and-suspended-frames
revision: 1
---
needs-revision

story:generator-functions-and-suspended-frames — The body makes [[GeneratorContext]] an inline `Option<SuspendedFrame>` holding the whole `Frame` (about 208 bytes: 16 fields, with `Handle` 12 bytes and `Value` 16), and it never says the frame is boxed. Today's largest kind (`EcmascriptFunction`, `BuiltinFunction`) is about 100 bytes, so every `ObjectRecord` slot would grow by about 100 bytes. That moves `HOST_OBJECT_LIVE_BYTES` (256) and `HOST_FUNCTION_LIVE_BYTES` (576), which `host_objects::host_186_a_host_function_charges_every_byte` asserts. The Writes line does not hold `crates/swell/src/host/limits.rs` and the acceptance filter does not run host_186. The fix is one of two: state that `SuspendedFrame` boxes the `Frame`, as `RegExpObject` boxes its `Program`, and add `test(/^host_objects::host_186/)` to the acceptance; or add `host/limits.rs` and the changelog's "Host interface" entry to the Writes line. — .engineering/planning/story/generator-functions-and-suspended-frames.md:135,137 (`crates/swell/src/object/regexp.rs:32`, `crates/swell/src/host/limits.rs:78-83`, `crates/swell/tests/host_objects.rs:961-1030`)

**What I read:** 1 drafted artifact against its parent. Commands: `aep plan artifact show epic:slice-07-generators` (read first, promises listed before the story), `aep plan artifact show story:generator-functions-and-suspended-frames`, `aep plan artifact graph` (no other item decomposes the epic). I also read `docs/plan/slice-07/{scan.awk,scan.tsv,story-generators.cases.txt}`, `docs/plan/waves.md`, `docs/plan/unit-invariants.md`, engine.md §12 item 7 and line 4045, host-interface.md HOST-73 and §10, and the code behind each item on the Writes line.

**Promises:** the epic makes 21 and all 21 are accounted for.
- **12 traced to this story:** `yield` saves and resumes the frame; `%GeneratorPrototype%`; declarations; expressions; the frame traced through its holder; the step budget; the memory budget; open question 1 (how a suspended frame is charged and freed), settled in "Bounds"; and the 4 constraints, carried by the unit invariants and by the clause citations.
- **8 named out to later stories in its "Out of scope" (line 108):** `yield*`, `GeneratorFunction`, generator methods, and acceptance items 1 to 5 (the close).
- **1 answered by the design:** the open question on the 39 features with no `swell-slices` line. engine.md:4045 says those features are "small additions inside the directory's own slice", as in the slice-06 scope round.

**Coverage:** no gap, no reach beyond the parent, no outcome claimed twice, no promise silently narrowed.

**Writes line against the code:** the exhaustive matches over `Expr`, `Instruction`, `ObjectKind`/`KindTag`, `BuiltinId`, `Intrinsic` and `Flow`, the struct literals of `Function`/`FunctionTemplate`, `KIND_SITES`, the generated `tests/main.rs`, and the directory-read fixtures of syntax, compiler, language and test262 all fall in listed files. The `Frame` fields, `Frame::trace` and `Agent::credit` are `pub(crate)` already, so leaving `frame.rs` and `agent.rs` out holds. The only miss is the finding above.

**What I could not establish, or left out of my lane:**
- **The split (the brief's first question; dividing work is outside my lane, so it does not set my verdict).** The claim "none of its parts could be shown alone" (line 145) does not hold for the parser part. If the refusal of `function*` moved after the early errors, the 115 negative parse cases on the list (line 62) and the syntax fixtures could pass without the compiler. Such a part would still run before the rest, in series, so the only gain is a checkpoint halfway. The suspended frame and the builtins cannot be shown without compiled generators, so those two stay together.
- **The Symbol sweep reaches none of the three new functions (acceptance lane).** `functions_of_the_realm` (`crates/swell/tests/builtins_symbol_values.rs:325-421`) walks from the global object. No global property leads to `%GeneratorPrototype%`. So "run over the three new functions" (line 141) is a no-op unless the walk gets another starting point, and that is more than the "rows" the Writes line names.
- **Stale epic (coordinator's rewrite):** it still carries open question 2 and the counts 1,432/2,771, while the base reads 1,491/2,841. Not a finding against the set.
- **The 2 `cross` cases** in `scan.tsv` are claimed by no named later story yet. That belongs to the later waves.
- I created no scratch under `.out/critic-scope/` and wrote nothing.

```findings
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 137
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The body makes [[GeneratorContext]] an inline `Option<SuspendedFrame>` holding the whole `Frame` (about 208 bytes: 16 fields, with `Handle` 12 bytes and `Value` 16), and it never says the frame is boxed. Today's largest kind (`EcmascriptFunction`, `BuiltinFunction`) is about 100 bytes, so every `ObjectRecord` slot would grow by about 100 bytes. That moves `HOST_OBJECT_LIVE_BYTES` (256) and `HOST_FUNCTION_LIVE_BYTES` (576), which `host_objects::host_186_a_host_function_charges_every_byte` asserts. The Writes line does not hold `crates/swell/src/host/limits.rs` and the acceptance filter does not run host_186. The fix is one of two: state that `SuspendedFrame` boxes the `Frame`, as `RegExpObject` boxes its `Program`, and add `test(/^host_objects::host_186/)` to the acceptance; or add `host/limits.rs` and the changelog's \"Host interface\" entry to the Writes line."
```
