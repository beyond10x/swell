---
format: aep.planning-md/3
id: story:a-parse-error-and-an-uncaught-throw-name-their-line-and-column
kind: story
status: implemented
title: After an entry that answered a parse error or an uncaught throw, the host reads the line and column where it arose
summary: One read of the agent, last_entry_error_site, gives the position of the first parse error of parse_script and of the instruction where the throw that ended an outermost entry arose, whether it arose in dynamic code, and the [[HostDefined]] of its script; no result type of an entry point changes.
relations:
- depends_on: story:interrupt-read-at-every-parse-entry
- depends_on: story:for-in-key-list-charged-while-the-loop-holds-it
- depends_on: story:frontend-scratch-on-the-memory-ledger
- depends_on: story:let-const-and-block-scopes
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:28Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:28Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:interrupt-read-at-every-parse-entry` (both write
`Agent::parse_script`, `crates/swell/src/host/agent.rs:443-486`),
`story:for-in-key-list-charged-while-the-loop-holds-it` (it writes `enter_handler` of
`crates/swell/src/interp/dispatch.rs:130-163` and `interp/frame.rs`) and
`story:frontend-scratch-on-the-memory-ledger` (it writes `crates/swell/src/script.rs` and
`crates/swell/src/dynamic_code.rs`, both written here, see Change 3); the merges of the slice 6
units `let-const-and-block-scopes` and `template-literals` (`interp/dispatch.rs`, `script.rs`,
`dynamic_code.rs`). Batch H3. Requirements HOST-172, HOST-173, HOST-174. Consumer: the browser,
requirement 1 of its list. Decision D1 (taken: option A, with its conditions, Change 4).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- A parse error has its position, and the host gets it only as text. `swell_syntax::SyntaxError`
  holds `line` and `column` (`crates/syntax/src/error.rs:76-77`). `Agent::syntax_errors`
  writes them into the message, `"<message> (<line>:<column>)"`, and drops the numbers
  (`crates/swell/src/script.rs:298-311`, the format at `:306`).
- An uncaught throw has no position. Every instruction has a `Span` (`positions` of the script's
  code and of each function template, `crates/compiler/src/bytecode.rs:549`, `:572`), whose
  `line` and `column` are those of the node's first code unit, both from 1, a column in UTF-16
  code units (`crates/syntax/src/ast.rs:11-19`). An instruction takes the span of its
  expression or statement: `GetNamed` the member expression's
  (`crates/compiler/src/expression.rs:432-435`, `:208`), `Call(n)` the call's (`:476-477`),
  `Throw` the statement's (`crates/compiler/src/statement.rs:288-291`).
- Where a throw is seen: `Agent::unwind` (`crates/swell/src/interp/dispatch.rs:91-128`) gets the
  thrown value with the running frame, whose `pc` minus one is the instruction that raised it
  (`:100`). A throw from a callee run in a loop of its own (a builtin's callback, a host
  function's `cx.call`) comes back to the caller's `unwind` as the same value at the call
  instruction. `enter_handler` (`:130-163`) enters a `catch` block with the value, or a
  `finally` block with the value and the tag of a throw completion (`:158-160`), which the end
  of the `finally` block throws again: the same value, a second time through `unwind`.
- A frame holds its code as `compiled: Rc<Compiled>` and `template: Option<u32>`, and its
  ScriptOrModule as `script: Handle`, a record of the `scripts` arena that a collection can free
  (`crates/swell/src/interp/frame.rs:80-110`).
- Eval code and `Function` code each get a script record of their own: `eval_code_of` compiles the
  eval String (`crates/swell/src/dynamic_code.rs:219`) and calls `make_script_record` (`:225`), with the
  caller's [[HostDefined]] for a direct eval (`:346`) and none for an indirect one (`:391`);
  `dynamic_function_steps` compiles `function anonymous(<P>\n) {\n<body>\n}` and calls it with
  none (compiled at `:615`, the record at `:629`), the spans counted in that source text (`:647`). Their frames run that
  record's `compiled`, not the calling script's. Nothing in `Compiled` or `ScriptRecord`
  (`crates/swell/src/script.rs:84-88`) says the record came from dynamic code.
- The reads of decision 13 run no code, call no hook and spend no step (`HOST-102`);
  `last_entry_steps` is one (`crates/swell/src/host/agent.rs:517-520`).

## Change

Failing tests first (Acceptance). Then:

1. **The type and the read** (decision 13 of `docs/design/host-interface.md`):

   ```rust
   pub struct ErrorSite {
       pub line: u32,
       pub column: u32,
       /// The position is in eval code or in the source text of a `Function` constructor,
       /// counted in that code's own text.
       pub dynamic: bool,
       /// The [[HostDefined]] of the script record whose code raised it.
       pub host_defined: Option<HostDefined>,
   }
   impl<H: Host> Agent<H> {
       pub fn last_entry_error_site(&self) -> Result<Option<ErrorSite>, HostError>;
   }
   ```

   `ErrorSite` goes in `crates/swell/src/host/outcome.rs`, the read beside `last_entry_steps`.
   Why `host_defined` is there: the browser's requirement 1 asks for "the line and column of the
   script's source where each arose, so that surf can report where a page's script failed"; a
   page holds many scripts, and a throw in a function one script defined and another called
   arises in the first one's source. The `Rc` the host gave `parse_script` for that script is
   how the host tells which source the line counts in.
2. **A parse error.** `syntax_errors` keeps the line and column of the first error it turns into
   an object; `Agent::parse_script` that answers `Parsed::Errors` records that site, with
   `dynamic` false and the `host_defined` it was given. The message keeps its suffix.
3. **A throw.** The agent records, once per throw, the site of the instruction where the throw
   completion arose in the innermost ECMAScript frame: a `throw` statement (of any value, an
   error or not), or an instruction whose operation threw (a call of a non-callable, a property
   read of `null`, a call into a builtin or a host function that answered `Throw`). The record
   is written in `unwind` when no throw is pending or when the value differs from the pending
   one, and kept while the same value unwinds through callers, builtins and `finally` blocks.
   It ends when a `catch` handler is entered (`enter_handler` with `HandlerKind::Catch`) and
   when a builtin or a host function answers normally after a throw in it (`builtin_call`,
   `crates/swell/src/object/builtin_function.rs:259-295`: the host's own catch). A `finally`
   handler does not end it: `try { null.x } finally {}` keeps the site of `null.x`. The record
   holds what survives the end of the entry and a collection: the frame's `Rc<Compiled>`, its
   `template`, the instruction index, and a clone of the script record's [[HostDefined]] taken
   at the throw (`script_record_host_defined`, `crates/swell/src/script.rs:542`); the thrown
   value it compares by identity only, never reads, and drops with the entry. The line and
   column are read from the code's `positions` only by `last_entry_error_site`. `dynamic` comes
   from a flag `Compiled` gains, set by a parameter of `make_script_record`
   (`crates/swell/src/script.rs:334-388`): false from `parse_script_in` (`:241`), true from
   `eval_code_of` (`crates/swell/src/dynamic_code.rs:225`) and `dynamic_function_steps`
   (`:629`). The record lives on the agent, not in a frame of the run loop.
4. **What the read answers** (D1's conditions). Every outermost entry clears the record at its
   start (`outermost_entry`, `parse_script`, `with_cx`, `create_realm`). After `parse_script`
   answered `Parsed::Errors`, the first error's site. After an entry of `Agent`
   (`evaluate_script`, `call`, `construct`) answered `Throw`, the site of the throw it answered;
   a job entry of slice 10 answers through the same `unwind` and gets the same read. After an
   entry that ended normally, in `Terminated`, in `Parsed::Parsed`, or after a throw the script
   caught: `None`, never the site of an earlier entry. A throw raised outside any ECMAScript frame
   (a host function called by `Agent::call`) has no site: `None`. A position in eval or
   `Function` code reads `dynamic` true, counted in that code's text; any other reads false.
5. **Stays as it is.** `Outcome`, `Parsed` and every signature of an entry point; the message of
   a parse error with its suffix; the step counter and determinism (the site is a function of the
   script and the calls); `Agent::execute` (`crates/swell/src/interp/dispatch.rs:236`) and its
   frame: the record is written in `unwind`, which is not the dispatch, through a function marked
   `#[inline(never)]`.
