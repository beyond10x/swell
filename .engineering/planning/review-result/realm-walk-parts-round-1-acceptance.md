---
format: aep.planning-md/3
id: review-result:realm-walk-parts-round-1-acceptance
kind: review-result
status: active
title: The realm walk in parts, round 1, the acceptance critic
relations:
- reviews: story:realm-walk-in-parts-under-the-cpu-bound
revision: 1
---
needs-revision

The red run and the two breaks work, and the Context is true on the tree apart from one wrong line cite. The stop condition for the number of parts, the contradictory gate item and an unobservable grep need to change.

Paths: the draft is `realm-walk-in-parts-under-the-cpu-bound.md` in the drafts directory. Other paths are relative to the tree root.

**What checked out**
- The `builtins_symbol_values.rs` cites match: 491-615, 351-446, 332, 431, 496, 535-541, 470-478, 559-566, 579-614, 483-486 and 468-469.
- The `language.rs` and `fuzz.rs` cuts match the draft. The `.config/nextest.toml` `slow-timeout` of 60 s matches. `16:245` is at `taskfiles/forge.yml:7`.
- The `AGENTS.md` cites are right: 252, 259, 295-304 and 310-315. The measuring command is quoted correctly.
- `git grep -c` prints 1 for the file, and `grep -n 'symbol_values\|walk' AGENTS.md` prints nothing.
- Red first is real: 3.243 and 3.081 are well over the 2.5 bound, and `e51191a` is in this tree's history.
- Both breaks can be made and seen. The `% (len + 1)` edit leaves one index in no part, so the cover test fails. A one-character change to a row's place fails the cover test, while the parts drop the row without noticing.
- `slot` stands in front of every command that compiles or tests. The `cargo test --no-run` builds go through the shim, and `sh taskfiles/test-main.sh check` compiles nothing.

**Findings**

