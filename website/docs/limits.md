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
| in a regular expression: the flags `i` and `v`, property escapes and group names outside ASCII | the rest of slice 5, `regexp` |
| identifiers outside ASCII | the rest of slice 5, with the Unicode tables |
| generators | slice 7, `generators` |
| classes, `super`, private names | slice 8, `classes` |
| promises, async functions, `for await` | slice 10, `promises-and-async` |
| modules, dynamic `import`, `import.meta` | slice 11, `modules` |
| the operator `**` | slice 4, which brings the deterministic `pow` |
| the web-compatibility semantics of function declarations in blocks (Annex B.3.2) | slice 18, `annex-b` |

## The builtins

The core library exists: `Object`, `Function`, `Array`, `String`, `Error`, `JSON`, `globalThis` and the URI functions, and with slice 6 `Symbol`, the Array and String iterators, `Array.from`, `Object.fromEntries` and `Object.groupBy`. Regular expressions run since release 0.6.0: the literal, `RegExp`, `exec` and `test`. Since release 0.7.0 `match` and `search` exist, and `replace`, `replaceAll` and `split` of `String` take a regular expression. The case methods, `normalize` and `localeCompare` take every String. A part of a regular expression that is not built yet is refused with a counted `SyntaxError`.

| missing | brought by |
|---|---|
| `Date`, `parseInt`, `parseFloat`, the rest of `Number`, all of `Math` but `max` | slice 4, `numbers-and-dates` |
| `String.prototype.matchAll` and `RegExp.escape` | the rest of slice 5, `regexp` |
| `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect` | slice 9 |
| `Promise` | slice 10 |
| `BigInt` | slice 12 |
| `ArrayBuffer`, typed arrays, `DataView` | slice 13 |

## The measure

<Num at="test262.passed" /> of the <Num at="test262.run" /> test262 cases the gate runs pass, <Num at="test262.passed_percent" /> %. <Num at="test262.excluded" /> more are excluded by recorded decision. Slices 1, 2 and 6 pass all their cases in all their runs, and slice 3 all but one. The [status](status.md) page has the counts per slice.

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
