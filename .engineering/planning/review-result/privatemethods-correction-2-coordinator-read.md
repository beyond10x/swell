---
format: aep.planning-md/3
id: review-result:privatemethods-correction-2-coordinator-read
kind: review-result
status: active
title: Coordinator's read of corrections 2 and 3 of the unit privatemethods
relations:
- reviews: story:private-names-and-private-methods
revision: 1
---
Coordinator's read of corrections 2 and 3 of the unit privatemethods (impl/privatemethods,
e11fb57 to 3df8b6b), 2026-11-27. Verdict: no defect; one branch without a test, sent back
and held by correction 3.

The unit's two adversary passes are spent (`review-result:adversary-privatemethods-pass-1`,
`review-result:adversary-privatemethods-pass-2`, both red), so the correction after the
second pass is read here. Read: the whole diff of `1ec5e24` in `crates/swell/src`
(`dynamic_code.rs`, `env/mod.rs`, `script.rs`), the reports of both corrections, and the
part runs' lines.

Correction 2 (`1ec5e24`, the merge `b03bc4e` of `wave/big-2`, `d99234d`):

- The blocker of pass 2 (the refusal rewrote the early error of every undeclared private name
  in direct eval code, also where the standard gives the SyntaxError; 7 cases of test262
  regressed). The rewrite now runs only after a parse of direct eval code failed, in
  `Agent::private_names_of_direct_eval`, and only for a name an enclosing class declares: a
  declarative record on the chain of the caller's LexicalEnvironment that names a slot `#x`
  (`Agent::chain_names_slot`). Read for soundness:
  - no binding of the language can be named `#x`, so only a class's private record answers;
  - the heritage runs before the record is entered, and the clause evaluates the heritage
    with the outer PrivateEnvironment, so a name of the class itself is undeclared there;
  - `Function` and an indirect eval have the global environment as their chain and keep the
    SyntaxError;
  - an Object Environment Record is passed without a read of its object, so no trap and no
    getter runs on the error path;
  - the name is looked up as an atom that exists (`Heap::atom_of`); a name outside ASCII
    uses a `ChargedVec` of its UTF-16 units, which credits its charge when it drops;
  - when declared and undeclared names stand in one eval code, the errors of the declared
    names are dropped and the first remaining error is thrown; the `retain` drops only
    messages of the kind `Refusal`, and a parse that holds a refusal of its own holds
    nothing else (`only_the_refusal`), so no other error is lost;
  - one step per record of the chain and the steps of each table's scan are charged before
    the work.
- The warning of pass 2 (a refusal counted beside a thrown grammar error).
  `Agent::syntax_errors` counts a refusal only for the first error, the one answered.
- The part of 242 files the adversary named: 445 runs, 211 passed (193 on the base run of
  `6f21df6d`), no run that passed on the base fails or is refused. 18 runs moved from a
  counted refusal to a pass. The story's own list and its guard, 2,500 files: all passed, no
  refusal.
- The six notes of pass 2: each mutant the adversary named turns a named test red (the
  report gives the lines); the adversary's 13 cases are committed under their own names.

The gap this read found: the branch of `enclosing_class_declares` for a name outside ASCII
ran in no test, and no test said which text of a name written with a Unicode escape the
parser's message carries.

Correction 3 (`3df8b6b`, one test, no engine code):
`private_methods::a_direct_eval_finds_a_name_outside_ascii_and_through_escapes`. The
parser's message carries the StringValue of the name, never the source text of an escape, so
`#ä`, `#\u{e4}`, a name of plane 2 and its escape, and `#a` against `#\u{61}` in both
directions are each found: the refusal; `#ö` in a class that declares `#ä` is the
SyntaxError. A sweep of the memory budget across the eval of `this.#ä` (default build): 223
runs ended in the budget, 98 answered, and the heap reading after each equals the reading
before. Two planted defects: the branch answering "not declared" without a lookup is red in
both builds; the buffer never credited is red in the default build (32 bytes per run).

Not covered: a run that ends at the buffer's own charge cannot be reached from script (the
parse before it needs more room than the 32 bytes); the counts of refusals at
`interp/dispatch.rs` and `builtins/regexp/mod.rs` were read by the unit, not tested for a
double count; the proxy trap-end test ran at padding 0 in the unit's run, and the lane's
realm sizes run it at every padding (the verification below).
