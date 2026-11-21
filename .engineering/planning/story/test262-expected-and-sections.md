---
format: aep.planning-md/3
id: story:test262-expected-and-sections
kind: story
status: implemented
title: 'expected.txt and sections.txt: the gate compares every line the runner prints'
summary: 33 sections of clauses 19 to 28 with their test262 directory, three omissions, every one incomplete; expected.txt of 278 lines; a changed count in either the file or the output fails the gate.
relations:
- decomposes: epic:foundation
- depends_on: story:test262-counting-runner
scope:
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: crates/test262
- confidence: cited
  path: taskfiles/test262.yml
- confidence: cited
  path: tests/gate
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T05:02:02Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Design decision 11: the gate compares the lines of the runner with
`conformance/test262/expected.txt` and fails on any difference, printing the lines that differ, so
a lost pass fails and a new pass is taken in by committing the new expected file; a second block
prints, per section of ECMA-262 listed in `conformance/test262/sections.txt` (section anchor, the
test262 paths that test it, its deliberate omissions), `complete` or `incomplete`. A section is
`complete` only when every case of its paths passes and its omission list is empty (`AGENTS.md`,
"Goals"). `story:test262-counting-runner` prints the directory, total, slice and error lines;
this story adds the section block, writes both files and makes the comparison the gate.

Settled here, the open item "which sections `sections.txt` lists first": every second-level
section of clauses 19 to 28 of the pinned ECMA-262 whose title, with a leading `The `, a trailing
` Object` or ` Objects` and a parenthesis removed, is the name of a directory
`test/built-ins/<name>/` of the pinned test262; that directory is its one path. The rule is a
command, so the set is checked and not chosen: in
`.cache/ecma262/74c33d406ec1763ca9ee5ad9165571661649d6e6/`,

```sh
grep -o 'href=#sec-[^ >]* title="[^"]*"><span class=secnum>[0-9A-Z.]*</span>' ecma262.html |
  sed -E 's/href=#(sec-[^ ]*) title="([^"]*)"><span class=secnum>([^<]*)<\/span>/\3\t\1\t\2/' |
  awk -F'\t' '!seen[$1]++' > toc.tsv
awk -F'\t' '$1 ~ /^(19|2[0-8])\.[0-9]+$/ {n = $3; sub(/^The /, "", n); sub(/ Objects?$/, "", n);
  sub(/ \(.*\)$/, "", n); print $1 "\t" $2 "\t" n}' toc.tsv |
  while IFS=$'\t' read num anchor name; do
    [ -d "$T/test/built-ins/$name" ] &&
      echo "$num $anchor test/built-ins/$name/ $(grep -c "^test/built-ins/$name/" files.txt)"
  done
```

(`T` the pinned test262 checkout, `files.txt` from `.cache/inventory/`) prints 33 sections with
their case counts, 17010 cases together. The only second-level section of clauses 19 to 28 with a
title the first command does not read (it is written without quotes) is 27.1 `sec-iteration`,
which has no directory (`grep -o 'href=#sec-[^ >]* title=[^"][^>]*><span class=secnum>\(19\|2[0-8]\)\.[0-9]*</span>'`).
Every anchor is an `id` of the pinned `ecma262.html` (checked against
`grep -o 'id="\?sec-[A-Za-z0-9.%_-]*' ecma262.html`). The 33, as number, anchor, path, cases:

  20.1 `sec-object-objects` `test/built-ins/Object/` 3411
  20.2 `sec-function-objects` `test/built-ins/Function/` 509
  20.3 `sec-boolean-objects` `test/built-ins/Boolean/` 51
  20.4 `sec-symbol-objects` `test/built-ins/Symbol/` 98
  20.5 `sec-error-objects` `test/built-ins/Error/` 93
  21.1 `sec-number-objects` `test/built-ins/Number/` 340
  21.2 `sec-bigint-objects` `test/built-ins/BigInt/` 77
  21.3 `sec-math-object` `test/built-ins/Math/` 327
  21.4 `sec-date-objects` `test/built-ins/Date/` 610
  22.1 `sec-string-objects` `test/built-ins/String/` 1223
  22.2 `sec-regexp-regular-expression-objects` `test/built-ins/RegExp/` 1879
  23.1 `sec-array-objects` `test/built-ins/Array/` 3083
  23.2 `sec-typedarray-objects` `test/built-ins/TypedArray/` 1453
  23.3 `sec-uint8array` `test/built-ins/Uint8Array/` 70
  24.1 `sec-map-objects` `test/built-ins/Map/` 204
  24.2 `sec-set-objects` `test/built-ins/Set/` 383
  24.3 `sec-weakmap-objects` `test/built-ins/WeakMap/` 141
  24.4 `sec-weakset-objects` `test/built-ins/WeakSet/` 85
  25.1 `sec-arraybuffer-objects` `test/built-ins/ArrayBuffer/` 221
  25.2 `sec-sharedarraybuffer-objects` `test/built-ins/SharedArrayBuffer/` 104
  25.3 `sec-dataview-objects` `test/built-ins/DataView/` 560
  25.4 `sec-atomics-object` `test/built-ins/Atomics/` 389
  25.5 `sec-json-object` `test/built-ins/JSON/` 166
  26.1 `sec-weak-ref-objects` `test/built-ins/WeakRef/` 29
  26.2 `sec-finalization-registry-objects` `test/built-ins/FinalizationRegistry/` 47
  27.3 `sec-disposablestack-objects` `test/built-ins/DisposableStack/` 93
  27.4 `sec-asyncdisposablestack-objects` `test/built-ins/AsyncDisposableStack/` 104
  27.5 `sec-promise-objects` `test/built-ins/Promise/` 732
  27.6 `sec-generatorfunction-objects` `test/built-ins/GeneratorFunction/` 23
  27.7 `sec-asyncgeneratorfunction-objects` `test/built-ins/AsyncGeneratorFunction/` 23
  27.10 `sec-async-function-objects` `test/built-ins/AsyncFunction/` 18
  28.1 `sec-reflect-object` `test/built-ins/Reflect/` 153
  28.2 `sec-proxy-objects` `test/built-ins/Proxy/` 311

