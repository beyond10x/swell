---
format: aep.planning-md/3
id: review-result:classsyntax-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit classsyntax: tests only, no defect'
relations:
- reviews: story:class-syntax-and-early-errors
revision: 1
---
Coordinator's read of correction 1 of the unit classsyntax (impl/classsyntax, 1e671094 to
7494b14c), 2026-11-26. Verdict: no defect; the correction adds tests and fixtures only.

The correction answers review-result:adversary-classsyntax-pass-1, whose findings 1 to 5 are
changes of the code that the unit's tests did not catch (the code itself had none of the
defects). Its diff is 49 added lines in 4 files, none of them under `src/`:

- `crates/syntax/tests/sources.rs`: the test `code_after_a_class_is_not_strict`. 36 sources (a
  class declaration, a class expression and a class expression with a method, each at top level
  and inside a function, each followed by one of six tails that only non-strict code admits)
  through the three entries, 108 answers. Each must be the hold of `class` alone; the tail
  `if (a) function f() {}` must be the refusal of slice 18. The test lists every wrong answer.
- `early-errors/class-method-name-octal.js` and `early-errors/class-method-name-octal-escape.js`:
  a legacy octal number and a legacy octal escape as the name of a class method are early
  errors (a class body is strict mode code).
- `held/class-string-constructor.js`: a class whose one method is named by the string
  `'constructor'` is held as a class, not reported as a second constructor.

What the implementor measured, in a copy of the tree with each change planted:

- finding 4 (both restores of the parser's strictness deleted): 54 of the 108 answers wrong;
- finding 1 (the walk's restore step dropped): 54 of 108 wrong, the other half. The
  adversary's proposed case `class A {} with (a) {}` catches finding 4, not finding 1: the
  parser checks `with`, the walk does not. The test covers both sites with three tails each;
- finding 2 (literal keys unchecked): each of the two fixtures alone turns two tests red;
- finding 3 (a string-keyed constructor): the held test and the compiler's held test go red.

Finding 5 (the hold of `class heritage` cannot be seen while `class` is held) gets no test
here: the unit that lifts the hold of `class` adds the fixture `held/class-expression-heritage.js`
and a test of the hold's line and column; the texts are kept for its brief. Finding 6 (a
failure message of the stack-bound helper that does not name the refusal) is older than the
unit and stays a note. Finding 7 is a note for the merge with the unit super; finding 8 (the
unit's report counted three helpers made `pub(super)`, the diff has four) changes nothing in
the code.

Two counts of the story's acceptance change with the fixtures: `early-errors/class-*.js` 22 to
24, `held/` 4 to 5. No test holds either number.

Read: the whole diff. Not read again: the implementation, which the correction does not touch.
