---
format: aep.planning-md/3
id: review-result:derived-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit derived: no defect'
relations:
- reviews: story:derived-classes-and-super-call
revision: 1
---
Coordinator's read of correction 1 of the unit derived (impl/derived, 36b82da to 6ae0127),
2026-11-27. Verdict: no defect.

Correction 1 is one commit of tests (fe1f267, two files, 269 lines added, 8 removed; no engine
code changed) and the merge of `wave/big-2` at aa50eaf, which holds the unit fields. It
answers review-result:adversary-derived-pass-1, whose five findings were notes. Read: the
whole diff of fe1f267 and the merge commit's resolution of its three conflicts.

- Finding 1 (the syntax test `a_script_is_not_parsed_as_eval_code` compared a script and eval
  code with no flag by the errors they share, so it held nothing its name said). Eval code
  with every flag of PerformEval false is parsed with the same parser set-up as a script; no
  source tells the two apart, and the implementor's mutant S0 (`parse_script` routed through
  `parse_eval` with every flag off) leaves the 79 tests of the syntax suite green. The test
  is renamed `a_script_refuses_what_each_eval_flag_admits`: for each flag one source that eval
  code admits with only that flag set (`new.target` with `in_function`, `super.x` with
  `in_method`, `super()` with `in_derived_constructor`) is an early error of a script. Mutant
  S1 (`parse_script` through `parse_eval` with `in_method`) turns it red at
  `crates/syntax/tests/sources.rs:815`.
- Finding 2 (mutant Ma: a SuperCall that reads the parent after its arguments passed the 19
  tests). The adversary's five cases are committed under names that say what they hold,
  assertions unchanged; `values_of_a_super_call_survive_garbage` is red with Ma.
- Finding 3 (mutant Mb: no test read the steps of the default derived constructor).
  `the_default_derived_constructor_charges_a_step_per_argument_per_level` reads
  `last_entry_steps()` of `Reflect.construct` with 100 arguments against none: 200 steps more
  straight into the base class (100 for the list of CreateListFromArrayLike, 100 for the
  base constructor's `arguments` object), 500 through three default constructors. Red with
  Mb: (200, 200) against (200, 500).
- Finding 4 (mutant Mc: `KeepThisEnv` hops counted without the environments the constructor
  pushed). `a_return_from_an_inner_environment_keeps_the_bound_this` returns from a block
  with a captured `let`, from such a block in `try` with a `finally`, from a `finally` block,
  from a `for (let ...)` body and from a `catch` block. Red with Mc. The five forms are one
  test and were not each run alone under the mutant; all reach the hop count through the same
  function of the compiler.
- Finding 5 (every arrow inside a derived constructor captures the hidden `this`: 184 live
  bytes more per kept construction, no extra step). No code change: the cost is stated in the
  story's As delivered and in `docs/design/engine.md`, "Classes", item 6, with the reason (a
  read of `this` can sit in a nested arrow or in a direct eval, and finding it would take a
  second pass over the arrow's body).
- The merge. Three conflicts, each resolved by keeping both sides: `HOLDS` of the parser
  (each unit had deleted its own line; neither `class field` nor `class heritage` stays), the
  imports and the two functions `default_derived_constructor` and `instance_initializer` of
  `crates/compiler/src/class.rs`, and the sites and sets of both units in
  `crates/compiler/src/resolve.rs`. The generated `crates/swell/tests/main.rs` merged without
  a conflict and equals what its generator writes.
- Times: each of the seven new tests alone is within the bounds (the largest: 1.228 CPU s in
  the lane's collect build, bound 2). The unit's first largest test,
  `a_chain_of_1000_derived_classes_ends_at_the_step_budget`, took 1.840 CPU s there on a
  loaded build node, bound 2.

What is open after the read: no committed test holds a field of a derived class. A probe on
the merged head gave what the clauses give (the fields run after `super()` returns, also with
the default derived constructor and with `super()` from an arrow; a second `super()` throws
and does not run them again; `super.v` in a field reads the parent's prototype; a static
field's `super.name` reads the parent class's name). The test goes to the next unit that
merges both, the correction of the unit privatemethods.

What the read did not cover again: the unit's first change, which the adversary's pass
covered; the test262 part run on the merged head (the wave's gate runs it); the lane's step
`test-realm-sizes` on 6ae0127, which the coordinator starts beside the merge (the
verification before the merge ran the syntax suite at full counts and
`test-stack-profiles`, not that step).