The omissions, each from a decision taken: `sec-string-objects` and
`sec-regexp-regular-expression-objects` omit the latest Unicode version, since swell is pinned to
17.0.0 (open item C, option (a): "the deviation is listed as an omission of every section that
depends on it", case mapping and normalization in 22.1, property escapes and case folding in
22.2); `sec-atomics-object` omits concurrent agents (decision 1 and open item B: the 112
`$262.agent` cases are out until surf needs workers; `grep -rl '\$262\.agent' test | wc -l` in the
checkout prints 112). Every section starts `incomplete`, as the epic says.

## Additions after waves 1 and 2 (2026-11-21)

- Read `AGENTS.md`, "Commands and the gate": the Task facts measured in wave 1 bind this story
  (a gate variable carries the prefix `TEST262_` and is written as `'{{.X | default "..."}}'`; a
  script stops at the first failing command; a check that finds nothing to read fails). Where
  this story names a task variable without the prefix, the prefixed name is meant; the runner's
  variables are `TEST262_DESIGN`, `TEST262_FEATURES`, `TEST262_OUT`, `TEST262_COMMIT`,
  `TEST262_CACHE`. A new one is reported so that the coordinator adds it to `GATE_VARIABLES`.
- The runner as landed (`crates/test262`, 17 tests) prints, beside the directory, slice and
  total lines, the lines `test262: 221 directory lines sum to the total`, `test262: 22 slice lines
  equal the design` and `test262-errors 0`, and reads CR LF, CR and LF as line ends. The line
  counts of this story (278 for `expected.txt`) were computed before that: measure which lines
  the expected file holds on the base of the wave and state the count you measured.
- From wave 2's record: no planted-defect script covers the failure paths of the task
  `test262`. This story adds them under `tests/gate/`, in the form of the others (each prints
  `<name>: ok` when the defect is refused, works only under `.out/gate-tests/<name>/`, and passes
  on the unchanged input first): `test262-refuses-a-feature-missing-from-the-list.sh`,
  `test262-refuses-a-slice-line-that-differs-from-the-design.sh`,
  `test262-refuses-a-count-that-differs-from-expected.sh` (the planted test of acceptance item 3
  of the epic) and `test262-refuses-a-section-marked-complete-with-a-failing-case.sh`. They are on
  this story's `Writes:` line.
- `Cargo.lock`: this story is alone in its wave; if it adds no crate the lock does not change.

## Change

Failing tests first: the tests below are written before the code and fail before it; `slot task
test262` keeps passing until the task passes `--expected`, and then fails until `expected.txt`
exists.

- `conformance/test262/sections.txt`, no comments, the 33 sections in the order above, each as
  `section <anchor>`, then `path <anchor> <test262 path>`, then any `omit <anchor> <reason>
  <decision>`: `omit sec-string-objects unicode-17.0.0 open-item-C`,
  `omit sec-regexp-regular-expression-objects unicode-17.0.0 open-item-C`,
  `omit sec-atomics-object concurrent-agents open-item-B`.
