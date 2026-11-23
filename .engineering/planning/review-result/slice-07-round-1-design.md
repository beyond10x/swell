---
format: aep.planning-md/3
id: review-result:slice-07-round-1-design
kind: review-result
status: active
title: 'Slice 7, round 1, the design critic: the first story'
relations:
- reviews: story:generator-functions-and-suspended-frames
revision: 1
---
needs-revision
story:generator-functions-and-suspended-frames — The Size line says no part of the story can be shown alone, but the parser part can. The grammar, the [Yield] flag, the early errors and the syntax fixtures read no instruction. Move the refusal out of `function_head` to after `early::check`, and give `Expr::Yield` a `CompileError::Tree` arm (that error already means "never for a parsed script"). Then the parser part is shown on its own by the list's 115 negative parse cases passing (218 runs; on the base 208 of them end on the refusal). So cut it into a first story. It needs none of the six design points of "Suspended frames" and can start now. Add a `depends_on` edge to it from the rest. The rest stays one story: the compiler, `suspend.rs`, the generator objects and the builtins, because nothing but `yield` suspends and nothing but `next`/`return`/`throw` resumes. — .engineering/planning/story/generator-functions-and-suspended-frames.md:145, crates/syntax/src/lib.rs:268, crates/syntax/src/parser/function.rs:45, crates/compiler/src/lib.rs:57
story:generator-functions-and-suspended-frames — The story holds [[GeneratorContext]] as `Option<SuspendedFrame>` directly inside `ObjectKind`. That puts a whole `Frame` (an `Rc`, two `Vec`s, two `Value`s and eleven more fields) into the enum, so it becomes the largest kind and every object's arena slot, charged at `size_of::<Slot<ObjectRecord>>()`, grows. The body neither boxes the frame, as `RegExpObject` boxes its `Program`, nor states the record size before and after. The acceptance runs `host_objects::host_187` but not `host_186_a_host_function_charges_every_byte`, which holds `HOST_OBJECT_LIVE_BYTES` (256). The `Writes:` line leaves out `crates/swell/host/limits.rs` and `crates/swell/tests/host_objects.rs`, which the larger record would change. — .engineering/planning/story/generator-functions-and-suspended-frames.md:137, crates/swell/src/interp/frame.rs:110, crates/swell/src/heap/arena.rs:69, crates/swell/src/host/limits.rs:83, crates/swell/tests/host_objects.rs:1029, crates/swell/src/object/regexp.rs:32

**What I read:** 1 artifact plus the parent epic, with `aep plan artifact show`, `relations`, `graph` and `validate` (validate says valid).
- **Edges:** I walked 3, outside the set too: the story `decomposes` the epic, the epic `depends_on` slice 6, and slice 8 `depends_on` slice 7. There is no cycle.
- **Acceptance reads:** I traced 5 reads (the list file, the base run, `requirements.txt`, `status.json` and the test262 fixtures). Each is produced by the coordinator or by the story itself, so none needs an edge.
- **Code checked against `Writes:`:** the matches over `Expr`, `Instruction`, `ObjectKind`, `KindTag`, `BuiltinId`, `Intrinsic` and `Flow`, `KIND_SITES`, the `language.rs` directory read, and the construction sites of `Function` and `FunctionTemplate`. Apart from finding 2, the line holds every file.

**What I could not establish:**
- **Sizes:** they are estimated from the field lists, not measured, because there was no build. Even so, `Frame` is well over the largest current kind (`EcmascriptFunction`, about 80 bytes).
- **Parser stack bound:** whether `yield yield … 0` and nested generator expressions fit `STACK_BYTES_PER_LEVEL` (224, `crates/syntax/src/lib.rs:80`) is unknown. The story does not list `lib.rs` and does not say what happens if they don't fit.
- **The rule against splitting:** AGENTS.md says "a story is never split to gain parallelism". The split in finding 1 is for a check partway through and for not waiting on the design paragraph. Whether that counts as an exception is the coordinator's call.
- **Out of my lane (acceptance):**
  - The suspended-generator termination test does not name all three terminations (step budget, memory budget, interrupt), only the running case does.
  - Check (b) prints `0` over every run rather than selecting runs by the list first. It does state the base value, 4872.

I made no scratch files.

```findings
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 145
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The Size line says no part of the story can be shown alone, but the parser part can. The grammar, the [Yield] flag, the early errors and the syntax fixtures read no instruction. Move the refusal out of function_head to after early::check, and give Expr::Yield a CompileError::Tree arm (that error already means 'never for a parsed script'). Then the parser part is shown on its own by the list's 115 negative parse cases passing (218 runs; on the base 208 of them end on the refusal). So cut it into a first story. It needs none of the six design points of 'Suspended frames' and can start now. Add a depends_on edge to it from the rest. The rest stays one story: the compiler, suspend.rs, the generator objects and the builtins, because nothing but yield suspends and nothing but next/return/throw resumes."
- file: .engineering/planning/story/generator-functions-and-suspended-frames.md
  line: 137
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The story holds [[GeneratorContext]] as Option<SuspendedFrame> directly inside ObjectKind. That puts a whole Frame (an Rc, two Vecs, two Values and eleven more fields) into the enum, so it becomes the largest kind and every object's arena slot, charged at size_of::<Slot<ObjectRecord>>(), grows. The body neither boxes the frame, as RegExpObject boxes its Program, nor states the record size before and after. The acceptance runs host_objects::host_187 but not host_186_a_host_function_charges_every_byte, which holds HOST_OBJECT_LIVE_BYTES (256). The Writes line leaves out crates/swell/src/host/limits.rs and crates/swell/tests/host_objects.rs, which the larger record would change."
```
