---
format: aep.planning-md/3
id: review-result:gate-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s reading of correction 2 of the unit gate (core-builds-without-std, step 4): approve'
relations:
- reviews: story:core-builds-without-std
revision: 1
---
The coordinator's reading of correction 2 of the unit `gate` of wave big-1
(`story:core-builds-without-std`, step 4), `impl/nostd-gate` at `659ee99a` over `4503978a`
(8 files, 256 lines added, 40 removed), 2026-11-26. Two adversary passes ran
(`review-result:adversary-gate-pass-1`, `review-result:adversary-gate-pass-2`); the budget is
two, so this diff was read by the coordinator and no third pass ran.

Verdict: approve.

## The seven findings of pass 2

1. The acceptance line on an example host without std: answered on the wave branch by the
   rewritten line of the story (`2d04fa12`), not by the unit.
2. `crates/swell/tests/source_scan.rs`, the angle count: a `<` counts as an angle bracket only
   right after a name or `::` and not as `<<` or `<=`; a `>` only right after a non-blank that
   is neither `-` nor `=`; the count cannot go below zero. Read against the forms rustfmt
   writes: `n < 3`, `1 << 7` and `a >= b` open nothing; `Vec<u8>`, `f::<T>` and a closing `>>`
   count as before. Where the scan takes a generic `<` for a comparison (a qualified path that
   starts with `<`), a comma ends the test item early and more code is scanned, which is the
   side that fails loudly. Two cases of the adversary pass now.
3. The same file, a `)` or `]` at depth 0 ends the test item; the depth is decremented only
   above zero. One case of the adversary passes now.
4. `taskfiles/forge.yml`, `toolchain-check`: reads an array over several lines and strings in
   `""` or `''`, stops at a comment, and exits 1 naming the key when `rust-toolchain.toml` has
   a `components` or `targets` key from which it read no string. Two scripts under
   `tests/gate/` hold it (the adversary's and one for the single-quoted form).
5. `crates/swell/tests/emulated_host.rs`: `system_api` names `print!`, `println!`, `eprint!`,
   `eprintln!` and `dbg!` by whole word; the case expects `["print!", "eprintln!"]` for a
   source with `eprintln!` and `print!`, so `eprint!` and `println!` are not reported for
   them.
6. `docs/design/engine.md` and `docs/design/host-interface.md`: the reason now says a program
   that runs the engine without std needs a global allocator, one over a fixed block needs
   `unsafe impl GlobalAlloc`, and a host library without std would need neither and would run
   nothing; both name what the gate holds in its place.
7. `taskfiles/nostd.yml`, the summary: says the build makes `swell` and its normal
   dependencies without features, and that the scan of HOST-2 also reads target-specific and
   optional dependencies, of which no engine crate has one.

## The unit's runs, from its report

`15 tests run: 15 passed` for `source_scan::` and `emulated_host::` (11 before the adversary's
cases); `gate-selftest: 88 passed, 0 failed` (86 before); `task forge-check`, `task
nostd-build`, `task nostd-symbols` and `task clippy` exit 0, with the lines of the wave page.
The coordinator runs the two test commands again on the wave branch after the merge and
records them as verification.

```findings
[]
```
