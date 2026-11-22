---
title: Roadmap
description: The nineteen delivery slices of swell, what each brings, its test262 cases, what passes today and which are delivered.
---

import {Num, S, Delivered, Pending} from '@site/src/components/Numbers';

# Roadmap

swell is delivered in slices, from the [design, section 12](https://github.com/beyond10x/swell/blob/main/docs/design/engine.md#12-delivery-order). A slice ends with the gate green and its test262 cases passing, but for an exception the design states and the [status](status.md#by-delivery-slice) page names. <Num at="delivered_count" /> of the <Num at="delivery_slices" /> delivery slices are delivered.

A case counts in the first slice after which it can pass, by its directory, its features and its flags. Cases that name no feature are counted by directory, so the counts say when a case can first pass, not that it will.

| slice | name | brings | cases | pass today | host requirements pending | state |
|---|---|---|---|---|---|---|
| 1 | <code><S n={1} f="name" /></code> | the parser and compiler for the ES5 grammar with strict mode, the interpreter, the heap, the builtins the harness calls, the runner | <S n={1} f="cases" /> | <S n={1} f="passed" /> | <Pending n={1} /> | <Delivered n={1} /> |
| 2 | <code><S n={2} f="name" /></code> | the rest of the ES5 language: `eval`, `with`, `arguments`, labels, automatic semicolon insertion, the global `Function` | <S n={2} f="cases" /> | <S n={2} f="passed" /> | <Pending n={2} /> | <Delivered n={2} /> |
| 3 | <code><S n={3} f="name" /></code> | `Object`, `Array`, `String`, `Error`, `JSON`, the remaining global functions, the rest of `Function` | <S n={3} f="cases" /> | <S n={3} f="passed" /> | <Pending n={3} /> | <Delivered n={3} /> |
| 4 | <code><S n={4} f="name" /></code> | `swell-num`, the rest of `Number`, `Math`, `Date`, `parseInt`, `parseFloat` | <S n={4} f="cases" /> | <S n={4} f="passed" /> | <Pending n={4} /> | <Delivered n={4} /> |
| 5 | <code><S n={5} f="name" /></code> | `swell-regexp`, `swell-unicode`, `RegExp` and the string methods that use it | <S n={5} f="cases" /> | <S n={5} f="passed" /> | <Pending n={5} /> | <Delivered n={5} /> |
| 6 | <code><S n={6} f="name" /></code> | `let`, `const`, destructuring, spread, arrows, templates, `Symbol`, iterators, `for-of` | <S n={6} f="cases" /> | <S n={6} f="passed" /> | <Pending n={6} /> | <Delivered n={6} /> |
| 7 | <code><S n={7} f="name" /></code> | generators: frames saved in objects | <S n={7} f="cases" /> | <S n={7} f="passed" /> | <Pending n={7} /> | <Delivered n={7} /> |
| 8 | <code><S n={8} f="name" /></code> | classes, `super`, `new.target`, private names, static blocks | <S n={8} f="cases" /> | <S n={8} f="passed" /> | <Pending n={8} /> | <Delivered n={8} /> |
| 9 | <code><S n={9} f="name" /></code> | `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect` | <S n={9} f="cases" /> | <S n={9} f="passed" /> | <Pending n={9} /> | <Delivered n={9} /> |
| 10 | <code><S n={10} f="name" /></code> | jobs, `Promise`, async functions and generators, `for await` | <S n={10} f="cases" /> | <S n={10} f="passed" /> | <Pending n={10} /> | <Delivered n={10} /> |
| 11 | <code><S n={11} f="name" /></code> | module records, the loader hook, dynamic `import`, `import.meta`, JSON modules | <S n={11} f="cases" /> | <S n={11} f="passed" /> | <Pending n={11} /> | <Delivered n={11} /> |
| 12 | <code><S n={12} f="name" /></code> | `BigInt` | <S n={12} f="cases" /> | <S n={12} f="passed" /> | <Pending n={12} /> | <Delivered n={12} /> |
| 13 | <code><S n={13} f="name" /></code> | `ArrayBuffer`, typed arrays, `DataView`, resizable buffers, base64 | <S n={13} f="cases" /> | <S n={13} f="passed" /> | <Pending n={13} /> | <Delivered n={13} /> |
| 14 | <code><S n={14} f="name" /></code> | `SharedArrayBuffer` and `Atomics` for one agent | <S n={14} f="cases" /> | <S n={14} f="passed" /> | <Pending n={14} /> | <Delivered n={14} /> |
| 15 | <code><S n={15} f="name" /></code> | `Iterator` helpers, `using`, `DisposableStack` | <S n={15} f="cases" /> | <S n={15} f="passed" /> | <Pending n={15} /> | <Delivered n={15} /> |
| 16 | <code><S n={16} f="name" /></code> | `WeakRef`, `FinalizationRegistry`, `$262.gc` | <S n={16} f="cases" /> | <S n={16} f="passed" /> | <Pending n={16} /> | <Delivered n={16} /> |
| 17 | <code><S n={17} f="name" /></code> | `PrepareForTailCall` | <S n={17} f="cases" /> | <S n={17} f="passed" /> | <Pending n={17} /> | <Delivered n={17} /> |
| 18 | <code><S n={18} f="name" /></code> | Annex B and `[[IsHTMLDDA]]` | <S n={18} f="cases" /> | <S n={18} f="passed" /> | <Pending n={18} /> | <Delivered n={18} /> |
| 19 | <code><S n={19} f="name" /></code> | what `test/staging/` asks beyond the above | <S n={19} f="cases" /> | <S n={19} f="passed" /> | <Pending n={19} /> | <Delivered n={19} /> |

Slices 2 to 5 are the language and library of ES5. Slices 6 to 8 are the syntax most later tests are written in. Slice 10 needs the suspended frames of slice 7, and slice 13 needs slice 12 for `BigInt64Array`. Slices 20 and 21, `Temporal` and `Intl`, are excluded at first, as are the features at stage 3.

## For surf

A script that touches no DOM and uses only ES5 can run after slice 4, and with regular expressions after slice 5. Most present-day scripts with no DOM, with classes, `async` and modules, can run after slice 11. A script on a real page becomes realistic after slice 18 on swell's side, and only once surf has the DOM, Web IDL bindings and an event loop. surf has none of these yet.
