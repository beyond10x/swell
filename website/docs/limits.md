---
title: Limits
description: What swell does not do yet, and the delivery slice that brings each part, from the limits of the release and the design.
---

import {Num} from '@site/src/components/Numbers';

# Limits

These are the limits of release <Num at="version" />, from the [changelog](https://github.com/beyond10x/swell/blob/main/CHANGELOG.md) and the [design](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md). A script that uses a construct of a later slice is refused with a `SyntaxError` whose message gives that slice.

## The language

| missing | brought by |
|---|---|
| regular expression literals | slice 5, `regexp` |
| identifiers outside ASCII | slice 5, with the Unicode tables |
| `let`, `const`, destructuring, spread, arrow functions, templates, `Symbol`, `for-of` | slice 6, `bindings-and-iteration` |
| generators | slice 7, `generators` |
| classes, `super`, private names | slice 8, `classes` |
| promises, async functions, `for await` | slice 10, `promises-and-async` |
| modules, dynamic `import`, `import.meta` | slice 11, `modules` |
| the operator `**` | slice 4, which brings the deterministic `pow` |

## The builtins

Most methods of the builtins are missing. Of `Array`, `isArray`, `join`, `map`, `push` and `toString` exist. `Function.prototype.apply`, `JSON.parse`, `parseInt`, `Date`, `RegExp` and all of `Math` but `max` do not.

| missing | brought by |
|---|---|
| the core library: `Object`, `Array`, `String`, `Error`, `JSON`, the rest of `Function` | slice 3, `builtins-core` |
| `Math`, `Date`, `parseInt`, `parseFloat`, the rest of `Number` | slice 4, `numbers-and-dates` |
| `RegExp` and the string methods that use it | slice 5, `regexp` |
| `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect` | slice 9 |
| `BigInt` | slice 12 |
| `ArrayBuffer`, typed arrays, `DataView` | slice 13 |

## The measure

<Num at="test262.passed" /> of the <Num at="test262.run" /> test262 cases the gate runs pass, <Num at="test262.passed_percent" /> %. <Num at="test262.excluded" /> more are excluded by recorded decision. The first two slices pass all their cases in all their runs. The [status](status.md) page has the counts per slice.

## The engine

A property is found by a scan of its object's property list. The comparisons of that scan are not charged to the step budget, so a script with large objects runs longer than its step count says. The slice that builds the design's index map covers it, slice 3 at the latest.

A run that fills a large memory budget with small objects is slow in a debug build. Where the time goes is to be measured before the builtins of slice 3 depend on it.

## The host interface

<Num at="host_requirements.test" /> of the <Num at="host_requirements.total" /> numbered requirements of the host interface are held by a test. None is pending for a delivered slice. The interface will change before 1.0.

## Out of scope at first

| left out | where it stands |
|---|---|
| `Temporal` | slice 20, after slice 19 with a design of its own: it is stage 4 but not in the pinned draft |
| ECMA-402, `Intl` | slice 21, open item A of the design: it needs locale data |
| features at stage 3 | excluded, each by name, until the pinned draft has it |
| concurrent agents, `$262.agent` | until surf needs workers |
| a JIT compiler, machine code, executable memory | not in scope |
| source maps, a debugger protocol, `Error.prototype.stack` | not in scope |
| Web IDL conversions and the DOM | the embedding program's, as surf's |

## The release

swell ships no binary and no command line, and the crates are not on a package registry. A program depends on the repository at the tag, as [embedding](embedding.md#the-dependency) shows. The gate's numbers are measured on x86_64 Linux only.
