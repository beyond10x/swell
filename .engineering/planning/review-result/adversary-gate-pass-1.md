---
format: aep.planning-md/3
id: review-result:adversary-gate-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit gate of wave big-1: 6 failing cases, 10 findings, 9 introduced'
relations:
- reviews: story:core-builds-without-std
revision: 1
---
Adversary pass 1 on unit `gate` of wave big-1 (`story:core-builds-without-std`, step 4), the tree `impl/nostd-gate` at `c6a7f86c`, 2026-11-26.
Verdict needs-change: 6 failing cases, 10 findings (9 introduced, 1 pre-existing; 2 of them not reachable while the crates forbid unsafe code). The report as the adversary returned it, with the paths of the workstation replaced:

unit: gate (story core-builds-without-std, step 4), worktree swell-impl-nostd-gate at c6a7f86c plus my uncommitted cases
verdict: NEEDS-CHANGE
cases: executed 9→15, red 6
origin: introduced 9 / pre-existing 1 / undecided 0
wrote-outside-worktree: 6 paths
needs-coordinator: is the story's last acceptance line (an example host built without std, with a fixed block as its allocator) given to another unit? Nothing in this unit delivers it.

**1. Diff stat** (only test files changed)
```
 crates/swell/tests/source_scan.rs | 43 +++++++++++++++++++++++++++++++++++++++
?? tests/gate/nostd-symbols-refuses-a-cxx-library-symbol.sh
?? tests/gate/nostd-symbols-refuses-a-weak-undefined-sqrt.sh
```

**2. Cases added.** All six are red now. Each was run alone first:
- `source_scan::adversary_a_raw_byte_string_hides_no_std_path_after_it` (`source_scan.rs:706`): `left: []  right: ["line 3: std"]`
- `source_scan::adversary_no_std_inside_a_block_comment_declares_nothing` (`:712`): `assertion failed: !declares_no_std("/*\n#![no_std]\n*/\n#![forbid(unsafe_code)]\n")`
- `source_scan::adversary_a_test_item_with_a_comma_in_its_generics_is_test_code` (`:724`): `left: ["line 2: std::num::ParseIntError"]  right: []`
- `source_scan::adversary_the_emulated_host_filter_sees_a_grouped_std_import` (`:737`): `the filter names nothing in "use std::{env, time::SystemTime};..."` This test applies a copy of the emulated host's filter to planted text.
- `tests/gate/nostd-symbols-refuses-a-weak-undefined-sqrt.sh`: `llvm-nm: w sqrt`, then `nostd-symbols: 1 library, 1 object(s), 8 undefined symbol(s): ... none from a C library or the system`, then `FAIL ... accepted`, exit 1.
- `tests/gate/nostd-symbols-refuses-a-cxx-library-symbol.sh`: `llvm-nm: U _ZNSt9exceptionD2Ev`, then `... 8 Rust's own ...; none from a C library`, then `FAIL ... accepted`, exit 1.

The first versions of both scripts kept `#![forbid(unsafe_code)]`. The build refused them: "usage of `core::arch::global_asm`" and "usage of an `unsafe extern` block". So I took that attribute out of both planted crates. That is why findings 6 and 7 are INFEASIBLE.

**3. Suite run** (after the cases existed)
- `cargo nextest run -p swell --test main -E 'test(/^source_scan::|^emulated_host::/)'`: 10 run, 6 passed, 4 failed (my four), exit 100. The same filter with `and not test(/adversary_/)`: 6 run, 6 passed, exit 0.
- `slot task nostd-build`: exit 0. `slot task nostd-symbols`: exit 0 (8932 undefined symbols, none refused).
- The unit's three scripts print `ok`. My two exit 1, as shown in part 2.

