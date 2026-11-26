---
format: aep.planning-md/3
id: story:buffer-growth-outside-the-charged-list-is-listed
kind: story
status: implemented
title: A growth call outside the charged list is listed with its charge or its bound
summary: A disallowed-methods list of clippy for the engine crate and an expect with a reason at each remaining growth call, so that a later builtin cannot hold a plain buffer the memory budget does not see.
relations:
- depends_on: story:copies-held-across-a-call-into-script-are-charged
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T12:36:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T13:13:52Z", actor: "human:operator", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-26T10:01:51Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}, executor: "agent:coordinator"}
---
Waits for: `story:copies-held-across-a-call-into-script-are-charged`, the only recorded edge. The
coordinator starts the unit on a base that holds that story and every unit of slice 4 merged by
then, and checks the base before the start. Consumer: the browser that embeds the engine. Its
condition for running page scripts is that no buffer of the engine stays alive without a charge
while the engine calls back into script.

The story this one waits for charges every such buffer it found, through `ChargedVec`, `Charge`
and `KeptRoom`; its test-only instrument sees only those types. This story guards the points where
a standard buffer of `crates/swell/src` reserves room. Each call of a reserving method of `Vec`,
`VecDeque` or `String` outside test code:

- carries an `#[expect(clippy::disallowed_methods, reason = "…")]` whose reason names the charge
  that covers the buffer or the limit that bounds it;
- or turns the gate's clippy step red.

The compiler checks that a reason is there, not what it says. The acceptance reads every reason
once. The list is not a proof that every buffer is charged: growth without a reservation, macros,
other collections and other crates stay outside it, as `## Not in this story` says.

## Context

Lines are of `wave/s10-1` at `76aa4fc`. The unit measures each count again on its own base.

- The gate's one clippy step is `cargo clippy --workspace --all-targets --locked -- -D warnings`
  (`Taskfile.yml:457`). It builds `swell` without features.
- No workflow names a clippy command: `grep -rn clippy .github/workflows` finds only `components:
  clippy` (`check.yml:56`, `slow.yml:79`).
- The features of `swell` are `collect-every-safe-point` and `fault-injection`
  (`crates/swell/Cargo.toml`, `[features]`).
- The root `clippy.toml` holds four keys, each `true`: `allow-unwrap-in-tests`,
  `allow-expect-in-tests`, `allow-panic-in-tests` and `allow-indexing-slicing-in-tests`.
- `CLIPPY_CONF_DIR` is set nowhere in the repository.
- Clippy reads one `clippy.toml`: the nearest one upward from the package's manifest directory,
  not merged with a file further up. So a file in `crates/swell` is the one read for `swell`, and
  it repeats the four keys. Control (b) of the acceptance shows that the file is read.
- `crates/swell/Cargo.toml:23-27` denies `unwrap_used`, `expect_used`, `panic` and
  `indexing_slicing` under `[lints.clippy]`.
- `crates/swell/src/lib.rs:6-8` says: "a type becomes public by being `pub` in `value.rs` or in a
  file of `host/`, so no later story edits this file."
- The rustfmt form of a long `expect` puts its arguments on the lines after `#[expect(`, as at
  `realm/intrinsics.rs:117` and `ops/conversion.rs:81`.
- Growth forms in `crates/swell/src`, test modules included (`git grep -nE '<form>' 76aa4fc --
  crates/swell/src | wc -l`):

  | form | lines |
  |---|---|
  | `.reserve(` | 6 |
  | `.reserve_exact(` | 0 |
  | `.resize(` | 11 |
  | `.extend_from_slice(` | 11 |
  | `.extend(` | 34 |
  | `.repeat(` | 15 |
  | `vec![` | 33 |
  | `format!` | 54 |
  | `String::with_capacity` | 0 |

  The reserving lines outside test modules, read by a scan that skips everything from
  `#[cfg(test)] mod` on: `.try_reserve(` 18, `.try_reserve_exact(` 43, `with_capacity(` 0. These
  are lines, not receivers. `heap/map.rs:50` and `:82` call `try_reserve` on a `HashMap`, which
  is not on the list. Clippy's output, not a grep, gives the sites.
- `story:math-constants-and-exact-functions` adds `coerce_all_charged`, with a
  `try_reserve_exact(args.len())` on a list that `coerce_all` charges.
