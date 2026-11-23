# Design: the host interface of swell

Written on 2026-11-21, before any code. It replaces the prose of decision 9 of
`docs/design/engine.md` with an interface a Rust implementor can write from. The first ESS
model of the interface (`spec/`, ess 0.96.0) states the lifecycles of agent, realm, script,
module, module load and job, and the rule that a faulted agent refuses every call; this
document is the specification of everything else. `docs/design/host-interface-model.md`
records what that model measured.

The document has 19 numbered decisions. Each gives the choice, the reason from the standard's
text (quoted, with the clause anchor), and what was rejected; the Rust signatures follow the
choice, and the requirements follow the signatures. What the texts cannot settle is under
"Open". A statement that could not be checked says "not checked".

## How to read the requirements

A requirement is one line of this form, and no other line starts with `**HOST-`:

`**HOST-<n>** (<source>) <one sentence an implementation can break, with the outcome a test checks> [slice <t>]`

- `<n>` counts from 1 and is never reused. HOST-1 to HOST-135 are in document order; a
  requirement added later takes the next free number where it is added.
- `<source>` is `ecma262:<anchor>` (an `id` of the pinned draft, `https://tc39.es/ecma262/#<anchor>`),
  `html:<anchor>` (of `multipage/webappapis.html`), `webidl:<anchor>`, or `swell:D<k>` where
  the requirement follows from decision k of this document and from no clause.
- `[slice <t>]` is the delivery slice of `engine.md` section 12 that first has to meet it.

Names used in requirements: `agent` is an `Agent<H>`, `cx` a `Cx` or `HookCx`, "the host" the
`H: Host` value the agent owns. "A script `S`" means `parse_script` of the source `S` in a
realm of the agent, then `evaluate_script` of the result.

The texts read, all fetched on 2026-11-21: ECMA-262 at `74c33d4`
(`.cache/ecma262/74c33d406ec1763ca9ee5ad9165571661649d6e6/`, as in `engine.md`); the HTML
Standard's "Web application APIs" page, "Last Updated 20 November 2026"
(`.cache/html-standard/webappapis.txt`); Web IDL, "Last Updated 14 November 2026"
(`.cache/webidl/webidl.txt`); surf's `AGENTS.md` ("Goals", "Components") and
`conformance/Cargo.toml` and `src/`. Clause numbers below are those of the draft; the draft
numbers HostPromiseRejectionTracker 27.5.1.9 (`sec-host-promise-rejection-tracker`).

## 1. The host type and where code may run

**The agent owns the host value; a hook is an associated function of the `Host` trait that
receives a context, not `&mut self`. Two context types: `Cx` can run ECMAScript code, `HookCx`
can read, create values, parse and finish module loads but runs no code.**

Why: a hook must act on the engine while the engine is inside one of its algorithms. HTML's
HostLoadImportedModule finishes the load inside the hook in its error cases: "Perform
FinishLoadingImportedModule(referrer, moduleRequest, payload, ThrowCompletion(a new
TypeError)). Return." (`html:hostloadimportedmodule`), and the draft allows it: the host "must
perform FinishLoadingImportedModule(...) ... either synchronously or asynchronously"
(`ecma262:sec-HostLoadImportedModule`). HTML's HostCallJobCallback runs code: "Let result be
Call(callback.[[Callback]], V, argumentsList)" (`html:hostcalljobcallback`). A hook taking
`&mut self` on a host stored in the agent cannot also hold the agent. Hooks that the draft
calls in the middle of an algorithm and that "return unused" (the job and load hooks, the
rejection tracker) get `HookCx`, so no host can run script in the middle of, for example,
RejectPromise; the type refuses it before it runs.

Rejected: the host passed to every entry point (`agent.evaluate_script(&mut host, ...)`): a
hook still cannot reach the agent. A `Box<dyn Host>` with `&self` hooks: no mutable host state
without `RefCell`, and still no agent. A run-time refusal of code inside hooks: the type does
it at compile time.

```rust
pub struct Agent<H: Host> { /* private */ }
pub trait Host: Sized + 'static { /* the hooks, decision 16 */ }
pub struct Cx<'a, H: Host> { /* private: the running agent and its host */ }
pub struct HookCx<'a, H: Host> { /* private: the agent inside a hook that runs no code */ }

impl<H: Host> Agent<H> {
    pub fn host(&self) -> &H;
    pub fn host_mut(&mut self) -> &mut H;
    /// A context for creating and converting values while no code runs. Calls made through it
    /// are outermost entries (decision 2); `realm` is their current realm (decision 7).
    pub fn with_cx<R>(&mut self, realm: &Realm, f: impl FnOnce(&mut Cx<'_, H>) -> R) -> Result<R, HostError>;
}
impl<'a, H: Host> Cx<'a, H> {
    pub fn host(&self) -> &H;
    pub fn host_mut(&mut self) -> &mut H;
}
impl<'a, H: Host> HookCx<'a, H> {
    pub fn host(&self) -> &H;
    pub fn host_mut(&mut self) -> &mut H;
}
```

`with_cx`, `host` and `host_mut` run no code. Every entry point of `Agent` takes `&mut self`,
so a call on the agent from code running inside it does not compile; inside, the host reaches
the engine only through `Cx` or `HookCx`.

**HOST-1** (swell:D1) The `swell` crate and every crate it depends on declare `#![forbid(unsafe_code)]`, name no package from outside the repository in the root `Cargo.lock`, and never call `std::thread::spawn` or `std::thread::Builder`, which a source scan in the gate checks. [slice 1]

**HOST-2** (swell:D1) The engine crates use none of `std::time::SystemTime`, `std::time::Instant`, `std::env`, `std::fs`, `std::net`, `std::process` and `std::collections::hash_map::RandomState`, which a source scan in the gate checks. [slice 1]

**HOST-3** (swell:D1) No entry point, read or hook dispatch panics for any argument: a fuzz test that issues seeded random sequences of interface calls with arbitrary arguments (stale, foreign and wrong-kind handles included) counts a panic or an abort as a failure. [slice 1]

**HOST-4** (swell:D1) `HookCx` has no method that calls a function, constructs or evaluates a script, which a `compile_fail` test of each such call holds. [slice 1]

**HOST-149** (swell:D1) `HookCx` has no method that performs a microtask checkpoint, which a `compile_fail` test holds. [slice 10]

**HOST-150** (swell:D1) `HookCx` has no method that evaluates a module, which a `compile_fail` test holds. [slice 11]

**HOST-151** (swell:D1) `HookCx` has no method that runs a job, which a `compile_fail` test holds. [slice 14]

**HOST-5** (swell:D1) A call on `&mut Agent` made from inside a host function or hook of the same agent does not compile, which a `compile_fail` test holds. [slice 1]

**An `Agent` is not `Send`; its `InterruptHandle` is.** Decided 2026-11-21 (was open item B,
option (a)). An agent is created, run and dropped on one thread; only the interrupt reaches it
from another (decision 12).

Why: the known need, as of 2026-11-21: surf does not move a page's agent between threads. It is
one process whose core runs on one thread, and no workers are planned. Going to `Send` later
changes no parameter or result of an entry point, read or hook. What changes inside is the root
handle type: its release queue (decision 4) goes from an `Rc`'d queue to an `Arc`'d one, with
an atomic operation on every drop. What a host sees beside that is bounds: `Send` on `H`, on
`HostData` (decision 15) and on `Microtask<H>` (decision 11), and `HostDefined` going from `Rc`
to `Arc`, so a host holding values that are not `Send` would then not compile.

Rejected: `Send` now (option (b)): an atomic operation on every root drop, for a need no user
has.

**HOST-136** (swell:D1) `Agent<H>` is not `Send` and `InterruptHandle` is `Send`, which a compile-time test holds: a `compile_fail` test that passes an `Agent` to a function requiring `Send`, and a static assertion that `InterruptHandle: Send`. [slice 1]

## 2. Creating an agent, and its limits

**`Agent::new` takes the host value, the limits and the seeds (decision 3). Each limit has a
unit, a meaning at 0, and one outcome when it is reached. 0 is a limit of zero, never "no
limit"; a host that wants no practical limit gives the type's maximum.** This answers the
author's finding 9.

```rust
pub struct Limits {
    pub steps_per_entry: u64,
    pub memory_bytes: u64,
    pub frame_depth: u32,
    pub reentry_depth: u32,
    pub parser_nesting: u32,
    pub string_length: u64,
    pub bigint_bits: u64,
}
impl Limits {
    /// The Rust stack, in bytes, a thread must have for an agent with these limits.
    pub const fn min_stack_bytes(&self) -> usize;
}
pub enum CreateError { AgentKeysExhausted }

impl<H: Host> Agent<H> {
    pub fn new(host: H, limits: Limits, seeds: Seeds) -> Result<Agent<H>, CreateError>;
}
```

| limit | unit | what it counts | reaching it ends in | at 0 |
|---|---|---|---|---|
| `steps_per_entry` | steps (decision 3 of `engine.md`: one per bytecode instruction, plus builtin charges and what a host function charges with `cx.charge_steps`, decision 14) | steps of one outermost entry, with every nested entry and every job of one checkpoint | `Terminated(StepBudget)` | an entry that would spend a step ends before spending it |
| `memory_bytes` | bytes charged by the accounting of `engine.md` decision 4 | the agent's heap over its life, and the bytes the host charged with `charge_host_bytes` while it holds their `HostBytes` (decision 14), read by `heap_bytes()`; a sweep credits what freed records own, and an arena's capacity stays charged | `Terminated(MemoryBudget)`; `RangeError` from the `ArrayBuffer`, `SharedArrayBuffer` and typed array constructors | `create_realm` ends in `Terminated(MemoryBudget)` |
| `frame_depth` | execution contexts | ECMAScript execution contexts on the engine's frame stack | a `RangeError` thrown where the context would be pushed | every entry that pushes a context answers `Throw(RangeError)` |
| `reentry_depth` | entries | calls from Rust into code (an ECMAScript function, a script or a host function) made while code runs, by a builtin or by host code, nested beyond the outermost entry | a `RangeError` thrown at the call that would re-enter | no callback runs from a builtin or host function |
| `parser_nesting` | levels | nesting levels of the recursive-descent parser | a `SyntaxError` whose message names `parser_nesting` (below) | every non-empty source fails to parse |
| `string_length` | UTF-16 code units | the length of a String created at run time from other values | a `RangeError` | every such operation with a non-empty result throws |
| `bigint_bits` | bits | the bit length of a BigInt's magnitude computed or converted at run time | a `RangeError`, before computing when the size is known in advance | only `0n` results |

An "entry" is a call of an `Agent` entry point, or a `Cx` call made while no ECMAScript code
runs (`with_cx`). An entry made by host code that runs inside another entry (a host function,
a hook, a host microtask) is nested: it spends from the outermost entry's budget, since a
nested entry that got a fresh budget would let a script loop for ever through one host
function. `string_length` is a String's length as 6.1.4 measures it: "up to a maximum length
of 2^53 - 1 elements" (`ecma262:sec-ecmascript-language-types-string-type`); a value above
2^53 - 1 acts as 2^53 - 1. A String from source text or from the host is bounded by the memory
budget, not by `string_length`.

A builtin charges steps in proportion to its work and before it (`engine.md` decision 3, "The
step budget"); added on 2026-11-22: a comparison of two Strings and a String made a property key
charge a step per code unit past 32 before the units are read, and compare in place (HOST-181,
HOST-182).

**Two readings of the heap.** Added on 2026-11-22 for resource accounting. `heap_bytes()`
(decision 13) reads what the memory budget has charged: the arenas' capacity with the
collector's workspace and the atom table's capacity, which stay charged while the agent lives,
what the records own, and what a running step holds charged (`engine.md` decision 4, "Memory
budget"). `memory_bytes` bounds this reading: an allocation that would take it past
`memory_bytes` is refused. `live_bytes()` reads what the records the heap holds own, each one's
slot included, and what the atom table's entries own, leaving out free slots and empty buckets;
after a collection these are the live records, so it falls by everything a sweep frees, while `heap_bytes()` falls only
by what the freed records owned. `live_bytes()` is at most `heap_bytes()`. `heap_bytes()` keeps
its name for the charged reading because the requirements and tests that set or check a budget
read it (HOST-10, and the tests that size `memory_bytes` from it); only HOST-29 and HOST-52,
which read what a collection gives back, move to `live_bytes()`. Amended on 2026-11-22 for the
host's own bytes (decision 14): both readings count the bytes the host charged with
`charge_host_bytes` while it holds their `HostBytes`; a dropped `HostBytes` is credited after
the next sweep or at the host's next charge, or at once by `Agent::credit_host_bytes`, which
empties it.

The draft words no allowance for these outcomes: `grep -i 'implementation limit'` over
`ecma262.txt` finds nothing. The depth, string and BigInt `RangeError`s and the parse failure
are deviations of swell's own, to be listed as omissions of the sections they touch
(`AGENTS.md`, "Rules"). The `ArrayBuffer` case is the draft's: "If it is impossible to create
such a Data Block, throw a RangeError exception." (`ecma262:sec-createbytedatablock`).

**Passing `parser_nesting` is a deliberate implementation limit, and it fails with a
`SyntaxError` whose message names the limit.** Decided 2026-11-21 (was open item A, option
(b)). ParseScript, ParseModule and PerformEval keep the result types the draft declares:
ParseScript "returns a Script Record or a non-empty List of SyntaxError objects"
(`ecma262:sec-parse-script`), ParseModule likewise (`ecma262:sec-parsemodule`), and PerformEval says "If script is a List of errors, throw a
SyntaxError exception." (`ecma262:sec-performeval`); CreateDynamicFunction says the same of its
parameters and body (`ecma262:sec-createdynamicfunction`). So `parse_script` and `parse_module`
answer `Parsed::Errors` holding one `SyntaxError`, and `eval` and the `Function` constructors
throw a `SyntaxError`. The deviation is only that a valid program is refused for its depth; a
script tells it from a grammar error by the message alone. The omission lists of the sections it
touches in `conformance/test262/sections.txt` (ParseScript, ParseModule, PerformEval,
CreateDynamicFunction, ParsePattern) record it, with the number of levels the test262 runner
gives `parser_nesting`. Rejected: a `RangeError`, as `engine.md` decision 3 had it: the List
ParseScript returns would hold an object of a type the draft does not allow, and PerformEval's
step would be broken.

Regular expression patterns have no separate bound: `Limits` has no field for them. The pattern
parser of `swell-regexp` counts its own nesting (groups, look-arounds, character classes and,
under `v`, nested class set operands) against `parser_nesting`, from zero at the start of the
pattern, and passing it is the same `SyntaxError`, as ParsePattern returns "a Parse Node or a
non-empty List of SyntaxError objects" (`ecma262:sec-parsepattern`): an early error of the
script or module for a literal, and thrown by the `RegExp` constructor at run time ("If
parseResult is a non-empty List of SyntaxError objects, throw a SyntaxError exception.",
`ecma262:sec-regexpinitialize`).

Limits reached while running and not while parsing (frames, re-entry, `JSON`, regular
expression matching) stay as `engine.md` decision 3 has them.

**What `min_stack_bytes()` covers.** Added on 2026-11-21 for slice 2. It covers the outermost
run, `reentry_depth` re-entries, and at the deepest re-entry one parse at `parser_nesting`
levels through any entry that parses: `cx.parse_script` (HOST-17), `eval` (HOST-152) and the
`Function` constructor (HOST-163), each with the Rust frames between the re-entry and the parser
on its own path. CreateDynamicFunction parses the parameter text, then the body text, then the
whole source text `function anonymous(<parameters>\n) {\n<body>\n}`
(`ecma262:sec-createdynamicfunction`); the three parses run one after another, never one
inside another, so the parser's share is that of one parse, but the path to it (ToString of
each argument, the source text built, the wrapping function expression of the third parse,
which counts as a level like any function) is not `eval`'s, and is measured on its own.
Added on 2026-11-22: it holds in the dev profile with full debug information, in the dev profile
without it, and in the release profile without it, each shown by the stack tests (the
profiles `dev` in the gate, `stack-debug` and `release` in the lane `slow`; `dev` and `release` are
checked to carry no debug section, `stack-debug` to carry it); a
host that builds swell with a profile of its own runs the same tests in it.

`CreateError::AgentKeysExhausted`: every handle carries its agent's key (decision 4), a `u32`
from a process-wide counter; creating more than 2^32 - 1 agents in one process fails.

Rejected: `Option<u64>` per limit, and 0 as "no limit" (a zeroed `Limits` would switch every
limit off; `AGENTS.md`: "Each limit ends in an outcome the host reads"); a wall-clock limit
(`engine.md` decision 3).

**HOST-6** (swell:D2) A script `while (true) {}` with `steps_per_entry` N ends in `Terminated(StepBudget)`, and `last_entry_steps()` then reads at most N. [slice 1]

**HOST-7** (swell:D2) After an entry ended in `Terminated(StepBudget)`, the next outermost entry has the whole budget again: a script of fewer than N steps then answers `Normal`. [slice 1]

**HOST-8** (swell:D2) A host function that evaluates the script `while (true) {}` through `cx.evaluate_script` inside an outermost entry makes that outermost entry answer `Terminated(StepBudget)` with `last_entry_steps()` at most N: a nested entry gets no budget of its own. [slice 1]

**HOST-9** (swell:D2) With `steps_per_entry` 0, the script `1` answers `Terminated(StepBudget)` and `last_entry_steps()` reads 0. [slice 1]

**HOST-10** (swell:D2) A script that allocates without end (`var a = []; while (true) a.push({});`) with `memory_bytes` M and a step budget large enough answers `Terminated(MemoryBudget)`, and `heap_bytes()` then reads at most M. [slice 1]

**HOST-11** (ecma262:sec-createbytedatablock) `new ArrayBuffer(n)` with n larger than what is left of the memory budget throws a `RangeError` the script can catch, and the agent stays `Alive`. [slice 13]

**HOST-12** (swell:D2) With `memory_bytes` 0, `create_realm` answers `Terminated(MemoryBudget)` and the agent stays `Alive`. [slice 1]

**HOST-169** (swell:D2) With `memory_bytes` enough for a flat source's copy and its script record but not for its tree, `parse_script` of that source answers `Parsed::Terminated(MemoryBudget)`, and `heap_bytes()` then reads what it read before the call. [slice 6]

**HOST-171** (swell:D2) At the return of every entry point and of `collect`, `live_bytes()` reads at most `heap_bytes()` and `heap_bytes()` at most `memory_bytes`; with `memory_bytes` M, after the script `(function () { var a = []; while (true) a.push({}); })()` answers `Terminated(MemoryBudget)`, two `collect`s lower `live_bytes()` by more than they lower `heap_bytes()`, and then, with k chosen from the readings so that `live_bytes()` + 2 × k ≤ M < `heap_bytes()` + k and `string_length` at least k, the script `"x".repeat(k).length` answers `Terminated(MemoryBudget)`. [slice 6]

**HOST-175** (swell:D2) A host function that charges 64 bytes with `cx.charge_host_bytes` and keeps the answer with a 64-byte buffer in the host on each call, run by the script `while (true) make();` with `memory_bytes` M and a step budget large enough, ends in `Terminated(MemoryBudget)`, and `heap_bytes()` then reads at most M; after the host drops the buffers with their `HostBytes`, the script `1` answers `Normal(1)`. [slice 6]

**HOST-176** (swell:D2) A `HostBytes` gives back exactly what it charged, once: with L0 the `live_bytes()` before a charge of 100, a host object made after the charge with that `HostBytes` in its host data and no root, and H1 the `heap_bytes()` once the object exists, after `collect()` frees the object `live_bytes()` reads exactly L0 and `heap_bytes()` exactly H1 − 100 − `size_of_val` of the object's host data, and a second `collect()` changes neither; `Agent::credit_host_bytes` of a second charge of 100 answers `Ok(100)`, both readings fall by exactly 100 at the call, and a later `collect()` changes neither; `Agent::credit_host_bytes` of a `HostBytes` of another agent answers `Err(HostError::ForeignHandle)` with this agent's readings unchanged, and the other agent's `live_bytes()` falls by its bytes at its next `collect()`. [slice 6]

**HOST-177** (swell:D2) A host function that calls `cx.charge_steps(1_000)` on each call, run by the script `while (true) walk();` with `steps_per_entry` N, ends in `Terminated(StepBudget)` with `last_entry_steps()` at most N; one that calls `cx.charge_steps(N + 1)` ends the entry at its first call with `last_entry_steps()` below N; an interrupt set before the charge ends it in `Terminated(Interrupted)`. [slice 6]

**HOST-181** (swell:D2) With `a` and `b` two Strings of 1,000,000 code units that differ only in the last, defined as globals by the host, the script `a === b` spends at least 999,968 steps more than the script `"x" === "y"`, and with `steps_per_entry` 10,000 it answers `Terminated(StepBudget)`; the same holds for `a < b` (1,000,000 steps), `a == b` and `[a].indexOf(b)`. [slice 6]

**HOST-182** (swell:D2) With a global object `o` and a String `k` of 1,000,000 code units defined by the host, and a `collect()` before the entry, the script `o[k]` spends at least 999,968 steps more than the same script with a key of one code unit, and with `steps_per_entry` 10,000 it answers `Terminated(StepBudget)` with `heap_bytes()` after the entry equal to `heap_bytes()` before it (no atom made). [slice 6]

**HOST-13** (swell:D2) With `frame_depth` D, the script `var n = 0; function f() { n++; f(); } f();` answers `Throw` of a `RangeError` of its realm, the script `n` then answers `Normal(D - 1)`, that script's own context being the first, and the script `try { f(); } catch (e) {}` answers `Normal`. [slice 1]

**HOST-14** (swell:D2) With `frame_depth` 0, the script `1` answers `Throw` of a `RangeError` of its realm. [slice 1]

**HOST-15** (swell:D2) With `reentry_depth` R, a host function that calls itself through `cx.call` counting its calls gets `Abrupt::Throw` of a `RangeError` from the call that would be re-entry R + 1, after R calls have run. [slice 1]

**HOST-16** (swell:D2) With `reentry_depth` 0, the script `[1].map(function (x) { return x; })` answers `Throw` of a `RangeError`. [slice 1]

**HOST-17** (swell:D2) On a thread whose stack has `limits.min_stack_bytes()` bytes, a script that reaches `reentry_depth` re-entries and at the deepest one calls a host function that parses a source nested `parser_nesting` levels through `cx.parse_script` ends without a stack overflow. [slice 1]

**HOST-152** (swell:D2) On a thread whose stack has `limits.min_stack_bytes()` bytes, a script that reaches `reentry_depth` re-entries and at the deepest one calls `eval` on a source nested `parser_nesting` levels ends without a stack overflow. [slice 2]

**HOST-163** (swell:D2) On a thread whose stack has `limits.min_stack_bytes()` bytes, a script that reaches `reentry_depth` re-entries and at the deepest one calls `Function(p, b)` with a parameter text `p` and a body text `b` each nested `parser_nesting` levels ends without a stack overflow, in `Normal` or in `Throw` of a `SyntaxError`. [slice 2]

**HOST-18** (swell:D2) A source of 100,000 nested brackets with `parser_nesting` 1,000 makes `parse_script` answer `Parsed::Errors` with one error object and no stack overflow. [slice 1]

**HOST-19** (swell:D2) With `parser_nesting` 0, `parse_script` of any non-empty source answers `Parsed::Errors`. [slice 1]

**HOST-20** (swell:D2) With `string_length` L and `memory_bytes` at least 8 × L, the script `var s = "x"; while (true) s = s + s;` answers `Throw` of a `RangeError` raised by the concatenation whose result would exceed L code units. [slice 1]

**HOST-21** (swell:D2) With `string_length` L, `"x".repeat(L)` returns a String of length L and `"x".repeat(L + 1)` throws a `RangeError`. [slice 3]

**HOST-22** (swell:D2) With `string_length` 0, the script `"a" + "b"` answers `Throw` of a `RangeError`. [slice 1]

**HOST-23** (swell:D2) With `bigint_bits` 2^20, `2n ** (2n ** 40n)` throws a `RangeError` and `last_entry_steps()` reads less than 10,000: the size is refused before computing. [slice 12]

**HOST-24** (swell:D2) With `bigint_bits` 0, the script `1n + 1n` answers `Throw` of a `RangeError` and `0n + 0n` answers `Normal(0n)`. [slice 12]

**HOST-137** (ecma262:sec-parse-script) With `parser_nesting` 1,000, `parse_script` of `0` inside 2,000 nested parentheses answers `Parsed::Errors` with one object whose prototype is the realm's `%SyntaxError.prototype%` and whose `message` contains `parser_nesting`, and of `0` inside 10 nested parentheses answers `Parsed::Parsed`. [slice 1]

**HOST-138** (ecma262:sec-parsemodule) With `parser_nesting` 1,000, `parse_module` of `export default 0` with the `0` inside 2,000 nested parentheses answers `Parsed::Errors` with one `SyntaxError` whose `message` contains `parser_nesting`. [slice 11]

**HOST-139** (ecma262:sec-performeval) With `parser_nesting` 1,000 and `s` a string of `0` inside 2,000 nested parentheses built by a loop, the script `try { eval(s); } catch (e) { e instanceof SyntaxError && e.message.indexOf("parser_nesting") >= 0 }` answers `Normal(true)`. [slice 2]

**HOST-140** (ecma262:sec-performeval) With the same `parser_nesting` and `s`, the indirect eval `(0, eval)(s)` throws a `SyntaxError` whose `message` contains `parser_nesting`, which the script catches. [slice 2]

**HOST-141** (ecma262:sec-createdynamicfunction) With the same `parser_nesting` and `s`, `Function("return " + s)` throws a `SyntaxError` whose `message` contains `parser_nesting`, which the script catches. [slice 2]

**HOST-142** (ecma262:sec-parsepattern) With `parser_nesting` 1,000, `parse_script` of a regular expression literal of 2,000 nested groups `/((...(a)...))/` answers `Parsed::Errors` with one `SyntaxError` whose `message` contains `parser_nesting`, and the literal of 10 nested groups parses. [slice 5]

**HOST-143** (ecma262:sec-regexpinitialize) With `parser_nesting` 1,000, `new RegExp(p)` with `p` a pattern of 2,000 nested groups throws a `SyntaxError` whose `message` contains `parser_nesting`, which the script catches. [slice 5]

## 3. Seeds, and where randomness comes from

**Two seeds are given at creation, and nothing else gives the engine randomness: the trait has
no random-bytes method. `hash_key` keys the SipHash of the engine's own hash maps;
`random_seed` seeds `Math.random`, each realm's generator taking the agent's seed and the
realm's creation index as its stream, so two realms of one agent never produce one
sequence.** This answers finding 8.

Why: ECMA-262 needs randomness in one place, `Math.random`, and requires per realm: "Each
Math.random function created for distinct realms must produce a distinct sequence of values
from successive calls." (`ecma262:sec-math.random`). The hash key is not observable to script,
because property and collection order is insertion order (`ecma262:sec-ordinaryownpropertykeys`),
so it only decides which keys collide. With both seeds fixed at creation, an agent's run is a
function of its inputs (decision 19); a hook called whenever the engine wanted fresh bytes
would add calls whose number depends on internal events such as map resizes. SipHash takes a
128-bit key, hence 16 bytes; 32 bytes for the generator is the state size of the generator
`swell-num` will choose, an upper bound it may use less of (not chosen yet; not checked).

Rejected: decision 9's "random bytes" from the trait; one seed for both purposes (a script that
reads enough `Math.random` output must learn nothing about the hash key; two independent seeds
make that hold without an argument about the derivation).