**4. Findings**
1. **Story acceptance not met** (`crates/swell/tests/emulated_host.rs:10`). The story asks for "an example host built without std … with a fixed block as its allocator". `emulated_host.rs` is `#![cfg(test)]` inside the std test binary and uses the system allocator. Both design files present it as the emulated adapter. Reached by: the story's acceptance list. Origin: introduced.
2. **A raw byte string hides a std path** (`source_scan.rs:141`). `code_of` treats `br"`/`br#"` (and `cr`) as an ordinary string, because the `b` before `r` counts as an identifier character. After `br#"x"y"#`, the code that follows is blanked. A `#[cfg(not(target_os = "none"))] extern crate std;` there is missed by the scan, and the bare build cannot see it either. Reached by: no engine source has such a literal today. Origin: pre-existing. `git show b0828801` has the same branch, so the old `std::fs` scan has the same blind spot.
3. **`declares_no_std` accepts `#![no_std]` inside a block comment** (`source_scan.rs:376`). Reached by: in practice nothing, because `nostd-build` fails such a crate. Origin: introduced.
4. **`without_test_items` stops at a `,` inside `<…>`** (`source_scan.rs:332`). A `#[cfg(test)] fn f() -> Result<u8, std::…>` is then refused as library code, a false refusal. Reached by: a future test helper; nothing today. Origin: introduced.
5. **`the_module_names_no_system_api` only searches for substrings** (`emulated_host.rs:124`). It looks for `"std::time"` and the like, so `use std::{env, time::SystemTime}` passes. `ambient_authority` and `std_group_names` in the same crate would catch it. Also, the clock test adds nothing beyond `date_clock.rs` (HOST-125), and no test checks the fixed seeds. Reached by: a future edit of the emulated host. Origin: introduced.
6. **Weak undefined symbols are dropped** (`taskfiles/nostd.yml:151`). The awk keeps only lines with type `U`; `w`/`v` lines are neither counted nor refused. Reached by: only `global_asm!` or `#[linkage]`, which HOST-1 refuses, so INFEASIBLE. Origin: introduced.
7. **`^_ZN` and `^_R` are prefixes, not mangled-name checks** (`taskfiles/nostd.yml:146`). C++ library names such as `_ZNSt9exceptionD2Ev` pass. The docs say "a mangled Rust name", and `host-interface.md:101` says "refuses any function of a C library". Reached by: only `unsafe extern`, which HOST-1 refuses, so INFEASIBLE. Origin: introduced.
8. **The list of six packages is typed into the task** (`taskfiles/nostd.yml:30`). Nothing compares it with `engine_crates()` or the workspace. If a crate is dropped from the list, or a seventh engine crate is added, it is still built as a dependency of `swell`, but its library's symbols are never read. Reached by: the next engine crate. Origin: introduced.
9. **The fmod script writes into the gate's own output** (`tests/gate/nostd-symbols-refuses-an-undefined-fmod.sh:53`). It runs `nostd-symbols NOSTD_LIBS=…` without `NOSTD_OUT`, so it overwrites the gate's `.out/nostd/nm.txt` and `nm.err`. I saw them holding `libclean.rlib` lines after a gate run. The script's header says it works only under `.out/gate-tests/<name>/`. Reached by: every `gate-selftest` run. Origin: introduced.
10. **`forge-check` would not catch the known workflow gap** (`taskfiles/forge.yml:60`). It does not compare the workflow's `components`/`targets` with `rust-toolchain.toml`. Reached by: the workflow gap you already know about. Origin: introduced.

Answers to your forge questions:
- `slow.yml` runs `task slow`, which has no nostd step. `release-build.yml` runs no `task` and already passes `targets: matrix.target`. Neither has the same gap. Both will read the new `targets` and `components` lines of `rust-toolchain.toml` when cargo runs. I did not check whether rustup then installs them on those runners (an extra download, not a failure).
- `forge-check` would not refuse the corrected workflow. `components: clippy, llvm-tools` and `targets: x86_64-unknown-none` pass, and so would `rustup target add` or `rustup component add`.

