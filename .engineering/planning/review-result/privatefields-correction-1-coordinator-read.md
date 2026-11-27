---
format: aep.planning-md/3
id: review-result:privatefields-correction-1-coordinator-read
kind: review-result
status: active
title: The coordinator's read of correction 1 of the unit privatefields
relations:
- reviews: story:private-fields-and-private-in
revision: 1
---
Coordinator's read of correction 1 of the unit privatefields (impl/privatefields, 3ebbc21 to
b30b601), 2026-11-27. Verdict: no defect.

The adversary's pass 1 was green (`review-result:adversary-privatefields-pass-1`: no failing
case, eight findings: two warnings and three notes on what the tests hold, one stale doc
line, one rule of the design that is older than the unit, one time reading). Read: the diff
of `a8f4193` (4 files, 169 lines added, 4 removed: three tests in
`crates/swell/tests/private_fields.rs`, one library test in the test module of
`crates/swell/src/object/private_names.rs`, doc lines in `crates/syntax/src/lib.rs` and
`crates/syntax/tests/sources.rs`; no line of the engine outside a test module), the merge
`b30b601` of `wave/big-2` at `9cdda90`, and the unit's two reports.

Correction 1 (`a8f4193`), each new test shown red with its defect planted in the unit's tree
and the file put back:

- Finding 1 (the first add of a level reserves the room of the whole level): the library
  test `the_first_field_add_of_a_level_reserves_room_for_the_whole_level` reads the charges
  of three adds, `[168, 0, 0]`; with `room.min(1)` planted it reads `[72, 48, 48]`.
- Finding 2 (`#x in` charges its find as a read does):
  `private_in_charges_its_find_as_a_read_does` reads 92 steps between `#f99 in o` and
  `#f0 in o` of a class with 100 private fields (one step per comparison past the first 8);
  with the steps put back after the find it reads 0, in both builds.
- Finding 4 (static fields are not counted into the room of an instance):
  `the_instance_room_leaves_out_the_static_fields` reads 72 bytes per instance of a class
  with one private field and three static ones; with the test of `is_static` planted out it
  reads 216. It runs in the default build only, like the other tests that read bytes.
- Finding 3 (the static level reserves its room once): a count of bytes cannot hold it, as
  the unit showed (one reservation of 24 + 3 x 48 equals 72 + 48 + 48), so
  `the_static_fields_of_a_class_reserve_their_level_once` reads where a memory budget ends
  the run: at the smallest budget with which two static initializers run, all three run, and
  one byte below exactly one. With the room of the static level planted to 0 it reads
  `(1, 2)` for `(1, 3)`, in both builds.
- Finding 7: the three doc lines that named the removed `Parser::hold` are written again.
- Finding 5 (a plant no test can observe: the environment of a field initializer is traced
  through the running frame): no change. Finding 6 (the private find charges after its
  scan; older than the unit, and the scan is bounded by a list whose bytes are charged): no
  change. Finding 8 (the test of 100 private fields at the full count): 2.250 CPU s in the
  default build at load 14.8 and 1.974 at load 8, under the bound of 2.5 s for
  `SWELL_FULL_COUNTS=1`; no change.

The coordinator's runs on `b30b601`, build node, before the merge: `swell-syntax` main 80
passed, and 80 with `SWELL_FULL_COUNTS=1`; `swell-compiler` main 17 passed; `swell` main 1710
passed, 1 skipped (default build) and 1682 passed, 1 skipped (collect build); `swell` lib 285
passed; `task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit
0; `cargo fmt --all -- --check` exit 0.

Not covered: `task test-realm-sizes` ran in the unit's own checks on `b30b601` (0 runs red)
and not in the coordinator's; the unit's part of test262 (1,507 cases, all 3,006 runs
passed) ran on `3ebbc21` and not again on `b30b601`, which changes no line outside tests
and doc comments; the test of the static level reads a boundary of the memory budget that a
change of any charge before the class moves, and it finds the boundary by halving, so it
holds no fixed number.