```rust
pub struct Seeds {
    pub hash_key: [u8; 16],
    pub random_seed: [u8; 32],
}
```

**HOST-25** (swell:D3) A script that inserts 10,000 keys into an object, a `Map` and a `Set` and prints them in iteration order prints the same bytes in two agents that differ only in `hash_key`. [slice 9]

**HOST-26** (swell:D3) The first 1,000 values of `Math.random()` in the first realm of two agents with the same `random_seed` are equal bit for bit. [slice 4]

**HOST-27** (ecma262:sec-math.random) The first 64 values of `Math.random()` in two realms of one agent differ as sequences. [slice 4]

A browser's `crypto.getRandomValues` comes from the host later, as an operation of a host
object (decision 15) that draws on the host's own randomness; it is not an engine hook.

**HOST-144** (ecma262:sec-math.random) The first 64 values of `Math.random()` in the first realm of two agents that differ only in `random_seed` differ as sequences, and are equal bit for bit in two agents that differ only in `hash_key`. [slice 4]

**HOST-145** (swell:D3) The hasher of the engine's hash maps is keyed by `hash_key` alone: a unit test of the map type gets equal hashes for each of 1,000 keys under two equal `hash_key`s, and a different hash for at least one of them under two different ones. [slice 1]

**HOST-146** (swell:D3) `Agent::new` is the only function of the interface that takes `Seeds`, and no other function, read or hook takes or returns seed or random bytes, which a scan of the `swell` crate's public items in the gate checks. [slice 1]

**HOST-147** (swell:D3) Two processes that each create an agent with the same `Seeds` and run the same script, printing 1,000 `Math.random()` values and the key order of a `Map` and an object of 10,000 keys each, print the same bytes, which a test that starts two child processes checks: the engine reads no randomness but its seeds. [slice 4]

## 4. Values at the boundary

**Values are a `Copy` enum over typed handles (decision 4 of `engine.md`); every handle carries
its agent's key, an index and a generation. A value handed to host code is valid for the
scope it was handed in (a host function call, a hook, a `with_cx` closure); to keep it longer
the host makes a `Root`. A value read from a root with `get` is handed out to the running scope
like any other. Realms, scripts and modules are held through roots. Strings cross in
two conversions, exact UTF-16 and lossy to a Rust `String`; source text crosses as UTF-8 or
UTF-16.**

Why: the scope keeps its values in a frame the collector scans (`engine.md` decision 4); a
handle kept past its scope is found by the generation check, never by reading the wrong object.
Added on 2026-11-22 (HOST-178): a handle the host keeps past its scope, with no `Root` and
listed by no `trace`, does not keep its record alive. While the record lives, every call with
the handle answers as it would inside the scope; once a collection freed it, every call answers
`Err(StaleHandle)`, never another record, because a generation is never used twice. A host that
keeps such a handle checks it before it returns it to the engine (HOST-160) and never lists it
in a `trace` (HOST-110). What keeps an object alive for the host is a `Root` (decision 4), or a
handle that host data the engine reaches lists in its `trace` from the scope it was handed in
onward (decision 15); such a handle is strong, not kept in this way.
The agent key lets the engine answer a handle of another agent (finding 5). The two string
conversions are Web IDL's: a `DOMString` is any sequence of code units (`webidl:idl-DOMString`)
and a `USVString` has no lone surrogate (`webidl:idl-USVString`). Source text: ParseScript
takes "sourceText (a String or a sequence of Unicode code points)" (`ecma262:sec-parse-script`);
HTML's script text can hold a lone surrogate (a DOM string set by script), which a `&str`
cannot, so UTF-16 is accepted beside UTF-8.

Rejected: lifetimes on `Value` tied to the context (`Value<'cx>`): it forbids no misuse the
generation check does not already turn into an error, and every host type that stores a value
would need the lifetime; roots for every value (a root-table entry per argument of every call).

```rust
#[derive(Clone, Copy)]
pub enum Value {
    Undefined, Null, Boolean(bool), Number(f64),
    String(StringRef), Symbol(SymbolRef), BigInt(BigIntRef), Object(ObjectRef),
}
#[derive(Clone, Copy, PartialEq, Eq)] pub struct ObjectRef { /* agent key, index, generation */ }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct StringRef { /* agent key, index, generation */ }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct SymbolRef { /* agent key, index, generation */ }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct BigIntRef { /* agent key, index, generation */ }
pub enum PropertyKey { String(StringRef), Symbol(SymbolRef) }

/// A value the host keeps across calls. Neither `Clone` nor `Copy`; dropping it releases it.
pub struct Root { /* agent key, root-table index, generation, release queue */ }
pub struct Realm(Root);
pub struct Script(Root);
pub struct Module(Root);
#[derive(Clone, Copy, PartialEq, Eq)] pub struct RealmRef { /* as ObjectRef */ }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct ScriptRef { /* as ObjectRef */ }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct ModuleRef { /* as ObjectRef */ }

pub enum Source<'s> { Utf8(&'s str), Utf16(&'s [u16]) }
pub enum ErrorKind { Error, EvalError, RangeError, ReferenceError, SyntaxError, TypeError, UriError }

impl<H: Host> Agent<H> {
    pub fn duplicate(&mut self, root: &Root) -> Result<Root, HostError>;
}
impl<'a, H: Host> Cx<'a, H> {   // the same methods on HookCx
    pub fn root(&mut self, value: Value) -> Result<Root, HostError>;
    pub fn get(&mut self, root: &Root) -> Result<Value, HostError>;
    pub fn realm_ref(&self, realm: &Realm) -> Result<RealmRef, HostError>;
    pub fn new_string(&mut self, text: &str) -> Completion<StringRef>;
    pub fn new_string_utf16(&mut self, units: &[u16]) -> Completion<StringRef>;
    pub fn to_utf16(&self, string: StringRef) -> Result<Vec<u16>, HostError>;
    pub fn to_rust_string_lossy(&self, string: StringRef) -> Result<String, HostError>;
    pub fn new_error(&mut self, realm: RealmRef, kind: ErrorKind, message: &str)
        -> Result<Completion<ObjectRef>, HostError>;
}
```

A `Root` dropped by the host is pushed on a release queue the agent drains at its next safe
point, so dropping needs no access to the agent. `Root`, and with it `Agent`, is therefore not
`Send` (decision 1).

`get` takes `&mut self` because it records the value it reads in the running scope, as
`new_string` records the String it makes. A host that reads a value with `get`, drops the only
root of it and calls back into the engine in the same scope (a safe point drains the release
queue and may collect) still holds a valid value until the scope ends. Changed on 2026-11-22
from `&self`, a breaking change of the interface. Handing a value out to a scope charges
nothing to either budget and changes neither `heap_bytes()` nor `live_bytes()`: the scope's list
is the agent's storage for the host's own calls, as the root table is, it grows by one entry per
value handed out and ends with the scope.

**HOST-28** (swell:D4) An object held only by a `Root` survives `collect`, and the root then reads the same object. [slice 1]

**HOST-29** (swell:D4) After the host drops the last `Root` of an object that nothing else reaches, the second `collect` after the drop reports a lower `live_bytes()` than before the drop. [slice 1]

**HOST-30** (swell:D4) A handle whose object was freed, or a released root, given to any entry point or read answers `Err(HostError::StaleHandle)`, changes nothing and leaves the agent `Alive`. [slice 1]

**HOST-170** (swell:D4) A host function that reads an object with `cx.get` from the only root that holds it, drops that root, calls through `cx.call` a function whose allocation starts a collection, and returns the object, answers `Normal` with that object, and the agent stays `Alive`. [slice 6]

**HOST-31** (swell:D4) A handle or root of another agent given to any entry point or read answers `Err(HostError::ForeignHandle)` and changes nothing. [slice 1]

**HOST-178** (swell:D4) A host object made in one `with_cx` and kept by the host as an `ObjectRef` with no root, stored by a script in a global, answers its brand in a later `with_cx` and after `collect`; once the script deleted the global and `collect` ran, `brand` of the kept handle answers `Err(HostError::StaleHandle)`, also after 1,000 new host objects were made in the freed slots, and `state()` reads `Alive`. [slice 6]

**HOST-32** (webidl:idl-DOMString) `to_utf16` returns every code unit of a String exactly, lone surrogates included: `new_string_utf16(&[0xD800, 0x61])` read back with `to_utf16` gives `[0xD800, 0x61]`. [slice 1]

**HOST-33** (webidl:idl-USVString) `to_rust_string_lossy` replaces each lone surrogate with U+FFFD and keeps every other code point: the String `[0xD800, 0x61, 0xD83D, 0xDE00]` gives `"\u{FFFD}a\u{1F600}"`. [slice 1]

**HOST-34** (swell:D4) `new_string` of a `&str` gives the String of its UTF-16 encoding, which `to_utf16` returns unchanged. [slice 1]

**HOST-35** (ecma262:sec-source-text) `parse_script` of the same text given as `Source::Utf8` and as `Source::Utf16` produces scripts whose evaluation answers the same value, and a UTF-16 source holding a lone surrogate inside a string literal yields a String holding that code unit. [slice 1]

## 5. Outcomes and the result type of each entry point

**Each entry point returns `Result<_, HostError>`: `Err` for a call the host should not have
made (decision 6), `Ok` with a result type of its own, taken from the draft's return type.
`Outcome<T>` is `Normal(T)`, `Throw(Root)` or `Terminated(Termination)`; parsing returns
`Parsed<T>`; inside a scope the same calls return `Completion<T>` over scoped values, inside
`Result<_, HostError>` when they take a handle (below).** This
answers finding 3.

| entry point | returns | from the draft |
|---|---|---|
| `create_realm` | `Outcome<Realm>` | InitializeHostDefinedRealm "returns either a normal completion containing unused or a throw completion" (`ecma262:sec-initializehostdefinedrealm`) |
| `parse_script` | `Parsed<Script>` | ParseScript "returns a Script Record or a non-empty List of SyntaxError objects" (`ecma262:sec-parse-script`) |
| `parse_module` | `Parsed<Module>` | ParseModule (`ecma262:sec-parsemodule`), the same shape |
| `parse_json_module` | `Outcome<Module>` | ParseJSONModule (`ecma262:sec-parse-json-module`) |
| `evaluate_script` | `Outcome<Root>` | ScriptEvaluation "returns either a normal completion containing an ECMAScript language value or an abrupt completion" (`ecma262:sec-runtime-semantics-scriptevaluation`) |
| `load_requested_modules` | `Outcome<Root>`, the Promise | LoadRequestedModules "returns a Promise" (`ecma262:sec-LoadRequestedModules`) |
| `link` | `Outcome<()>` | Link "returns either a normal completion containing unused or a throw completion" (`ecma262:sec-moduledeclarationlinking`) |
| `evaluate_module` | `Outcome<Root>`, the Promise | Evaluate "returns a Promise" (`ecma262:sec-moduleevaluation`) |
| `finish_loading_imported_module` | `Outcome<()>`, never `Throw` | FinishLoadingImportedModule "returns unused" (`ecma262:sec-FinishLoadingImportedModule`) |
| `run_job` | `Outcome<()>` | a Job "must return a normal completion" (`ecma262:sec-jobs`); `Throw` only for a cleanup job (decision 11) |
| `perform_microtask_checkpoint` | `Outcome<CheckpointReport>`, never `Throw` | decision 11 |
| `call`, `construct` | `Outcome<Root>` | Call, Construct (`ecma262:sec-call`, `ecma262:sec-construct`) |
| `collect` | `Outcome<CollectReport>`, only `Terminated(EngineFault)` besides `Normal` | decision 18 |

Why: a parse has no value to give as `Normal(value)`, a module's exception never arrives as a
throw (Evaluate rejects its promise: "Perform ! Call(promiseCapability.[[Reject]], undefined,
« result.[[Value]] »)", `ecma262:sec-moduleevaluation`), and realm creation can throw and can
pass the memory budget. One `Outcome` for all, as decision 9 has it, would leave the host to
handle cases that never occur and to guess the meaning of `Normal`.