1. **blocker** — Change item 4, `realm-walk-in-parts-under-the-cpu-bound.md:118-121`: "if `W` alone is at 2.0 or more in the full collect cell, no number of parts reaches the room; the unit stops".
   - The stop covers only the 2.0 cell. Every part pays `W`, so a cell is reachable only when `W` is below that cell's four-fifths bound.
   - The collect gate-count cell needs `W` below 1.6 and the default cell below 1.2. The list is the same work at the gate and the full count in a mode, so `W` between 1.6 and 2.0 passes the stop and still can never reach the 1.6 cell.
   - Nothing caps `n` either. As `W` nears the bound, the smallest passing `n` grows without limit and the unit keeps adding parts.
   - The same gap is in the `AGENTS.md` sentence (`:143-154`) and in item 5 (`:122-128`): "a part over four fifths of its bound ... makes it add a part". Adding a part never lowers `W`. A later unit whose `W` alone is over four fifths would loop. State a stop per cell (`W` plus the smallest part's calls over the bound) and a cap on `n` or a total-CPU limit, and put the stop into the `AGENTS.md` sentence.

2. **warning** — Change item 4, `:115-117`: "every function the realm gains costs the part that holds it a list step and the calls".
   - A new function adds to the list, which every part builds. So it adds its list step to every part, and its calls only to the part that holds it.
   - Growth therefore raises `W` in all `n` parts. The room claim and the "add a part" remedy need that stated.

3. **warning** — Acceptance item 7, `:243-248`: "`slot --timeout 30m task check` exits 0 ... If the gate's CPU bound is the red line, the unit does not change `CPU_BOUNDS_BY_CORES`; it reports the readings."
   - Exit 0 and a red `gate-cpu` line cannot both hold. The unit has no stop for a cut that adds `(n-1) x W` in each of two modes and turns the gate red; `AGENTS.md` says a red `gate-cpu` line holds the commit.
   - Say what the unit does in that case, for example stop and report.

4. **warning** — Acceptance item 6, `:240-242`: "`git grep -n '\.create_realm(' -- 'crates/swell/tests/*.rs' ':!crates/swell/tests/support'` prints the lines of that comment, one for each `file::function`, and no other."
   - That command excludes `support/` and so prints the call sites in the test files. They are 20 source lines, not comment lines. The check as written cannot show the comment line.
   - Use `git grep -n 'builtins_symbol_values.rs::' -- crates/swell/tests/support/host.rs` and name the line it must print.

5. **warning** — Change item 4 and Acceptance, `:114-115` against `:195-196` and `:208-212`: "2.0 in either mode at its full count" against "The cells are three".
   - Item 4 gives the default mode a full-count bound of 2.0. The cells and the `AGENTS.md` sentence give the default mode only 1.2, since its gate count equals its full count.
   - The measure job's `<f>` for the default cell is not named: empty or `1`. Say that the default cell is read once, with `<f>` named, against 1.2.

6. **note** — Context, `:24-25`: "`crates/swell/tests/support/host.rs:365-369` lists it as an exception".
   - The walk's entry is lines 367-369. Line 365 is the `budgets.rs` entry.

7. **note** — Acceptance item 2, `:204-207`: the cover test prints `<n> parts` before `n` is chosen.
   - `n` comes from the `W` that this run measures. Name a provisional `WALK_PARTS` of 2 parts for that run.

8. **note** — Change item 4 and "Not in this story", `:263-265`: the list is built once per test, `n+1` times, which is the shape the folder rule names ("an expensive product is made once").
   - The fallback is reserved for `W` of 2.0 or more. The draft should say why the list cannot be shared across nextest processes (its handles belong to one agent) and what `W` counts as cheap enough.

9. **note** — Acceptance item 6, `:236`: the grep pattern `144 functions\|576 calls\|1\.8 CPU` leaves the "288 at the gate count" that Change item 7 (`:138-140`) also drops. Add `288`.

10. **note** — Context, `:42-44`: 3.243, 3.081, 3.577 and 232 are cited to the realm-size unit.
    - They are not in the tree. `e51191a` records only "3.24" and "3.08". The brief gave these figures, so the draft should name the source.

Outside my lane: the units of slice 4 whose acceptance runs the old test name by `--exact` now run only part 0. That belongs to the scope and parallel-safety critics.

```findings
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 118
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The stop ('if W alone is at 2.0 or more in the full collect cell') covers one cell; the 1.6 gate-count cell and the 1.2 default cell are unreachable once W reaches them, no cap on n exists, and the AGENTS.md sentence (lines 143-154) and item 5 (122-128) tell a later unit to add a part with no stop when W alone is over four fifths."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 115
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'every function the realm gains costs the part that holds it a list step and the calls' is wrong: every part builds the whole list, so growth adds its list step to all n parts and the calls to one; the room rule must say so."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 243
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Item 7 requires task check to exit 0 and also says the unit reports and changes nothing if the gate's CPU bound is the red line; the two cannot both hold and no stop is named for a cut that turns gate-cpu red."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 240
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The git grep with ':!crates/swell/tests/support' prints the test files' call sites, not the doc-comment line in support/host.rs, so 'prints the lines of that comment' cannot be observed; grep support/host.rs for 'builtins_symbol_values.rs::' instead."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 114
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Item 4 gives '2.0 in either mode at its full count' while the acceptance has three cells with the default cell at 1.2, and the measure job's <f> for the default cell is not named."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 24
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'crates/swell/tests/support/host.rs:365-369' should be 367-369; line 365 is the budgets.rs entry."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 204
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Acceptance item 2 prints '<n> parts' before n is chosen from W; name a provisional WALK_PARTS of 2 for that run."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 263
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The list is built once per part (n+1 times), the shape of the folder rule 'made once, never per test'; the draft does not say why the list cannot be shared across nextest processes or what W counts as cheap enough."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 236
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The grep '144 functions\\|576 calls\\|1\\.8 CPU' leaves '288 at the gate count', which Change item 7 also drops; add 288."
- file: realm-walk-in-parts-under-the-cpu-bound.md
  line: 42
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "3.243, 3.081, 3.577 and 232 functions are not recorded on the tree (e51191a's message has 3.24 and 3.08); name the source."
```
