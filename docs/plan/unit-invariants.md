# Repository invariants for a unit of a wave

Copied unchanged into every unit brief. A unit that finds one of these wrong says so in its
report and does not work around it.

- Work only in the worktree the brief names. Build output goes to `target/` inside that
  worktree: never set `CARGO_TARGET_DIR`, never build in another tree.
- `cargo` in the shell is a shim that queues in the shared build pool. Prefix every other
  command that compiles or tests with `slot` (`slot task check`). `slot: job N queued` means
  waiting. Never pass `-j`, never set `CARGO_BUILD_JOBS`, never bypass the pool. A job asks for
  one slot: never pass `--tokens` or a priority to `slot`. A unit that was told to measure a
  step "alone" asked for all 17 slots on 2026-11-22; its job stood first in the queue for 13
  minutes and held 54 jobs of other sessions behind it. Nothing is measured alone on this
  machine: measure in an ordinary job and report CPU seconds and the load beside wall seconds.
- A job in the pool that runs longer than 10 minutes is killed (a rule of the organization
  since 2026-11-22; the wait in the queue does not count; the daemon ends it). A job that
  needs longer names its time, at most 30 minutes (a longer value counts as 30 minutes; a job that needs more is split): `slot --timeout 30m task test262`, or
  `SLOT_TIMEOUT=30m cargo nextest run ...` for a cargo call the shim wraps. A wrapped
  `timeout` extends nothing since slot 0.22.0: `slot timeout 1800 ...` ends at 10 minutes
  with exit 124, so where a story still writes that form, run `slot --timeout 30m ...`. A
  job ended at its limit is not started again unchanged: split it, shorten it, or name its
  time. A script that builds and tests nothing stays off the pool.
- No third-party crate, as a dependency or a development dependency. A `Cargo.toml` holds path
  dependencies on crates of this repository and nothing else. The one exception is a separate
  workspace of tooling that the brief names.
- Every crate starts with `#![forbid(unsafe_code)]` and builds with warnings denied. Engine
  crates have no `unwrap`, no `expect`, no `panic!` and no unchecked indexing outside tests.
- No ambient authority in an engine crate: no file, network, clock, randomness or environment
  access, no `RandomState`, no thread. The runner and the command-line host are tooling and
  may read files and use threads.
- Do not use `std`'s transcendental `f64` functions in an engine crate (`AGENTS.md`, "Rules").
- The failing test comes first: write the test, run it, see it fail for the reason the story
  names, then write the code. The report quotes the failing run and the passing run.
- Specification first: no host-interface behaviour that `spec/` does not state, and no language
  behaviour that ECMA-262 does not state. A gap is reported, not filled in code. A builtin's
  module and comments cite the clause by its anchor (`#sec-...`).
- The pinned ECMA-262 draft is `.cache/ecma262/<commit>/`: `ecma262.html` holds the anchors
  (`id="sec-..."`), `ecma262.txt` is the same page as text (it starts with style rules; search
  it for the clause title). The pinned test262 is `.cache/test262/<commit>/`. Read the clause
  there, not from memory. Never edit either, and never commit a file of test262.
- A number in a test, a comment or the report comes from a command's output on the pinned
  checkout; the report says which command.
- Never write under `.engineering/`. No `aep plan artifact` write verb; reads are fine.
- No `git commit`, `git add`, `git stash`, branch or worktree command. Changes are left in the
  working tree; the coordinator commits. `Cargo.lock`, the root `Taskfile.yml`, `AGENTS.md` and
  `README.md` are the coordinator's unless the brief assigns one.
- Nothing under `/tmp` or `/dev/shm`. Scratch output goes to `.out/` in the worktree or to the
  scratch directory the brief assigns.
- The shell runs as `zsh -c … < /dev/null`: no heredoc into a command that reads standard
  input. Write a script with the file-writing tool and run it by path.
- No Python in any command: not in the repository, not in a script, a probe or a manual check,
  and not to ask for its version (`python3 --version` counts; three units did it, in wave 3
  and wave 5 of slice 1 and in wave 1 of slice 2, each time inside a search for a tool). Do
  not look for it at all. The repository must be workable on a machine that has none, and a
  probe that needs it proves nothing there. Use `sh`, `awk`, `sed`, `grep`, `jq` or Rust.
- Before you write: a unit that lifts a refusal of the parser or delivers a builtin searches
  the whole tree (`grep -rn` over `crates/`, `conformance/`, `tests/`) for a test or fixture
  that relies on the absence: a source expected to be refused, a name expected to be
  undefined. In wave 1 of slice 2 all three units met one outside their `Writes:` line. If
  it is outside yours, stop and report it with the file and line before you go on; the
  coordinator extends the story.
- A new instruction does its work in a function of its own, called from the run loop
  (`crates/swell/src/interp/dispatch.rs`); it adds no local to `Agent::execute`, whose frame
  every re-entry pays. A unit does not change `STACK_BYTES_PER_REENTRY` or a stack test unless
  its brief says so: a stack test that fails is reported with the bytes per re-entry before and
  after the change.
- A wait is a wait on a file or a process (`tail --pid`, a loop with `sleep`), never a loop
  that prints: one unit wrote 64 MB into the session's tool results that way.
- No network, unless the brief names the address (the test262 fetch, the Unicode data). No
  `gh`, no push, no remote.
- Nothing personal in any file: no person's name (write "the operator"), no home directory
  path (write `$HOME`), no machine name.
- Anything that runs longer than about 30 seconds runs in the background. Every process a unit
  starts is stopped before it reports, checked with `kill -0`.
- The gate for a unit is package-scoped (`cargo nextest run -p <package>`, or `slot task <name>`
  where the brief says so). The whole gate, `slot task check`, runs once on the wave branch.
  A unit that changes what a script evaluates to (a builtin, the parser, the compiler, the
  interpreter) also runs `cargo nextest run --locked -p swell-test262`: its tests run the
  engine over a fixture tree and state counts. The twelve builtin units of slice 3 ran the
  package `swell` only; one count of that package went stale and the wave's gate found it
  (`an_unknown_rule_kind_fails_the_run`, 2026-11-22).
- A report quotes each suite's own summary line and starts with these six lines:

  ```
  unit: <story id>
  result: green | red | partial
  gate: <the command> -> exit <status>
  summary: <the suite's own summary line>
  files: <count> written: <paths>
  open: <findings, or "none">
  ```