- `crates/test262/src/sections.rs`: the reader (a `path` or `omit` of an undeclared section, a
  section declared twice, or a path that matches no case fails the run, naming the line) and the
  judgement: `complete` only when every case of the paths passed and there is no omission; an
  excluded or not-applicable case is not a pass. Output, after the `test262-errors` line, one
  line per section in file order:
  `test262-section <anchor> <complete|incomplete> <cases> <passed> <omissions>`.
- `crates/test262/src/expected.rs` and the `count` options `--sections <file>` and
  `--expected <file>`: with `--expected`, every printed line is compared with the file in order;
  each line that differs goes to standard error as `-<expected line>` and `+<printed line>`, then
  `test262: <n> line(s) differ from <file>`, and the run exits 1; when all are equal it prints
  `test262: <n> lines equal <file>` to standard error.
- Tests in `crates/test262/tests/`, each named: `sections_file_reads_sections_paths_and_omissions`,
  `a_section_is_incomplete_while_a_case_of_its_paths_does_not_pass`,
  `a_section_with_an_omission_is_incomplete_when_every_case_passes` and
  `a_section_whose_cases_all_pass_without_omissions_is_complete` (both on the judgement with
  counts, since the engine passes nothing), `a_section_path_that_matches_no_case_fails_the_run`,
  `a_changed_count_in_the_expected_file_fails_the_run` (the fixture tree of the runner with an
  expected file whose one count is changed: exit 1, that line printed with `-` and `+`),
  `a_changed_count_in_the_printed_lines_fails_the_run` (a copy of the fixture tree under
  `CARGO_TARGET_TMPDIR` with one case added, against the unchanged expected file: exit 1),
  `equal_lines_pass_the_comparison`. Fixture files under `crates/test262/tests/fixtures/` as
  these tests need.
- `conformance/test262/expected.txt`: the lines `swell-test262 count` prints on the pinned
  checkout with `sections.txt`: 221 directory lines, the total line, 22 slice lines,
  `test262-errors 0`, 33 section lines; 278 lines. Its total line is
  `test262 total 53801 8982 7 44812 0 44812 0` and its section lines read
  `test262-section <anchor> incomplete <cases> 0 <omissions>` with the case counts above (for
  example `test262-section sec-object-objects incomplete 3411 0 0`,
  `test262-section sec-string-objects incomplete 1223 0 1`).
- `taskfiles/test262.yml`: the task `test262` passes `--sections conformance/test262/sections.txt
  --expected conformance/test262/expected.txt`, prints the differing lines when it fails, and
  prints the `lines equal` line besides its lines from before.

Gate task: none new; `test262` is already in `check`.

Writes: `conformance/test262/sections.txt`, `conformance/test262/expected.txt`,
`crates/test262/src/sections.rs`, `crates/test262/src/expected.rs`, `crates/test262/src/main.rs`,
`crates/test262/src/lib.rs`, `crates/test262/tests/sections.rs`,
`crates/test262/tests/expected.rs`, `crates/test262/tests/fixtures/` (added files),
`taskfiles/test262.yml`.

## Acceptance

`cargo test -p swell-test262` passes the 8 tests named in Change, among them
`a_changed_count_in_the_expected_file_fails_the_run` and
`a_changed_count_in_the_printed_lines_fails_the_run`; and `slot task test262` exits 0 and prints
`test262 total 53801 8982 7 44812 0 44812 0` and
`test262: 278 lines equal conformance/test262/expected.txt`, with 33 `test262-section` lines in
`.out/test262/lines.txt`, each `incomplete`, among them, in full,
`test262-section sec-object-objects incomplete 3411 0 0`,
`test262-section sec-string-objects incomplete 1223 0 1`,
`test262-section sec-regexp-regular-expression-objects incomplete 1879 0 1`,
`test262-section sec-atomics-object incomplete 389 0 1` and
`test262-section sec-proxy-objects incomplete 311 0 0` (case counts from the rule command in
Context, omission counts from the three `omit` lines).

## Out of scope

Sections of clauses 5 to 18, of Annex B and the language clauses, whose test262 paths are not one
directory each: each slice adds the sections it works on. Raising any count: slice 1. Checking the
anchors against ECMA-262 in the gate: there is no task that fetches or checks the ECMA-262 pin.
Any edit of the design, `AGENTS.md` or `README.md`: the coordinator records the line format and
the counts.