- A site with one call in its statement: `Agent::reserve_descriptors`
  (`builtins/object/integrity.rs:283-291`). Its one statement `if
  descriptors.try_reserve(count).is_err() { … }` holds one listed call.

## Change

1. **`crates/swell/clippy.toml`** (new), with no comment:
   - the four keys of the root file, with the same values;
   - `disallowed-methods`, each entry `{ path = "…", reason = "reserves room outside ChargedVec:
     name its charge or bound" }`, for these methods:
     - `std::vec::Vec`: `with_capacity`, `reserve`, `reserve_exact`, `try_reserve`,
       `try_reserve_exact`;
     - `std::collections::VecDeque`: `with_capacity`, `reserve`, `reserve_exact`, `try_reserve`,
       `try_reserve_exact`;
     - `std::string::String`: `with_capacity`, `reserve`, `reserve_exact`, `try_reserve`,
       `try_reserve_exact`;
     - `<[T]>::to_vec`: the unit tries the path form once (`alloc::slice::<impl [T]>::to_vec`).
       If clippy flags a `.to_vec()` call with it, it stays on the list. If not, it leaves the
       list, the `AGENTS.md` sentence does not name it, and `## Not in this story` keeps it.
   - Each listed path is shown to resolve by one flagged site. For a path with no site in the
     crate, the unit adds one temporary call in a statement, sees it flagged by the count command
     of the acceptance, and removes it. The report lists those paths.
2. **One `expect` per site.** A site is one place clippy reports. Its `#[expect(clippy::
   disallowed_methods, reason = "…")]` sits on the statement that holds the call: a `let`, an
   expression statement, or an `if`.
   - Where the language takes no attribute at that place, the `expect` goes on the smallest item
     that holds the site. Its reason then names every site it covers, by what each reserves.
   - A reason names one of two things:
     - the charge that covers the buffer, for example "charged by `Charge::grow` before it
       grows", or "charged by `coerce_all`, 8 bytes per argument";
     - or the limit that bounds it, for example "at most `MOST_FLAGS` units", or "dropped before
       any call into script, bounded by the charged text".
3. **Test code.** `crates/swell/src/lib.rs` gets `#![cfg_attr(test,
   allow(clippy::disallowed_methods, reason = "test code is not held to the charged list"))]`.
   - An `expect` inside engine code still fires in a test build, because an attribute on an inner
     item sets the level for that item.
   - Sites in `crates/swell/tests` that clippy flags get an `expect` each. That crate is a
     separate target that the `allow` does not reach, and its `tests/main.rs` is generated.
   - The header sentence of `lib.rs` (`:6-8`) becomes: "a type becomes public by being `pub` in
     `value.rs` or in a file of `host/`, so no story edits this file for the interface; its one
     crate attribute besides `forbid(unsafe_code)` lets test code call the methods
     `crates/swell/clippy.toml` disallows."
4. **What turns the gate red.** The gate's clippy step runs with `-D warnings`, so:
   - A listed call with no `expect` is an error of `clippy::disallowed_methods`.
   - An `expect` whose statement holds no listed call is an error of
     `unfulfilled_lint_expectations`, which warns by default. This is the direction that turns a
     stale `expect` red when its call goes away.
   - A missing or misnamed `crates/swell/clippy.toml` leaves every listed method allowed, so every
     `expect` is unfulfilled and red.
5. **Joins after this story.**
   - When a later unit adds a listed call, the gate is red until that call has its `expect`.
   - The coordinator adds the `expect` at the join of each unit planned before this story. A
     unit planned after it adds its own.
6. **`AGENTS.md`, "Rules"** (`:101`). The coordinator writes it on the merge; the text:

   > A call in `crates/swell/src` outside test code that reserves room in a `Vec`, `VecDeque` or
   > `String` (`with_capacity`, `reserve`, `reserve_exact`, `try_reserve`, `try_reserve_exact`)
   > outside `ChargedVec` needs `#[expect(clippy::disallowed_methods, reason = "…")]` on its
   > statement, whose reason names the charge that covers the buffer or the limit that bounds it;
   > `crates/swell/clippy.toml` holds the list, and a change of one of its four `allow-*-in-tests`
   > keys is made in the root `clippy.toml` too. The list guards the points of reservation only,
   > not every growth (`docs/design/engine.md`, "Memory budget").

   If `<[T]>::to_vec` stays on the list (item 1), the methods in brackets add `to_vec`.

