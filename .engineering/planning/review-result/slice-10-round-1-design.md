---
format: aep.planning-md/3
id: review-result:slice-10-round-1-design
kind: review-result
status: active
title: 'Slice 10, round 1, the design critic: the four stories of the first wave'
relations:
- reviews: story:job-queue-and-microtask-checkpoint
- reviews: story:async-function-syntax-and-early-errors
- reviews: story:promise-constructor-then-and-jobs
- reviews: story:async-functions-and-await
revision: 1
---
needs-revision

story:async-functions-and-await — Its `Writes:` line gives "`crates/swell/src/ops/promise.rs` or `crates/swell/src/host/jobs.rs`" as the file for the arm of an `await`. But the Promise story puts a match over the handler of a reaction in each file: the realm of a reaction job in NewPromiseReactionJob (ops), and the run of the job in the checkpoint loop (jobs.rs). Also, PerformPromiseThen (ops) takes handlers as values, and the `await` handler is not one. So the line should name both files, or the Promise story should put every match over the handler in `object/promise.rs` and say so. — .engineering/planning/story/async-functions-and-await.md:139, .engineering/planning/story/async-functions-and-await.md:155, .engineering/planning/story/promise-constructor-then-and-jobs.md:88, .engineering/planning/story/promise-constructor-then-and-jobs.md:90

**Is four the right cut:** yes. Each story can be shown on its own:
- **Job queue:** host microtasks, HOST-77/79/81/149, and the runner fixtures, with 160 `$DONE` runs moving.
- **Syntax:** its 138 negative parse cases. This is the form slice 7 accepted.
- **Promise:** about 1,700 lines, but one abstraction. Cutting the constructor and `resolve` apart from `then` would leave the reaction record, the thenable job, its hooks and its charge in both halves, which is a split abstraction rather than two stories.
- **Async functions:** the consumer's `fetch_json` shape.

No story needs a later story to be shown.

**Edges:**
- No cycle and no unexplained chain. Each of the 3 `depends_on` edges has a reason written in the body.
- `async-functions-and-await` reaches the job queue story through the Promise story, so it needs no edge of its own.
- `aep plan artifact waves` orders the set as the edges say.

**Design preconditions:** each of the 9 design changes is named by the story that needs it. Nothing in the set builds what "Suspended frames" items 1 to 6 forbid. The activation as holder, the frame in a box, the local slot and the credit at the sweep all match items 2, 3 and 6.

**What I read:** the 4 stories whole, the epic, and the slice 7 design review results for rounds 1 to 3, with `aep plan artifact show`, `relations`, `graph`, `validate` (it says valid) and `waves`.
- I walked 52 graph edges, including ones outside the set (the slice 7 and slice 5 stories and both epics).
- I traced 14 acceptance reads to their producers. The 4 that a sibling in the set produces are covered by an edge (directly or through the Promise story).
- I checked against the code: `engine.md` "Suspended frames"; `suspend.rs`, `agent.rs` (`begin`, `charge_steps`, `outermost_entry`, `trace_roots`) and `hooks.rs`; the matches over `Behaviour` and `ObjectKind`; the `Agent` destructuring patterns (they end in `..`); the requirements line format; and the runner's harness loading.
- A count over the base copy: 14 passing cases outside `built-ins/Promise` mention `Promise`, all negative parse cases, so the Promise story needs no guard list.

**What I could not establish:**
- **Pre-existing, outside the set:** the edge `epic:slice-10-promises-and-async depends_on epic:slice-09-collections-proxy-reflect` contradicts the delivery order (10 before 9). `waves` ignores it, so it does not hold the stories back.
- **Out of my lane (acceptance):**
  - No test in `story:job-queue-and-microtask-checkpoint` or `story:promise-constructor-then-and-jobs` reaches `reentry_depth` through a job run from `Cx::perform_microtask_checkpoint`.
  - The job queue story has no test of a host microtask that queues itself without end; the Promise story's HOST-80 covers it only once that story lands.
- **Out of my lane (design of one item):** `story:job-queue-and-microtask-checkpoint` does not say whether the values handed out through each microtask's `Cx` (`Agent::scope`, a root) are released after each entry. As one outermost entry, the checkpoint would keep them until it returns.
- **Out of my lane (scope):** the 3 `Function.prototype.toString` cases on the async functions list need the function's span to start at `async`. Neither story says so.
- I made no scratch files.

```findings
- file: .engineering/planning/story/async-functions-and-await.md
  line: 155
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Writes line names crates/swell/src/ops/promise.rs or crates/swell/src/host/jobs.rs for the arm of an await, but story:promise-constructor-then-and-jobs puts a match over the handler of a reaction in each (the realm of a reaction job in NewPromiseReactionJob, ops; its run in the loop of the checkpoint, jobs.rs) and PerformPromiseThen in ops takes the handlers as values, which an await does not pass, so the line names both files or the story of Promise states that every match over the handler lies in object/promise.rs"
```