6. **Out of scope, named.** A site for `cx.parse_script` and for a throw inside a `with_cx`
   closure (no outermost entry of `Agent` answers it); the stack of frames a throw passed; the
   site of an error object's creation as browsers show it for `Error` objects (decision D1,
   option C).

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` section 13 gains the type, the read and, after "Reads run no
code, call no hook and spend no step." (`:1037-1038`): "After an outermost entry that answered
`Parsed::Errors` or `Throw`, `last_entry_error_site` gives the line and column where the first
error or the throw arose, counted from 1 in UTF-16 code units of the source that holds it,
whether that source is eval or `Function` code, and the [[HostDefined]] of its script; after any
other entry it gives none. A throw is recorded where it arose and kept through `finally`
blocks; a `catch` ends it." And the lines of HOST-172 to HOST-174 below, with
`HOST-172 pending slice 6` to `HOST-174 pending slice 6` in `conformance/host/requirements.txt`.

New requirements:

- **HOST-172** (swell:D13) `parse_script` of the two-line source `var a;\n  var 1;` answers
  `Parsed::Errors`, and `last_entry_error_site()` then reads `Some` with line 2, the column the
  error's `message` ends with, `dynamic` false and the `host_defined` given to the parse (the
  same `Rc`). [slice 6]
- **HOST-173** (swell:D13) The script `function f() {\n  null.x;\n}\nf();` answers `Throw` of a
  `TypeError` and `last_entry_error_site()` reads line 2, column 3; `[1].map(function (x) {\n
  throw x;\n});` reads line 2, column 3 (a throw of a value that is not an error is sited at
  its `throw` statement); `try { throw 1; } catch (e) {\n  throw e;\n}` reads line 2, column 3;
  `try { null.x } finally {}` reads line 1, column 7; `eval("1;\n  throw 2;")` reads line 2,
  column 3 with `dynamic` true; `new Function("\n  null.x;")()` reads line 4, column 3 with
  `dynamic` true and `host_defined` `None`. [slice 6]
- **HOST-174** (swell:D13) `last_entry_error_site()` reads `None` after an entry that answered
  `Normal`, `Terminated(StepBudget)` or `Parsed::Parsed`, after the script
  `try { throw 1; } catch (e) {} 1`, and after a `Normal` entry that follows a `Throw` entry;
  reads the same value twice and after a `collect()`; and leaves `last_entry_steps()`,
  `heap_bytes()` and the count of hook calls as they were. [slice 6]

Writes: `crates/swell/src/host/outcome.rs` (`ErrorSite`), `crates/swell/src/host/agent.rs`
(the record field, a `#[cfg(test)]` count of records written, the read, the clears in
`outermost_entry`, `parse_script`, `with_cx` and `create_realm`, the record in `parse_script`),
`crates/swell/src/script.rs` (`syntax_errors`, lines 298-311; a flag of `Compiled`, lines
55-69; the parameter of `make_script_record`, lines 334-388, and its call at `:241`),
`crates/swell/src/dynamic_code.rs` (the two calls of `make_script_record`, lines 225 and 629,
nothing else), `crates/swell/src/interp/dispatch.rs` (`unwind` and `enter_handler`, lines
91-163, and a new `#[cfg(test)]` module at the end of the file),
`crates/swell/src/object/builtin_function.rs` (`builtin_call`, lines 259-291),
`crates/swell/tests/scripts.rs`, `crates/swell/tests/calls.rs`, and in
`conformance/host/requirements.txt` the lines of HOST-172 to HOST-174 only, each from pending to
`test <path>`. Not written: `crates/swell/src/interp/frame.rs` (the record reads the frame's
existing fields), `crates/syntax`, `crates/compiler`, `Outcome` and `Parsed`.

