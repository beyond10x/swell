---
title: What swell is
description: swell is an ECMAScript engine written from scratch in Rust, for a program that runs JavaScript it does not trust.
---

import {Num} from '@site/src/components/Numbers';

# swell

swell is an ECMAScript (JavaScript) engine written from scratch in Rust. It parses scripts, compiles them to bytecode and interprets them. It is written to be embedded by [surf](https://github.com/beyond10x/surf), a browser built from scratch, and it is a project of its own.

## Who it is for

swell is for a Rust program that runs JavaScript it does not trust and must stay in control of it. That program, the host, gives swell the clock, the time zone, random numbers, module loading and the scheduling of jobs. swell has no other way to reach files, the network, time or randomness. A run has a step budget, a memory budget and depth limits, and each one ends the run with an outcome the host reads.

## Where it stands

swell runs scripts of the ES5 grammar, in strict and non-strict mode. <Num at="delivered_count" /> of <Num at="delivery_slices" /> delivery slices are delivered, and <Num at="test262.passed" /> of the <Num at="test262.run" /> test262 cases it runs pass. Most of the standard library and the language after ES5 are missing. Release <Num at="version" /> is a library with no binary and no command line.

## Where next

| page | what it holds |
|---|---|
| [Status](status.md) | what passes today, per delivery slice |
| [Limits](limits.md) | what is missing, and the slice that brings it |
| [Architecture](architecture.md) | the crates, the path of a script, the heap and the budgets |
| [Embedding](embedding.md) | the host interface: dependency, entry points, limits, hooks, outcomes |
| [Principles](principles.md) | the seven principles, and what swell is not |
| [Roadmap](roadmap.md) | the delivery slices, their cases and what each brings |

The source is [beyond10x/swell](https://github.com/beyond10x/swell), and its releases are in [`CHANGELOG.md`](https://github.com/beyond10x/swell/blob/main/CHANGELOG.md).