Rejected: one `Outcome` for every entry point (decision 9's text).

```rust
pub enum Termination { StepBudget, MemoryBudget, Interrupted, EngineFault }
pub enum Outcome<T> { Normal(T), Throw(Root), Terminated(Termination) }
pub enum Parsed<T> { Parsed(T), Errors(Vec<Root>), Terminated(Termination) }
pub enum Abrupt { Throw(Value), Terminated(Termination) }
pub type Completion<T> = Result<T, Abrupt>;

pub enum HostError {
    Faulted,
    StaleHandle,
    ForeignHandle,
    WrongKind { expected: ValueKind },
    ModuleStatus { found: ModuleStatus },
    ModuleTerminated,
    EvaluationInProgress,
}
pub enum ValueKind { Function, Constructor, Object, Promise, String }
```

**A method that takes a handle returns `Result<_, HostError>`.** Every method with a parameter
that can carry a handle from the host (a `Value`, `PropertyKey`, `ObjectRef`, `StringRef`,
`RealmRef`, `ScriptRef` or `ModuleRef`, a `Root`, `Realm`, `Script` or `Module`, a `Job` or
`LoadRequest`, or a struct holding one) returns `Result<_, HostError>` (`FinishError` for a
finish, decision 9), so that decision 6's answers for a stale handle, a handle of another agent
and a wrong kind have a place; its `Ok` holds what it returns otherwise: `Outcome<T>`,
`Parsed<T>` or a read's value on `Agent`, `Completion<T>` on `Cx` and `HookCx`, which gives
`Result<Completion<T>, HostError>` as `Agent` has `Result<Outcome<T>, HostError>`. A method that
takes no handle (`new_string`, `new_promise_capability`, `cx.perform_microtask_checkpoint`)
keeps its type. The handles are checked after the `Faulted` check (decision 6) and before the
call does anything else, before the sticky
termination of decision 10 too: a stale handle given while a run is terminated answers
`Err(StaleHandle)` and the run stays terminated. HOST-30 and HOST-31 cover every such method,
those of `Cx` and `HookCx` included. The rule is for handles the host passes in as arguments (a
handle it returns: decision 6); a
handle the engine reads from its own heap, host data listed by `trace` included, is not one, and
a stale one there is a broken engine invariant (`Terminated(EngineFault)`, HOST-110). Rejected: a
`HostError` case in `Abrupt` (`Abrupt` is how a run completes; a host function could hand the
host's mistake on to the script as its own completion).

**HOST-36** (ecma262:sec-parse-script) `parse_script` of `var 1;` answers `Parsed::Errors` with at least one object whose prototype is the realm's `%SyntaxError.prototype%`, and no code runs. [slice 1]

**HOST-37** (ecma262:sec-runtime-semantics-scriptevaluation) The script `throw 1` answers `Throw` whose root reads the Number 1, and the script `1 + 1` answers `Normal` whose root reads 2. [slice 1]

**HOST-38** (ecma262:sec-moduleevaluation) `evaluate_module` of a linked module whose body is `throw 1` answers `Normal` with a promise that `promise_state` reads as `Rejected(1)`, never `Throw`. [slice 11]

**HOST-39** (ecma262:sec-ContinueModuleLoading) `load_requested_modules` never answers `Throw`: a load finished with a thrown value rejects the promise it returned with that value. [slice 11]

**HOST-40** (ecma262:sec-moduledeclarationlinking) `link` of a module that imports a name its dependency does not export answers `Throw` of a `SyntaxError`, and `module_status` then reads `Unlinked` for that module. [slice 11]

**HOST-41** (ecma262:sec-initializehostdefinedrealm) `create_realm` whose global object refuses the definition of a clause 19 property (a host object whose `define_own_property` override returns `false`) answers `Throw` of a `TypeError`. [slice 9]

## 6. Misuse ends in a defined answer

**Every call the draft "only asserts away", and every handle the call should not get, ends in
`Err(HostError::...)`, changes nothing and leaves the agent usable. A load cannot be finished
twice: the token is moved into the call. A faulted agent answers every entry point and every
read but `state()` and `refusals()` (decision 13) with `Err(HostError::Faulted)`.** This
answers finding 5.

| call | the draft | answer |
|---|---|---|
| `link` on a module whose [[Status]] is `new` | Link step 1: "Assert: module.[[Status]] is one of unlinked, linked, evaluating-async, or evaluated." (`ecma262:sec-moduledeclarationlinking`) | `Err(ModuleStatus { found: New })` |
| `evaluate_module` on `new` or `unlinked` | Evaluate step 2: "Assert: module.[[Status]] is one of linked, evaluating-async, or evaluated." (`ecma262:sec-moduleevaluation`) | `Err(ModuleStatus { found })` |
| `cx.evaluate_module` while another Evaluate runs in the agent | Evaluate step 1: "Assert: This call to Evaluate is not happening at the same time as another call to Evaluate within the surrounding agent." | `Err(EvaluationInProgress)` |
| `evaluate_module` on a module whose graph holds a module cut by a termination | none (decision 10) | `Err(ModuleTerminated)` |
| any handle of another agent | an agent's constituents "belong exclusively to that agent" (`ecma262:sec-agents`) | `Err(ForeignHandle)` |
| a freed handle or a released root | none | `Err(StaleHandle)` |
| a value of the wrong kind where the interface needs a kind (`promise_state` of a non-promise) | none | `Err(WrongKind { expected })` |
| finishing a load twice | "The host environment must perform FinishLoadingImportedModule" once per call (`ecma262:sec-HostLoadImportedModule`) | does not compile (decision 9) |
| finishing a (referrer, request) pair already loaded with another module | "Assert: record.[[Module]] and result.[[Value]] are the same Module Record." (`ecma262:sec-FinishLoadingImportedModule`) | `FinishError { error: ModuleMismatch, request }`, the token handed back |
| finishing a `type: "json"` request with a module not made by `parse_json_module` | "result must either be the Completion Record returned by an invocation of ParseJSONModule or a throw completion" (`ecma262:sec-HostLoadImportedModule`) | `FinishError { error: NotJsonModule, request }` |

`call` of a value that is not callable is not misuse: Call throws ("If IsCallable(F) is false,
throw a TypeError exception", `ecma262:sec-call`), so it answers `Throw(TypeError)`.

Rejected: a panic, an `EngineFault`, or undefined behaviour for a host mistake in a call: the
host's mistake is not the engine's broken invariant, and `engine.md` decision 4 keeps
`EngineFault` for the latter.

**A handle the host returns.** Every handle in a value the host returns to the engine (the `Ok`
or `Abrupt::Throw` value of a `HostFn`, the `GlobalObjects` of a `GlobalSetup`, the result of an
override, the answer of a hook such as `host_get_code_for_eval` or
`host_get_import_meta_properties`) is checked before the engine uses it. A stale handle, a handle
of another agent, or one of the wrong kind where the engine needs a kind ends the run with
`Terminated(EngineFault)`: the agent is faulted, `state()` then reads `Faulted`, and every
entry point and read but `state()` and `refusals()` answers `Err(HostError::Faulted)` (HOST-48). No call is there to answer
`Err` to, and the run cannot go on with a value it cannot read. Decision 15's sentence on an
override result is this rule applied to overrides, and a stale handle listed by `trace` answers
the same (HOST-110). A value the engine does not use, such as what a host function returns in a
terminated run (decision 10), is not checked.

**HOST-160** (swell:D6) A host function whose `Ok` value or `Abrupt::Throw` value holds a stale handle or a handle of another agent ends the run with `Terminated(EngineFault)`, and a `GlobalSetup` closure whose `GlobalObjects` hold one ends its `create_realm` call with `Terminated(EngineFault)`; in both cases `state()` then reads `Faulted`, every entry point and read but `state()` and `refusals()` answers `Err(HostError::Faulted)`, and nothing panics. [slice 1]

The other return sites (an override's result, `host_get_import_meta_properties` and the other
hooks whose answer holds a handle) get their own requirement when their slice is decomposed.
Slice 2's two, both reached from `eval` or `Function`, are HOST-162: the `Some` of
`host_get_code_for_eval`, an `Option<StringRef>` that cannot throw, so only a stale or a
foreign String handle can reach the engine; and the thrown value of an `Abrupt::Throw` from
`host_ensure_can_compile_strings`, whose answer is a `Completion<()>` (decision 16).

**HOST-162** (swell:D6) A `host_get_code_for_eval` whose `Some` value is a stale String handle or a String handle of another agent ends the run of the script that called `eval(o)` with `Terminated(EngineFault)`, and a `host_ensure_can_compile_strings` whose `Abrupt::Throw` value holds a stale handle or a handle of another agent ends the run of the script that called `eval` or `Function` with `Terminated(EngineFault)`; in both cases no code of a string runs, `state()` then reads `Faulted`, every entry point and read but `state()` and `refusals()` answers `Err(HostError::Faulted)`, and nothing panics. [slice 2]

**HOST-42** (ecma262:sec-moduledeclarationlinking) `link` of a module whose status is `New` answers `Err(HostError::ModuleStatus { found: New })` and `module_status` still reads `New`. [slice 11]

**HOST-43** (ecma262:sec-moduleevaluation) `evaluate_module` of a module whose status is `New` or `Unlinked` answers `Err(HostError::ModuleStatus { found })` with that status and runs no module code. [slice 11]

**HOST-44** (ecma262:sec-moduleevaluation) A host function called by module code during `evaluate_module` that calls `cx.evaluate_module` of any module answers `Err(HostError::EvaluationInProgress)`, and the outer evaluation continues. [slice 11]

**HOST-45** (swell:D6) A `Realm` of agent A given to `parse_script` of agent B answers `Err(HostError::ForeignHandle)`; so does a `Root` or `Script` of A given to any call of B. [slice 1]

**HOST-153** (swell:D6) A `Module` or `LoadRequest` of agent A given to any call of agent B answers `Err(HostError::ForeignHandle)`. [slice 11]

**HOST-154** (swell:D6) A `Job` of agent A given to any call of agent B, `run_job` included, answers `Err(HostError::ForeignHandle)`. [slice 14]

**HOST-46** (swell:D6) `promise_state` of a root that reads a non-promise answers `Err(HostError::WrongKind { expected: ValueKind::Promise })`. [slice 10]

**HOST-47** (ecma262:sec-call) `agent.call` of a root that reads a non-callable value answers `Throw` of a `TypeError`, and `agent.construct` of a non-constructor answers `Throw` of a `TypeError`. [slice 1]

**HOST-48** (swell:D6) After an entry answered `Terminated(EngineFault)`, `state()` reads `Faulted` and every entry point and read of the agent but `state()` and `refusals()` answers `Err(HostError::Faulted)` without calling a hook. [slice 1]

Inside a scope still on the Rust stack when the agent faults (a host function whose returned
handle faulted it, HOST-160, say), HOST-48 holds for every scoped method whose type carries
`HostError`: it answers `Err(HostError::Faulted)` before it checks its handles. A scoped method
whose type is `Completion<T>` alone (`new_string`, `new_string_utf16`,
`new_promise_capability`, `cx.perform_microtask_checkpoint`) cannot carry it and answers
`Err(Abrupt::Terminated(EngineFault))`, as decision 10's sticky termination says; neither kind
runs code or calls a hook.

## 7. Realms

**`create_realm` performs InitializeHostDefinedRealm. The host may give a setup closure that
builds the global object and the global `this` value after the realm's intrinsics exist and
before the clause 19 properties are defined, and a `[[HostDefined]]` value. A realm ends when
nothing reaches it: there is no destroy call.**

Why: the draft creates the global "in a host-defined manner" between CreateIntrinsics and
SetDefaultGlobalBindings: "If the host requires use of a specific object to serve as realm's
global object, then Let global be such an object created in a host-defined manner." ... "If
the host requires that the this binding in realm's global scope return an object other than the
global object, then Let thisValue be such an object created in a host-defined manner." ...
"Perform ? SetDefaultGlobalBindings(realm)." (`ecma262:sec-initializehostdefinedrealm`). HTML
uses those customisations for `Window` and `WindowProxy` ("Perform
InitializeHostDefinedRealm() with the provided customizations for creating the global object
and the global this binding", `html:creating-a-new-javascript-realm`). The global's prototype
chain needs the new realm's intrinsics, so the closure runs inside the operation, and it reaches
the one it needs through `Prototype::ObjectPrototypeOf(realm)` of a `HostObjectSpec` (decision
15) or by `intrinsic`: the closure's `HookCx` reads no property and names only the intrinsics
and symbols of the closed list below. The draft gives a realm no end; a realm is objects and
records in the heap, freed like them.

**The closed list.** Added on 2026-11-22. The host names the intrinsics and well-known symbols
of `HostIntrinsic` and `WellKnownSymbol` (decision 15) through `intrinsic(realm, which)` and
`well_known_symbol(which)` on `Cx` and `HookCx`: a closed list of what Web IDL takes from a realm
for the interfaces a host defines. An entry is added by a decision of this document with its
Web IDL clause and a line of the changelog under "Host interface" ("How the gate holds this
document"); the list grows in no other way. An intrinsic is the realm's original object,
whatever a script did to the globals since; a well-known symbol is the agent's, the same in
every realm.

| entry | what Web IDL takes it for | arrives |
|---|---|---|
| `HostIntrinsic::FunctionPrototype`, `%Function.prototype%` | the [[Prototype]] of an interface object whose interface has no parent: "Let constructorProto be realm.[[Intrinsics]].[[%Function.prototype%]]." (`webidl:interface-object`) | HOST-183 |
| `HostIntrinsic::ErrorPrototype`, `%Error.prototype%` | the [[Prototype]] of `DOMException`'s interface prototype object: "Otherwise, if interface is the DOMException interface, then set proto to realm.[[Intrinsics]].[[%Error.prototype%]]." (`webidl:interface-prototype-object`) | HOST-183 |
| `HostIntrinsic::ObjectPrototype`, `%Object.prototype%` | the [[Prototype]] of any other interface prototype object: "Otherwise, set proto to realm.[[Intrinsics]].[[%Object.prototype%]]." (`webidl:interface-prototype-object`) | HOST-183 |
| `WellKnownSymbol::ToStringTag`, `%Symbol.toStringTag%` | an object's class string: "If an object has a class string classString, then the object must, at the time it is created, have a property whose name is the %Symbol.toStringTag% symbol" (`webidl:dfn-class-string`) | HOST-184 |
| `WellKnownSymbol::Unscopables`, `%Symbol.unscopables%` | the object of an interface's `[Unscopable]` members: "Perform ! DefinePropertyOrThrow(interfaceProtoObj, %Symbol.unscopables%, desc)." (`webidl:interface-prototype-object`) | HOST-184 |
| `WellKnownSymbol::Iterator`, `%Symbol.iterator%` | "Iterable declarations": "If definition has an indexed property getter, then: Perform DefineMethodProperty(target, %Symbol.iterator%, %Array.prototype.values%, false)." (`webidl:define-the-iteration-methods`) | HOST-185, with slice 6's `Symbol` and iterator stories |
| `HostIntrinsic::ArrayPrototypeValues`, `%Array.prototype.values%` | the same step, and `values` of a value iterator (`webidl:define-the-iteration-methods`) | HOST-185, as above |
| `HostIntrinsic::ArrayPrototypeEntries`, `%Array.prototype.entries%` | "If definition has a value iterator, then: Perform ! CreateDataPropertyOrThrow(target, "entries", %Array.prototype.entries%)." (`webidl:define-the-iteration-methods`) | HOST-185, as above |
| `HostIntrinsic::ArrayPrototypeKeys`, `%Array.prototype.keys%` | the same step for "keys" (`webidl:define-the-iteration-methods`) | HOST-185, as above |
| `HostIntrinsic::ArrayPrototypeForEach`, `%Array.prototype.forEach%` | the same step for "forEach" (`webidl:define-the-iteration-methods`) | HOST-185, as above |

The first five are made by the realm today. The last five come with slice 6: `%Symbol.iterator%`
with the story of the `Symbol` constructor and the well-known symbols, `%Array.prototype.values%`
with the story of the iterator protocol, and `entries`, `keys` and `forEach` as intrinsics of the
realm recorded where their builtins are installed, so that each is the original function.

Rejected: a host-made global passed in before the call (its prototype could not come from the
realm's own intrinsics); an explicit `destroy_realm` (objects of a destroyed realm reachable
from another realm would dangle, or the call would have to refuse); an accessor for any
intrinsic of a realm (a door to every intrinsic for one need: an object on `%Object.prototype%`,
on which the host builds the rest of its chain with its own objects).

```rust
pub struct GlobalObjects { pub global: ObjectRef, pub this: Option<ObjectRef> }
pub type GlobalSetup<'g, H> = &'g mut dyn FnMut(&mut HookCx<'_, H>, RealmRef) -> Completion<GlobalObjects>;
pub type HostDefined = std::rc::Rc<dyn HostData>;   // HostData: decision 15

impl<H: Host> Agent<H> {
    pub fn create_realm(&mut self, setup: Option<GlobalSetup<'_, H>>, host_defined: Option<HostDefined>)
        -> Result<Outcome<Realm>, HostError>;
    pub fn global_object(&mut self, realm: &Realm) -> Result<Root, HostError>;
    pub fn realm_host_defined(&self, realm: &Realm) -> Result<Option<HostDefined>, HostError>;
}
```

`$262.createRealm` and same-origin frames are realms of one agent (`engine.md` decision 9).

**The current realm is the running context's; outside a run, the host names it.** The draft's
current Realm Record is "The value of the Realm component of the running execution context"
(`ecma262:sec-execution-contexts`), so inside a run it is that: a host function runs in its own
[[Realm]], a hook in the realm of the code that reached it. Outside a run no context runs, and
the entry decides: `with_cx`, `Agent::call` and `Agent::construct` take a `&Realm` that is current
for the whole call; every other entry point has its realm from its arguments: `create_realm` the
new realm (InitializeHostDefinedRealm: "Set the Realm of newContext to realm. ... Push newContext
onto the execution context stack", `ecma262:sec-initializehostdefinedrealm`), `parse_script`, `parse_module` and `parse_json_module` the
realm given, `evaluate_script` the script's ("Set the Realm of scriptContext to
scriptRecord.[[Realm]]", `ecma262:sec-runtime-semantics-scriptevaluation`),
`load_requested_modules`, `link` and `evaluate_module` the module's [[Realm]],
`finish_loading_imported_module` the referrer's, `run_job` the job's, and
`perform_microtask_checkpoint` that of each job it runs while the job runs: a promise job's
realm, or the realm current when it was queued if its realm is null, and a host microtask's the
realm it was queued with (decision 11, "A host microtask's realm"; added on 2026-11-23). `HookCx` needs none: each of
its methods that makes an object names the realm or the prototype. A `Realm` is a root, so the
answer depends on no collection and no creation order, an agent with no realm cannot reach a call
that needs one, and a stale or foreign `Realm` answers `Err(StaleHandle)` or `Err(ForeignHandle)`
(decision 6). Rejected: the realm created last that no collection has freed yet, with a null
prototype when there is none (the answer would change with when the collector ran, which
decision 19 and the collect-at-every-safe-point mode would observe, and the host could not see
which realm it got).

**HOST-49** (ecma262:sec-initializehostdefinedrealm) The setup closure is called once per `create_realm`, can create, by `new_host_object` with `Prototype::ObjectPrototypeOf` of the realm it receives, an object whose prototype is the new realm's `%Object.prototype%`, and the global it returns has the clause 19 properties the engine defines (with that object returned as the global and no `this`, the script `this.Array === Array && Object.getPrototypeOf(this) === Object.prototype` answers `true` in the realm). [slice 1]

**HOST-50** (ecma262:sec-initializehostdefinedrealm) When the setup returns a `this` object, the script `this` in the realm answers that object. [slice 1]

**HOST-155** (ecma262:sec-initializehostdefinedrealm) When the setup returns a `this` object, `globalThis` in a script of the realm answers that object as well (`globalThis` is the global environment's this value). [slice 3]

**HOST-51** (ecma262:sec-initializehostdefinedrealm) Without a setup closure, the realm's global object is an ordinary object whose prototype is the realm's `%Object.prototype%`. [slice 1]

**HOST-52** (swell:D7) After the host drops every root of a realm and of objects of that realm, and no other realm reaches them, the second `collect` returns `live_bytes()` to the value read before `create_realm`. [slice 1]

**HOST-161** (swell:D7) With realm A created and then realm B, `cx.to_object(Value::Boolean(true))` in `with_cx` of A answers an object whose prototype is A's `%Boolean.prototype%`, and `agent.call` with A of the Number 1 answers `Throw` of a `TypeError` whose prototype is A's `%TypeError.prototype%`; both hold after the host dropped every root of B and with the collector run at every safe point. [slice 1]

**HOST-183** (webidl:interface-object) In the setup closure and in `with_cx`, `intrinsic(realm, FunctionPrototype)`, `intrinsic(realm, ErrorPrototype)` and `intrinsic(realm, ObjectPrototype)` answer the objects the script `Function.prototype`, `Error.prototype` and `Object.prototype` read in that realm, also after a script set the globals `Function`, `Error` and `Object` to `null`; a second realm's answers differ from the first's; a stale realm answers `Err(HostError::StaleHandle)`. [slice 6]

**HOST-184** (webidl:interface-prototype-object) A host object whose own `@@toStringTag` data property, defined with `well_known_symbol(ToStringTag)`, is `"Node"` makes the script `Object.prototype.toString.call(n)` answer `"[object Node]"`; one with an own `@@unscopables` object whose `before` is `true` makes `with (n) { before }` read the outer binding; the symbols are the same in two realms of the agent. [slice 6]

**HOST-185** (webidl:define-the-iteration-methods) A host object `list` with indexed own data properties `0` and `1` holding the host objects `a` and `b`, a `length` of 2, and own data properties `@@iterator`, `entries`, `keys`, `values` and `forEach` defined from `well_known_symbol(Iterator)` and the four intrinsics, makes the script `var r = [], it = list[Symbol.iterator](); r.push(it.next().value === a, it.next().value === b, it.next().done); var e = list.entries().next().value; r.push(e[0] === 0 && e[1] === a, list.keys().next().value === 0, list.values().next().value === a); list.forEach(function (n, i) { r.push(n === (i ? b : a)); }); r.join()` answer `"true,true,true,true,true,true,true,true"` (no `for`-`of`, which a later story of slice 6 brings), also after a script replaced `Array.prototype.values`, `entries`, `keys` and `forEach` before the host defined them; `intrinsic(realm, ArrayPrototypeValues)` answers the same object as `Array.prototype[Symbol.iterator]` read before that script. [slice 6]

## 8. Scripts

**`parse_script` performs ParseScript with the host's `[[HostDefined]]`; `evaluate_script`
performs ScriptEvaluation. A script may be evaluated again: each call runs ScriptEvaluation
again, as the draft allows.**

Why: ScriptEvaluation does not forbid a second evaluation, and the draft defines what it does:
GlobalDeclarationInstantiation throws for a name declared twice ("If HasLexicalDeclaration(envRecord,
name) is true, throw a SyntaxError exception.", `ecma262:sec-globaldeclarationinstantiation`).
HTML makes a new script per run, so surf will not rely on it either way. Rejected: refusing
the second evaluation (a rule the draft does not have).

```rust
impl<H: Host> Agent<H> {   // the same on Cx over scoped handles, the result type by decision 5's rule
    pub fn parse_script(&mut self, realm: &Realm, source: Source<'_>, host_defined: Option<HostDefined>)
        -> Result<Parsed<Script>, HostError>;
    pub fn evaluate_script(&mut self, script: &Script) -> Result<Outcome<Root>, HostError>;
    pub fn script_host_defined(&self, script: &Script) -> Result<Option<HostDefined>, HostError>;
}
```

**HOST-53** (ecma262:sec-parse-script) `parse_script` calls no hook, runs no code and defines no global binding: after `parse_script` of a script that declares `var x`, and before it is evaluated, the script `Object.prototype.hasOwnProperty.call(this, "x")` answers `false` in a realm whose setup returned no `this`. [slice 1]

**HOST-54** (ecma262:sec-globaldeclarationinstantiation) Evaluating one parsed script `var a = 1;` twice answers `Normal` twice. [slice 1]

**HOST-156** (ecma262:sec-globaldeclarationinstantiation) Evaluating one parsed script `let b = 1;` twice answers `Throw` of a `SyntaxError` the second time. [slice 6]

**HOST-55** (ecma262:sec-HostLoadImportedModule) The `host_defined` value given to `parse_script` or `parse_module` is the one `script_host_defined` or `module_host_defined` returns (the same `Rc`), and the referrer of a load requested from that script or module is that script or module. [slice 11]

## 9. Modules, and the load hand-over

**The host receives, in `host_load_imported_module`, a `LoadRequest`: the referrer (a script, a
module or a realm), the module request (specifier and attributes), the graph's
`[[HostDefined]]`, and the payload kept opaque inside the token. While it loads it holds the
token, which keeps the referrer and the payload alive. It ends the load once, by moving the
token into `finish_loading_imported_module` with a `Module` it parsed itself
(`parse_module` or `parse_json_module`) or with a thrown value. It may do so inside the hook or
later.** This answers finding 2.

Why: the draft's result is a Module Record or a throw completion, not source text: "result is
either a normal completion containing the loaded Module Record or a throw completion"; and "The
operation must treat payload as an opaque value to be passed through to
FinishLoadingImportedModule." (`ecma262:sec-HostLoadImportedModule`). HTML parses the module
itself and attaches its module script as `[[HostDefined]]` ("Set referencingScript to
referrer.[[HostDefined]]"; "Otherwise, set completion to NormalCompletion(moduleScript's
record)", `html:hostloadimportedmodule`), and reads the referrer's [[RequestedModules]] ("For
each ModuleRequest record requested of referrer.[[RequestedModules]]"). The referrer is a realm
for an `import()` with no active script (`ecma262:sec-HostLoadImportedModule`, Note 1).

Rejected: decision 9's "with the module's source" (the host could not attach its own
`[[HostDefined]]` or report its own parse error); a `Copy` load identifier checked at run time
(the type makes "finished twice" impossible instead of answered).

```rust
/// One call of HostLoadImportedModule. Neither `Clone` nor `Copy`.
pub struct LoadRequest { /* agent key, roots of the referrer and of the payload, the request */ }
pub enum Referrer { Script(ScriptRef), Module(ModuleRef), Realm(RealmRef) }
pub struct ImportAttribute { pub key: Vec<u16>, pub value: Vec<u16> }
pub enum FinishErrorKind { Faulted, StaleHandle, ForeignHandle, ModuleMismatch, NotJsonModule }
pub struct FinishError { pub error: FinishErrorKind, pub request: LoadRequest }

impl LoadRequest {
    pub fn referrer(&self) -> Referrer;
    pub fn specifier(&self) -> &[u16];
    pub fn attributes(&self) -> &[ImportAttribute];
    /// The hostDefined of LoadRequestedModules; None for import().
    pub fn host_defined(&self) -> Option<&HostDefined>;
    /// True when the payload is a PromiseCapability Record (import()).
    pub fn is_dynamic(&self) -> bool;
}

impl<H: Host> Agent<H> {   // parse_*, finish and the reads also on HookCx and Cx, by decision 5's rule
    pub fn parse_module(&mut self, realm: &Realm, source: Source<'_>, host_defined: Option<HostDefined>)
        -> Result<Parsed<Module>, HostError>;
    pub fn parse_json_module(&mut self, realm: &Realm, source: Source<'_>) -> Result<Outcome<Module>, HostError>;
    pub fn load_requested_modules(&mut self, module: &Module, host_defined: Option<HostDefined>)
        -> Result<Outcome<Root>, HostError>;
    pub fn finish_loading_imported_module(&mut self, request: LoadRequest, result: Result<&Module, &Root>)
        -> Result<Outcome<()>, FinishError>;
    pub fn link(&mut self, module: &Module) -> Result<Outcome<()>, HostError>;
    pub fn evaluate_module(&mut self, module: &Module) -> Result<Outcome<Root>, HostError>;
    pub fn module_host_defined(&self, module: &Module) -> Result<Option<HostDefined>, HostError>;
    pub fn requested_modules(&self, module: &Module) -> Result<Vec<(Vec<u16>, Vec<ImportAttribute>)>, HostError>;
}

// in trait Host (decision 16)
const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[];
fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest);
fn host_get_import_meta_properties(cx: &mut HookCx<'_, Self>, module: ModuleRef) -> Completion<Vec<(PropertyKey, Value)>>;
fn host_finalize_import_meta(cx: &mut HookCx<'_, Self>, import_meta: ObjectRef, module: ModuleRef);
```

A dropped `LoadRequest` releases its roots at the next safe point; its graph's promise then
never settles, which is the host's choice.

**HOST-56** (ecma262:sec-InnerModuleLoading) `load_requested_modules` calls `host_load_imported_module` once for each request of each `new` module of the graph that has no loaded module yet and whose attribute keys are all in `SUPPORTED_IMPORT_ATTRIBUTES`, in the order of [[RequestedModules]]. [slice 11]

**HOST-57** (ecma262:sec-HostLoadImportedModule) The `LoadRequest` carries the specifier and attributes as written in the source, the referring module, and the `host_defined` value given to `load_requested_modules`. [slice 11]

**HOST-58** (ecma262:sec-InnerModuleLoading) A request with an attribute key not in `SUPPORTED_IMPORT_ATTRIBUTES` causes no hook call, and the promise of `load_requested_modules` is rejected with a `SyntaxError`. [slice 11]

**HOST-59** (ecma262:sec-HostLoadImportedModule) A load finished inside the hook through `cx.finish_loading_imported_module` continues the graph's loading before the hook returns, and the result equals that of a load finished after `load_requested_modules` returned. [slice 11]

**HOST-60** (ecma262:sec-InnerModuleLoading) When the last pending load of a graph is finished with a module, every module the graph visited whose status was `New` reads `Unlinked`, and the promise reads `Fulfilled(undefined)`. [slice 11]

**HOST-61** (ecma262:sec-ContinueModuleLoading) When a load is finished with a thrown value, the graph's promise reads `Rejected` with that value, and every later finish of a load of that graph answers `Normal(())` and changes no module's status. [slice 11]

**HOST-62** (ecma262:sec-HostLoadImportedModule) A second `finish_loading_imported_module` of one `LoadRequest` does not compile, which a `compile_fail` test holds. [slice 11]

**HOST-63** (ecma262:sec-FinishLoadingImportedModule) Finishing a load of a (referrer, request) pair that is already loaded with a different module answers `FinishError { error: ModuleMismatch, request }` with the token handed back, and the referrer keeps its first module. [slice 11]

**HOST-64** (ecma262:sec-HostLoadImportedModule) Finishing a load whose request has the attribute `type: "json"` with a module made by `parse_module` answers `FinishError { error: NotJsonModule, request }`, and with a module made by `parse_json_module` answers `Normal(())`. [slice 11]

**HOST-65** (ecma262:sec-evaluate-import-call) `import("./x.mjs")` in a script calls the hook with the referrer `Referrer::Script`, `is_dynamic()` true and `host_defined()` `None`. [slice 11]

**HOST-66** (ecma262:sec-HostLoadImportedModule) `import()` evaluated by a host function called while no script or module is on the stack (from `agent.call` of a function made by `new_host_function`) calls the hook with `Referrer::Realm` of the current realm. [slice 11]

**HOST-67** (swell:D9) A `LoadRequest` dropped without being finished leaves its graph's promise `Pending`, and the second `collect` after the drop frees what only the token kept alive. [slice 11]

**HOST-68** (ecma262:sec-meta-properties) The first evaluation of `import.meta` in a module calls `host_get_import_meta_properties` once with that module and then `host_finalize_import_meta` once, and the object has the returned properties; a second evaluation calls neither. [slice 11]

## 10. The state a Terminated run leaves

**A `Terminated` run ends at once: the execution context stack is emptied, no `catch` or
`finally` block runs, and every object keeps the state it had. After `StepBudget`,
`MemoryBudget` and `Interrupted` the agent stays usable; after `EngineFault` it is faulted for
good (decision 6). What each record is left in is fixed below.** This answers finding 4.

Why: HTML defines aborting a script this way: "This causes any ScriptEvaluation or Source Text
Module Record Evaluate invocations to cease immediately, emptying the JavaScript execution
context stack without triggering any of the normal mechanisms like finally blocks."
(`html:killing-scripts`); for a module "If Evaluate fails to complete as a result of the user
agent aborting the running script, then set evaluationPromise to a promise rejected with a new
QuotaExceededError." (`html:run-a-module-script`), a value the host makes. The draft has no
text for an aborted evaluation, so the states below are swell's, each chosen so that no later
step of the draft meets a state its assertions exclude.

- **Termination is sticky for the run.** Once a run is terminated, every `Cx` call made by
  host code still on the Rust stack answers `Abrupt::Terminated` with the same reason (after
  the check of its handles, decision 5; on a faulted agent a call whose type carries
  `HostError` answers `Err(Faulted)`, decision 6), and what
  a host function returns is ignored; the outermost entry answers `Terminated`.
- **Generators and async functions.** A generator that was executing stays `executing`, so a
  later `next` throws: "If state is executing, throw a TypeError exception."
  (`ecma262:sec-generatorvalidate`). An async function or generator cut mid-step leaves its
  promise pending. Added on 2026-11-23 for slice 10 (`engine.md` decision 3, "`async` functions
  on suspended frames", item 5): an `async` function whose frame was on the stack, in its call or
  in the job that resumed it, loses its frame (credited), runs no `finally`, and its promise
  stays pending for good, since nothing can resume it (HOST-195); one suspended at an `await`
  keeps its frame and a later checkpoint resumes it, unless its job had started and a check of
  its resumption was refused, or `clear_microtasks` dropped its job: its reaction is then spent
  and leaves its promise pending.
- **Module evaluation.** Every module on the evaluation stack (status `evaluating`) stays
  `evaluating` and is marked terminated; `evaluate_module` of any module whose graph reaches a
  marked module answers `Err(ModuleTerminated)`. Left unmarked, the next Evaluate would meet
  "Assert: module.[[Status]] is one of linked, evaluating-async, or evaluated." (Evaluate step
  2), and an importer would pass it by "If module.[[Status]] is evaluating, return index."
  (`ecma262:sec-innermoduleevaluation`) and run against uninitialised bindings. Setting it to
  `evaluated` would need an [[EvaluationError]], a thrown value the engine does not have. A
  module in `evaluating-async` whose execution was cut keeps its status and its promise stays
  pending.
- **Linking** runs no ECMAScript code, so a termination during Link (steps or memory) resets
  the stack's modules to `unlinked` as Link does for an abrupt completion: "For each Cyclic
  Module Record requiredModule of stack, do ... Set requiredModule.[[Status]] to unlinked."
  (`ecma262:sec-moduledeclarationlinking`).
- **Loading.** A termination during `load_requested_modules` or a finish sets the graph's
  [[IsLoading]] to false and leaves its promise pending; later finishes of it change nothing,
  as ContinueModuleLoading's first step already says ("If state.[[IsLoading]] is false, return
  unused.", `ecma262:sec-ContinueModuleLoading`). Its modules stay `new`.
- **Scripts** are records only; a terminated script can be evaluated again.
- **The microtask checkpoint.** The job that was running was dequeued ("Let oldestMicrotask be
  the result of dequeuing from the event loop's microtask queue", `html:perform-a-microtask-checkpoint`)
  and is not run again; the jobs before it ran; the jobs after it stay queued, in order;
  ClearKeptObjects is not performed; the next checkpoint starts afresh. Added on 2026-11-23: a
  job whose step of taking off was refused, by the step budget or the interrupt, was not taken
  off: it is still first in the queue and the next checkpoint runs it, so nothing of a job is
  lost between two jobs; the jobs left stay charged; the engine drops none of them, and the host
  may with `clear_microtasks` (HOST-199, HOST-200); and a promise job whose
  `host_prepare_to_run_job` was called gets its `host_clean_up_after_job` with `Ok(())` (decision
  11, "The checkpoint is one entry").
- **The interrupt flag** is cleared when it ends a run (decision 12).

Rejected: rolling the heap back (no algorithm for it, and HTML says the script ceases, not
that it is undone); faulting the agent on every termination (surf could not stop one script
and keep the page); resetting cut modules to `linked` (their code would run a second time,
which the draft never does).

**After `MemoryBudget`, "the agent stays usable" means four things and no more.** Added on
2026-11-22. `state()` reads `Alive`, and every call that takes no memory answers: reading the
outcomes and the values the host holds, dropping roots, `collect()`, `credit_host_bytes` and
dropping the agent (HOST-188). An entry whose parse and script record the room does not cover
ends in `Terminated(MemoryBudget)` again, the same way on every run from the same state
(HOST-189). The host reads the state without a trial entry: `heap_bytes()` is what is charged
(HOST-171) and `memory_bytes` is the limit it gave. Once the host made room, by dropping roots
and collecting or by giving back bytes it charged, an entry the room covers runs as in an agent
that never ended in the memory budget: the termination leaves no state behind (HOST-190). The
room a run leaves when it ends in the memory budget has no lower bound, since the run ends at
the first growth refused whatever was left below it; so a host makes room before the next
entry and does not count on what the run left.

**HOST-69** (html:killing-scripts) The script `var f; try { while (true) {} } finally { f = 1; }` ends in `Terminated(StepBudget)` and the script `f` then answers `Normal(undefined)`. [slice 1]

**HOST-70** (swell:D10) After `Terminated(StepBudget)` or `Terminated(Interrupted)`, `state()` reads `Alive` and a following script `1` answers `Normal(1)`; after `Terminated(MemoryBudget)` `state()` reads `Alive` and the next entry runs as HOST-188 to HOST-190 state. [slice 1]

**HOST-188** (swell:D10) After an entry answered `Terminated(MemoryBudget)` and the host charged with `charge_host_bytes` all the room `memory_bytes` − `heap_bytes()` left, the calls that take no memory answer: `state()` reads `Alive`, `collect()` answers `Normal` with the room still 0, a root the host kept from before the entry reads its value in `with_cx`, and dropping that root, `Agent::credit_host_bytes` and dropping the agent end with no fault and no panic. [slice 6]

**HOST-189** (swell:D10) After an entry answered `Terminated(MemoryBudget)`, an entry whose parse and script record the room `memory_bytes` − `heap_bytes()` does not cover ends in `Terminated(MemoryBudget)`, from `parse_script` as `Parsed::Terminated(MemoryBudget)`, with `state()` still `Alive` and the same answer and `heap_bytes()` each time it is made from the same state: with 1,000 bytes of room the script `1` answers `Terminated(MemoryBudget)` twice in a row with the same `heap_bytes()` after each, and `parse_script` of `1` then answers `Parsed::Terminated(MemoryBudget)` with that reading. [slice 6]

**HOST-190** (swell:D10) After an entry answered `Terminated(MemoryBudget)` and the host made room (dropped roots and called `collect()`, or gave bytes back with `Agent::credit_host_bytes`), with the room read as `memory_bytes` − `heap_bytes()` and no trial entry, the script `1` answers `Normal(1)` when the room is at least 3,072 bytes, spending the `last_entry_steps()` it spends in an agent that never ended in the memory budget and has the same room. [slice 6]

**HOST-71** (swell:D10) A broken engine invariant found inside `parse_script`, `create_realm` or `collect` makes that call answer `Terminated(EngineFault)` and `state()` then read `Faulted`, which a test build with fault injection at each of those calls shows. [slice 1]

**HOST-157** (swell:D10) A broken engine invariant found inside `parse_module` or `link` makes that call answer `Terminated(EngineFault)` and `state()` then read `Faulted`, which a test build with fault injection at each of those calls shows. [slice 11]

**HOST-72** (swell:D10) A host function that catches `Abrupt::Terminated` from `cx.call` and returns `Ok(Value::Undefined)` does not end the termination: the outermost entry answers `Terminated` with the same reason and no further script statement runs. [slice 1]

**HOST-73** (ecma262:sec-generatorvalidate) A generator whose body was running when the run ended in `Terminated(StepBudget)` throws a `TypeError` from its next `next()` in a later entry. [slice 7]

**HOST-74** (ecma262:sec-moduleevaluation) A module graph whose evaluation ended in `Terminated` leaves each module that was evaluating at `Evaluating` with `module_terminated` true, and `evaluate_module` of it or of a module importing it answers `Err(HostError::ModuleTerminated)` without running code. [slice 11]

**HOST-75** (ecma262:sec-moduledeclarationlinking) A `link` that ends in `Terminated` leaves every module of the graph that was `Unlinked` before the call at `Unlinked`, and a later `link` with a larger budget answers `Normal(())`. [slice 11]

**HOST-76** (ecma262:sec-ContinueModuleLoading) After `load_requested_modules` or a finish ended in `Terminated(MemoryBudget)`, the graph's promise stays `Pending`, its modules stay `New`, and a later finish of one of its loads answers `Normal(())` and changes nothing. [slice 11]

**HOST-77** (html:perform-a-microtask-checkpoint) When the third of five queued jobs ends the checkpoint in `Terminated(StepBudget)`, the first two have run, the third never runs again, and the next checkpoint runs the fourth and fifth in order. [slice 10]

**HOST-195** (swell:D10) The script `(async function () { await null; for (;;) {} })()` answers `Normal` with a promise; the next `perform_microtask_checkpoint` answers `Terminated(StepBudget)`, and `promise_state` of that promise reads `Pending` after it and after a later checkpoint, which answers `Normal` with both counts 0. [slice 10]

## 11. Jobs and the microtask checkpoint

**Promise jobs stay in the agent, in one FIFO microtask queue that also takes host microtasks,
and `perform_microtask_checkpoint` drains it under one step budget for the whole call. Generic,
timeout and FinalizationRegistry cleanup jobs are handed to the host as `Job` tokens, and the
host runs each with `run_job` when it chooses, one entry each.** This answers finding 6.

Why:

- A Job is not a function: "A Job is an Abstract Closure with no parameters that initiates an
  ECMAScript computation when no other ECMAScript computation is currently in progress."
  (`ecma262:sec-jobs`). So `call` cannot run one; the host runs a token with `run_job`.
- Promise jobs are ordered: "Jobs must run in the same order as the HostEnqueuePromiseJob
  invocations that scheduled them." (`ecma262:sec-hostenqueuepromisejob`). HTML puts them in
  the event loop's one microtask queue ("Queue a microtask to perform the following steps",
  `html:hostenqueuepromisejob`) beside its own microtasks (`queueMicrotask`, mutation
  observers), and drains it "While the event loop's microtask queue is not empty"
  (`html:perform-a-microtask-checkpoint`). One queue in the agent keeps both kinds in one
  order; a promise queue in the agent beside a host queue would not.
- One budget per checkpoint: `function f() { Promise.resolve().then(f); } f();` queues a new
  job from each job, each far under any per-job budget; with a budget per job, as decision 9
  had it until 2026-11-23, the checkpoint never returns and the host hangs (`AGENTS.md`: "Hostile input must
  never hang, crash or exhaust the host").
- HTML's steps around a promise job ("If job settings is not null, then prepare to run script
  with job settings. ... Let result be job(). ... If result is an abrupt completion, then
  report an exception", `html:hostenqueuepromisejob`) run on the host's side, so the engine
  calls two hooks around each promise job.
- Generic, timeout and cleanup jobs become HTML tasks: "Queue a global task on the JavaScript
  engine task source given global to perform job()." (`html:hostenqueuegenericjob`); the
  cleanup job is "Let result be the result of performing CleanupFinalizationRegistry(finalizationRegistry)"
  in a queued task (`html:hostenqueuefinalizationregistrycleanupjob`). Tasks are the host's,
  so the host holds those jobs.
- The checkpoint performs ClearKeptObjects after the queue is empty, as HTML's checkpoint does
  ("Perform ClearKeptObjects().", `html:perform-a-microtask-checkpoint`). The host performs
  HTML's other steps (notify about rejected promises, Indexed Database clean-up) after the call
  returns; they run no script, so the order is not observable.
- A checkpoint called while a checkpoint runs returns at once with nothing run: "If the event
  loop's performing a microtask checkpoint is true, then return." (`html:perform-a-microtask-checkpoint`).

Rejected: running jobs through `call` (decision 9's text until 2026-11-23); a budget per promise
job (decision 9's "each with the budget", likewise); a promise queue held by the host (the engine could not bound a
checkpoint, and the host would keep two queues in one order).

```rust
pub enum JobKind { Generic, Timeout, FinalizationRegistryCleanup }
/// A Job Abstract Closure handed to the host. Neither `Clone` nor `Copy`.
pub struct Job { /* agent key, root of the closure, kind, realm */ }
impl Job {
    pub fn kind(&self) -> JobKind;
    pub fn realm(&self) -> Option<RealmRef>;
}
pub type Microtask<H> = Box<dyn FnOnce(&mut Cx<'_, H>)>;
pub struct CheckpointReport { pub promise_jobs: u64, pub host_microtasks: u64 }

impl<H: Host> Agent<H> {
    pub fn run_job(&mut self, job: Job) -> Result<Outcome<()>, HostError>;
    /// `realm` is the current realm of the microtask while it runs; it is checked first.
    pub fn queue_microtask(&mut self, realm: &Realm, task: Microtask<H>) -> Result<(), HostError>;
    /// Drop every queued entry unrun; answers how many. No script, no hook, no step.
    pub fn clear_microtasks(&mut self) -> Result<usize, HostError>;
    pub fn perform_microtask_checkpoint(&mut self) -> Result<Outcome<CheckpointReport>, HostError>;
}
impl<'a, H: Host> Cx<'a, H> {   // no run_job: a job runs only when no code runs (9.5)
    /// Queued with the current realm of this `Cx` (decision 7).
    pub fn queue_microtask(&mut self, task: Microtask<H>) -> Result<(), HostError>;
    pub fn perform_microtask_checkpoint(&mut self) -> Completion<CheckpointReport>;
}

// in trait Host (decision 16)
fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, realm: Option<RealmRef>);
fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, realm: RealmRef);
fn host_clean_up_after_job(cx: &mut HookCx<'_, Self>, realm: Option<RealmRef>, result: Result<(), Value>);
fn host_make_job_callback(cx: &mut HookCx<'_, Self>, callback: ObjectRef) -> Option<HostDefined>;
fn host_call_job_callback(cx: &mut Cx<'_, Self>, callback: ObjectRef, host_defined: Option<&HostDefined>,
    this: Value, args: &[Value]) -> Completion<Value>;
fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job);
fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64);
fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job);
```

`run_job` of a cleanup job answers `Throw` when the cleanup callback threw, so that the host
performs its "host-defined steps for reporting the error" (`ecma262:sec-weakref-host-hooks`).
A dropped `Job` releases its roots. The test262 runner answers `host_enqueue_timeout_job` by
running the job when the queue is otherwise empty (`engine.md` decision 11).

**A host microtask's realm.** Added on 2026-11-23 for slice 10. A host microtask runs with a
realm, as every entry does (decision 7): the realm it was queued with is the current realm of
every call it makes through its `Cx` while it runs. `Agent::queue_microtask` takes that realm
and checks it first, as `Agent::call` does: a stale or released one answers
`Err(StaleHandle)`, one of another agent `Err(ForeignHandle)`, and nothing is queued.
`Cx::queue_microtask` takes the current realm of its `Cx`: inside a host function that
function's [[Realm]], inside `with_cx` the realm `with_cx` took, inside a host microtask that
microtask's (HOST-192). Neither call runs code, spends a step or calls a hook; no hook is called
around a host microtask, whose own code does what the host needs before and after it. The
queue holds the realm while the microtask is queued. Rejected: a microtask with no realm (a
`Cx` call in it would have no current realm, which decision 7 excludes); the realm the
checkpoint was called in (`Agent::perform_microtask_checkpoint` takes none, and one host
microtask may need another realm than the next).

**The checkpoint is one entry.** Added on 2026-11-23 for slice 10, the rule of `engine.md`
decision 3, "The microtask checkpoint is one entry". `perform_microtask_checkpoint` is one
outermost entry: its steps start at 0, and every entry it takes off the queue spends from them,
one step for taking it off and then what it runs; every promise job queued costs a step more.
So a checkpoint takes at most `steps_per_entry` entries off the queue, and a job that queues a
job without end ends it in `Terminated(StepBudget)` (HOST-80). `Cx::perform_microtask_checkpoint`
inside a host function runs inside the running entry and spends its steps (HOST-193). A
termination of one job ends the checkpoint with the jobs behind it queued (section 10, HOST-77);
`host_clean_up_after_job` is called for every promise job whose `host_prepare_to_run_job` was
called, after a termination with `Ok(())`, and the checkpoint's outcome carries the reason.
After it the host reads `last_entry_steps()` and `pending_microtasks()`; `Normal` with its
`CheckpointReport` comes only once the queue is empty (HOST-79, HOST-148). One budget, but a
scope per job (section 4): a value handed out to host code while a job runs, to a host
microtask, a hook or a host function the job calls, is valid until that job ends, and the
`ScriptRef`s `cx.parse_script` handed out in it are held until then; after it, a handle the host
kept with no `Root` keeps nothing alive, and a collection later in the same checkpoint, or a
`collect()` after it, frees its record once nothing else reaches it, after which the handle
answers `Err(StaleHandle)` (HOST-196, as HOST-178 for `with_cx`). A checkpoint therefore holds
what the running job was handed, not what every job touched.

**Steps, the interrupt and what a checkpoint leaves.** Added on 2026-11-23 for the first
embedder (`engine.md` decision 3, "The microtask checkpoint is one entry"; decision 9, "Jobs").
`last_entry_steps()` after `perform_microtask_checkpoint()` reads the steps of the whole
checkpoint, every entry taken off and everything the jobs ran, at most `steps_per_entry`, the
same after `Normal` and after a termination; after a checkpoint that found the queue empty it
reads 0; a nested checkpoint that returned at once (HOST-81) adds 0 to the entry around it; so a
host that sums the reading after every entry counts every step once (HOST-197). The interrupt
ends a running checkpoint with `Terminated(Interrupted)` at three places: at its start, before
any entry is taken off, so a flag set before the checkpoint ends it with nothing taken off, as
it ends any entry that starts (decision 12); before each entry is taken off, when the step of
that entry is charged, so a flag set between two jobs leaves the next entry first in the queue;
and while a job runs, wherever the step budget is checked (HOST-198). A checkpoint ended by the
step budget, the interrupt or the memory budget leaves every entry it did not start queued, in
its order and charged, and the next checkpoint runs them under its own budget (HOST-199). The
engine drops none of them: HTML stops a script without clearing the microtask queue, and only
the host knows whether the page goes on. The host empties the queue with
`Agent::clear_microtasks`, so that a page that floods the queue cannot spend the budgets of
later entries: it drops every queued entry unrun, credits what the promise jobs among them were
charged, and answers how many entries it dropped; it runs no script, calls no hook and spends no
step, and since no job runs, neither `host_prepare_to_run_job` nor `host_clean_up_after_job` is
called; a dropped host microtask is dropped as a Rust value, the host's own code. A promise
whose reaction job it dropped stays pending for good, as after a termination (HOST-200). It
cannot be called inside a running checkpoint: it takes `&mut Agent`, which no code inside an
entry holds, and `Cx` and `HookCx` have no such method, so a job cannot drop the entries behind
it and change the order of the checkpoint that runs it. Rejected: a no-op answer inside a
checkpoint (0 with entries queued would mislead); the engine emptying the queue at every
termination (it would drop jobs of a page the host keeps). Dropping the agent with entries
queued frees them and calls nothing of the host: no hook, job or microtask runs. A host function
called from a job may settle a promise through a resolve function it holds; the reaction job
that queues runs in the same checkpoint, behind every entry already queued (HOST-78, HOST-79).

**What a queued job costs.** Added on 2026-11-23 for slice 10 (`engine.md` decision 4, "Queued
jobs"). A promise job is charged to the memory budget while it is queued and credited when a
checkpoint takes it off; everything it holds is traced through the queue, a root. A host
microtask is not charged: its closure is the host's own storage, bounded by the host's calls,
and holds the engine values it needs in `Root`s; dropping the agent drops it unrun.

**The hooks of an `await`.** Added on 2026-11-23 for slice 10 (`engine.md` decision 3, "`async`
functions on suspended frames", item 3). An `await` makes the two functions of the draft's Await
and hands them to PerformPromiseThen, so the host sees what it sees for a `then`: two calls of
`host_make_job_callback` per `await`, and for the job that resumes the function
`host_prepare_to_run_job` with the function's realm, one call of `host_call_job_callback` with one
of the two functions and the value made for it, and `host_clean_up_after_job` (HOST-194). A
function of an `await` resumes the activation only while it is suspended at that `await`; called
at any other time, by a host that kept it, it does nothing and returns `undefined`. Rejected: an
`await` with no functions and no job-callback hooks, which no script can tell apart but a host
that implements either hook can.

**HOST-78** (ecma262:sec-hostenqueuepromisejob) Promise jobs and host microtasks run in the order they were queued: a script that queues `then` callbacks a and c with a host microtask b queued between them through a host function records the order a, b, c. [slice 10]

**HOST-79** (html:perform-a-microtask-checkpoint) `perform_microtask_checkpoint` runs the jobs queued while it runs before it returns, and returns `Normal` only when the queue is empty (`pending_microtasks()` reads 0). [slice 10]

**HOST-80** (swell:D11) After the script `function f() { Promise.resolve().then(f); } f();`, `perform_microtask_checkpoint` with `steps_per_entry` N answers `Terminated(StepBudget)` and `last_entry_steps()` reads at most N. [slice 10]

**HOST-81** (html:perform-a-microtask-checkpoint) A `perform_microtask_checkpoint` called through `cx` by a host microtask during a checkpoint answers `Normal` with both counts 0 at once, and the outer checkpoint goes on. [slice 10]

**HOST-82** (html:hostenqueuepromisejob) For every promise job whose realm is not null, the engine calls `host_prepare_to_run_job` with that realm before the job and `host_clean_up_after_job` after it, and passes `Err(value)` to the latter when the job's completion was a throw. [slice 10]

**HOST-83** (ecma262:sec-hostenqueuepromisejob) `host_enqueue_promise_job` is called once for each HostEnqueuePromiseJob of the draft, with the job's realm or `None`, before that job can run. [slice 10]

**HOST-84** (ecma262:sec-hostmakejobcallback) `host_make_job_callback` is called when `then` is called with a callable, and `host_call_job_callback` is called when that handler runs, with the value the former returned. [slice 10]

**HOST-85** (ecma262:sec-hostenqueuegenericjob) The resolution of an `Atomics.waitAsync` promise reaches the host as `host_enqueue_generic_job` with a `Job` of kind `Generic`, and nothing of it runs until the host passes that job to `run_job`. [slice 14]

**HOST-86** (ecma262:sec-hostenqueuetimeoutjob) `host_enqueue_timeout_job` receives a `Job` of kind `Timeout` and a non-negative finite `milliseconds`, and `run_job` of it answers `Normal(())`. [slice 14]

**HOST-87** (ecma262:sec-weakref-host-hooks) `run_job` of a cleanup job whose callback throws answers `Throw` with the thrown value, and the registry's other emptied cells stay for a later cleanup. [slice 16]

**HOST-88** (ecma262:sec-jobs) `Cx` has no `run_job`, which a `compile_fail` test holds: no job runs while code runs. [slice 14]

**HOST-89** (html:perform-a-microtask-checkpoint) A `WeakRef` whose target was returned by `deref()` in a job keeps that target through `collect` until the checkpoint that ran the job returns, and the first `collect` after it frees the target when nothing else reaches it. [slice 16]

**HOST-148** (swell:D11) The host can tell which checkpoint spent its budget: with `steps_per_entry` N, a checkpoint after a script that queued three `then` callbacks answers `Normal(CheckpointReport)` with `promise_jobs` 3; the script `function f() { Promise.resolve().then(f); } f();` then answers `Normal`; and the next `perform_microtask_checkpoint` answers `Terminated(StepBudget)` with `last_entry_steps()` at most N. [slice 10]

**HOST-192** (swell:D11) A host microtask runs in the realm it was queued with: one queued by `Agent::queue_microtask` with the second of two realms that calls a value that is not callable through `cx.call` gets a `TypeError` whose prototype is the second realm's `%TypeError.prototype%`; one queued by `cx.queue_microtask` in a host function of the second realm called from a script of the first gets the same; and `Agent::queue_microtask` with a released realm answers `Err(HostError::StaleHandle)`, and with a realm of another agent `Err(HostError::ForeignHandle)`, each queueing nothing (`pending_microtasks()` unchanged). [slice 10]

**HOST-193** (swell:D11) A checkpoint is one outermost entry and spends one step for each entry it takes off the queue: with k host microtasks queued that make no call, `perform_microtask_checkpoint` answers `Normal` with `host_microtasks` k and `last_entry_steps()` then reads k; with nothing queued it answers `Normal` with both counts 0 and `last_entry_steps()` 0; and a script that calls a host function which queues three such microtasks and then calls `cx.perform_microtask_checkpoint` reads, in `last_entry_steps()` after the script, at least 3 steps more than the same script whose host function queues none. [slice 10]

**HOST-196** (swell:D4) A checkpoint holds what a job was handed only until that job ends: of three host microtasks queued in one checkpoint, the first makes a host object with `cx.new_host_object` and the host keeps it as an `ObjectRef` with no root, the second calls through `cx.call` a function whose allocations start a collection, and the third then gets `Err(HostError::StaleHandle)` from `brand` of the kept handle, the checkpoint answering `Normal` with `host_microtasks` 3 and `state()` reading `Alive`. [slice 10]

**HOST-197** (swell:D11) `last_entry_steps()` after a checkpoint reads the steps of the whole checkpoint, also after a termination: with `steps_per_entry` 3 and five host microtasks queued that make no call, `perform_microtask_checkpoint` answers `Terminated(StepBudget)`, `last_entry_steps()` reads 3 and `pending_microtasks()` reads 2; and a checkpoint of two host microtasks that make no call, the first of which calls `cx.perform_microtask_checkpoint` (which answers both counts 0), answers `Normal` with `host_microtasks` 2 and `last_entry_steps()` then reads 2. [slice 10]

**HOST-198** (swell:D12) The interrupt ends a checkpoint with `Terminated(Interrupted)` at each of three places, and the flag is clear after: set before `perform_microtask_checkpoint` with two host microtasks queued, the checkpoint answers it with `last_entry_steps()` 0 and `pending_microtasks()` 2; set through the agent's `InterruptHandle` by the first of two host microtasks, which then returns, it answers it with `pending_microtasks()` 1, and the next checkpoint runs the second; set from another thread while a `then` callback loops without end, it answers it, and the next checkpoint answers `Normal` without running that callback again. [slice 10]

**HOST-199** (html:perform-a-microtask-checkpoint) A checkpoint ended by its budget leaves every entry it did not start queued, in order, for the next one: with `steps_per_entry` N, after a script that queues 20 `then` callbacks, each of which first appends its number to a global array and then spends about N / 4 steps, every checkpoint answers `Terminated(StepBudget)` with `last_entry_steps()` at most N until one answers `Normal`, and the array then holds 1 to 20, each once and in order. [slice 10]

**HOST-200** (swell:D11) `Agent::clear_microtasks` empties the queue without running anything: after the script `var p = Promise.resolve().then(function () { return 1; }); p`, which answers `Normal` with the promise p, `clear_microtasks` answers `Ok(1)` and calls no hook, `pending_microtasks()` then reads 0 and `heap_bytes()` less than before the call, the next checkpoint answers `Normal` with both counts 0, and `promise_state` of p reads `Pending`; `Cx` and `HookCx` have no `clear_microtasks`, which a `compile_fail` test holds. [slice 10]

**HOST-194** (ecma262:await) The script `(async function () { await 1; })()` calls `host_make_job_callback` twice, with two distinct functions, and the checkpoint after it runs one promise job, for which the engine calls `host_prepare_to_run_job` with the script's realm, then `host_call_job_callback` once, with the first of those two functions and the value `host_make_job_callback` returned for it, then `host_clean_up_after_job` with `Ok(())`. [slice 10]

## 12. Interrupting

**`InterruptHandle::interrupt` is a store into an `Arc<AtomicBool>` and answers nothing. The
engine reads the flag at the places it checks the step budget, at the start of every
outermost entry, at the start of every parse (`parse_script`, `cx.parse_script`, `eval`, the
`Function` constructor), and at least once per 4,096 code units a parse reads and per 4,096
nodes its later passes and the compile step visit. The engine clears the flag when it ends a run with `Terminated(Interrupted)`;
a flag set while nothing runs ends the next entry before it spends a step. `clear` withdraws
an interrupt not yet used.** This answers finding 7.

Why: no standard text says how to abort a script; HTML only allows it (`html:killing-scripts`).
Clearing at use and not at entry is what makes the handle race-free: a watchdog thread that
sets the flag just before an entry begins would otherwise see its interrupt lost. "The agent
answers every call with EngineFault" (decision 9) is about calls on the agent; the handle is
not one, so setting it on a faulted agent is a store nobody reads, as is setting it after the
agent was dropped. A parse runs no instruction, so without reads of its own it would run to its
end after an interrupt, for a source of any size the memory budget admits; reading at its start
and every 4,096 units of its work bounds the delay by a constant amount of work. A parse ended
by the flag answers `Parsed::Terminated(Interrupted)` from `parse_script` and `cx.parse_script`
(inside a host function the run then ends as HOST-96 says) and ends the run in
`Terminated(Interrupted)` from `eval` and `Function`; everything it charged is credited, no
`SyntaxError` object is made and `refusals()` does not move. For `eval` and `Function` the read
at the start comes after HostEnsureCanCompileStrings has answered.

Rejected: clearing the flag at the start of every entry; an interrupt that faults the agent.

```rust
#[derive(Clone)]
pub struct InterruptHandle { /* Arc<AtomicBool> */ }   // Send + Sync
impl InterruptHandle {
    pub fn interrupt(&self);
    pub fn clear(&self);
    pub fn is_set(&self) -> bool;
}
impl<H: Host> Agent<H> {
    pub fn interrupt_handle(&self) -> InterruptHandle;
}
```

**HOST-90** (swell:D12) `interrupt()` called from another thread while the script `while (true) {}` runs with a step budget of 2^62 makes `evaluate_script` answer `Terminated(Interrupted)`. [slice 1]

**HOST-91** (swell:D12) After an entry answered `Terminated(Interrupted)`, `is_set()` reads false and the next script `1` answers `Normal(1)`. [slice 1]

**HOST-92** (swell:D12) `interrupt()` called while no entry runs makes the next entry answer `Terminated(Interrupted)` with `last_entry_steps()` 0. [slice 1]

**HOST-93** (swell:D12) `clear()` after `interrupt()` and before the next entry lets that entry run to `Normal`. [slice 1]

**HOST-94** (swell:D12) `InterruptHandle` is `Send` and `Sync`, which a test that moves a clone into a spawned thread compiles. [slice 1]

**HOST-95** (swell:D12) `interrupt()` on the handle of a faulted agent returns, and the agent's calls still answer `Err(HostError::Faulted)`. [slice 1]

**HOST-96** (swell:D12) An interrupt set while a host function runs ends the run with `Terminated(Interrupted)` at the first check after the host function returns to the engine. [slice 1]

**HOST-166** (swell:D12) `interrupt()` called while no entry runs makes the next `parse_script` answer `Parsed::Terminated(Interrupted)`; `is_set()` then reads false, and `refusals()` and `heap_bytes()` read what they read before the call. [slice 6]

**HOST-167** (swell:D12) A host function that calls `interrupt()` and then `cx.parse_script` gets `Parsed::Terminated(Interrupted)`, and the outermost entry answers `Terminated(Interrupted)`. [slice 6]

**HOST-168** (swell:D12) A `host_ensure_can_compile_strings` that calls `interrupt()` and answers `Ok(())` makes the script `try { eval(r); } catch (e) {}` end in `Terminated(Interrupted)` with `refusals()` unchanged, and the same for `Function(r)`. [slice 6]

## 13. The read surface

**The host can read the agent's state, a module's [[Status]] and whether it was cut by a
termination, a module's evaluation error, a promise's state and result and its
[[PromiseIsHandled]], set [[PromiseIsHandled]], read what the last entry spent and how many
refusals of a later slice's construct the agent raised. A
checkpoint and a collection answer a report. Reads run no code, call no hook and spend no
step.** This answers finding 1.

**Where an entry's error arose.** Added on 2026-11-22. After an outermost entry that answered
`Parsed::Errors` or `Throw`, `last_entry_error_site` gives the line and column where the first
error or the throw arose, counted from 1 in UTF-16 code units of the source that holds it,
whether that source is eval or `Function` code, and the [[HostDefined]] of its script; after any
other entry it gives none. A throw is recorded where it arose and kept through `finally`
blocks; a `catch` ends it.

- Defined after every entry that can end in a parse error or an uncaught throw: `parse_script`
  that answered `Parsed::Errors` gives its first error's site; `evaluate_script`, `call` and
  `construct` of `Agent` that answered `Throw` give the site of the throw they answered, and
  `run_job` that answers `Throw` (decision 11) the same once it exists. Every outermost entry clears the
  record at its start, so after an entry that answered `Normal`, `Terminated`,
  `Parsed::Parsed` or `Parsed::Terminated`, or after a throw the script caught, the read gives
  `None`, never the site of an earlier entry.
- The site of a throw is the instruction where the throw completion arose in the innermost
  ECMAScript frame: a `throw` statement, of any value, an error or not (a throw of a value that
  is not an error gives the site of its `throw` statement); or an instruction whose operation
  threw (a call of a value that is not callable, a property read of `null`, a call of a builtin
  or a host function that answered `Throw`, sited at the call). The record is kept while the
  same value unwinds through callers, builtins and `finally` blocks (`try { null.x } finally {}`
  keeps the site of `null.x`); it ends when a `catch` block is entered, and when a builtin or a
  host function answers normally after a throw inside it. A throw raised outside any ECMAScript
  frame (a host function called by `Agent::call`) has no site: `None`.
- `ErrorSite::dynamic` is true when the position is in eval code or in the source text of a
  `Function` constructor, `function anonymous(<parameters>\n) {\n<body>\n}`, counted in that
  text; false in the source given to `parse_script`. `ErrorSite::host_defined` is the
  [[HostDefined]] of the script record whose code holds the position: the `Rc` given to
  `parse_script` for that script (a function one script defined and another called names the
  first), the caller's for a direct eval, `None` for an indirect eval and for `Function` code.
- The record holds what survives the end of the entry and a collection (the compiled code and a
  clone of the [[HostDefined]]), so the read answers the same after `collect`. It is a read: no
  code, no hook, no step, no allocation. A site for `cx.parse_script`, for a throw inside a
  `with_cx` closure and the frames a throw passed are not recorded.

**An error object, read without running code.** Added on 2026-11-22. `error_summary` reads
[[ErrorData]] and the own `message` data property: the kind recorded by the constructor that
made the object, or given by the host in `HostObjectSpec::error_kind`, never its prototype or
`name`. It answers `None` for a value that is not an object with [[ErrorData]] (a primitive, a
plain object, a host object made with `error_kind: None`). The message is read from the object's
own property list with no [[Get]]: an accessor, a value that is not a String, or no own
`message` gives `message: None`, and an inherited `message` is not read; its code units are
exact, as a `DOMString`. The kind is the constructor's whatever prototype `new_target` gave the
object. Like every read it runs no code, calls no hook, spends no step and allocates nothing in
the heap.

Why: HTML reads a promise's fields when it notifies about rejected promises: "If
p.[[PromiseIsHandled]] is true, then continue." and "attribute initialized to
p.[[PromiseResult]]" (`html:notify-about-rejected-promises`), and Web IDL sets one: "To mark as
handled a Promise<T> promise, set promise.[[Promise]].[[PromiseIsHandled]] to true."
(`webidl:mark-a-promise-as-handled`). A module's [[Status]] is a field of the draft's record
(`ecma262:sec-cyclic-module-records`), and the conformance target needs it to check the
model's lifecycle (`host-interface-model.md`). The budget requirements of decision 2 can only
be tested with `last_entry_steps`, `heap_bytes` and `live_bytes` (the two readings of decision 2).

Rejected: no read surface (decision 9); reads through script (a global the host would add to
every realm).

```rust
pub enum AgentState { Alive, Faulted }
pub enum ModuleStatus { New, Unlinked, Linking, Linked, Evaluating, EvaluatingAsync, Evaluated }
pub enum PromiseState { Pending, Fulfilled(Root), Rejected(Root) }
pub struct CollectReport { pub heap_bytes_before: u64, pub heap_bytes_after: u64 }
pub struct ErrorSite {
    pub line: u32,
    pub column: u32,
    /// The position is in eval code or in the source text of a `Function` constructor,
    /// counted in that code's own text.
    pub dynamic: bool,
    /// The [[HostDefined]] of the script record whose code raised it.
    pub host_defined: Option<HostDefined>,
}
pub struct ErrorSummary {
    pub kind: ErrorKind,
    /// The own "message" property's value when it is a data property holding a String.
    pub message: Option<Vec<u16>>,
}

impl<H: Host> Agent<H> {   // module_status and promise reads also on Cx and HookCx
    pub fn state(&self) -> AgentState;
    pub fn module_status(&self, module: &Module) -> Result<ModuleStatus, HostError>;
    pub fn module_terminated(&self, module: &Module) -> Result<bool, HostError>;
    pub fn module_evaluation_error(&mut self, module: &Module) -> Result<Option<Root>, HostError>;
    pub fn promise_state(&mut self, promise: &Root) -> Result<PromiseState, HostError>;
    pub fn promise_is_handled(&self, promise: &Root) -> Result<bool, HostError>;
    pub fn mark_promise_as_handled(&mut self, promise: &Root) -> Result<(), HostError>;
    pub fn last_entry_steps(&self) -> Result<u64, HostError>;
    pub fn heap_bytes(&self) -> Result<u64, HostError>;
    pub fn live_bytes(&self) -> Result<u64, HostError>;
    pub fn pending_microtasks(&self) -> Result<usize, HostError>;
    pub fn refusals(&self) -> u64;
    pub fn last_entry_error_site(&self) -> Result<Option<ErrorSite>, HostError>;
    pub fn error_summary(&self, value: &Root) -> Result<Option<ErrorSummary>, HostError>;
}
impl<'a, H: Host> Cx<'a, H> {   // also on HookCx
    pub fn error_summary(&self, value: Value) -> Result<Option<ErrorSummary>, HostError>;
}
```

`Linking` is in the enumeration for completeness; Link calls no host code, so the host never
reads it.

**A count of refusals.** Decided on 2026-11-21 for slice 2. While the engine is delivered slice
by slice, it raises a `SyntaxError` whose message names a later slice when a construct belongs
to that slice (`engine.md` section 12, "A part before its slice": the message
`<construct> is not supported before slice <n>`, which `swell_syntax::message_kind` reads as a
refusal). It raises one while parsing, through
`parse_script`, `parse_module`, `eval` or `Function`, and while running: a regular expression
literal evaluated before slice 5, a `String` method given a code unit outside ASCII before
slice 5, and `RegExp` given a pattern or flags the engine does not take yet (both below).
Through `eval`, `Function`, a literal or `RegExp` a script can catch
that error, and a host that measures conformance cannot then tell a refusal from a
`SyntaxError` of the language. So the agent counts every refusal it raises, whatever entry or
instruction raised it and whether or not the script catches it, and `refusals()` reads the
count: a `u64` that starts at 0, is never reset, saturates instead of wrapping, and is read in
every state of the agent, `Faulted` included (it has no `Result`). The nesting limit
(decision 2) is not a refusal, nor is any other `SyntaxError`. Once the last slice is
delivered the engine refuses no construct, and the count stays 0 for every source. The
conformance runner fails every run in which the count moved (`engine.md` decision 11).
Rejected: a mark on the error object a script could read or remove; counting parses only (a
caught refusal of a regular expression literal would be missed).

**A refusal raised by a builtin method.** Added on 2026-11-21 for slice 3. Before slice 5 the
four case methods of `String.prototype` (`toLowerCase`, `toUpperCase`, `toLocaleLowerCase`,
`toLocaleUpperCase`) and `localeCompare` refuse a String with a code unit of 0x80 or above
(`engine.md` section 12, "A part before its slice", the third form). That refusal is raised
while running and is counted like a refused parse. The script sees a `SyntaxError` object
whose prototype is `%SyntaxError.prototype%` of the method's realm (the current realm while a
built-in function runs, `ecma262:sec-built-in-function-objects-call-thisargument-argumentslist`)
and whose `message` is exactly `case mapping outside ASCII is not supported before slice 5`
(the four case methods) or `comparison outside ASCII is not supported before slice 5`
(`localeCompare`), with no position after it, since nothing was parsed;
`swell_syntax::message_kind` reads either as a refusal of slice 5. Each such throw moves
`refusals()` by 1, and a script that catches the error does not undo the count, as for a
refused parse through `eval` or `Function`. An error that RequireObjectCoercible or ToString
throws before the method reaches its refusal is not a refusal and is not counted. Once slice 5
is delivered the five methods refuse no input, and the count stays 0 for every input of them.
HOST-164 names the refusals of a parse and of a regular expression literal only; HOST-165
holds these.

**A refusal raised by `RegExp`.** Added on 2026-11-23 for slice 5. Until the story of slice 5
that brings each, RegExpInitialize (`ecma262:sec-regexpinitialize`), which the `RegExp`
constructor, the evaluation of a literal and the `String` methods that make a RegExp run,
refuses one part of the pattern grammar and the flags (`engine.md` decision 7, "Refusals
until the later stories of slice 5"; section 12, the third form): the flag `v`. A group name
with a code point outside ASCII was refused until
`story:identifiers-and-group-names-outside-ascii`, and the flag `i` and a modifier `i` until
`story:regexp-case-insensitive-matching-and-modifiers`. That
refusal is raised while running and is counted like a refused parse. The script sees a
`SyntaxError` object of the current realm whose `message` is exactly `class set expressions of
the v flag is not supported before slice 5`, with no position after it; each such throw moves
`refusals()` by 1, caught or not. An error that ToString of the pattern or the flags or the check of the flags
throws before the parse, a grammar error of the pattern and the nesting limit are not refusals
and are not counted. In a literal `v` is a refusal of the parse (HOST-164). The methods of `%RegExp.prototype%` keyed by `%Symbol.match%`, `%Symbol.replace%`,
`%Symbol.search%` and `%Symbol.split%` (`ecma262:sec-regexp.prototype-%symbol.match%`,
`ecma262:sec-regexp.prototype-%symbol.replace%`, `ecma262:sec-regexp.prototype-%symbol.search%`,
`ecma262:sec-regexp.prototype-%symbol.split%`; `engine.md` decision 7, the same paragraph) refuse
no call, and `String.prototype.replace`, `replaceAll` and `split` given a RegExp answer through
them; a call of theirs moves `refusals()` only by a refusal of RegExpInitialize it reaches, as
when `@@split` constructs its splitter. Once slice 5 is delivered `RegExp` refuses no input, and
the count stays 0 for every pattern and flags.

**HOST-191** (swell:D13) Before slice 5 is delivered, on a new agent, the script `try { new RegExp("a", "v"); false } catch (e) { e instanceof SyntaxError && e.message === "class set expressions of the v flag is not supported before slice 5" }` answers `Normal(true)` and `refusals()` then reads 1; the same script with `/a/v` in place of the construction answers `Normal(true)` and moves the count by 1 more, and so does `try { new RegExp("a", "v"); false } catch (e) { e instanceof SyntaxError && e.message === "class set expressions of the v flag is not supported before slice 5" }`; `try { new RegExp("("); false } catch (e) { e instanceof SyntaxError }`, `try { new RegExp("a", "gg"); false } catch (e) { e instanceof SyntaxError }` and `new RegExp("a", "g").test("a")` each answer `Normal(true)` with the count unchanged; and once slice 5 is delivered, `new RegExp("a", "i").test("A")` and `/a/i.test("A")` answer `Normal(true)` with the count unchanged. [slice 5]

**HOST-97** (ecma262:sec-cyclic-module-records) `module_status` reads `New` after `parse_module`, `Unlinked` after the graph's last load finished, `Linked` after `link`, and `Evaluated` or `EvaluatingAsync` after `evaluate_module`, for a graph of three modules. [slice 11]

**HOST-98** (ecma262:sec-innermoduleevaluation) `cx.module_status` of a module, read by a host function its own body calls during `evaluate_module`, answers `Evaluating`. [slice 11]

**HOST-99** (ecma262:sec-async-module-execution-fulfilled) A module with a top-level `await` reads `EvaluatingAsync` after `evaluate_module` and `Evaluated` after the checkpoint that settles it. [slice 11]

**HOST-100** (html:notify-about-rejected-promises) `promise_state` reads `Pending`, then `Rejected(reason)` after the promise is rejected, and `promise_is_handled` reads false until a handler is added and true after. [slice 10]

**HOST-101** (webidl:mark-a-promise-as-handled) After `mark_promise_as_handled` of a rejected promise with no handler, `promise_is_handled` reads true, and a handler added to it later causes no `host_promise_rejection_tracker` call with `Handle`. [slice 10]

**HOST-102** (swell:D13) A read changes no value `last_entry_steps()`, `heap_bytes()` and `live_bytes()` report, and calls no hook. [slice 1]

**HOST-164** (swell:D13) `refusals()` reads 0 on a new agent; with `r` a source the engine refuses (its error's `message` read by `swell_syntax::message_kind` as a refusal), it reads 1 more after `parse_script` of `r`, 1 more after the script `try { eval(r); } catch (e) {}` answers `Normal`, and 1 more after the script `try { Function(r); } catch (e) {}` answers `Normal`, and while a regular expression literal is refused, 1 more after the script `try { /a/; } catch (e) {}` answers `Normal`; it reads the same after `parse_script` of a grammar error and of `0` inside 2,000 nested parentheses with `parser_nesting` 1,000; it is read, as a `u64`, after the agent faulted; and once the engine refuses no construct, it reads 0 after each of those scripts. [slice 2]

**HOST-165** (swell:D13) Before slice 5, on a new agent, the script `try { "é".toLowerCase(); false } catch (e) { e instanceof SyntaxError && e.message === "case mapping outside ASCII is not supported before slice 5" }` answers `Normal(true)` and `refusals()` then reads 1; the same script with `toUpperCase`, `toLocaleLowerCase` and `toLocaleUpperCase` each answers `Normal(true)` and moves the count by 1 more; `try { "a".localeCompare("é"); false } catch (e) { e instanceof SyntaxError && e.message === "comparison outside ASCII is not supported before slice 5" }` and the same with `"é".localeCompare("a")` each answer `Normal(true)` and move it by 1 more; `"AbC".toLowerCase() === "abc" && "AbC".toUpperCase() === "ABC" && "a".localeCompare("b") < 0` answers `Normal(true)`, and `try { String.prototype.toLowerCase.call({ toString: function () { throw 1; } }); } catch (e) { e === 1 }` answers `Normal(true)`, each with the count unchanged; and once slice 5 is delivered, each of these method calls leaves the count unchanged. [slice 3]

**HOST-172** (swell:D13) `parse_script` of the two-line source `var a;\n  var 1;` answers `Parsed::Errors`, and `last_entry_error_site()` then reads `Some` with line 2, the column the error's `message` ends with, `dynamic` false and the `host_defined` given to the parse (the same `Rc`). [slice 6]

**HOST-173** (swell:D13) The script `function f() {\n  null.x;\n}\nf();` answers `Throw` of a `TypeError` and `last_entry_error_site()` reads line 2, column 3; `[1].map(function (x) {\n  throw x;\n});` reads line 2, column 3 (a throw of a value that is not an error is sited at its `throw` statement); `try { throw 1; } catch (e) {\n  throw e;\n}` reads line 2, column 3; `try { null.x } finally {}` reads line 1, column 7; `eval("1;\n  throw 2;")` reads line 2, column 3 with `dynamic` true; `new Function("\n  null.x;")()` reads line 4, column 3 with `dynamic` true and `host_defined` `None`. [slice 6]

**HOST-174** (swell:D13) `last_entry_error_site()` reads `None` after an entry that answered `Normal`, `Terminated(StepBudget)` or `Parsed::Parsed`, after the script `try { throw 1; } catch (e) {} 1`, and after a `Normal` entry that follows a `Throw` entry; reads the same value twice and after a `collect()`; and leaves `last_entry_steps()`, `heap_bytes()` and the count of hook calls as they were. [slice 6]

**HOST-179** (swell:D13) After the script `throw new TypeError("a\uD800")` answered `Throw`, `error_summary` of its root answers kind `TypeError` and message `[0x61, 0xD800]`, also after the script set the error's prototype to `null` and defined an inherited getter `message` on `Object.prototype`; for an error whose own `message` is an accessor whose getter throws it answers `message: None` and runs no code (a counter in the getter stays 0); a host object made with `error_kind: Some(TypeError)` on which the host defined an own data property `message` holding `"m"` answers kind `TypeError` and message `[0x6D]`, `Error.isError` of it answers `true` and `Object.prototype.toString.call` of it `"[object Error]"`; for `throw 1`, a plain object and a host object made with `error_kind: None` and the same own `message` it answers `None`, and `Error.isError` of that host object `false`; and `last_entry_steps()`, `heap_bytes()` and the count of hook calls are unchanged by it. [slice 6]

## 14. `call`, `construct`, host functions, re-entrancy

**A host function is a built-in function object whose steps are a Rust function with the
`HostFn` signature. Host code runs ECMAScript code through `Cx` (`call`, `construct`, the
abstract operations, nested script evaluation); each such call that runs code (an ECMAScript
function, a script or a host function) is a re-entry, counted against `reentry_depth`,
spending from the outermost entry's budget, and checked for the interrupt.**

Why: built-in function objects are the draft's form for functions defined outside ECMAScript
code, called with "thisArgument" and "argumentsList" and constructed with "newTarget"
(`ecma262:sec-built-in-function-objects-call-thisargument-argumentslist`,
`ecma262:sec-built-in-function-objects-construct-argumentslist-newtarget`), made by
CreateBuiltinFunction with a name and a length (`ecma262:sec-createbuiltinfunction`). Web IDL's
conversions are surf's binding layer over the abstract operations (`engine.md` decision 9), so
the interface exposes those operations. HTML calls scripts re-entrantly ("These algorithms are
not invoked by one script directly calling another, but they can be invoked reentrantly",
`html:clean-up-after-running-script`), so a nested `evaluate_script` from a host function is
allowed.

Rejected: closures as host functions (`Box<dyn Fn>` per function object: a heap object holding
Rust state the collector cannot trace; state goes in host data, decision 15).

```rust
pub struct HostCall<'c> {
    pub this: Value,
    pub args: &'c [Value],
    pub new_target: Option<ObjectRef>,
    pub callee: ObjectRef,
}
pub type HostFn<H> = fn(cx: &mut Cx<'_, H>, call: HostCall<'_>) -> Completion<Value>;

impl<'a, H: Host> Cx<'a, H> {   // a method that takes a handle: Result<Completion<T>, HostError> (decision 5)
    pub fn new_host_function(&mut self, realm: RealmRef, name: &str, length: u32, steps: HostFn<H>,
        constructor: bool, data: Option<Box<dyn HostData>>) -> Result<Completion<ObjectRef>, HostError>;
    pub fn call(&mut self, function: Value, this: Value, args: &[Value]) -> Result<Completion<Value>, HostError>;
    pub fn construct(&mut self, function: Value, args: &[Value], new_target: Option<Value>) -> Result<Completion<Value>, HostError>;
    pub fn get(&mut self, object: ObjectRef, key: PropertyKey) -> Result<Completion<Value>, HostError>;
    pub fn set(&mut self, object: ObjectRef, key: PropertyKey, value: Value, throw: bool) -> Result<Completion<bool>, HostError>;
    pub fn define_property_or_throw(&mut self, object: ObjectRef, key: PropertyKey, descriptor: PropertyDescriptor) -> Result<Completion<()>, HostError>;
    pub fn has_property(&mut self, object: ObjectRef, key: PropertyKey) -> Result<Completion<bool>, HostError>;
    pub fn delete_property_or_throw(&mut self, object: ObjectRef, key: PropertyKey) -> Result<Completion<()>, HostError>;
    pub fn to_primitive(&mut self, value: Value, hint: Option<PreferredType>) -> Result<Completion<Value>, HostError>;
    pub fn to_number(&mut self, value: Value) -> Result<Completion<f64>, HostError>;
    pub fn to_numeric(&mut self, value: Value) -> Result<Completion<Value>, HostError>;
    pub fn to_string(&mut self, value: Value) -> Result<Completion<StringRef>, HostError>;
    pub fn to_object(&mut self, value: Value) -> Result<Completion<ObjectRef>, HostError>;
    pub fn to_property_key(&mut self, value: Value) -> Result<Completion<PropertyKey>, HostError>;
    pub fn get_iterator(&mut self, value: Value, kind: IteratorKind) -> Result<Completion<IteratorRecord>, HostError>;
    pub fn iterator_step_value(&mut self, iterator: &IteratorRecord) -> Result<Completion<Option<Value>>, HostError>;
    pub fn iterator_close(&mut self, iterator: &IteratorRecord, completion: Completion<Value>) -> Result<Completion<Value>, HostError>;
    pub fn new_promise_capability(&mut self) -> Completion<PromiseCapability>;
    pub fn evaluate_script(&mut self, script: ScriptRef) -> Result<Completion<Value>, HostError>;
    /// O.[[GetPrototypeOf]]().
    pub fn get_prototype_of(&mut self, object: ObjectRef) -> Result<Completion<Option<ObjectRef>>, HostError>;
    /// O.[[SetPrototypeOf]](V): whether it was set (false for a cycle, a non-extensible object
    /// with another prototype, or %Object.prototype%).
    pub fn set_prototype_of(&mut self, object: ObjectRef, prototype: Option<ObjectRef>)
        -> Result<Completion<bool>, HostError>;
    /// Charge `steps` of the host's own work to the running outermost entry.
    pub fn charge_steps(&mut self, steps: u64) -> Completion<()>;
}
impl<'a, H: Host> Cx<'a, H> {   // also on HookCx
    /// Charge `bytes` the host holds for the agent to its memory budget; the answer holds the
    /// charge, which goes back to the budget when the answer is dropped.
    pub fn charge_host_bytes(&mut self, bytes: u64) -> Completion<HostBytes>;
    /// Change the bytes `charge` holds to `bytes`: growing charges the difference first,
    /// shrinking gives it back at once.
    pub fn resize_host_bytes(&mut self, charge: &mut HostBytes, bytes: u64) -> Result<Completion<()>, HostError>;
}
/// Bytes the host charged for the agent. Neither `Clone` nor `Copy`, no public method;
/// dropping it gives its bytes back.
pub struct HostBytes { /* the bytes, the agent key, the agent's queue of host credits */ }
impl<H: Host> Agent<H> {
    pub fn call(&mut self, realm: &Realm, function: &Root, this: &Root, args: &[&Root]) -> Result<Outcome<Root>, HostError>;
    pub fn construct(&mut self, realm: &Realm, function: &Root, args: &[&Root], new_target: Option<&Root>) -> Result<Outcome<Root>, HostError>;
    /// Give back the bytes of `charge` at once; answers them.
    pub fn credit_host_bytes(&mut self, charge: HostBytes) -> Result<u64, HostError>;
}
// PropertyDescriptor, PreferredType, IteratorKind, IteratorRecord, PromiseCapability: the draft's
// records of the same names, as plain Rust structs and enums over Value.
```

**Prototypes.** Added on 2026-11-22. An interface object whose interface has a parent gets the
parent's interface object as [[Prototype]] by `set_prototype_of` after `new_host_function`
("If I inherits from some other interface P, then set constructorProto to the interface object
of P in realm.", `webidl:interface-object`; without a parent it keeps `%Function.prototype%`);
`get_prototype_of` serves the binding steps that walk a chain ("Let prototype be
O.[[GetPrototypeOf]]()", the named property visibility algorithm,
`webidl:dfn-named-property-visibility`). Both follow the ordinary [[SetPrototypeOf]] and
[[GetPrototypeOf]] and run no script code for an object without overrides. They act on any
object the host made or was handed, through a handle it holds in its scope or a root (a stale
handle answers `Err(HostError::StaleHandle)`, a foreign one `Err(HostError::ForeignHandle)`). A
cycle, a non-extensible object given another prototype and `%Object.prototype%` given another
value are refused with `Ok(Ok(false))`, an outcome the host reads. They are on `Cx` only: once a
host object has an override table (slice 9) they run host code, which `HookCx` cannot.

**The host's own work and bytes.** Added on 2026-11-22. A host function charges the steps of its
own work with `cx.charge_steps`, as a builtin does, before the work; and the bytes it holds for
the agent outside the engine's heap with `charge_host_bytes` before it allocates them, keeping
the answer with them: dropping it gives the bytes back, in a sweep too, with no agent at hand; a
`HostBytes` kept for good keeps its bytes charged for the agent's life. For bytes whose size
changes while their holder lives (a text node's data, an attribute list), `resize_host_bytes`
changes the charge in place: growing charges the difference before the host grows its storage
and, past the budget, ends the entry as any charge does, the handle keeping its old charge;
shrinking gives the difference back at once. Replacing the handle by a new one charges both
until the old one is dropped.

- Both charges go through the one accounting of the memory budget and the one counter of steps:
  there is no second budget. `charge_steps` reads the interrupt flag first; steps past
  `steps_per_entry` are not spent and end the run in `Terminated(StepBudget)`, an interrupt in
  `Terminated(Interrupted)`. `charge_host_bytes` and a growing `resize_host_bytes` that would
  pass `memory_bytes` charge nothing and end the run in `Terminated(MemoryBudget)`, sticky as
  decision 10 says, as the engine's own charge does; neither is a safe point, so no collection
  runs in them. The steps are the running outermost entry's: inside `with_cx` those of the last
  `Cx` call that began an entry (decision 2), inside a host function the outermost entry's.
- A dropped `HostBytes` adds its bytes to a queue of the agent it holds, touching nothing else,
  so it cannot fail, also when its agent faulted or was dropped. The host keeps it with what it
  charged for: in the host data of the host object that owns the storage, so the sweep that frees
  the object drops it, or in its own state. The agent credits the queue after each sweep and
  before each `charge_host_bytes` and growing `resize_host_bytes`, so a charge dropped outside a
  collection, or inside a host function, gives its room to the next charge.
- `Agent::credit_host_bytes` takes the handle, credits its bytes at once and empties it, so its
  drop adds nothing; it needs no realm and no scope (a dropped document). A credit is always the
  bytes of one charge as resized, given back once, to the agent that charged them: no credit
  passes what the host charged and nothing wraps. A `HostBytes` of another agent given to
  `credit_host_bytes` or `resize_host_bytes` answers `Err(HostError::ForeignHandle)` and changes
  nothing (dropped there, it gives its bytes back to its own agent); a faulted agent answers
  `Err(HostError::Faulted)`.
- A charge, a resize, a drop and a credit are inputs the host gives (decision 19).

**What a host function, a host object and a property cost.** Added on 2026-11-22. A host
function's record is charged before it is made with the boxes of its steps and of its data, and
the sweep that frees the function credits them. A host function costs at most
`HOST_FUNCTION_LIVE_BYTES` = 576 bytes of `live_bytes()` plus its data, a property definition
`PROPERTY_LIVE_BYTES` = 448; a realm of 300 host functions and 40 host objects with 600
definitions raises `heap_bytes()` by at most `BINDINGS_REALM_HEAP_BYTES` = 506,112, arena growth
included (a host object: decision 15). Each is a public constant of `host/limits.rs`; a test
compares the values written here with the constants. Each value is the rise the test
`tests/host_objects.rs` measures plus an eighth, rounded up to a multiple of 64, as the
constants' doc comments say. A release that raises a ceiling names it under the changelog's
Host interface heading. Steps: making a host function or a host object spends no step and leaves
the counter of the running entry unchanged; defining a property that does not compact spends
none; `set_prototype_of` spends one per object on the new prototype's chain; a call of a host
function spends one step at its entry besides the instructions of the call, which is 2 fewer than
a call of a script function with an empty body, and what the host charges with
`cx.charge_steps`.

**HOST-103** (ecma262:sec-built-in-function-objects-call-thisargument-argumentslist) A host function called from script as `o.f(1, 2)` receives `this` = `o`, the arguments `[1, 2]` and `new_target` `None`, and its `Ok(v)` is the call's value. [slice 1]

**HOST-104** (ecma262:sec-built-in-function-objects-construct-argumentslist-newtarget) A host function made with `constructor: true` and called as `new F()` receives `new_target` `Some(F)`; one made with `constructor: false` makes `new F()` throw a `TypeError` without being called. [slice 1]

**HOST-105** (swell:D14) A host function that returns `Err(Abrupt::Throw(v))` makes the calling script's `catch (e)` receive `e === v`. [slice 1]

**HOST-106** (ecma262:sec-createbuiltinfunction) A host function made with name `"print"` and length 1 has the own properties `name` `"print"` and `length` 1 with the attributes CreateBuiltinFunction gives them. [slice 1]

**HOST-107** (html:clean-up-after-running-script) A host function that evaluates a second parsed script through `cx.evaluate_script` gets that script's completion value, and the outer script then continues with the bindings the inner one created. [slice 1]

**HOST-180** (webidl:interface-object) A host function `Node` and a host function `Element`, the second given `Node` as [[Prototype]] by `cx.set_prototype_of` answering `Ok(Ok(true))`, make the script `Object.getPrototypeOf(Element) === Node && Object.getPrototypeOf(Node) === Function.prototype` answer `true`, and `cx.get_prototype_of(Element)` answers `Node`; `set_prototype_of(Node, Some(Element))` then answers `Ok(Ok(false))` (a cycle), on an object made non-extensible another prototype answers `Ok(Ok(false))`, and a stale handle answers `Err(HostError::StaleHandle)`; no call runs script code (a getter on `Function.prototype` counting its calls stays at 0). [slice 6]

**HOST-186** (swell:D14) After a realm's setup, `new_host_function` with a name of at most 32 code units and data of a `T` raises `live_bytes()` by at most `HOST_FUNCTION_LIVE_BYTES` plus `size_of::<T>()`, and by the same amount for each further function of the same name length and type; `new_host_object` with data of a `T` by at most `HOST_OBJECT_LIVE_BYTES` plus `size_of::<T>()`; 300 definitions of new properties on one host object by at most 300 × `PROPERTY_LIVE_BYTES`; once the host drops them and `collect` ran twice, `live_bytes()` reads its value from before them. [slice 6]

**HOST-187** (swell:D14) A realm with 300 host functions and 40 host objects whose properties are defined in `with_cx` raises `heap_bytes()`, from its reading after `create_realm` and before the first `with_cx`, by at most `BINDINGS_REALM_HEAP_BYTES`; with the baseline `last_entry_steps()` read after a `with_cx` whose last call began an entry (a `define_property_or_throw`, which reads 0 after it), a `with_cx` of only `new_host_function` or only `new_host_object` calls leaves `last_entry_steps()` unchanged; `set_prototype_of` with a new prototype whose chain has k objects reads k; three of its host functions, bound after the `heap_bytes()` reading as the global data properties `f0`, `f150` and `f299`, make the scripts `f0()`, `f150()` and `f299()` spend the same `last_entry_steps()`, which is 2 less than that of the script `g()` of a script function `g` with an empty body; once the realm's roots are dropped and `collect` ran twice, `live_bytes()` reads its value from before `create_realm` (the three bindings are properties of the realm's global and go with it), and building the same realm again, with the same bindings and the same scripts as the first build, leaves `heap_bytes()` after its scripts at most where the first build's scripts left it. [slice 6]

## 15. Host objects: brand, host data, `trace`, overrides

**The host makes objects with a brand (the Web IDL interface they implement), host data of any
Rust type that implements `HostData`, and optionally a table of internal-method overrides.
The engine traces host data through `HostData::trace`. After each override returns, the engine
checks the result against the invariants of 6.1.7.3 with the checks the `Proxy` internal
methods use, and a broken invariant ends the run with `Terminated(EngineFault)`. An object may
carry [[IsHTMLDDA]].** (`engine.md` decision 9, kept.)

Why: "Any implementation provided exotic objects must also maintain these invariants for those
objects. Violation of these invariants may cause ECMAScript code to have unpredictable
behaviour and create security issues." and "An implementation must not allow these invariants
to be circumvented in any manner" (`ecma262:sec-invariants-of-the-essential-internal-methods`).
A broken invariant from the host is a host bug the engine cannot recover from, so it faults the
agent instead of throwing to the script. Web IDL's "is a platform object" checks need the
brand (`webidl:is-a-platform-object`); legacy platform objects and `WindowProxy` need the
overrides (`webidl:js-legacy-platform-objects`; HTML 7.2.3 is on another page, not read here,
not checked).

Rejected: throwing a `TypeError` to the script on a broken invariant, as a `Proxy` does (the
script did nothing wrong, and continuing with a host object known to be broken is the security
issue the clause names).

```rust
#[derive(Clone, Copy, PartialEq, Eq)] pub struct Brand(pub u32);
pub trait HostData: std::any::Any {
    fn trace(&self, tracer: &mut Tracer<'_>);
}
pub struct Tracer<'t> { /* private */ }
impl<'t> Tracer<'t> {
    pub fn value(&mut self, value: Value);
    pub fn object(&mut self, object: ObjectRef);
}
pub struct Overrides<H: Host> {
    pub get_prototype_of: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<Option<ObjectRef>>>,
    pub set_prototype_of: Option<fn(&mut Cx<'_, H>, ObjectRef, Option<ObjectRef>) -> Completion<bool>>,
    pub is_extensible: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<bool>>,
    pub prevent_extensions: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<bool>>,
    pub get_own_property: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<Option<PropertyDescriptor>>>,
    pub define_own_property: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, PropertyDescriptor) -> Completion<bool>>,
    pub has_property: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<bool>>,
    pub get: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, Value) -> Completion<Value>>,
    pub set: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey, Value, Value) -> Completion<bool>>,
    pub delete: Option<fn(&mut Cx<'_, H>, ObjectRef, PropertyKey) -> Completion<bool>>,
    pub own_property_keys: Option<fn(&mut Cx<'_, H>, ObjectRef) -> Completion<Vec<PropertyKey>>>,
}
/// The [[Prototype]] of a new host object.
pub enum Prototype {
    Object(ObjectRef),
    Null,
    /// The `%Object.prototype%` of that realm, without a call (decision 7).
    ObjectPrototypeOf(RealmRef),
}
pub struct HostObjectSpec<H: Host> {
    pub prototype: Prototype,
    pub brand: Brand,
    pub data: Option<Box<dyn HostData>>,
    /// `Some(kind)` gives the object an [[ErrorData]] slot of that kind.
    pub error_kind: Option<ErrorKind>,
    pub overrides: Option<&'static Overrides<H>>,
    pub is_html_dda: bool,
}
/// The intrinsics a host can name: the closed list of decision 7.
#[non_exhaustive]
pub enum HostIntrinsic {
    ObjectPrototype, FunctionPrototype, ErrorPrototype,
    // with slice 6's iterator story (HOST-185):
    ArrayPrototypeValues, ArrayPrototypeEntries, ArrayPrototypeKeys, ArrayPrototypeForEach,
}
/// The well-known symbols a host can name: the closed list of decision 7.
#[non_exhaustive]
pub enum WellKnownSymbol {
    ToStringTag, Unscopables,
    // with slice 6's `Symbol` story (HOST-185):
    Iterator,
}
impl<'a, H: Host> Cx<'a, H> {   // also on HookCx
    pub fn new_host_object(&mut self, spec: HostObjectSpec<H>) -> Result<Completion<ObjectRef>, HostError>;
    pub fn brand(&self, object: ObjectRef) -> Result<Option<Brand>, HostError>;
    pub fn host_data<T: HostData>(&self, object: ObjectRef, brand: Brand) -> Result<Option<&T>, HostError>;
    pub fn host_data_mut<T: HostData>(&mut self, object: ObjectRef, brand: Brand) -> Result<Option<&mut T>, HostError>;
    /// The intrinsic `which` of `realm`, handed out.
    pub fn intrinsic(&mut self, realm: RealmRef, which: HostIntrinsic) -> Result<ObjectRef, HostError>;
    /// The well-known symbol `which` (shared by all realms of the agent), handed out.
    pub fn well_known_symbol(&mut self, which: WellKnownSymbol) -> Result<SymbolRef, HostError>;
}
```

`intrinsic` and `well_known_symbol` are reads (added on 2026-11-22, decision 7's closed list):
no code, no hook, no step, no allocation; the value is handed out to the running scope, as every
value given to host code is, and a realm `create_realm` did not make, or one freed, answers
`Err(StaleHandle)`.

**A host object with [[ErrorData]].** Added on 2026-11-22. A host object made with an
`error_kind` has an [[ErrorData]] slot of that kind, as Web IDL gives a `DOMException` ("If
interface is DOMException, append [[ErrorData]] to slots.", "internally create a new object
implementing the interface", `webidl:internally-create-a-new-object-implementing-the-interface`;
"It also has [[ErrorData]] and [[Stack]] slots, like all built-in exceptions.", "DOMException
custom bindings", `webidl:js-DOMException-specialness`): `Error.isError` answers true for it and
`Object.prototype.toString` gives it the builtin tag "Error" (an own or inherited
`@@toStringTag` still wins), and `error_summary` (decision 13) reads its kind. The browser makes
a `DOMException` with `Some(ErrorKind::Error)`. The field is a breaking change of the interface:
every `HostObjectSpec` literal names it. The [[Stack]] slot is not made.

**What a host object costs.** Added on 2026-11-22. A host object costs at most
`HOST_OBJECT_LIVE_BYTES` = 256 bytes of `live_bytes()` plus its data, a public constant of
`host/limits.rs` that a test compares with the value written here; what the data owns behind its
box is the host's to charge with `charge_host_bytes` (decision 14). Making one spends no step.

**HOST-108** (webidl:is-a-platform-object) `host_data::<T>` with the object's own brand and type returns `Some`, and with another brand, another type, or an object made by script returns `Ok(None)`. [slice 1]

**HOST-109** (swell:D15) An object reachable only through a value that a host object's `trace` lists survives `collect`; the same object not listed is freed by it. [slice 1]

**HOST-110** (swell:D15) A `trace` that lists a stale handle makes `collect` answer `Terminated(EngineFault)`. [slice 1]

**HOST-111** (swell:D15) The host data of a freed host object is dropped by the `collect` that frees the object, which a `Drop` counter in a test's host data shows. [slice 1]

**HOST-112** (ecma262:sec-invariants-of-the-essential-internal-methods) A host object whose `get_own_property` override reports a property as non-configurable and later reports it absent makes the run that observes the second answer end in `Terminated(EngineFault)`, and `state()` then reads `Faulted`. [slice 9]

**HOST-113** (ecma262:sec-invariants-of-the-essential-internal-methods) A `get_own_property` override that returns a descriptor that is not fully populated ends the run in `Terminated(EngineFault)`. [slice 9]

**HOST-114** (ecma262:sec-invariants-of-the-essential-internal-methods) Every internal method of a host object with an override table runs the override where one is given and the ordinary method where none is, which a test with one override per method observes. [slice 9]

**HOST-115** (ecma262:sec-IsHTMLDDA-internal-slot) For a host object with `is_html_dda`, the script `typeof o` answers `"undefined"`, `!o` answers `true` and `o == null` answers `true`. [slice 18]

## 16. The hook trait

**One trait, `Host`, with every host hook of the draft's Annex D that `engine.md` decision 9
lists, HTML's `HostSystemUTCEpochNanoseconds`, the time zone operations, and the job hooks of
decision 11. A hook with a default in the draft has that default as a provided method; a hook
the draft gives no default (job scheduling, module loading, the clock) is a required method.
`HostEventSet` is not a hook.**

Why: Annex D lists the host hooks (`ecma262:sec-host-hooks-summary`): HostCallJobCallback,
HostEnqueueFinalizationRegistryCleanupJob, HostEnqueueGenericJob, HostEnqueuePromiseJob,
HostEnqueueTimeoutJob, HostEnsureCanCompileStrings, HostFinalizeImportMeta,
HostGetCodeForEval, HostGetImportMetaProperties, HostGrowSharedArrayBuffer,
HostHasSourceTextAvailable, HostLoadImportedModule, HostGetSupportedImportAttributes,
HostMakeJobCallback, HostPromiseRejectionTracker, HostResizeArrayBuffer,
InitializeHostDefinedRealm (decision 7). HostEnsureCanAddPrivateElement is host-defined
(`ecma262:sec-hostensurecanaddprivateelement`) though Annex D does not list it.
`HostEventSet` (29.5.3, `ecma262:sec-hosteventset`) "returns a Set of Memory events" of a
candidate execution: an operation of the memory model, not a host hook; decision 9 lists it in
error. A required method has no default because a default would break a "must" of the draft:
a dropped job is never "performed" (`ecma262:sec-jobs`), an unfinished load never finishes
(`ecma262:sec-HostLoadImportedModule`), and an engine with no ambient authority has no time
to give.

| hook | source | signature (all associated functions of `Host`) | default |
|---|---|---|---|
| HostEnsureCanCompileStrings | `ecma262:sec-hostensurecancompilestrings`, `html:hostensurecancompilestrings(realm,-parameterstrings,-bodystring,-codestring,-compilationtype,-parameterargs,-bodyarg)` | `fn host_ensure_can_compile_strings(cx: &mut Cx<'_, Self>, request: &CompileStrings<'_>) -> Completion<()>` | `Ok(())` |
| HostGetCodeForEval | `ecma262:sec-hostgetcodeforeval` | `fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, arg: ObjectRef) -> Option<StringRef>` | `None` (no-code) |
| HostEnsureCanAddPrivateElement | `ecma262:sec-hostensurecanaddprivateelement` | `fn host_ensure_can_add_private_element(cx: &mut HookCx<'_, Self>, obj: ObjectRef) -> Completion<()>` | `Ok(())` |
| HostHasSourceTextAvailable | `ecma262:sec-hosthassourcetextavailable` | `fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, func: ObjectRef) -> bool` | `true` |
| HostMakeJobCallback | `ecma262:sec-hostmakejobcallback` | decision 11 | `None` (empty) |
| HostCallJobCallback | `ecma262:sec-hostcalljobcallback` | decision 11 | `cx.call(callback, this, args)`; the engine gave those handles, so an `Err` from it is a broken engine invariant and answers `Abrupt::Terminated(EngineFault)` (decision 5) |
| HostEnqueuePromiseJob | `ecma262:sec-hostenqueuepromisejob` | decision 11 | nothing |
| (HTML's steps around a promise job) | `html:hostenqueuepromisejob` | `host_prepare_to_run_job`, `host_clean_up_after_job`, decision 11 | nothing |
| HostEnqueueGenericJob | `ecma262:sec-hostenqueuegenericjob` | decision 11 | required |
| HostEnqueueTimeoutJob | `ecma262:sec-hostenqueuetimeoutjob` | decision 11 | required |
| HostEnqueueFinalizationRegistryCleanupJob | `ecma262:sec-host-cleanup-finalization-registry` | decision 11 | required |
| HostPromiseRejectionTracker | `ecma262:sec-host-promise-rejection-tracker` | `fn host_promise_rejection_tracker(cx: &mut HookCx<'_, Self>, promise: ObjectRef, operation: RejectionOperation)` | nothing |
| HostLoadImportedModule | `ecma262:sec-HostLoadImportedModule` | decision 9 | required |
| HostGetSupportedImportAttributes | `ecma262:sec-hostgetsupportedimportattributes` | `const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str]` | `&[]` |
| HostGetImportMetaProperties | `ecma262:sec-hostgetimportmetaproperties` | decision 9 | `Ok(vec![])` |
| HostFinalizeImportMeta | `ecma262:sec-hostfinalizeimportmeta` | decision 9 | nothing |
| HostResizeArrayBuffer | `ecma262:sec-hostresizearraybuffer` | `fn host_resize_array_buffer(cx: &mut HookCx<'_, Self>, buffer: ObjectRef, new_byte_length: u64) -> Completion<Handled>` | `Ok(Handled::Unhandled)` |
| HostGrowSharedArrayBuffer | `ecma262:sec-hostgrowsharedarraybuffer` | `fn host_grow_shared_array_buffer(cx: &mut HookCx<'_, Self>, buffer: ObjectRef, new_byte_length: u64) -> Completion<Handled>` | `Ok(Handled::Unhandled)` (slice 14 completes shared memory) |
| HostSystemUTCEpochNanoseconds | `html:hostsystemutcepochnanoseconds` | decision 17 | required |
| SystemTimeZoneIdentifier and the three others | `ecma262:sec-systemtimezoneidentifier` and beside it | decision 17 | the draft's UTC-only steps |

```rust
pub enum RejectionOperation { Reject, Handle }
pub enum Handled { Handled, Unhandled }
pub enum CompilationType { DirectEval, IndirectEval, Function }
pub struct CompileStrings<'r> {
    pub callee_realm: RealmRef,
    pub param_strings: &'r [StringRef],
    pub body_string: StringRef,
    pub code_string: StringRef,
    pub compilation_type: CompilationType,
    pub param_args: &'r [Value],
    pub body_arg: Value,
}
```

The getter of `SUPPORTED_IMPORT_ATTRIBUTES` is a constant because the draft requires "Each time
this operation is called, it must return the same List with the same contents in the same
order." (`ecma262:sec-hostgetsupportedimportattributes`); HTML's list is `« "type" »`
(`html:hostgetsupportedimportattributes`). swell is a host that "is a web browser" for every
normative optional clause, as Annex B is in (`engine.md` decision 1), so the engine calls
HostEnsureCanAddPrivateElement where the draft says "If the host is a web browser".

**HOST-116** (ecma262:sec-performeval) `eval("1")` calls `host_ensure_can_compile_strings` once with `body_string` `"1"`, `compilation_type` `DirectEval` or `IndirectEval` as called, before parsing; a `Throw` from the hook is thrown by `eval` and no code of the string runs. [slice 2]

**HOST-117** (ecma262:sec-createdynamicfunction) `Function("a", "return a")` calls `host_ensure_can_compile_strings` with `param_strings` `["a"]`, `body_string` `"return a"` and `compilation_type` `Function`. [slice 2]

**HOST-118** (ecma262:sec-hostgetcodeforeval) `eval(o)` with an object `o` calls `host_get_code_for_eval(o)`; `None` makes `eval` return `o`, and `Some(s)` makes it evaluate `s`. [slice 2]

**HOST-119** (ecma262:sec-hostensurecanaddprivateelement) The engine calls `host_ensure_can_add_private_element` only for host objects with an override table, and a `Throw` from it makes the private field definition throw that value. [slice 8]

**HOST-120** (ecma262:sec-hosthassourcetextavailable) For a function for which `host_has_source_text_available` answers false, `Function.prototype.toString` returns a NativeFunction string. [slice 3]

**HOST-121** (ecma262:sec-host-promise-rejection-tracker) The script `Promise.reject(1)` calls `host_promise_rejection_tracker` with `Reject`, and adding a handler to that promise in a later entry calls it with `Handle`. [slice 10]

**HOST-122** (ecma262:sec-hostresizearraybuffer) `buffer.resize(n)` on a resizable `ArrayBuffer` calls `host_resize_array_buffer` with n; after `Handled` the engine does not resize and `byteLength` reads what the host set, after `Unhandled` the engine resizes to n. [slice 13]

**HOST-123** (ecma262:sec-hostgrowsharedarraybuffer) `sab.grow(n)` calls `host_grow_shared_array_buffer` with n, and `Unhandled` makes the engine grow the buffer to n. [slice 14]

**HOST-124** (swell:D16) Each provided hook method other than `host_get_import_meta_properties` and `host_finalize_import_meta`, left as provided, gives the draft's default answer listed in the table above, which one unit test per provided method checks. [slice 1]

**HOST-158** (swell:D16) `host_get_import_meta_properties` and `host_finalize_import_meta`, left as provided, give the draft's defaults listed in the table above (`Ok(vec![])` and nothing), which one unit test per method checks. [slice 11]

## 17. The clock and time zones

**The engine reads the current time only through HTML's `HostSystemUTCEpochNanoseconds`,
given the current realm's global object, and clamps the answer to the instant range. A time
value is the answer divided by 10^6, rounded down. The time zone comes from the four draft
operations, whose provided methods are the draft's UTC-only steps.**

Why: the draft defers the time: `Date.now` "returns the time value designating the UTC date and
time of the occurrence of the call to it" (`ecma262:sec-date.now`); HTML defines the hook a
browser gives: "Let ns be the number of nanoseconds from the Unix epoch to time, rounded to the
nearest integer. Return the result of clamping ns between nsMinInstant and nsMaxInstant."
(`html:hostsystemutcepochnanoseconds`). The time zone operations are implementation-defined
with defaults "to be used for ECMAScript implementations that do not include local political
rules for any time zones" (`ecma262:sec-getnamedtimezoneoffsetnanoseconds`). Rejected: a
millisecond clock (HTML's hook is in nanoseconds; Temporal, slice 20, needs them).

```rust
// in trait Host
fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, global: ObjectRef) -> i128;
fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String;                        // "UTC"
fn get_named_time_zone_offset_nanoseconds(cx: &mut HookCx<'_, Self>, id: &str, epoch_ns: i128) -> i128; // 0
fn get_named_time_zone_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, id: &str, local: IsoDateTime) -> Vec<i128>; // « GetUTCEpochNanoseconds(local) »
fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier>; // « UTC »
pub struct IsoDateTime { pub year: i32, pub month: u8, pub day: u8, pub hour: u8, pub minute: u8,
    pub second: u8, pub millisecond: u16, pub microsecond: u16, pub nanosecond: u16 }
pub struct TimeZoneIdentifier { pub identifier: String, pub primary: String }
```

**HOST-125** (ecma262:sec-date.now) `Date.now()` calls `host_system_utc_epoch_nanoseconds` once with the current realm's global object and returns the answer divided by 10^6 and rounded toward negative infinity: an answer of `-1` gives `-1`, an answer of `1_999_999` gives `1`. [slice 4]

**HOST-126** (html:hostsystemutcepochnanoseconds) An answer outside ±8.64 × 10^21 is clamped to that range: an answer of `i128::MAX` makes `Date.now()` return 8.64 × 10^15. [slice 4]

**HOST-127** (ecma262:sec-systemtimezoneidentifier) With the provided time zone methods, `new Date(0).getTimezoneOffset()` answers 0 and `new Date(0).toString()` contains `GMT+0000`. [slice 4]

**HOST-128** (ecma262:sec-getnamedtimezoneoffsetnanoseconds) A host answering an offset of 3,600 × 10^9 ns for its system time zone makes `new Date(0).getHours()` answer 1. [slice 4]

## 18. Collection and weak references

**`collect` runs a full collection at the call. The collector's roots are those of `engine.md`
decision 4, the host's roots and the agent's [[KeptAlive]] list. When a collection empties a
cell of a FinalizationRegistry, the engine calls `host_enqueue_finalization_registry_cleanup_job`
once for that registry in that collection. Collections at other times start only at safe
points, triggered by bytes allocated, never by time.** The timing of `WeakRef` clearing beyond
what the draft fixes is completed by slice 16.

Why: the draft leaves the choice open: "Let enqueueCleanup be an implementation-defined choice
of either true or false." (`ecma262:sec-weakref-execution`); choosing true each time keeps
cleanup deterministic. Neither ClearKeptObjects nor cleanup "may interrupt synchronous
ECMAScript execution" (`ecma262:sec-weakref-processing-model`), so `collect` runs no code. A
`deref`ed target is kept: "it adds the target to a list that will point strongly at the target
until ClearKeptObjects is called" (`ecma262:sec-addtokeptobjects`).

```rust
impl<H: Host> Agent<H> {
    pub fn collect(&mut self) -> Result<Outcome<CollectReport>, HostError>;
}
```

**HOST-129** (ecma262:sec-weakref-processing-model) `collect` runs no ECMAScript code and calls no hook other than `host_enqueue_finalization_registry_cleanup_job`. [slice 1]

**HOST-130** (ecma262:sec-weakref-execution) After `collect` frees a target registered with a FinalizationRegistry, `host_enqueue_finalization_registry_cleanup_job` has been called exactly once for that registry in that collection. [slice 16]

**HOST-131** (ecma262:sec-addtokeptobjects) A target returned by `deref()` survives every `collect` until the next checkpoint returns, even when nothing else reaches it. [slice 16]

**HOST-132** (swell:D18) `collect` answers a `CollectReport` whose `heap_bytes_after` equals the `heap_bytes()` read after it. [slice 1]

## 19. Determinism

**An agent's run is a function of its inputs: the limits, the seeds, the calls the host makes
with their arguments, and the answers its hooks give. Nothing else enters: no clock, no
address, no thread timing except the interrupt flag.**

Why: `engine.md` decisions 4 and 9 (collections count bytes; the command-line host gives the
clock, time zone and seed "so a run repeats exactly") and decision 13 (non-determinism is a
threat). The interrupt is the one input whose timing the host does not control; a run without
an interrupt repeats exactly.

**HOST-133** (swell:D19) Two agents created with the same limits and seeds, given the same calls with the same arguments by hosts whose hooks answer the same values, make the same sequence of hook calls with the same arguments and return the same outcomes, which a test over a fixed corpus of scripts checks in one process and across two processes. [slice 1]

**HOST-159** (swell:D19) The same holds for a fixed corpus of modules: two agents created with the same limits and seeds, given the same calls with the same arguments by hosts whose hooks answer the same values, make the same sequence of hook calls with the same arguments and return the same outcomes, in one process and across two processes. [slice 11]

**HOST-134** (swell:D19) The sequence of `host_enqueue_finalization_registry_cleanup_job` calls of a script that creates garbage and registers targets is the same in two runs with the same inputs. [slice 16]

**HOST-135** (swell:D19) A script run twice with the same inputs prints the same bytes, `Math` results and `Number` to string conversions included, on every release platform. [slice 4]

## Open

Nothing is open. Items A (the error of a parse that passes `parser_nesting`) and B (whether an
`Agent` is `Send`) were decided on 2026-11-21 as recommended, and are now part of decisions 2
and 1.

## Where this document departs from `engine.md`

The coordinator changes these in `engine.md`, which this unit does not edit:

1. Decision 9, "Entry points ... each returning an Outcome": each entry point has its own
   result type (decision 5), and misuse answers `Err(HostError)` (decision 6).
2. Decision 9, "After EngineFault the agent answers every call with EngineFault": it answers
   `Err(HostError::Faulted)`; the interrupt handle is not a call (decisions 6, 12).
3. Decision 9, `perform_microtask_checkpoint` "each with the budget": one budget for the whole
   checkpoint (decision 11). Changed in `engine.md` on 2026-11-23.
4. Decision 9, "Generic and timeout jobs are handed to the host, which runs them through
   `call`": through `run_job`; cleanup jobs likewise (decision 11). Changed in `engine.md` on
   2026-11-23.
5. Decision 9, "the host later calls `finish_loading_imported_module` with the module's source
   or an error": with a `Module` the host parsed, or a thrown value (decision 9).
6. Decision 9, "a hash and random seed" and "random bytes" from the trait: two seeds, no
   random-bytes method (decision 3).
7. Decision 9 lists `HostEventSet` as a hook: it is a memory-model operation (decision 16).
8. Decision 4, a stale handle "ends the run with `Terminated(EngineFault)`": a stale handle
   from the host answers `Err(HostError::StaleHandle)`; only one inside the engine faults.
9. Decision 3, parser nesting "through eval or Function as a `RangeError`": passing
   `parser_nesting` is a `SyntaxError` whose message names the limit, through the host,
   `eval` and `Function` alike, recorded as an omission in `conformance/test262/sections.txt`
   (decision 2).
10. Decision 9's limit names: `bigint_bits` replaces "`BigInt` sizes" with a unit (decision 2).
11. Decision 3, "pattern nesting is bounded at parse, reported as a `RangeError`": there is no
    separate bound; pattern nesting counts against `parser_nesting` and passing it is the same
    `SyntaxError` (decision 2).

## What the ESS model must change

For the specification author's next round; this unit changes nothing under `spec/`.

**(a) The markers.** Each `UNMAPPED:` marker gets the identifiers below, or `ESS limit`. The
table in `host-interface-model.md` gives each marker's first words.

- `system.yaml:16`: HOST-133, HOST-134, HOST-135, HOST-159
- `host.yaml:40`: HOST-9, HOST-12, HOST-14, HOST-16, HOST-19, HOST-20, HOST-22, HOST-23, HOST-24
- `host.yaml:43`: HOST-6, HOST-7, HOST-8, HOST-10, HOST-80
- `host.yaml:118`: HOST-35
- `host.yaml:129`: HOST-4, HOST-26, HOST-27, HOST-83, HOST-84, HOST-116 to HOST-128, HOST-149, HOST-150, HOST-151, HOST-158
- `host.yaml:179`: HOST-52
- `host.yaml:214`: HOST-54, HOST-156
- `host.yaml:228`: HOST-74, HOST-98
- `host.yaml:288`: HOST-65, HOST-66
- `host.yaml:402`: ESS limit
- `host.yaml:417`: HOST-18, HOST-36, HOST-137, HOST-138
- `host.yaml:424`: HOST-37
- `host.yaml:429`: HOST-43
- `host.yaml:458`: HOST-25, HOST-26, HOST-27, HOST-144, HOST-145, HOST-146
- `host.yaml:480`: HOST-49, HOST-50, HOST-51, HOST-155
- `host.yaml:494`: HOST-12, HOST-41, HOST-71
- `host.yaml:507`: HOST-55
- `host.yaml:508`: HOST-31, HOST-45
- `host.yaml:543`: HOST-71
- `host.yaml:630`: HOST-45, HOST-55
- `host.yaml:664`: HOST-157
- `host.yaml:675`: HOST-39, HOST-60, HOST-61
- `host.yaml:678`: HOST-57
- `host.yaml:694`: HOST-48
- `host.yaml:761`: HOST-57, HOST-59, HOST-62, HOST-63, HOST-64
- `host.yaml:765`: HOST-31, HOST-61
- `host.yaml:767`: HOST-56, HOST-60
- `host.yaml:812`: HOST-42
- `host.yaml:825`: HOST-48
- `host.yaml:846`: HOST-157
- `host.yaml:862`: HOST-38
- `host.yaml:865`: HOST-43
- `host.yaml:932`: HOST-70, HOST-74, HOST-188 to HOST-190
- `host.yaml:882`: HOST-48
- `host.yaml:898`: HOST-43
- `host.yaml:969`: ESS limit
- `host.yaml:983`: HOST-79
- `host.yaml:985`: HOST-77, HOST-80
- `host.yaml:990`: HOST-79
- `host.yaml:1077`: HOST-85, HOST-86, HOST-88
- `host.yaml:1133`: HOST-100, HOST-101, HOST-121
- `host.yaml:1157`: HOST-90, HOST-96
- `host.yaml:1161`: HOST-91, HOST-92, HOST-93
- `host.yaml:1178`: HOST-87, HOST-129, HOST-130, HOST-131
- `host.yaml:1208`: HOST-15, HOST-47, HOST-103 to HOST-107
- `host.yaml:1214`: HOST-108 to HOST-115
- `host.yaml:1216`: HOST-28, HOST-29, HOST-30
- `host.yaml:1242`: HOST-37
- `host.yaml:1248`: HOST-37
- `host.yaml:1289`: HOST-79, HOST-97, HOST-100

**(b) Where the model as merged contradicts this document.**

1. `swell.host.EvaluateModule`, outcomes `step-budget`, `memory-budget`, `interrupted` and
   `engine-fault` (`updates: swell.host.Module`, the module left `Linked`): decision 10 leaves
   the graph's evaluating modules at `evaluating`, marked terminated, and a later
   `EvaluateModule` answers `ModuleTerminated`; the model accepts a second `EvaluateModule` and
   moves the module to `Evaluated`.
2. `swell.host.PerformMicrotaskCheckpoint`, the comment "runs promise jobs until the queue is
   empty, each with the budget": decision 11 gives the whole checkpoint one budget.
3. `swell.host.CreateAgent`, inputs `hash_seed` and `random_seed` of type `Bytes`: decision 3
   takes exactly 16 and 32 bytes; an input of another length has no call in the interface.
4. The comment above `swell.host.HostEnqueueGenericJob`, "which runs it through `call` when it
   chooses": decision 11 runs it through `run_job`.
5. `swell.host.EngineFault`, summary "Terminated(EngineFault): ... answers every call with it":
   decision 6 answers `HostError::Faulted`, not a `Terminated` outcome.

## How the gate holds this document

- `conformance/host/requirements.txt` has one line per requirement of this document, sorted by
  number, in one of three forms:
  - `HOST-<n> test <crate>::<test path>`: a test holds it;
  - `HOST-<n> pending slice <t>`: not implemented yet; slice t, the requirement's mark, will
    close it;
  - `HOST-<n> omitted <reason>`: a deliberate omission that stays, with its reason.
- `conformance/slices-delivered.txt` names the delivered slices, one number per line (a line
  starting with `#` is a comment). A slice is complete only when no requirement is pending for
  it: the coordinator adds a slice's number in the change that closes the slice.
- `task host-requirements-check` fails on: a requirement of this document with no line, stated
  twice, or listed twice; a line whose identifier this document does not have; a line of none
  of the three forms; an `omitted` line whose reason starts with `slice` and ends with
  `not delivered` (the old form of `pending`); a `pending slice <t>` line whose t is not the
  requirement's mark; a `pending` line whose slice `slices-delivered.txt` names; a line of that
  file that is not a comment, blank or a slice number; every `test` line, until the first story
  that writes one also checks that its test exists; and an `UNMAPPED:` marker under `spec/`
  that names neither an existing `HOST-<n>` nor `ESS limit`, or names a `HOST-<n>` this document
  does not have. It prints the three counts and the pending requirements by slice.
- A requirement is written here before its code. A story that implements part of the host
  interface names the `HOST-<n>` it closes, and the change that closes one turns its `pending`
  line into a `test` line.
- An identifier is never reused. A requirement withdrawn later keeps its number unused.
- Every change of the host interface is named in the changelog. Added on 2026-11-22. A public
  item of `crates/swell/src/host/` and `crates/swell/src/value.rs` (a function or method, a
  type, a variant, a `pub` field, a constant, a method of the `Host` trait or of
  `scoped_methods!`) or a `HOST-<n>` requirement of this document that changed since the last
  release, added and removed included, stands in backticks under a heading `### Host interface`
  of `## [Unreleased]` in `CHANGELOG.md`, in one name form, which `docs/releasing.md` states: a
  function or method by its bare name (`charge_steps`), with its type before it where two types
  of the interface have a method of that name (`Agent::host`); a method of `scoped_methods!`,
  which `Cx` and `HookCx` both have, once, by its bare name (`get`); a variant or a field as
  `Type::name` (`HostError::StaleHandle`, `ErrorSite::line`); a type, trait, constant or type
  alias by its name (`HostBytes`); a requirement as `HOST-<n>`. A ceiling of decision 14 raised
  (the constant's value) and an entry added to decision 7's closed list (a variant) are such
  changes.
  The gate holds it from the tree alone: `host-surface-check` compares a scan of the tree with
  `conformance/host/surface.txt`, the scan at the last release, renewed in the commit of each
  release, and fails, one line per name, for a changed name the heading does not hold in that
  form.

## Not checked

- `min_stack_bytes` and the Rust stack one re-entry needs: no code exists; slice 1 measures it.
- The generator `swell-num` uses for `Math.random`, and so how much of the 32-byte seed it uses.
- HTML 7.2.3 (`WindowProxy`) and `Location`, on `nav-history-apis.html`, were not read here.
- Whether the draft's text allows the `RangeError`s of decision 2 for strings, BigInts and
  depth: `grep -i 'implementation limit'` finds no such text, and no other wording was looked
  for.
- That HTML's Indexed Database clean-up, run after the microtask queue drains, runs no script:
  its algorithm is on another page and was not read. "Notify about rejected promises" was read:
  it queues a global task and runs no script itself (`html:notify-about-rejected-promises`).
