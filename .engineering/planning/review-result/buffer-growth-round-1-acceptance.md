---
format: aep.planning-md/3
id: review-result:buffer-growth-round-1-acceptance
kind: review-result
status: active
title: Buffer growth outside the charged list, round 1, the acceptance critic
relations:
- reviews: story:buffer-growth-outside-the-charged-list-is-listed
revision: 1
---
needs-revision

- **Before count (draft line 110).** `grep -c 'disallowed_methods'` does not count sites. Clippy's headline reads "use of a disallowed method `…`" and names the lint only in the single `= note: #[warn(clippy::disallowed_methods)] on by default` per target. Expect a count near the number of targets, not N. The same grep decides the negative control ("exactly one warning", line 117). Change both to count `use of a disallowed method` headlines, or read `--message-format=json` and count `code.code == "clippy::disallowed_methods"`. Name the form and say how duplicates across targets are removed. I did not run it, so this follows from how clippy words the lint.
- **After count (line 115).** `grep -rn 'expect(clippy::disallowed_methods'` misses multi-line attributes. rustfmt breaks `#[expect(` and puts the arguments on following lines, so the long reasons here will be split. The repo already has this shape at `realm/intrinsics.rs:117` (`Intrinsic` enum variant) and `ops/conversion.rs:81` (the `impl` of `Agent<H>`). Use a count that works on both shapes, for example `grep -rc 'clippy::disallowed_methods' crates/swell` summed, or `grep -Pzo`.
- **After count, second defect (line 115).** The count equals the site count only if each site has its own `expect`. Item 2 puts the `expect` on "the smallest item that holds it", so one `expect` can cover two sites. Say "one `expect` per site, none covering two", or compare against the number of expects instead of the report's site count.
- **Negative control (lines 117-119).** It does not name the site, the command or the output. State the site (`reserve_vec_within` in `heap/accounting.rs`, around line 429, is the first candidate), the command (with `-D warnings`), the expected line (`error: use of a disallowed method \`std::vec::Vec::try_reserve_exact\``, with file:line) and the clean run after it is put back. It also tests only one of the two `unfulfilled_lint_expectations` directions. Add the second: an `expect` placed on a site with no flagged call turns the gate red. That is the draft's own "joins" claim (item 4) and the check that a retired site stops compiling.
- **"No behaviour moves" (lines 102-104, 122).** The draft asserts it and names no command, and the one nextest run is `--lib` only. Name a command that shows the claim. For instance, `git diff -U0` outside the `lib.rs` attribute and `clippy.toml` has only added `#[expect(clippy::disallowed_methods` lines or their continuation lines. Run the swell integration tests (`slot cargo nextest run --locked -p swell`) and compare the summary with the base. Say which counts are compared: tests run and passed, and the step and byte counts the tests print, if any.
- **Order of the "Before" run (line 110).** It says "no `expect` yet" but does not say whether the `lib.rs` `cfg_attr(test, allow)` exists at that point. Without it, the test modules inflate N, and the draft then reports them as "not flagged, because a test module" (line 113). State the order.
- **Fallback for `<[T]>::to_vec` (lines 60-61, 120-121).** The grep count is a human check, not a gate. A sixth `.to_vec()` added later passes the gate, which breaks statement 1 for that method. Say so in "Not in this story" and in the `AGENTS.md` sentence, or name a gate step that holds the grep count.
- **Where the lint does not reach (lines 125-133).** Statement 4 is only half met. The macros (`vec![x; n]`, `format!`) and the other growth methods are missing. `Vec::reserve`/`reserve_exact`, `String::with_capacity`/`try_reserve` and `VecDeque::try_reserve`/`with_capacity` are not on the list, and `extend_from_slice`, `resize`, `clone` and `to_owned` are not named. `HashMap::try_reserve` at `heap/map.rs:50`/`:82` is named only as a skipped receiver. The `AGENTS.md` sentence (lines 83-87) says "a new call that grows a standard buffer" and lists five methods. Add one clause saying the list is not a proof, with the gaps. The opening claim "or fails the gate" (line 10) overstates the same way, and `Vec::reserve`, which aborts on out-of-memory, is the largest hole.
- **"A reason the compiler checks" (line 11).** The compiler checks that a reason exists, not that it names a charge or a limit. The acceptance has no statement for that half of item 1. Add a command that lists every reason (`-A4` on the attribute), or a review step with its own pass/fail.

Out of my lane, for the design critic: the `lib.rs` header comment says "no later story edits this file", and the draft now edits it. Whether `clippy.toml` repeats exactly the four root keys is the design critic's to confirm. I read the root `clippy.toml` and it has the same four keys.

What I read: the brief, the draft (133 lines), `Taskfile.yml` (clippy task), the root `clippy.toml`, `crates/swell/Cargo.toml` and `lib.rs`, `reserve_vec_within` in `heap/accounting.rs`, `tests/source_scan.rs` (it does not scan attributes, so the added `expect`s do not move a test), and the existing multi-line `#[expect(` sites. I did not read the first story's file.

What I could not establish: the exact clippy output text, because I did not run it. The `to_vec` path form was not checked against rustc 1.99.1's clippy.

```findings
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 110
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the Before and negative-control count `grep -c disallowed_methods` matches the lint name only in one note per target, not once per flagged site, so it does not read N or "exactly one"; count the "use of a disallowed method" headlines or the JSON code instead'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 115
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the After grep for `expect(clippy::disallowed_methods` misses the multi-line form rustfmt produces (as at realm/intrinsics.rs:117), and one `expect` on a smallest item can cover several sites, so the count need not equal the report; name a count that works for both shapes and say one `expect` per site'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 117
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the negative control names no site, no command with -D warnings and no expected output line, and does not test the reverse direction (an expect on a site with no flagged call turns the gate red)'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 122
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the claim that no test, step or byte count moves is asserted, with only a --lib nextest run as evidence; name a diff-shape command and a before/after integration-test summary comparison'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 110
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the Before run does not say whether the lib.rs cfg_attr(test, allow) is already in place, so N may include test-module sites'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 120
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'if to_vec has no clippy path, its sites are held by a grep that no gate runs, so a later to_vec passes unflagged; say so in Not-in-this-story and the AGENTS.md sentence or name a gate step'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 125
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the story does not say where the lint stops (macros such as vec![x; n], Vec::reserve, String and VecDeque growth, extend_from_slice, resize, clone), and the AGENTS.md sentence reads as complete; add that it is not a proof'
- file: .engineering/planning/story/buffer-growth-outside-the-charged-list-is-listed.md
  line: 11
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'the compiler checks only that an expect has a reason, not that it names a charge or a limit, and no acceptance statement covers that half; add a command that lists every reason for review'
```
