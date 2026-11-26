---
format: aep.planning-md/3
id: review-result:adversary-gate-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit gate (core-builds-without-std, step 4): needs change, 7 findings'
relations:
- reviews: story:core-builds-without-std
revision: 1
---
Adversary pass 2 of 2 on the unit gate, branch impl/nostd-gate at 4503978a, wave big-1.
Verdict: needs change. 7 findings (1 blocker on the acceptance line, 3 warnings with red cases, 3 notes); 5 cases added, all red.

unit: gate (story core-builds-without-std, step 4), worktree swell-impl-nostd-gate at 4503978a plus my uncommitted cases
verdict: NEEDS-CHANGE
cases: executed 11→15, red 4 (nextest filter), plus 1 gate script red
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: yes. Pass-1 finding 1 is still open. The story's acceptance line (an example host built without std, with a fixed block as its allocator) cannot be met without `unsafe`, so only you can drop it from the story or give it to another unit.

**Pass 1, finding by finding**
1. **Not closed.** The story still asks for the host at `core-builds-without-std.md:97`. The correction only changed the documents (`engine.md:284`, `host-interface.md:98`: "No host of the repository is built without `std`").
2. Closed: `source_scan.rs:139` calls `raw_prefix(&chars, i)`, and `a_raw_byte_string_hides_no_std_path_after_it` passes.
3. Closed: `source_scan.rs:395` reads `code_of(root).lines()`.
4. Closed: `source_scan.rs:337` (`',' if depth == 0 && angles > 0 => {}`). That line opens new finding 2.
5. Closed: `emulated_host.rs:111-115` (`system_api` uses `ambient_authority` and `std_paths`), and the seed test is at `:137`.
6. Closed: `nostd.yml:201` (`$2 !~ /^[Uwv]$/`), with the weak-sqrt script.
7. Closed: `nostd.yml:182` requires `length(last) == 17` and `E` at the end. A C++ function cannot pass, because its parameter types follow the `E`.
8. Closed: `nostd.yml:39` (`default "swell"`) and `:62-65` take every rlib under `/<target>/`. My run listed 6 libraries.
9. Closed: the fmod script passes `NOSTD_OUT` on lines 50, 58 and 64.
10. Closed for one-line arrays: `forge.yml:451-484`, and `check.yml` now installs `llvm-tools` and `x86_64-unknown-none`. Arrays in other forms open new finding 4.

**1. Diff stat.** Only test files changed:
```
 crates/swell/tests/emulated_host.rs |  9 ++++++
 crates/swell/tests/source_scan.rs   | 60 +++++++++++++++++++++++++++++++++++++
?? tests/gate/adversary-toolchain-check-reads-a-components-array-over-several-lines.sh
```

**2. Cases added.** All five are red now. Each was run alone before the suite, and each test takes 0.03 s or less.
- `source_scan::adversary_a_shift_in_a_test_variant_hides_the_next_variant` (`:774`): `left: []  right: ["line 4: std::string::String"]`
- `source_scan::adversary_a_comparison_in_a_test_match_arm_hides_the_next_arm` (`:799`): `left: []  right: ["line 8: std", "line 9: std::eprintln"]`
- `source_scan::adversary_a_test_parameter_hides_the_function_body` (`:818`): `left: []  right: ["line 4: std", "line 5: std::eprintln"]`
- `emulated_host::adversary_the_filter_sees_an_output_macro_of_the_std_prelude` (`:185`): fails because `system_api` names nothing for `eprintln!(...); print!(...)`.
- `tests/gate/adversary-toolchain-check-reads-a-components-array-over-several-lines.sh` exits 1 with `FAIL toolchain-check exited 0 with rust-toolchain.toml naming llvm-tools and x86_64-unknown-none over several lines and check.yml installing neither (... installs the 0 component(s) and 0 target(s) ... names)`. It builds nothing and runs `rustc --version`; I did not time it.

**3. Suite run** (after the cases existed)
- `cargo nextest run -p swell --test main -E 'test(/^source_scan::/) | test(/^emulated_host::/)'`: 15 run, 11 passed, 4 failed (the four above), exit 100. The 11 that pass are the unit's own tests.
- `slot task nostd-build`: exit 0, `6 libraries for x86_64-unknown-none`.
- `slot task nostd-symbols`: exit 0, `6 libraries, 53 object(s), 9030 undefined symbol(s): 8890 Rust's own, 55 memory functions, 85 compiler runtime routines`.
- `slot task forge-check`: exit 0, `installs the 2 component(s) and 1 target(s)`.
- The unit's toolchain script prints `ok`; mine exits 1, as shown in part 2.

