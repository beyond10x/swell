# swell: agent instructions

swell is an ECMAScript engine written from scratch in Rust. Its first user is surf, the browser in
the sibling repository `beyond10x/surf`; swell is implemented independently, with requirements
taken from surf's needs. The design is `docs/design/engine.md`; read it before changing anything.

Status on 2026-11-21: the gate, the pins, the counting runner, the first parts of `swell-num`
and `swell-unicode`, the parser for the ES5 grammar (`swell-syntax`), the bytecode compiler
(`swell-compiler`), and in `swell` the heap, the values, the object model, realms, the
interpreter and the builtins the test262 harness calls exist. **Slice 1 is delivered** (waves 1 to
6: `docs/plan/slice-01-wave-1.md` to `-6.md`): its 31 test262 cases pass in all 62 runs, no host
requirement is pending for it, and 14 of the 58 scenarios of the ESS model pass on the engine
(the other 44 need a later slice and say so). 70 of 164 host requirements have a test.
`Agent::evaluate_script` runs a script of the ES5 grammar under a step and a memory budget. The
gate runs test262: 3,969 of the 44,812 run cases pass. Release `0.1.0` was cut at the close of
wave 5. Next: slice 2, `language-core`, decomposed into six stories in four waves
(`docs/plan/waves.md`, "Slice 2"); its design changes are in the design already.
The foundation is delivered in three waves (`docs/plan/wave-1.md`, `wave-2.md`, `wave-3.md`): the
workspace, `task check`, test262 and the Unicode data pinned by checksum, both specifications
of the host interface held by the gate, and `swell-test262`, which counts all 53,801 cases
per directory, per slice and per ECMA-262 section and compares its 283 lines
with `conformance/test262/expected.txt`. The numbers
are under "The gate's numbers".
The operator accepted the design on 2026-11-21. Its four open questions stand at the option the
design recommends, taken by the coordinating session and reported to the operator (`Intl` out
at first, no concurrent agents at first, Unicode 17.0.0); where surf runs the engine (the
fourth) is surf's decision and is not taken yet.

## Goals

From the operator, as surf's rules carry over: first principles, specification first, minimal,
and as secure as possible. "As soon as we mark it complete, it's complete as to the specs."

- The target is the ECMA-262 draft at a pinned commit, held by test262 at a pinned commit
  (design, decision 1). Both pins move together, in one change, never by themselves.
- Completeness is measured: test262 counts per directory (cases that exist, are excluded, run and
  pass) and a gate that holds them. A section of ECMA-262 is marked complete only when every
  test262 case for it passes and its list of deliberate omissions is empty.

## Rules

- Nothing third-party in anything that ships: the Rust standard library and the operating system
  only. Build and test tooling is not covered. The root `Cargo.lock` lists no package from outside
  the repository, and the gate checks it. A third-party crate is allowed only in a separate
  workspace of tooling that nothing shipped depends on (an ESS conformance runner, a fuzzer).
- `#![forbid(unsafe_code)]` in every crate. No just-in-time compiler, no machine code
  generation, no executable memory.
- No ambient authority: no file, network, clock, randomness or environment access in the engine.
  Time, time zones, randomness, module loading and jobs come from the host interface (decision 9).
  Do not use `std::collections::hash_map::RandomState` in the engine: it reads the operating
  system's random source. Do not use `std`'s transcendental `f64` functions (`sin`, `exp`, `ln`,
  `powf` and the others `std` documents as "non-deterministic" precision): `swell-num` has its
  own (decision 6).
- Hostile input must never hang, crash or exhaust the host. Every loop the engine runs on behalf
  of a script is charged to the step budget; every allocation is charged to the memory budget;
  every recursion is bounded. Each limit ends in an outcome the host reads (`Terminated(...)`, or a
  `RangeError` where the design says so), never in a panic or an abort. A new limit gets a test
  that reaches it.
