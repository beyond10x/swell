---
format: aep.planning-md/3
id: review-result:slice-12-revision-2-coordinator-read
kind: review-result
status: active
title: 'Slice 12, revision 2 read by the coordinator: approve, no finding'
relations:
- reviews: story:swell-num-bigint-core
- reviews: story:swell-num-bigint-powers-shifts-and-bits
- reviews: story:bigint-values-and-literals
- reviews: story:bigint-comparison-and-loose-equality
- reviews: story:bigint-arithmetic-and-bitwise-operators
- reviews: story:bigint-constructor-and-prototype
- reviews: story:bigint-json-number-and-joined-cases
- reviews: story:slice-12-conformance-close
revision: 1
---
The coordinator's reading of revision 2 of the eight draft stories of `epic:slice-12-bigint`
(`plan/slice-12`, the diff of eight story files from `82dde80a` to `0a1577f2`), 2026-11-28. It
takes the place of a third round of the plan critics: two rounds ran, as two adversary passes
run on a unit, and the last revision is read by the coordinator.

Verdict: approve. Each of the ten points of round 2 is answered in the text, and the lines of
code and the figures the new text rests on were read in the tree at `0a1577f2`.

- Design 1, acceptance 2 (`swell-num-bigint-powers-shifts-and-bits`): the bounds of `pow_bits`
  are `((base_bits − 1)·e + 1, max(1, base_bits·e))`. For an absolute base of `b` bits, the
  base lies in [2^(b−1), 2^b − 1], so its power of `e` lies in [2^((b−1)e), 2^(be) − 1] and
  has between (b−1)e + 1 and be bits; at e = 0 the pair is (1, 1), the bits of `1n`. The
  enclosure test names the exponents 0 and 1, the bases 0, 1 and −1, the shifts, the 2,000
  bitwise pairs and `as_n_bits` for the bits 0 to 130.
- Parallel-safety 1 (`bigint-values-and-literals`): `crates/compiler/src/lib.rs` is in scope
  and Writes. Read in the code: `Code {` at line 305 with `numbers: constants.numbers` at line
  313, `struct Constants` at line 326, `Constants::number` at line 361, the emitter's `number`
  at line 753, `code_walk` at line 1457. The search the revision made over every addition of
  the slice added `crates/swell/src/host/mod.rs` to the same story (`scoped_get`, line 211,
  hands out `Value::String(_) | Value::Symbol(_) | Value::Object(_)` at line 217, and a BigInt
  names a record too) and the sums over the arenas of `heap/mod.rs` (the `scripts` arena is
  read at lines 353, 380, 399, 416, 428, 449 and 466). The story file has 28 `path:` lines.
- Acceptance 3 (`swell-num-bigint-core`): one generic `zeroed<T>` holds the one
  `try_reserve_exact` and the one `resize`; `to_radix` and `from_radix_digits` answer a
  `Result`. The grep's pattern `vec!|Vec::with_capacity|\.push\(|\.extend` counts 19 lines in
  `crates/num/src/nat.rs` of `f962c324`, so it finds such code where there is some.
- Design 2 (`swell-num-bigint-core`): `mul_with(Method, a, b, counter)` runs one method at any
  length, and both tests of the counts call it.
- Design 3, acceptance 4 (`swell-num-bigint-core`): the "not checked" words of
  `docs/design/engine.md` line 2561 go; of the item of lines 7263 and 7264 only the BigInt
  half goes and a grep holds the "pattern nesting" half. Both places read as the body quotes
  them.
- Acceptance 5 (`bigint-comparison-and-loose-equality`, `bigint-constructor-and-prototype`):
  one assertion of the limit each, with `bigint_bits` 1,000 and 64, and the sentence that the
  String's form is read before the limit.
- Acceptance 1 (`bigint-arithmetic-and-bitwise-operators`): the awk command of the body
  prints 183254036694; a cost that reads lengths only adds 1050158, for 183255086852; both
  are below 2^38.
- The own-commits check: every body tests that `BASE` is set and names a commit before it
  reads `git log --no-merges --first-parent`.
- `get_v` (`bigint-json-number-and-joined-cases`): `crates/swell/src/ops/objects.rs`, `get_v`
  calls `to_object(value)` and then `internal_get(object, key, value)`, the value as the
  receiver, as the body now says.
- Scope's three closing points: the two `omit` lines stand in `sec-bigint-objects` and
  `sec-applystringornumericbinaryoperator`; the token `decision-3` is the one the `string-length-limit` and
  `parser-nesting-limit` lines of `conformance/test262/sections.txt` carry today (lines 22
  and 26 the first of them), and
  `docs/design/host-interface.md` line 181 holds `bigint_bits` in the table of limits; the
  comment of `crates/num/src/literal.rs` line 66 is the core story's.

`aep plan artifact validate`: `valid`. `sh docs/plan/slice-12/lists.sh` over the base run:
`slice 381 lists 381 incoming 0`. `aep plan artifact waves`: five waves for the eight
stories, the three stories of the third order in one wave.

Left for the join of the two slices, the coordinator's: the edge from
`story:slice-12-conformance-close` to the close of slice 11, and the edges between
`bigint-values-and-literals` and the stories of slice 11 that write the same 16 files.

```findings
[]
```
