---
title: Architecture
description: The crates of swell and what depends on what, the path of a script from source text to an outcome, the heap and the budgets.
---

import EngineFigure from '@site/src/components/EngineFigure';
import {Crates} from '@site/src/components/Numbers';

# Architecture

swell is a stack-based bytecode interpreter in safe Rust. The host gives it source text and reads one outcome back. This page follows the design, [`docs/design/engine.md`](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md), sections 2 to 4, 9 and 13.

<EngineFigure variant="full" />

## Crates

The workspace holds these crates today, read from `Cargo.lock` with their dependencies.

<Crates />

`swell` is the engine and its public host interface. `swell-syntax` is the lexer and the parser, `swell-compiler` the bytecode format and the compile step. `swell-regexp` parses a regular expression, compiles it to a program and matches it by backtracking with an explicit stack, charged to the budgets of its caller. `swell-num` parses and prints numbers and holds the keyed hash. `swell-test262` is the conformance runner, and `swell-unicode-gen` writes the Unicode tables of `swell-unicode` and does not ship.

The design plans one more crate: `swell-cli`, a command-line host. A crate is created in the first slice that needs a part of it, with that part only. A crate below `swell` holds no heap value, so it can be tested and fuzzed alone. The builtins stay inside `swell` because they read and write the heap. The dependency graph has no cycle ([design, section 2](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#2-crates-and-what-depends-on-what)).

## The path of a script

1. The host calls `parse_script` with source text. `swell-syntax` parses the ES5 grammar in strict and non-strict mode and checks the early errors of strict code. The answer is `Parsed::Parsed` with a `Script`, `Parsed::Errors` with the `SyntaxError` objects, or `Parsed::Terminated`.

2. `swell-compiler` compiles the tree to bytecode for a stack machine. A name gets a slot at compile time. A name that a `with` statement or a non-strict direct `eval` can change is looked up at run time, one step per environment record it visits.

3. The host calls `evaluate_script`. The run loop of `swell` executes the bytecode over the heap. A frame is data on the engine's own frame stack, not on the Rust stack, so a generator can later be suspended in safe code.

4. A builtin that calls back into script enters the run loop again from Rust. That re-entry counts toward `reentry_depth`.

5. The run ends in an `Outcome`: `Normal` with the completion value, `Throw` with what the script did not catch, or `Terminated` with its reason.

The design rejects a tree walker, which could only suspend a generator by keeping the Rust stack alive. It rejects a register machine for now: a stack machine has fewer ways to be wrong, and the speed difference is not measured ([design, section 3](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#3-execution-model)).

## The heap

An agent has one heap made of arenas, one per kind of thing. A handle is an index and a generation, typed by kind, so a string handle cannot be used as an object. A freed slot's generation goes up, and every access compares it. A stale handle inside the engine ends the run as `Terminated(EngineFault)`, and one the host passes in gets `Err(HostError::StaleHandle)`.

`Value` is a Rust enum of a tag and a payload, and the test `value_is_16_bytes` of `crates/swell` holds its size. NaN-boxing is rejected until a measurement asks for it. The collector is a precise mark and sweep that runs at safe points. Its triggers count bytes, not time, so the same inputs collect the same objects at the same points.

Hash maps inside the engine use a SipHash keyed from the host's seed `hash_key`. The standard `RandomState` reads the operating system's random source, which is ambient authority ([design, section 4](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#4-values-and-the-heap-without-unsafe-code)).

## Budgets

The host sets the limits of an agent when it creates it. A limit of 0 is a limit of zero. A host that wants no practical limit gives the type's maximum.

| limit | counts | when reached |
|---|---|---|
| `steps_per_entry` | steps of one outermost entry with its nested entries | `Terminated(StepBudget)` |
| `memory_bytes` | bytes charged to the heap over its life, a sweep crediting | `Terminated(MemoryBudget)` |
| `frame_depth` | execution contexts on the frame stack | a `RangeError` |
| `reentry_depth` | calls from Rust into code beyond the outermost entry | a `RangeError` |
| `parser_nesting` | nesting levels of the parser | a `SyntaxError` naming `parser_nesting` |
| `string_length` | code units of a string made at run time | a `RangeError` |
| `bigint_bits` | bits of a `BigInt` computed at run time | a `RangeError` |

A step is one bytecode instruction, plus steps that builtins charge in proportion to their work. The counter is checked at backward jumps, calls and returns, re-entries and inside builtin loops, so a loop cannot run without passing a check. The host's interrupt is read at the same places and ends the run as `Terminated(Interrupted)`. After a `Terminated` outcome, no `catch` or `finally` block of the script runs.

A script sees a `RangeError` for depth and can catch it. A script that catches it and recurses again ends at the step budget, which it cannot catch. `Limits::min_stack_bytes()` gives the Rust stack a thread needs for an agent's limits ([design, section 3](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#3-execution-model), [host interface, decision 2](https://github.com/beyond10x/swell/blob/main/docs/design/host-interface.md#2-creating-an-agent-and-its-limits)).

## Sources

- [Section 9, the host interface](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#9-the-host-interface), settled in [`host-interface.md`](https://github.com/beyond10x/swell/blob/main/docs/design/host-interface.md).
- [Section 13, security](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#13-security): the threat model and what the absence of unsafe code removes.