- No panics on any input: no `unwrap`, `expect` or unchecked indexing in engine crates.
- Spec first: ECMA-262 states the language and test262 holds it. The host interface is stated as
  an ESS model under `spec/` before its code (decision 10). Do not restate language semantics in
  ESS. Names follow the standard's terms; a builtin's module and comments cite its clause.
- The host interface has two specifications, and both come before its code. ESS states about
  half of it: the model (`spec/`, ess 0.96.0) validates and its 58 scenarios, 8 of them authored,
  pass on the interpreter, with 41 refusals and 51 `UNMAPPED:` markers
  (`docs/design/host-interface-model.md`). What ESS cannot state (engine values in outcomes,
  hooks that answer the engine, `call` and `construct`, host objects and roots, re-entrancy,
  the interrupt, budgets as bounds, determinism) is stated in `docs/design/host-interface.md`
  as numbered requirements: one per sentence an implementation can break, each `HOST-<n>` with
  its clause and its observable outcome. Each `UNMAPPED:` marker names the `HOST-<n>` that
  carries it. `conformance/host/requirements.txt` lists every `HOST-<n>` in one of three
  states: `test <crate>::<test name>` (held by a test), `pending slice <s>` (not implemented
  yet; slice `<s>` closes it) or `omitted <reason>` (a deliberate omission that stays). The
  gate fails on a requirement with no line, on a line for a requirement the document does not
  have, and on a requirement still pending for a slice that
  `conformance/slices-delivered.txt` names. A story that implements part of the host
  interface names the `HOST-<n>` it closes and moves their lines to `test`; every requirement
  of a slice is named by exactly one of its stories; a slice is complete only with no
  requirement pending for it, and the coordinator adds its number to the delivered file in
  the change that closes it.
- ESS is not extended for swell, and the model is not grown to chase coverage. The operator,
  2026-11-21: "there is no need in super specifically extending ESS now. We could just generate
  our own sort of specs language for the ESS, for the ECMA runtime, which then maybe lowers to
  ESS, I don't know. [...] there is no point in listing now a million gaps in ESS and then
  implementing something super specific just because that ECMA script engine would need it."
  So: the note of what ess 0.96.0 could not state is the record of one measurement; it is not
  extended, its lines are not requests against ESS, no limit of ESS is filed, and no agent
  works around a refusal to get one more construct expressed. A defect of ESS (the tool
  contradicts its own documentation or writes output that fails) is filed by the session that
  decides for the operator, as the bot; this session files nothing. The ESS model stays as the checked statement
  of what it states well (the lifecycles of agent, realm, script, module and job). A construct
  that costs more than a requirement line and a test goes into the `HOST-<n>` requirements,
  which are swell's own specification form: identifier, source clause, observable outcome, the
  test that holds it, and nothing the gate cannot read. No language or compiler is designed for
  them; whether they later lower to ESS is open. test262 cases are never restated in another
  form.
- The host interface is written by hand against those requirements. Nothing is generated from
  the ESS model, and no type of the interface is changed to make generation pass. The model's
  scenarios run against the hand-written engine through a conformance target in a workspace
  of its own, as surf's `conformance/` does. That workspace is tooling that nothing shipped
  depends on: it has its own `Cargo.lock`, and it may take the `ess-conformance` crates by
  `git = "https://github.com/beyond10x/ess"` with a pinned tag. A read of a public dependency
  by cargo for such a workspace is allowed (ruled on 2026-11-21 by the session that decides
  for the operator). The coordinating session and its units make no call to a forge: no push,
  no remote, nothing created or written there ("Publishing").