Writes:

- `crates/swell/clippy.toml` (new);
- `crates/swell/src/lib.rs`: the attribute and the header sentence;
- the sites clippy flags in `crates/swell/src` and `crates/swell/tests`, by the `expect` only. As a
  guide, `grep -rlE '\.try_reserve(_exact)?\(|\.reserve(_exact)?\(|with_capacity\('
  crates/swell/src crates/swell/tests` names the files. It lists too many: it matches `HashMap`
  receivers and test modules.

Not written by the unit: `AGENTS.md`, which the coordinator writes with item 6's text, and the
root `clippy.toml`.

## Tests

No new test function and no new gate step. The checks are clippy's, and the acceptance proves
both of their directions with two controls. The edits are attributes only, and the acceptance
shows that no test count moves.

## Acceptance

Each command is one job of the build pool, at most 10 minutes unless it names its time.

The count command: `slot cargo clippy -p swell --all-targets --locked --message-format=short
2>&1 | grep 'use of a disallowed method' | cut -d: -f1-3 | sort -u | wc -l`.

- It prints the number of distinct `file:line:column` places. Clippy prints a site once per
  target that compiles it; `sort -u` keeps one.
- Without `| wc -l`, the same pipe lists the places.
- With `-- -D warnings` added after `--locked`, the lines read `error:` in place of `warning:`
  and the grep still matches them.

1. **Before.** `crates/swell/clippy.toml` and the `cfg_attr` of `lib.rs` are in place, and no
   `expect` yet. The count command prints a number N. The report lists each place, with the
   method and the receiver type, and gives N.
2. **After.**
   - `slot cargo clippy --workspace --all-targets --locked -- -D warnings` exits 0 and prints no
     `warning:` and no `error:` line.
   - The count command prints 0.
3. **Each feature.**
   - `slot cargo clippy -p swell --all-targets --locked --features collect-every-safe-point --
     -D warnings` and the same with `--features fault-injection` each exit 0.
   - The count command with each feature prints 0.
   - Sites that only that code compiles get their `expect` too.
   - This is a statement of the acceptance, not a gate step.
4. **Control (a): a missing `expect`.**
   - The `expect` on the one statement of `Agent::reserve_descriptors`
     (`builtins/object/integrity.rs`) is removed.
   - `slot cargo clippy -p swell --all-targets --locked -- -D warnings` exits non-zero.
   - The count command prints 1, and the place it lists is that statement's line in
     `crates/swell/src/builtins/object/integrity.rs`. The error line reads `error: use of a
     disallowed method` with the `try_reserve` path.
   - The `expect` is put back, and item 2 holds again.
5. **Control (b): an `expect` with nothing to expect.**
   - A statement `let _control = 0;` is added at the start of `reserve_descriptors`, with
     `#[expect(clippy::disallowed_methods, reason = "control")]` on it. It holds no listed call.
   - The same clippy command exits non-zero and prints `unfulfilled_lint_expectations` once.
   - It is removed, and item 2 holds again.
6. **The two `clippy.toml` files.** `diff <(grep '^allow-' clippy.toml | sort) <(grep '^allow-'
   crates/swell/clippy.toml | sort)` prints nothing and exits 0, and `grep -c '^allow-'` reads 4
   for each file.
7. **No behaviour moves.**
   - Run against the unit's base B: `git diff -U0 B -- crates/swell ':(exclude)crates/swell/clippy.toml'
     | grep -E '^[-+][^-+]' | grep -vE '^\+\s*(#\[expect\(.*|clippy::disallowed_methods,|reason =
     ".*",?|\)\]|#\[expect\(clippy::disallowed_methods, reason = ".*"\)\])$'`. It prints only
     the lines of `lib.rs`: the `cfg_attr` and the header sentence, removed and added.
   - `slot --timeout 30m cargo nextest run --locked -p swell --profile default` on the base and
     after reads the same `N tests run: N passed` in its `Summary` line.
   - So does `slot --timeout 30m cargo nextest run --locked -p swell --features
     collect-every-safe-point --config .cargo/collect-profile.toml --cargo-profile collect
     --profile default`.
   - The report gives the four `Summary` lines.