**5. Attacked and not broken**
- Empty, missing or stale library list; llvm-nm missing or failing; empty nm output: each fails. Cargo reports fresh artifacts, and `libs.txt` is removed first.
- No package is built for both the host and the bare target, so `tail -n 1` in the jq step reads the bare library.
- A version suffix (`sqrt@GLIBC…`), `__adddf3_evil`, `__errno_location`, `__stack_chk_fail`, `__cxa_*`, `__tls_get_addr`, `__libc_*`, `malloc`, `write`, `strlen`, `pthread_*`: all refused, read from the code (the patterns are anchored). Not run.
- `extern crate std as x;`, `::std`, `r#std`, std inside a macro body, `cfg(any(test, …))` and `cfg(not(test))` items, a `tests` module without the attribute, and `cfg_attr(…, no_std)`: all refused (read from the code, not run). The unit's planted asserts check the line and the message, and include passing texts, so a scan that refuses everything would fail.
- Engine crates have no build script, no target-specific dependency and no std feature. Std paths behind the `fault-injection` and `collect-every-safe-point` features are caught by the scan. Those two features are not built for the bare target, and the release profile was not checked.
- Timing: `nostd-build` and `nostd-symbols` run before the clock. All three new scripts run `cargo generate-lockfile` and a cold `cargo build` inside the timed `gate-selftest` step. Six scripts at the base already do the same.
- Documents: nothing found beyond findings 5 and 7. The CHANGELOG entry holds.

**6. Paths written outside the worktree**
- `<scratch>/wave/gate/adversary-1/` with `cases-scan.log`, `case-weak.log`, `case-cxx.log`, `suite-after.log`, `suite-before.log`, `suite-gate.log`

Inside the worktree, under the ignored `.out/`: `.out/gate-tests/nostd-symbols-refuses-a-{weak-undefined-sqrt,cxx-library-symbol}/`. My runs of the unit's scripts and tasks also rewrote `.out/nostd/` and `.out/gate-tests/nostd-*`.

**7.**
```findings
- file: crates/swell/tests/emulated_host.rs
  line: 10
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The story's example host built without std with a fixed-block allocator does not exist; the emulated host is a cfg(test) module of the std test binary on the system allocator.
- file: crates/swell/tests/source_scan.rs
  line: 141
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: code_of reads br"..."/br#"..."# as an ordinary string, so the code after such a literal is blanked and a hosted-only extern crate std there escapes both the HOST-2 scan and the bare build.
- file: crates/swell/tests/source_scan.rs
  line: 376
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: declares_no_std reads the raw source and accepts a #![no_std] line inside a block comment.
- file: crates/swell/tests/source_scan.rs
  line: 332
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: without_test_items ends a cfg(test) item at a comma inside angle-bracket generics, so a std path in that test function's signature is refused as library code.
- file: crates/swell/tests/emulated_host.rs
  line: 124
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the_module_names_no_system_api matches literal "std::time"-style substrings and passes a grouped import such as use std::{env, time::SystemTime}, which the crate's own ambient_authority scanner would catch.
- file: taskfiles/nostd.yml
  line: 151
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: nostd-symbols keeps only type-U lines and silently passes a weak undefined sqrt, reachable only through global_asm which forbid(unsafe_code) refuses.
- file: taskfiles/nostd.yml
  line: 146
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: The ^_ZN and ^_R prefixes pass C++ library names such as _ZNSt9exceptionD2Ev although the summary and host-interface.md promise only mangled Rust names; reachable only through unsafe extern, which HOST-1 refuses.
- file: taskfiles/nostd.yml
  line: 30
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The six packages are a hard-coded default that nothing compares with engine_crates() or the workspace, so a dropped or added engine crate is built as a dependency but its symbols are never read.
- file: tests/gate/nostd-symbols-refuses-an-undefined-fmod.sh
  line: 53
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The script runs nostd-symbols without NOSTD_OUT and overwrites the gate's .out/nostd/nm.txt, contrary to its header saying it works only under .out/gate-tests/<name>/.
- file: taskfiles/forge.yml
  line: 60
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: forge-check does not compare a workflow's components and targets with rust-toolchain.toml, so the missing llvm-tools and x86_64-unknown-none on the forge are invisible to the local gate.
```