## Acceptance

In `crates/swell/tests/scripts.rs`, each in both collection modes:

- `host_172_a_parse_error_names_its_line_and_column`: as HOST-172, with `Source::Utf8` and
  `Source::Utf16`. Today it does not compile (no `last_entry_error_site`); with a stub answering
  `None`, it fails on the `Some`.
- `host_173_an_uncaught_throw_names_where_it_arose`: the six scripts of HOST-173, and `throw 1`
  on line 1 (line 1, column 1), each through `evaluate_script` of a parsed script whose
  `host_defined` is a fresh `Rc` that the site's `host_defined` is (`Rc::ptr_eq`), the `Function`
  one excepted (`None`); a function defined by a first script and called from a second one names
  the first script's `Rc`; between the entry and the read the test drops every root it holds of
  the scripts and calls `collect()`, and the read answers the same. Today fails as above.
- `host_174_the_error_site_is_a_read`: as HOST-174. Today fails as above.
- `a_parse_error_site_matches_its_message` (a guard): 50 sources with one grammar error each,
  from a generator seeded in the test (an identifier, a literal or a keyword placed where the
  grammar refuses it, at a line and column the generator picks): the site equals the
  `(<line>:<column>)` suffix of the first error's message.

In `crates/swell/tests/calls.rs`, both collection modes:

