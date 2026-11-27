---
format: aep.planning-md/3
id: review-result:reflect-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit reflect: tests only, no defect'
relations:
- reviews: story:reflect-namespace
revision: 1
---
Coordinator's read of correction 1 of the unit reflect of wave big-2
(`impl/reflect`, `3cc59442..8a2c03f4`, one file, `crates/swell/tests/builtins_reflect.rs`,
594 lines added, 10 removed), 2026-11-26. The adversary's pass 1 was green with 5 notes and
the correction changes tests only, so this read stands in for a second pass.

## What was read

`git diff --stat 3cc59442 8a2c03f4`: one file, under `crates/swell/tests`; no line of
`crates/swell/src`. In the file: `charge_counts`,
`reflect_charges_are_given_back_on_every_way_out` and `charge_rounds_in_one_agent` whole;
`reflect_get_with_an_undefined_receiver_runs_the_getter_with_undefined`,
`reflect_set_with_an_undefined_receiver_runs_the_setter_with_undefined` and
`reflect_delete_property_runs_the_delete_of_an_array` whole; the doc comments of the
interrupt test and of `reflect_reads_its_arguments_in_the_order_of_each_clause`. The other
11 tests are the adversary's own cases of pass 1 under permanent names; the adversary ran
them green on `3cc59442`.

## The five notes of pass 1

- F1 (the interrupt test raises its interrupt after `ownKeys` returned): the unit read
  `charge_steps` (`host/agent.rs:998`), the one place the flag is read, called once per
  builtin call and once per element of the answer. Without a Proxy no host or script code
  runs inside `ownKeys`, so no test of this unit can raise the flag there. The doc comment
  of the test says what it holds. The trap of
  `story:proxy-descriptor-prototype-and-extensibility-traps` can, and that unit's brief
  gets the line.
- F2 (the per-element charge of the argument list): the doc comment of the memory test now
  says it holds that a long copy ends at the memory budget and that its charge is given
  back, and not that each element is charged. No test holds that each element is charged;
  the charge is made by `create_list_from_array_like` of `builtins/function.rs`, older than
  the unit, and the growth lint of wave big-1 holds its reservation.
- F3 (the credit of the value list of `ownKeys`): the charge test reads `heap_bytes()` after
  each entry of each script alone and refuses a reading that differs from the second. With
  the credit dropped the unit showed it red (`ownKeys: [366356, 366900, 367444, 367988]`).
- F4 (`Reflect.get` with an explicit `undefined` receiver): a test, shown red with the
  mutant; the same for `Reflect.set`.
- F5 (`Reflect.deleteProperty` on a dense Array): a test, shown red with `ordinary_delete`
  in place of the target's own [[Delete]].

## What the read found

- No defect in the lines read. No `#[allow`, no `#[ignore`, no name with `adversary_`.
- The module has 28 tests (13 before); the story's acceptance names 13. The coordinator
  states the delivered count in the story at the close.
- Three tests pass 1 CPU second with a collection at every safe point (1.63, 1.48 and 1.25,
  measured by the unit on a loaded machine), inside the story's bound of 2; the module's
  sum there is 13.5 CPU seconds over 28 tests.
- One doc comment line of the charge test is longer than 100 columns; `cargo fmt --check`
  exits 0 on it.

## Verdict

The correction closes F3, F4 and F5 with tests shown red, and states F1 and F2 where the
tests are. The claim is run again by the coordinator on `8a2c03f4` before the merge and
recorded as verification.

```findings
[]
```