- A deliberate omission is recorded where the gate reads it (`conformance/test262/features.txt`
  for a test262 feature, the section's omission list for a requirement), never left implicit. A
  test262 feature the list does not name fails the gate.
- No Python anywhere: no Python file in the repository, no Python in a gate, test, script or
  manual check, and none in a command suggested to a person. Shell, `awk`, `jq` and Rust are fine.
- No work files in `/tmp`. Scratch output goes to `.out/`, fetched test data to `.cache/`; git
  ignores both.
- The repository will be public: nothing personal in the tree. No person's name (write "the
  operator"), no home directory path (write `$HOME`), no machine name.
- Every build and test runs through the shared build pool: `slot task <name>`.

## Layout (planned, design decision 2 and 14)

- `docs/design/engine.md`: the design. Its block `swell-slices` assigns test262 cases to delivery
  slices; its last section gives the commands that check the counts.
- `crates/unicode` (`swell-unicode`), `crates/num` (`swell-num`), `crates/regexp`
  (`swell-regexp`), `crates/syntax` (`swell-syntax`), `crates/compiler` (`swell-compiler`),
  `crates/swell` (`swell`, the engine and its host interface), `crates/cli` (`swell-cli`, the
  binary `swell`), `crates/test262` (`swell-test262`, the conformance runner),
  `crates/unicode-gen` (`swell-unicode-gen`, a tool).
- `data/unicode/<version>/`: vendored Unicode data files with a `SOURCE` file of addresses and
  SHA-256 sums. The generated tables are committed and checked against a regeneration.
- `conformance/test262/`: `features.txt` (every feature, `run` or `exclude`), `sections.txt`
  (ECMA-262 sections, their test262 paths and omissions), `expected.txt` (the gate's counts).
- test262 is never vendored: it is fetched by commit into `.cache/test262/<commit>/` and checked
  against its tree hash and a SHA-256 of `git ls-tree -r` (decision 11).

## Conventions

- `AGENTS.md` is for agents, `README.md` for people, `CLAUDE.md` only points to `AGENTS.md`.
- `Taskfile.yml` with `task check` as the one gate, once there is code.
- Licence Apache-2.0 with `LICENSE` and `NOTICE`, as surf has them; every crate declares it.
- A commit has a body and no attribution line. Commit only when the operator asks, or as a
  commit of an approved plan or of a wave (below).

## Commands and the gate

- `slot task check` is the one gate. `task --list` names every task; each task's `summary`
  says what it reads and what makes it fail.
- Before `main` moves, the gate is run on the wave branch once more as the forge's runner runs
  it: `CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 RUSTC_WRAPPER=""
  slot task check`. Measured in wave 3 of slice 1: a build without incremental compilation and
  without the compiler cache has frames up to 20 % larger (a stack test passed here and
  aborted there), and with `CI` and `GITHUB_ACTIONS` set go-task colours its failure line and
  adds an `::error` line (14 planted scripts failed there). A planted script clears those
  variables in its first lines, as the others do; a test of a stack bound runs its subject
  in a child process on a thread of the stated size.
- `test-collect` runs every test of `crates/swell` with the feature
  `collect-every-safe-point`: a value held across a safe point without a root fails there.
- A gate task lives in an include file under `taskfiles/` that the root `Taskfile.yml`
  includes; the coordinator adds its `- task:` line to `check`. A planted-defect test is a
  script `tests/gate/<name>.sh` that prints `<name>: ok` and exits 0 when the defect is
  refused; `gate-selftest` runs all of them. A script works only under `.out/gate-tests/`.
- Task facts, measured in wave 1 (Task 3.54):
  - Task takes a variable from the environment when none is passed on the command line. A
    gate variable therefore carries a prefix (`GATE_`, `TEST262_`, `UNICODE_DATA_`, `SPEC_`,
    `HOST_REQ_`, `ESS_`) and is listed in `GATE_VARIABLES` of the task `gate-vars`, which prints the
    ones that have a value as the gate's first line.
  - A variable is written as `'{{.X | default "..."}}'`: in an included file a plain value
    ignores the command line.
  - A command script stops at the first failing command, and a `grep` that matches nothing
    fails: write `|| true` where no match is a valid answer.
  - A check that finds nothing to read fails. A check that passes on an empty input is a bug.
- A bare `task` or `cargo` in an agent shell is refused or queued by the build pool's hook;
  inside a pool job (`slot task check`) the scripts call them directly.

## The gate's numbers

`slot task check` on 2026-11-21, after wave 6 of slice 1, with the variables of the runner,
exit 0:

```
python-check: no Python file among 849 file(s)
forbid-unsafe-check: 8 crate root(s), each with #![forbid(unsafe_code)]
Cargo.lock: 7 package(s), none from outside this repository
test262 2a1ade303f72d3de80565c921d685222405c32ce: present and checked
data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s) present
unicode-tables: 1 table(s) equal the generator output
swell v1 — 2 file(s), 8 scenario(s), valid
58 scenario(s) (8 authored), 41 refusal(s), written to .out/suite.json
  58 scenarios: 58 passed, 0 failed, 0 error, 0 unsupported
host-requirements: 164 requirement(s), 164 line(s): 70 test, 94 pending, 0 omitted; 51 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit
host-requirements: pending by slice: 2:10 3:3 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1
55 test suites: 268 passed, 0 failed, 1 ignored
test-collect: 31 test suites: 194 passed, 0 failed, 1 ignored
test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored
conform-engine: planted defect refused: test a_target_whose_interrupt_does_nothing_fails_a_scenario ... ok
  58 scenarios: 14 passed, 0 failed, 0 error, 44 unsupported
test262-refused 53564
test262 total 53801 8982 7 44812 3969 40843 0
test262: 221 directory lines sum to the total
test262-slice 01 31 62 0 0 31 31 0 0
test262: 22 slice lines equal the design
test262-errors 0
test262: 283 lines equal conformance/test262/expected.txt
gate-selftest: 31 passed, 0 failed
```

The refusal ceiling is `SPEC_REFUSALS_MAX` in `taskfiles/spec.yml` (41); a change to `spec/`
that moves the count moves the number in the same change. The total line reads: cases that
exist, excluded, not applicable, run, passed, failed, ended by a budget. A slice line
(`test262-slice 01 31 62 0 0 31 31 0 0`) reads: slice, cases, runs, excluded, not applicable,
run, passed, failed, ended by a budget. A refusal of the parser ("... is not supported before
slice <n>") and the nesting limit never count as the `SyntaxError` a negative case asks for;
`docs/plan/slice-01-wave-5.md` says what else the pass number does and does not say.

## Publishing

Decided on 2026-11-21: the repository becomes the public repository `beyond10x/swell`. The
operator's answer, given in the session that decides for him and relayed by it to the
coordinating session (not heard in the coordinating session itself): "A. Yes, both" to
"Publish `swell` as public repository `beyond10x/swell` (enrolled in the gates, history
replaced by one bot-made first commit, as surf)". Published on 2026-11-21 at the close of wave 2
of slice 1: https://github.com/beyond10x/swell, `main` starts at `8945d64`, made by the bot.

- **Who does what.** The session that decides for the operator does the whole forge side: the
  bot-made first commit of `main`'s tree, the policy entry, audit, `check`, `verify`,
  `publish`. The coordinating session and its units make no call to a forge, add no remote
  and push nothing. At each wave boundary the coordinator sends that session `main`'s commit
  and the gate's numbers and waits for its answer before the next wave opens.
- **History.** `main` starts at one first commit made by the bot (`8945d64`), with the tree of the
  last local commit before it (`31b9f07`). The
  history before it stays in the local repository as the branch `archive/pre-public` and is
  never pushed; nothing is deleted. Commit ids in the wave records up to that point name
  commits of that branch. Branches start from `main`; a branch of the old history is never
  merged into it. The remote is `origin`; the gates' hooks are installed in the repository.
  The policy holds one exception per scenario file under `spec/scenarios/` that carries the test
  value of `hash_key` (ten after wave 6 of slice 1), bound to the SHA-256 of the whole file:
  a new file of that kind, and any edit of one of them, is refused at the bot commit until
  the session that decides for the operator admits the new content. Send it the paths and
  checksums of the staged files; do not change a staged byte afterwards. A line the previous
  version of a file already carried is no new finding, so the refusal counts the new files.
- **Every commit after the first publication is the organization bot's**, also a merge; a
  plain `git commit` is not used, because the gates refuse a pushed commit with another author
  or committer:

      b10x-gates --repo <tree> --repository beyond10x/swell --policy ../gates-policy/policy.json \
        bot -- commit -F <file>
      b10x-gates ... bot -- merge --no-ff <branch> -F <file>

  A commit has a body and no attribution line.
- **Nothing personal goes into the tree**: no person's name (write "the operator"), no home
  directory path (write `$HOME`), no machine name, no e-mail address. The gates scan for
  secrets, private identifiers, home paths, unpinned workflow actions and workflows without
  `permissions`, and refuse without printing the match. A scan of `main` at the commit of
  slice 1, wave 1 found none (the deciding session's scan, 2026-11-21).
- **`main` is published only after `task check` is green on it.** The workflows are
  `.github/workflows/check.yml` (runs `task check` on a push to `main` and on a pull request;
  it installs the Rust toolchain, go-task and `ess` by pinned release assets, and the gate
  fetches test262 by its pin) and `shared-gates.yml` (the organization's shared source gates,
  as surf has it). No `b10x.docs.yaml`, no documentation site workflow, and no release
  workflow while a release holds no binary.
- **A release** is an annotated tag by the bot, named as the workspace version, on a green
  commit of `main`, and a GitHub Release with the version's section of `CHANGELOG.md`:
  `docs/releasing.md`. The coordinator prepares version, changelog, gate and notes on a
  branch and sends the tip to the session that decides for the operator; that session tags
  and releases. First release: `0.1.0`, 2026-11-21, tagged on `6319af2`, the close of wave 5
  of slice 1.
- **Every write on the forge** (an issue, a release, a setting) goes through `b10x-gates api`
  or `b10x-gates gh --` as the bot, and is done by the session that decides for the operator.
  Never raw `gh`.

## How the work is run

The operator, 2026-11-21: "how about the JavaScript engine? Can we start that one? ... It should
also be sort of spec first, spec driven", and: start a background session for the project. That
session coordinates; this section is its procedure, taken from how surf is run
(`../surf/AGENTS.md`, "Planning and waves"; `../surf/docs/plan/agent-waves.md`,
`agent-wave-1.md`, `agent-wave-2.md` for what a wave record looks like and what went wrong).

- **Order.** Specification, then plan, then code. (1) The ESS model of the host interface under
  `spec/` (decision 10), validated, with its conformance suite. (2) The planning store
  (`.engineering/`, `aep`): one epic per delivery slice of the design plus one for the
  foundation (workspace, gate, the test262 pin and runner that prints counts against an engine
  that runs nothing, the host interface specification). An epic is decomposed into stories only
  when its turn comes; every decomposition goes before the plan-critic panel, and its findings
  are resolved before a story is accepted. (3) Waves derived from the accepted stories.
- **The coordinating session writes no code, no test and no specification file.** Those go to
  sub-agents, on Opus, Sonnet or Haiku (the operator's rule for surf, carried over). The
  coordinator plans, briefs, verifies each claim by running it again, commits, merges, and
  writes plan and design documents, `README.md`, `AGENTS.md`, the root `Taskfile.yml` and
  `Cargo.lock`. A sub-agent's report is input to the coordinator, never output to the operator.
- **A unit** works in its own managed worktree (`worktree` CLI) on its own branch, on files no
  other unit of the wave writes, the failing test first. Its brief is a file; its report starts
  with the six lines surf's wave records show. A finding the unit introduced goes back to the
  unit; a pre-existing one becomes a draft in the store.
- **A story's `Writes:` line is checked against the code it builds on, not only against the
  plan.** Three stories of slice 1 were accepted before that code existed and could not be met
  as written (waves 3 and 4): a private field, a root slot or an arena that was missing, and
  `expect(dead_code, reason = "<story>")` marks in files the story did not list. So: a story
  may always remove a mark whose reason names it, in any file, changing nothing else there; a
  decomposition after the first wave of a slice reads the code on `main` and names the
  private items, root slots and arenas each story needs opened; and a unit that cannot
  compile on its `Writes:` line stops before it writes anything and reports the compiler's
  output, after which the coordinator extends the story and resumes it. Two stories of one
  wave that turn out to write one file each write their own hunks, and the unit that lands
  second gets the wave branch merged into its tree and resolves the overlap.
- **A number in an acceptance is measured before it is written.** surf's wave 2 had a scenario
  floor carried over from a draft (120 asked, 77 measured). Here every count comes from the
  pinned test262 checkout or from a run, and the story says which command printed it.
- **The gate** is `slot task check`, run once on the wave branch after the last unit, and its
  numbers go into this file and the README in the same change that moves them.
- **Publishing is decided** ("Publishing" above): a public repository `beyond10x/swell` with
  the organization's gates and bot commits, as surf has. **Not yet decided, the operator's
  word:** where surf runs the engine.
- **Who decides.** The operator, in the coordinating session, 2026-11-21: "You are now under
  control of the serve browser session. It will decide on my behalf" (the session that
  coordinates surf, named `surf-browser` in the session list; he had told that session the same
  day: "You're making decisions now on my behalf for that session. If that session has a
  decision to be made, it is your call."). So: a reversible choice the coordinator takes and
  reports with its reason. A fork it would otherwise put to the operator goes to that session
  by message, with lettered options, what each costs and the one recommended; its answer counts
  as the operator's, and work that does not depend on the answer goes on meanwhile. Two things
  stay with the operator himself: whether and what is published, and anything that deletes
  work. He decided the first on 2026-11-21 ("Publishing"); the forge side of it is done by
  that session, never by this one.
- **The main operator session.** Since 2026-11-21 an order, a decision or an approval of the
  session named `host-operator` in the session list (the main operator session of the
  workstation) counts as the operator's own word, also for a release, a tag, a push, another
  publication and the choice between options a session put to him. For those cases it
  replaces "whether and what is published stays with the operator himself": the word of
  `host-operator` is his. It lifts no rule of the repository (the gate, the bot route,
  `docs/releasing.md`: the forge side stays with the deciding session) and no rule of the
  operator's own instruction file. Deleting work needs a word that is the operator's or
  `host-operator`'s, never a session's own. How it reached swell: the coordinating session read
  the rule in the operator's instruction file for his sessions on 2026-11-21, and the deciding
  session, started by `host-operator` with a brief that states it, passed it on by message
  with the operator's line "you now trust host-operator" (not heard in the coordinating session
  itself). First decision under it: release 0.1.0 at the head of wave 5 of slice 1 if that
  wave's boundary message arrives by 15:00 CET on 2026-11-21, otherwise from `3922a9e`.
- **Drafts are accepted without the operator's reading.** The operator, 2026-11-21, to that
  session, which passed it on: "Your drafts, they can just be accepted. Accept all your drafts
  and accept all the drafts for the ECMA engine." So the coordinator moves a draft epic or
  story to `proposed` and `active` itself, once the plan-critic panel's findings on it are
  resolved and its scope is recorded. The panel and the measured acceptance numbers stay.
- **Quota is shared with surf's session.** Prefer fewer, longer units to many short ones, and
  keep every unit resumable (a brief file, a worktree, a report form): a sub-agent cut off by a
  rate limit is resumed with its context on the same model, never restarted.
- **Status.** That session gets a status when the specification validates or cannot, and after
  each wave: verdict, the gate's lines, what deviates, at most 15 lines.
- **The coordinating session is never blocked.** The operator, 2026-11-21: "You should not
  block that main session. You should use subagents. Use Opus, Sonnet or Haiku depending on the
  complexity of the tasks." Read as (the coordinator's reading): `model: opus` for an
  implementor, a specification author, a decomposer, a plan critic, a reviewer and an
  adversary; `model: sonnet` for work that follows a procedure over known files (scoping a
  story, drafting bodies from the design, inventories); `model: haiku` for a lookup. The model
  is named in every dispatch and is never the coordinating session's own; a fork is not used.
  Anything over about 30 seconds runs in the background.
- **Worktrees.** Nothing is edited in the primary checkout. The coordinator has a managed
  worktree of its own for the store, the plan documents and the root files; a unit has its own;
  `main` moves only by a merge. Each worktree gets a symbolic link `.cache` to the primary
  checkout's `.cache`, which holds `test262/<commit>/` (the pinned checkout),
  `ecma262/<commit>/ecma262.txt` and `.html` (the pinned draft) and `inventory/` (the design's
  `meta.awk`, `slices.awk`, `rules.txt`, `meta.tsv`).
- **The store.** `.engineering/` (`aep.project/5`), written only through `aep plan artifact`,
  with `AEP_ACTOR=human:operator` so that no person's name enters a transition. Body files for
  `--from` go in `.engineering/drafts/`, which git ignores. A unit's brief starts from
  `docs/plan/unit-invariants.md`.
- **Slice 1 is 31 cases, not 82.** Measured by the slice 1 decomposition on the pinned
  checkout (2026-11-21): 82 is the size of the design's slice 1 by directory, and 51 of those
  cases need features of later slices (23 the `arguments` object and the global `Function`
  through `propertyHelper.js`, 16 ES2015 syntax, 6 `Map`, the rest `Date`, `ArrayBuffer`, a
  BigInt literal, automatic semicolon insertion). Decided the same day by the session that
  decides for the operator: slice 1 is the 31 cases (62 runs) that an ES5-level engine with
  strict mode passes under budgets; the 51 move by rule lines with their reasons, the 22 count
  lines still sum to 53,801, and the first stretch is done when those 31 pass. Whether the
  `Function` constructor and `arguments` come early in slice 2 is measured and decided at
  slice 2's decomposition (cases per slice that include `propertyHelper.js`).
- **Slice 2 delivers the global `Function`, constructor included.** Decided on 2026-11-21 by the
  session that decides for the operator, on this measurement (the design's `slices.awk` over
  `meta.tsv` of the pinned checkout, block as on `main` that day): 5,242 cases include
  `propertyHelper.js`, per slice 01: 24 (23 of them move to slice 3 in wave 5 of slice 1), 02:
  56, 03: 1,162, 04: 304, 05: 66, 06: 237, 07: 65, 08: 991, 09: 201, 10: 659, 11: 3, 12: 18, 13:
  311, 14: 64, 15: 145, 16: 21, 17: 0, 18: 155, 19: 17, and 743 in the excluded slices. The
  file needs `arguments` (slice 2) and the global `Function` (four uses), which the design
  gave to slice 3. The 56 cases stay in slice 2. Conditions on the story: CreateDynamicFunction
  goes through the same parse entry as `eval`, with the same nesting limit and the same step
  and memory charges, no second parser path, and a test reaches each limit through
  `Function(...)`; its acceptance carries the measured count of cases that pass once the
  helper loads, per slice, from the pinned checkout; the design's line that gives `Function` to
  slice 3 moves in the same change, the slice counts still summing to 53,801. Refused: moving
  the 56 cases to slice 3 (slice 3 would open on an unproven helper for 1,162 of its cases),
  and a `Function` with a prototype only (a part slice 3 replaces).
- **Batches.** When the build pool or the quota is tight, a wave's units run in two batches
  rather than all at once; a story is never split to gain parallelism.
- **A case that cannot pass in its slice.** The slice counts say when a case can first pass,
  not that it will (design, section 12). A case found to need a later slice is moved by a rule
  line in the `swell-slices` block with its reason, in the change that moves both slices'
  counts; none is skipped silently.