- `a_throw_from_a_host_function_is_sited_at_its_call`: a host function `fail` that answers
  `Err(Abrupt::Throw(..))`; the script `1;\n  fail();` reads line 2, column 3; a host function
  that calls `cx.call` of a script function that throws, catches the `Abrupt::Throw` and
  answers `Ok`, followed by `throw 3` on line 4 of the same script, reads line 4, column 1;
  `Agent::call` of the host function `fail` itself reads `None`. Today it does not compile.

In the lib test binary, `crates/swell/src/interp/dispatch.rs` (the new module), reading the
`#[cfg(test)]` count of records written:

- `one_site_record_per_throw`: a script whose throw unwinds through three script frames, a
  `map` callback and a `finally` block writes 1 record; `try { throw 1 } catch (e) {} throw 2`
  writes 2; a script of 10,000 loop iterations that throws nothing writes 0. Today it does not
  compile (no count).

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `scripts`, `calls`, `eval`,
`dynamic_function`, `limits_scripts`, `eval_stack` and `budgets` (the last three are the stack
and budget tests, unchanged), the same with `--features collect-every-safe-point`, `slot cargo
nextest run --locked -p swell --profile default --lib` in both modes, `slot cargo clippy
--workspace --all-targets --locked -- -D warnings`; at the end, once, `slot timeout 1800 task test262
TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared with that of `slot timeout 1800 task test262
TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each,
and `cmp` of the two outputs exits 0; and last, once, in the background, `slot timeout 1800 task
host-requirements-check` (it builds every test target of the workspace,
`taskfiles/host-requirements.yml:335` in the gate unit's tree). Never `cargo test -p swell`
whole, `slot task test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except the one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (the count of records
is a field of the agent). Each new test is timed, and one over 5 s is named in the report with
its time. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same script under the
same budgets ends the same way at the same count on every run, and its site is the same.
Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack
tests are unchanged. No third-party crate, no unsafe code.

## Cost

Bound, a counter: `one_site_record_per_throw` reads 1 record per throw that unwinds (one `Rc`
clone of the code and one of [[HostDefined]] each), and 0 records on a run that throws
nothing; one record per parse that answers errors. Nothing is written on a run without a throw.
The stack tests pass on seven eighths of `min_stack_bytes()` unchanged.

## Report

The new tests with their times, and this line for the changelog's "Host interface" heading
(names in the form of `story:every-host-interface-change-is-named-in-the-changelog`):
"`last_entry_error_site` and `ErrorSite` (new), with `ErrorSite::line`, `ErrorSite::column`,
`ErrorSite::dynamic` and `ErrorSite::host_defined`: after `parse_script` answered
`Parsed::Errors` or an entry answered `Throw`, the line and column where the first error or the
throw arose, whether in eval or `Function` code, and the [[HostDefined]] of its script
(`HOST-172`, `HOST-173`, `HOST-174`)."