**4. Findings**
1. **Acceptance still unmet** (`emulated_host.rs:11`). The acceptance line stands and the documents now say nothing here delivers it. A fixed-block global allocator needs `unsafe impl GlobalAlloc`, which every crate forbids, so the unit cannot meet the line. Reached by: the story's acceptance list. Origin: introduced.
2. **Angle count hides library code** (`source_scan.rs:337`). A `<` or `<<` inside an expression of a test-only enum variant, match arm or struct-literal field keeps `angles` above zero. The `,` then does not end the test item, and the blanking runs to the enclosing `}` or through the next arm's block. That hides an `extern crate std` that only a feature build compiles, which the bare build never sees. Reached by: nothing today; a future edit. Origin: introduced by correction 1 (at `c6a7f86c` the `,` ended the item).
3. **A `)` at depth 0 does not end a test item** (`source_scan.rs:332`). With `#[cfg(test)]` on a function parameter, the whole function body is blanked, including any `extern crate std` and `std::` path in it. Reached by: nothing today. Origin: introduced by the unit (`0c10d2f7`).
4. **toolchain-check reads only the key's own line** (`forge.yml:440-443`). If `rust-toolchain.toml` spreads its arrays over several lines, the check reads 0 components and 0 targets and passes, whatever `check.yml` installs. Arrays with single-quoted TOML strings read the same way; I read that from the code (`"[^"]*"` only), because the hook blocked running `task` outside the pool. Reached by: any reformatting of `rust-toolchain.toml`. Origin: introduced.
5. **"Calls no system API" claims more than the check holds** (`emulated_host.rs:111`; `host-interface.md:97`; `engine.md:284`). The std prelude's `print!`, `eprintln!` and `dbg!` write to the process's streams without naming `std`, and the check passes them. Reached by: a future edit of the emulated host. Origin: introduced.
6. **The design's reason is wrong for a library** (`engine.md:284`, `host-interface.md:98`). The text says a host built without std "needs a global allocator and an entry point, both `unsafe`". A `#![no_std]` host library built by nostd-build needs neither; only a fixed-block allocator needs `unsafe`. Reached by: your decision on finding 1. Origin: introduced.
7. **Summary says the build set equals the scan set** (`nostd.yml:20`). It says the libraries are "the set the HOST-2 scan reads". `engine_crates()` also reads target-specific and optional dependencies, which the bare build does not build, so their symbols are never read. Reached by: nothing (no engine crate has such a dependency). Origin: introduced.

Covers the tree at 4503978a plus the uncommitted cases above.

**5. Attacked and not broken**
- `code_of`, read from the code and not run as separate cases: `b'"'`, `'"'`, lifetimes against char literals, `r##"…"#…"##`, nested `/* /* */ */`, doc comments, `r#type`, `b"\""` and `c"…"`. All read correctly.
- `declares_no_std`: both `cfg_attr(…, no_std)` forms are refused. `#! [no_std]` is also refused, which is only a false refusal (rustfmt writes `#![no_std]`). The attribute inside a string is blanked. Inside a submodule or after an item, rustc errors or the crate is std, and nostd-build fails it.
- Angle tracking: turbofish, a closing `>>`, `->` and `=>` inside generics, and `>=` all behave correctly, apart from findings 2 and 3.
- `nostd-symbols`: `fmod`, `sqrt`, `pow`, `malloc`, `write`, `abort` and `__errno_location` are refused (anchored patterns). Empty planted output fails with "read no undefined symbol". A C name that looks like a v0 name (`_R[CMXYNIB]…C<digit>`) passes, but a crate can only leave one through `unsafe extern`. An exported `NOSTD_NM_OUTPUT` does reach the gate, and `gate-vars` names it on the gate's first line, as it does for every gate variable.
- `nostd-build`: dev-dependencies are not built, build-script and proc-macro artifacts lie outside `/<target>/`, and a crate that `swell` does not depend on is outside both sets alike.
- `toolchain-check`: commented lines, trailing comments, a component in another step, and quoted values are all read correctly. The YAML list form gives only a false refusal.
- `math_random_draws_the_same_from_the_fixed_seeds`: a constant generator would fail its `assert_ne!`.

**6. Paths written outside the worktree**
- `<scratch>/wave/gate/adversary-2/cases-alone.log`, `suite-after.log`, `nostd-build.log`, `nostd-symbols.log`, `forge-check.log`

Inside the worktree, under the ignored `.out/`: `.out/gate-tests/adversary-toolchain-check-reads-a-components-array-over-several-lines/`. My task runs also rewrote `.out/nostd/`.

**7.**
```findings
- file: crates/swell/tests/emulated_host.rs
  line: 11
  category: acceptance
  severity: blocker
  verdict: INFEASIBLE
  origin: introduced
  message: The story still asks for an example host built without std on a fixed-block allocator, which needs unsafe impl GlobalAlloc, and the correction only changed the documents.
- file: crates/swell/tests/source_scan.rs
  line: 337
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A < or << in an expression of a test-only variant, match arm or field keeps the comma from ending the test item, so the following library code, an extern crate std behind a feature included, is blanked.
- file: crates/swell/tests/source_scan.rs
  line: 332
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A closing paren at depth 0 does not end a test item, so cfg(test) on a function parameter blanks the whole function body from the std scan.
- file: taskfiles/forge.yml
  line: 440
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: toolchain-check reads components and targets only from the key's own line and double-quoted strings, so a multi-line or single-quoted array yields 0 wanted and passes a check.yml that lacks llvm-tools.
- file: crates/swell/tests/emulated_host.rs
  line: 111
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The design says the emulated host calls no system API, but its check passes the std prelude's print!, eprintln! and dbg!, which name no std path.
- file: docs/design/engine.md
  line: 284
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The reason given for no host being built without std is wrong for a library, since a no_std host library needs neither an allocator nor an entry point and only the fixed-block allocator needs unsafe.
- file: taskfiles/nostd.yml
  line: 20
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: The summary equates the bare-target build set with engine_crates(), which also holds target-specific and optional dependencies that the bare build never builds or reads.
```
