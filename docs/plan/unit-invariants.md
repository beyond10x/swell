# Repository invariants for a unit of a wave

Copied unchanged into every unit brief. A unit that finds one of these wrong says so in its
report and does not work around it.

- Work only in the worktree the brief names. Build output goes to `target/` inside that
  worktree: never set `CARGO_TARGET_DIR`, never build in another tree.
- `cargo` in the shell is a shim that queues in the shared build pool. Prefix every other
  command that compiles or tests with `slot` (`slot task check`). `slot: job N queued` means
  waiting. Never pass `-j`, never set `CARGO_BUILD_JOBS`, never bypass the pool.
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
- No Python anywhere: not in the repository, not in a script, a probe or a manual check.
  Shell, `awk`, `jq` and Rust are fine.
- No network, unless the brief names the address (the test262 fetch, the Unicode data). No
  `gh`, no push, no remote.
- Nothing personal in any file: no person's name (write "the operator"), no home directory
  path (write `$HOME`), no machine name.
- Anything that runs longer than about 30 seconds runs in the background. Every process a unit
  starts is stopped before it reports, checked with `kill -0`.
- The gate for a unit is package-scoped (`cargo test -p <package>`, or `slot task <name>` where
  the brief says so). The whole gate, `slot task check`, runs once on the wave branch.
- A report quotes each suite's own summary line and starts with these six lines:

  ```
  unit: <story id>
  result: green | red | partial
  gate: <the command> -> exit <status>
  summary: <the suite's own summary line>
  files: <count> written: <paths>
  open: <findings, or "none">
  ```
