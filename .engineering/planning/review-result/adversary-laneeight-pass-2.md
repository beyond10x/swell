---
format: aep.planning-md/3
id: review-result:adversary-laneeight-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on step 1 of the unit laneeight: needs-change, 4 findings in the guard, 4 failing cases'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Adversary pass 2 on step 1 of the unit laneeight of wave big-2, on impl/laneeight at c773ed4a (correction 1, the diff 3dba5760..c773ed4a), 2026-11-28.
The adversary's own report follows as it was given.

```
unit: laneeight step 1, correction 1, head c773ed4a with my 4 cases uncommitted
verdict: NEEDS-CHANGE
cases: executed 298→302, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (<scratch>/wave2/laneeight/adversary-2)
needs-coordinator: none
```

Two of the fixes still let neighbouring forms through. The macro rule misses two shapes, and the empty-list rule misses two spellings; each is a red case below.

**1. Diff of the worktree:**
```
 crates/swell/tests/host_types.rs | 54 ++++++++++++++++++++++++++++++++++++++++
 1 file changed, 54 insertions(+)
```
It is a test file only. `git status --short` shows only that file. The plants in `host_types.rs` and `support/scripted.rs` were put back from saved copies. `rustfmt --edition 2024 --check` exits 0.

**2. Cases** (all in host_types.rs, red now). They ran alone in job A before anything else: `4 tests run: 0 passed, 4 failed`, exit 100.
- `an_impl_of_a_path_that_ends_in_a_macro_variable_is_refused` (:766): `panicked at crates/swell/tests/host_types.rs:769:5: the impl of Host the macro writes for Mine passes the guard: []`
- `an_impl_whose_header_is_a_macro_repetition_is_refused` (:780): `panicked at …:783:5: the impl of Host the macro writes for Mine passes the guard: []`
- `an_own_constant_with_the_empty_list_holding_a_comment_is_refused` (:793): `panicked at …:796:5: an own-constant type with the empty list passes the guard: []`
- `an_own_constant_with_the_empty_list_behind_a_cast_is_refused` (:806): `panicked at …:809:5: an own-constant type with the empty list passes the guard: []`

**3. Runs** (all after the cases existed):
- **Job A, suite:** the 17 folded modules plus `host_types`, `scripted_host` and `hooks`. Result: `302 tests run: 298 passed, 4 failed`, exit 100. Only my 4 failed.
- **Job B, guard mutants:** the planted tree, each mutant on its own.
  - The unit's three new cases each fail alone under the mutant of their rule: G3 fails `a_lifetime_parameter_is_no_type_parameter`, G4 fails `an_own_constant_that_only_reads_the_constant_is_refused`, G15 fails `a_where_clause_is_not_part_of_the_type_name`.
  - Each new rule is held by its own cases:

    | Mutant | Rule taken out | Cases that fail |
    |---|---|---|
    | G21 | macro-variable rule | the F1 case and `an_impl_of_a_macro_variable_names_the_variable` |
    | G22 | the `$crate::` exception | `an_impl_of_a_macro_variable_names_the_variable` |
    | G23 | support-file rule | `provided_defaults_is_taken_once_each_and_only_in_the_support_file` |
    | G24 | once-each rule | `provided_defaults_is_taken_once_each_and_only_in_the_support_file` |
    | G25 | empty-list rule | both empty-list cases |
    | G26 | name rule | `a_third_provided_defaults_is_refused` |

- **Job B, `scripted_host` mutants:**
  - S1, empty type name in the accessor's message: fails `the_accessor_of_another_type_panics_naming_both`.
  - S2, the required clock entry made a no-op: fails `a_required_hook_left_to_the_provided_table_panics_naming_it`.
  - S3, empty type name in `state_mut`'s message only: `2 passed`, not caught.

**4. Findings:**
- **F9** (host_types.rs:239): the macro rule only catches a trait path that starts with `$`, so two forms still pass. Origin: introduced (the F1 fix).
  - `impl swell::$t for $ty {}` called with `Host`: the last segment is a variable.
  - `impl $($header)* {}` called with `swell::Host for Mine`: no `for` in the text, so `split_header` skips it.
  - Fix: refuse a variable as the last segment of the trait path, and an `impl` inside a `macro_rules!` body whose header has a `$` and no `for`.
- **F10** (host_types.rs:224, `constant_value`): the value is read from the raw source, so a comment counts as part of it. `&[ /* none yet */ ]` and `&[] as &[&str]` pass as non-empty values. A named empty constant (`= NONE;`) also passes; a text comparison cannot follow a name. Origin: introduced (the F3 fix). Fix: strip comments only, not string contents. Refuse a cast, or any value that is not a list literal.
- **F11** (host_types.rs:239 with `starts_an_item` removed): the unit argued that an `impl Trait` in type position never has a `for` before the next `{`. That is false: an argument `impl $tr` followed by `where for<'a> &'a T: Copy`, or by an argument of type `for<'a> fn(&'a u8)`, is now refused by the macro rule. Removing the rule only widens the scan, so no form is newly accepted. A false refusal of `Host` itself needs `Host` right before a top-level `for`, which a type position cannot hold. No macro of the tests has this form. Origin: introduced.
- **F12** (support/scripted.rs:155): no test pins `state_mut`'s own panic message; mutant S3 survives. The F8 test covers only `state`. Origin: introduced.

**5. Attacked, could not break:**
- `impl $t for` with any fragment kind, and `$($p)::+` as the trait path: refused.
- Must still pass, and do: `impl $crate::Trait for`, `impl $name {}` with no `for`, `impl Trait for $t`.
- Support-file rule: a path ending in `support/host.rs` deeper under the tests is refused, because the name is compared exactly; the same type marked twice in the file is refused.
- G2 removed (body_start no longer stops at an unopened `)` or `]`): it only affects type-position `impl`s, and no form is newly accepted. The unit's argument holds.
- G14 removed: in a `use` tree, `Host` is followed only by `,`, `}`, `;` or `as`. The unit's argument holds.
- The real tree: no `macro_rules!` under `crates/swell/tests/` writes an `impl`.
- Not attacked: the folds, the copies and the build readings (outside this pass, per the brief).

**6. Paths written outside the worktree:** <scratch>/wave2/laneeight/adversary-2/, holding:
- `orig/host_types.rs` and `orig/scripted.rs` (saved originals)
- `host_types.cases.rs`, `host_types.planted.rs`, `scripted.planted.rs`
- `plant-guard2.awk`, `plant-scripted2.awk`
- `jobA.sh`, `jobA.out`, `jobB.sh`, `jobB.out`

```findings
- file: crates/swell/tests/host_types.rs
  line: 239
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the macro rule only refuses a trait path that starts with a variable, so impl swell::$t for and an impl whose whole header is a token repetition still write an unmarked impl of Host
- file: crates/swell/tests/host_types.rs
  line: 224
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the constant's value is read from the raw source, so the empty list holding a comment or behind a cast, and a named empty constant, pass as values of their own
- file: crates/swell/tests/host_types.rs
  line: 239
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: with starts_an_item removed, a type-position impl of a macro variable followed by a higher-ranked where clause or fn-pointer argument is refused, which disproves the stated reason for the removal though no macro of the tests has the form
- file: crates/swell/tests/support/scripted.rs
  line: 155
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: no test pins the panic message of state_mut, so a change to it stays green
```
