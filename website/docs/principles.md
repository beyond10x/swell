---
title: Principles
description: The seven principles swell is built by, what checks each one, and what swell is not.
---

# Principles

The principles are those of the [README](https://github.com/beyond10x/swell#principles). The right column gives what holds each one in the gate, `task check`, where something does.

| principle | what it says | what checks it |
|---|---|---|
| Specification first | ECMA-262 states the language, and swell follows it by its section numbers and terms. | test262 at a pinned commit, counted per directory, per slice and per section of ECMA-262 |
| Complete means complete | A part of the standard is marked complete only when every test262 case for it passes and nothing is left out of it. | the section lines of the test262 runner and the omission lists of `conformance/test262/sections.txt` |
| No third-party code in what ships | The Rust standard library and the operating system only. | `lock-check` over the root `Cargo.lock` |
| No unsafe code | Every crate forbids it. No JIT compiler, no executable memory. | `forbid-unsafe-check`, and a source scan for `HOST-1` |
| No ambient authority | Everything from outside comes through the host interface. | a source scan for `HOST-2`: no clock, file, network, process, environment or random source access |
| Bounded | Steps, memory, stack depth and regular expression matching have limits. | a test that reaches each limit |
| Small | Nothing is added before a planned step needs it. | no check: the delivery slices order the work |

A section of ECMA-262 is marked complete only when every case of its test262 paths passes and its list of omissions is empty. No section is marked complete yet.

The rule on third-party code covers what ships. Build and test tooling may use a third-party crate in a separate workspace that the engine does not depend on, as the ESS conformance runner under `conformance/ess` does.

## What swell is not

| swell is not | what that means |
|---|---|
| fast first | It has no JIT compiler, a compiler that writes machine code while a program runs. swell writes no machine code. |
| a wrapper | It does not wrap V8, SpiderMonkey, JavaScriptCore or any other engine. |
| complete | Nothing is marked complete yet. The [status](status.md) and [limits](limits.md) pages say how much is missing. |
| a browser | The DOM and everything else a web page sees beyond the language belong to the program that embeds swell. |

## What the absence of unsafe code removes

With `#![forbid(unsafe_code)]` in every crate and handles that are indices into vectors, an engine bug cannot read or write memory outside Rust's own checks. That rules out use after free, type confusion between memory layouts and an unchecked out of bounds access. With no JIT compiler, swell writes no executable memory, so the attacks that turn a type confusion into machine code do not apply. What the Rust standard library does with unsafe code is outside this claim.

What remains, and how the design tests it ([section 13](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#13-security)):

| risk | tested by |
|---|---|
| a logic error in the object model, such as a frozen object changed or a stale handle | test262, the generation check, and a test mode that collects at every safe point |
| a panic on unexpected input | clippy's `unwrap_used`, `expect_used`, `panic` and `indexing_slicing` denied in the engine crates, overflow checks in every profile, fuzzing |
| denial of service: an endless loop, deep recursion, huge strings | one test per budget and bound, which must end in its outcome |
| a host object that trusts a value | brand checks, the invariant checks of host overrides, a test per override |
| a result that differs by platform or by run | the same scripts run twice must print the same bytes |

The threat model assumes the attacker controls the scripts, every value they compute and every string passed to `eval`, `Function` and `JSON.parse`. The attacker does not control the host.
