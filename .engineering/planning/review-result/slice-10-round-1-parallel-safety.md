---
format: aep.planning-md/3
id: review-result:slice-10-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 10, round 1, the parallel-safety critic: the four stories of the first wave'
relations:
- reviews: story:job-queue-and-microtask-checkpoint
- reviews: story:async-function-syntax-and-early-errors
- reviews: story:promise-constructor-then-and-jobs
- reviews: story:async-functions-and-await
revision: 1
---
needs-revision
story:async-functions-and-await — Its `Writes:` line leaves out `crates/syntax/src/parser/mod.rs` (the documentation of `held`) and `crates/syntax/src/lib.rs` (the documentation of `parse_script`). `story:async-function-syntax-and-early-errors` writes both to name the `async function` refusal as held back. This story lifts that refusal, so both texts become false. Once `lib.rs` is added to the line, the story must also name it as shared with `story:generator-function-constructor` and `story:identifiers-and-group-names-outside-ascii`. The same kind of stale text is already in the code from slice 7: `lib.rs:146-150` and `parser/mod.rs:80-84` still name the generator refusal — .engineering/planning/story/async-functions-and-await.md:155, .engineering/planning/story/async-function-syntax-and-early-errors.md:73, .engineering/planning/story/async-function-syntax-and-early-errors.md:85
story:async-functions-and-await — Its list of files shared with open stories leaves out `crates/syntax/src/parser/function.rs`. This story writes it (the places that record the held refusal: the function head and `arrow_refusal`, line 573). `story:generator-function-constructor` writes it too (`dynamic_function_text`, line 285, which that story cites as line 245). Both surfaces are cited from both Writes lines. The hunks are separate, so no edge is needed, but the file has to be named — .engineering/planning/story/async-functions-and-await.md:155, .engineering/planning/story/generator-function-constructor.md:67

**What I read:** the 4 draft stories in full, the Writes lines of 15 other open stories, the code at `397ffff`, and the output of `aep plan artifact list`, `graph`, `relations` and `waves`.
- **Where each story lands:** I placed all 4 from what their bodies cite (4 cited, 0 inferred, 0 unplaced).
- **Acceptance reads:** I traced 7 to a producer in the set. All 7 have a `depends_on` edge; one of them (story 4 runs the `jobs::` tests of story 1) is covered through story 3.
- **Pairs inside the wave:**
  - `story:job-queue-and-microtask-checkpoint` and `story:async-function-syntax-and-early-errors` share no file and neither reads what the other writes. They can run at the same time.
  - `story:promise-constructor-then-and-jobs` and `story:async-function-syntax-and-early-errors` also share nothing.
  - Every other pair has an edge.
- **Other `Writes:` claims I checked against the code, all correct:**
  - **Matches with no wildcard:** `Expr` (the five sites), `ObjectKind`/`KindTag` (`object/mod.rs` 219, 240), `Behaviour` (359), `BuiltinId` (the dispatch), `Intrinsic` (`get` and `trace`), `Instruction` (`target` 599, `disasm.rs`, `dispatch.rs` 244).
  - **Single constructions:** `Function` is built only at `parser/function.rs:75`, `FunctionTemplate` only at `compiler/src/function.rs:117`, and `Agent` only in `Agent::new`. The two destructurings of `Agent` both end in `..`, so a new field does not break them.
  - **Collector lists:** `KIND_SITES` has 16 sites and `ROOT_SITES` 6.
  - **Recorded runner lines:** none of story 1's moves. The two `async` cases of `fixtures/tree` fail before and after, and the lines record counts only.
  - **Host surface:** the scan reads the glob `host/*.rs`, so the new `host/jobs.rs` is covered. `surface.txt` is only written at a release; between releases the check compares against `CHANGELOG.md`.

**What I could not establish or is outside my lane:**
- Story 4 names the file of the reaction job's match only as "`ops/promise.rs` or `host/jobs.rs`". That stays open until story 3 lands; the edge covers it.
- Acceptance lane: story 1 adds `Agent::queue_microtask(&Realm, …)`, but its line names `misuse.rs` only for `host_48` and `host_102`. It does not name `agent_entries_refuse_foreign` (`misuse.rs:750`), the list of every `Agent` call that takes a realm or root that HOST-31 and HOST-45 run.
- Scope lane: `website/docs/embedding.md:34` ("promise jobs and the microtask checkpoint are specified and not built yet") becomes false after stories 1 and 3. No story names it, but no story so far has updated the website prose either.

```findings
- file: .engineering/planning/story/async-functions-and-await.md
  line: 155
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Writes line does not hold crates/syntax/src/parser/mod.rs (the documentation of `held`) or crates/syntax/src/lib.rs (the documentation of `parse_script`), which story:async-function-syntax-and-early-errors writes to name the async function refusal as held back and which become false when this story lifts it; with lib.rs on the line, the story must also name it as shared with story:generator-function-constructor and story:identifiers-and-group-names-outside-ascii"
- file: .engineering/planning/story/async-functions-and-await.md
  line: 155
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its list of files shared with open stories leaves out crates/syntax/src/parser/function.rs, which this story writes (the records of the held refusal) and story:generator-function-constructor writes too (dynamic_function_text); the hunks are separate and no edge is needed, but the file is not named"
```