8. **Every reason, read once.**
   - `grep -rn -A4 'clippy::disallowed_methods' crates/swell/src crates/swell/tests` prints each
     attribute with the lines after it, in both the one-line and the wrapped form.
   - The unit reads each reason against item 2: a charge, or a limit.
   - The report lists every reason that names neither. The list must be empty before the
     merge.

## Not in this story

Where the list does not reach, each with its reason:

- Growth without a reservation: `push`, `extend`, `extend_from_slice`, `resize`, `repeat`,
  `clone`, `to_owned`, `collect`. These are the common way to fill a buffer, and a list that
  held them would flag most of the crate's code. The instrument of
  `story:copies-held-across-a-call-into-script-are-charged` covers the buffers that story
  changed.
- Macros: `vec![x; n]` and `format!`. Their expansion calls the allocator inside the standard
  library, not a listed method at the call site.
- `<[T]>::to_vec`, if clippy takes no path form for it (Change, item 1). Then nothing holds a
  later `.to_vec()`.
- `HashMap`, `BTreeMap` and `BTreeSet`: not on the list. The engine's maps grow through
  `KeyedMap` (`heap/map.rs`) under their own charges, and `on_stack` is named in the design text
  of the first story.
- Code of another crate (`syntax`, `compiler`, `regexp`, `num`). None calls into script, so no
  buffer of theirs is alive across such a call.
- `swell` code behind `collect-every-safe-point` or `fault-injection`. The gate's clippy step
  builds no feature, so the gate does not hold it after this story; only acceptance item 3
  reads it once.
- Whether a reason is true. Item 8 reads them once; no check holds that later.

## As delivered

The unit of wave big-1 delivered the story in three commits (`3a16be13`, `1d48bbcd`,
`058eba11`) after two passes of the adversary. Where the delivered state differs from the
text above, this section holds and the text above is the story as it was approved.

- **Acceptance, item 7, first bullet.** The filter no longer prints only the lines of
  `lib.rs`. Read with the story's own filter over the unit's commits, it leaves lines in
  these files, each a change a correction ordered:
  - `crates/swell/Cargo.toml`: 2 lines, `allow_attributes = "deny"` and
    `allow_attributes_without_reason = "deny"` under `[lints.clippy]` (correction 1: an
    `allow` of the listed lint would otherwise pass in silence).
  - `crates/swell/src/object/async_generator.rs` (2 lines), `crates/swell/src/script.rs`
    (4 lines) and `crates/swell/tests/support/mod.rs` (3 lines): four `allow` attributes of
    other lints became `expect` with a reason, which the two denied lints require.
  - `crates/swell/src/lib.rs`: 10 lines, the `cfg_attr` and the header sentence, as the
    story names.
  - `crates/swell/src/builtins/json/parse.rs`: 4 lines in correction 1 and 1 in
    correction 2. This is a change of behaviour: the stack of open values of `JSON.parse` is
    freed when the parse has ended, before the reviver is called, and the reason at its
    `try_reserve` states the bound the code holds. The changelog names it under "Fixed".
  - `crates/swell/tests/builtins_json_parse.rs` (79 lines) and
    `crates/swell/tests/source_scan.rs` (532 lines): the tests of the two corrections.
  The other three bullets of item 7 (the `Summary` lines of the package on the base and
  after, in both modes) hold with the count of tests raised by the tests named here.
- **"Not in this story", the bullet on features.** It says the gate does not hold code behind
  `collect-every-safe-point` or `fault-injection`. That is still so for clippy. The source
  scan reads every `.rs` file of `crates/swell/src` and `crates/swell/tests` whatever the
  feature, so an `allow` of the listed lint behind a feature is refused by the gate's test
  step.
- **"Not in this story", the bullet on other crates.** The scan
  `source_scan::no_other_crate_names_the_listed_lint` refuses an attribute that names the
  lint in a crate that has no list, where it would hold nothing.
- **What holds the rule after the story**, beyond item 2: the tests
  `source_scan::growth_attributes_are_expects_with_a_reason`,
  `source_scan::no_other_crate_names_the_listed_lint` and
  `source_scan::the_growth_scan_finds_planted_attributes`, and the rule in `AGENTS.md`
  ("Rules", the bullet after "A buffer whose size a script controls"). Two limits of a scan
  of text are stated in `review-result:growth-correction-2-coordinator-read`.
- **Left to the backlog:** three sites reserve before they charge
  (`story:charge-comes-before-the-reservation`).
