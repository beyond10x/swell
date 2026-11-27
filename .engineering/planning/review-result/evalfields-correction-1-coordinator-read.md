---
format: aep.planning-md/3
id: review-result:evalfields-correction-1-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 1 of the unit evalfields
relations:
- reviews: story:eval-in-class-field-initializers
revision: 1
---
Coordinator's read of correction 1 of the unit evalfields (impl/evalfields, 5451f25 to
6547693), 2026-11-27. Verdict: no defect.

Correction 1 is one commit (6547693: three files, 137 lines added, 8 removed; tests and one
doc sentence, no engine code changed). It answers review-result:adversary-evalfields-pass-1,
whose pass was green: no failing case on the unit's code, three single-line changes of that
code that every test passed, and one judgement note. Read: the whole diff of 6547693.

- Finding 1 (`in_class_field_initializer: in_initializer || in_method` in
  `Agent::eval_context` passed every test of `swell::main` and `swell-syntax::main`). New test
  `class_field_eval::eval_of_arguments_in_a_method_of_a_class_with_fields_reads_its_own`: a
  direct eval of `arguments` in the constructor body, a method, a static method and a getter
  of a class with a field, and in a method of an object literal, reads that function's
  `arguments`, in both modes. Red under the mutant: `Throw("SyntaxError")` against
  `Normal("true")`. It also holds what the removed test of the refusal held for an eval in a
  method of a class with fields.
- Finding 2 (the walk of ContainsArguments looking into `FunctionKind::Declaration` passed
  every test). `class_field_eval::arguments_in_a_function_inside_the_eval_is_allowed` gains a
  function declaration, a generator declaration, an `async` function, an `async` method and a
  setter, each reading its own `arguments` in the eval code of an initializer; it held a
  function expression, a method, a getter, a generator expression, a class's constructor and
  method before. Red under the mutant.
- Finding 3 (the credit of the walk's stack removed passed the 13 tests; the tree then held
  1,024 bytes over its capacity). New test
  `sources::contains_arguments_of_eval_code_holds_only_the_tree` in the syntax suite, with a
  ledger that counts charges and credits: eval code of 121 numbers with the flag set leaves the
  ledger holding the tree's bytes, equal to `capacity_bytes()`; with `arguments` at the end
  the parse answers the one error and the ledger holds nothing; with each of the 263 and 261
  charges refused in turn the parse answers `Stopped(OverBudget)` and the ledger holds
  nothing. Red under the mutant: 12489 against 11465. 0.056 CPU s alone.
- Finding 4 (the walk's error is answered before a held refusal of the eval code, so such
  eval code counts no refusal). No change of code: the parser's rule on holds orders every
  error before a held refusal, PerformEval leaves the order of its checks to the engine, and
  none of the story's 84 cases reaches it. The doc comment of `parse_eval_with` states it.

Each mutant was applied to the tree for one run and the file restored from the head; the
tree was clean with three changed files before the commit. The adversary also ran the four
planted defects the unit had only reasoned about: each red (the flag left false turns four
tests red, not three as the unit read).

Not covered by the pass or the correction: the story's rerun after `story:class-static-blocks`
is merged (an eval in a static block), which that unit's acceptance holds; the stack lanes
in the release and stack-debug profiles were run by the unit, not by the adversary.
