---
format: aep.planning-md/3
id: story:unicode-data-pin
kind: story
status: implemented
title: Vendor the Unicode 17.0.0 data files with their SOURCE checksums and licence
summary: data/unicode/17.0.0/ with the 14 files of decision 8, emoji-sequences.txt, emoji-zwj-sequences.txt and LICENSE, a SOURCE of 17 lines with address and SHA-256, and a gate task that refuses a missing, changed or unlisted file.
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: data/.gitattributes
- confidence: cited
  path: data/unicode/17.0.0
- confidence: cited
  path: taskfiles/unicode-data.yml
- confidence: cited
  path: tests/gate/unicode-data-check-refuses-a-changed-file.sh
- confidence: cited
  path: tests/gate/unicode-data-check-refuses-a-missing-file.sh
- confidence: cited
  path: tests/gate/unicode-data-check-refuses-an-unlisted-file.sh
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:04:14Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Design decision 8 pins one Unicode version for every table and vendors the data files under
`data/unicode/<version>/`, with a `SOURCE` file holding the address of each file and SHA-256 and the
licence of the data beside them. The version is 17.0.0 (open item C, taken at the recommended
option, `AGENTS.md` "Status"): test262 is written for it. Measured: `find
test/built-ins/RegExp/property-escapes/generated -type f | wc -l` prints 469 and
`grep -rl 'Unicode v17.0.0' test/built-ins/RegExp/property-escapes/generated/ | wc -l` prints 469,
in the pinned checkout `.cache/test262/2a1ade303f72d3de80565c921d685222405c32ce/` (the design says
469; it matches). The generator and the tables belong to slice 5; this story brings the data in
and makes the gate hold its bytes.

The files decision 8 lists, 14, by their path under `https://www.unicode.org/Public/17.0.0/ucd/`:
`UnicodeData.txt`, `SpecialCasing.txt`, `CaseFolding.txt`, `CompositionExclusions.txt`,
`DerivedNormalizationProps.txt`, `PropertyAliases.txt`, `PropertyValueAliases.txt`,
`Scripts.txt`, `ScriptExtensions.txt`, `PropList.txt`, `DerivedCoreProperties.txt`,
`extracted/DerivedGeneralCategory.txt`, `extracted/DerivedBinaryProperties.txt`,
`emoji/emoji-data.txt`. Beside them, "the emoji sequence files of the same version" for the
properties of strings under the `v` flag. The pinned ECMA-262 names seven of those (counted with
`grep -o 'RGI_Emoji[A-Za-z_]*\|Basic_Emoji\|Emoji_Keycap_Sequence' ecma262.html | sort | uniq -c`
in `.cache/ecma262/74c33d406ec1763ca9ee5ad9165571661649d6e6/`): `Basic_Emoji`,
`Emoji_Keycap_Sequence`, `RGI_Emoji`, `RGI_Emoji_Flag_Sequence`, `RGI_Emoji_Modifier_Sequence`,
`RGI_Emoji_Tag_Sequence`, `RGI_Emoji_ZWJ_Sequence`.

Settled here, the open item of the epic and "Not checked" of the design: where the 17.0.0 emoji
sequence files are published, and the text of the Unicode data licence. Both are read from
`unicode.org`, recorded in `SOURCE` and `LICENSE`, and reported, so that the coordinator moves
both out of "Not checked" in the design. The files expected are `emoji-sequences.txt` and
`emoji-zwj-sequences.txt` (UTS #51 defines the seven properties above in them); the story
confirms this in UTS #51 of version 17.0.0 and reports what it found, also when it differs.

Network: only these addresses may be fetched, and nothing else: the 14 files under
`https://www.unicode.org/Public/17.0.0/ucd/`; the listings
`https://www.unicode.org/Public/17.0.0/emoji/` and `https://www.unicode.org/Public/emoji/17.0/`
and the two sequence files from whichever of them holds the 17.0.0 files;
`https://www.unicode.org/license.txt`; `https://www.unicode.org/reports/tr51/`. Everything is
fetched into an empty directory under `.out/unicode-fetch/` first, read as data only, then
copied.

The gate is put together as `story:workspace-and-gate` describes: this story writes its own
include file `taskfiles/unicode-data.yml`, which the root `Taskfile.yml` includes as optional,
and its planted-defect tests as scripts under `tests/gate/` in the form that story gives (each prints `<test name>: ok` when the defect is refused). It
runs alone with `task -d . -t taskfiles/unicode-data.yml unicode-data-check`.

## Change

Failing test first: `tests/gate/unicode-data-check-refuses-a-missing-file.sh` is written before
the task and the data; it prints `FAIL` (no task) before the change and `ok` after.

- `data/unicode/17.0.0/ucd/`: the 14 files at the paths above, byte for byte as fetched.
- `data/unicode/17.0.0/emoji/`: `emoji-sequences.txt` and `emoji-zwj-sequences.txt` of 17.0.0.
  If UTS #51 names other files for the seven properties, the unit stops before writing the
  task, reports what UTS #51 says, and the coordinator revises this story; the acceptance below
  is not met with other names.
- `data/unicode/17.0.0/LICENSE`: the text of `https://www.unicode.org/license.txt` as fetched.
- `data/unicode/17.0.0/SOURCE`: exactly one line per other file of the directory, 17 lines,
  sorted by path, `<sha256> <path> <address>`, the path relative to `data/unicode/17.0.0/` and
  the address the file was fetched from. No comment lines.
- `data/.gitattributes`: `* -text`, so git never changes a byte of the data.
- `taskfiles/unicode-data.yml`, task `unicode-data-check`, variables `UNICODE_DATA_DIR` (default
  `data/unicode/17.0.0`) and `UNICODE_DATA_REQUIRED`, the 17 paths the directory must hold: the 14
  `ucd/` paths above, `emoji/emoji-sequences.txt`, `emoji/emoji-zwj-sequences.txt`, `LICENSE`.
  It fails, one line per fault, on: a required path that is not a file under `UNICODE_DATA_DIR`
  (`<path>: required, missing`); a regular file under `UNICODE_DATA_DIR` other than `SOURCE` without exactly
  one `SOURCE` line (`<path>: no SOURCE line` or `<path>: <k> SOURCE lines`); a `SOURCE` line
  without three fields (`SOURCE:<line>: not <sha256> <path> <address>`) or naming no file
  (`<path>: in SOURCE, missing`); a SHA-256 that differs (`<path>: sha256 <actual>, SOURCE
  gives <expected>`). Success: `<UNICODE_DATA_DIR>: <n> file(s), each with the SHA-256 its SOURCE line
  gives; <r> required path(s) present`.
- `tests/gate/unicode-data-check-refuses-a-missing-file.sh`: a copy of the directory under its
  sandbox without `ucd/Scripts.txt` and without its `SOURCE` line; the task with `UNICODE_DATA_DIR=<copy>`
  exits non-zero and prints `ucd/Scripts.txt: required, missing`.
- `tests/gate/unicode-data-check-refuses-a-changed-file.sh`: a copy with one byte of
  `ucd/CaseFolding.txt` changed; non-zero, the `sha256` line for that file only.
- `tests/gate/unicode-data-check-refuses-an-unlisted-file.sh`: a copy with one file added;
  non-zero, the `no SOURCE line` line for it.

Gate task (the coordinator adds it to `check` in `Taskfile.yml`): `- task: unicode-data-check`,
which prints the success line with `17 file(s)` and `17 required path(s) present`.

Writes: `data/unicode/17.0.0/` (new: `ucd/` with the 14 files, `emoji/` with the two, `LICENSE`,
`SOURCE`), `data/.gitattributes`, `taskfiles/unicode-data.yml`,
`tests/gate/unicode-data-check-refuses-a-missing-file.sh`,
`tests/gate/unicode-data-check-refuses-a-changed-file.sh`,
`tests/gate/unicode-data-check-refuses-an-unlisted-file.sh`.

## Acceptance

`slot task -d . -t taskfiles/unicode-data.yml unicode-data-check` exits 0 and prints
`data/unicode/17.0.0: 17 file(s), each with the SHA-256 its SOURCE line gives; 17 required path(s)
present`, the directory holding exactly these 17 files besides `SOURCE`: `ucd/UnicodeData.txt`,
`ucd/SpecialCasing.txt`, `ucd/CaseFolding.txt`, `ucd/CompositionExclusions.txt`,
`ucd/DerivedNormalizationProps.txt`, `ucd/PropertyAliases.txt`, `ucd/PropertyValueAliases.txt`,
`ucd/Scripts.txt`, `ucd/ScriptExtensions.txt`, `ucd/PropList.txt`, `ucd/DerivedCoreProperties.txt`,
`ucd/extracted/DerivedGeneralCategory.txt`, `ucd/extracted/DerivedBinaryProperties.txt`,
`ucd/emoji/emoji-data.txt`, `emoji/emoji-sequences.txt`, `emoji/emoji-zwj-sequences.txt`,
`LICENSE`, and `SOURCE` holding exactly 17 lines, one per file, each with its address under
`https://www.unicode.org/` and its SHA-256 (`wc -l < data/unicode/17.0.0/SOURCE` prints 17);
run in the worktree of this story, `sh tests/gate/<name>.sh` prints
`unicode-data-check-refuses-a-missing-file: ok`, `unicode-data-check-refuses-a-changed-file: ok`
and `unicode-data-check-refuses-an-unlisted-file: ok`, each with exit 0; and the report gives the
address each emoji sequence file came from and the first line of `LICENSE`.

## Out of scope

`crates/unicode-gen`, `crates/unicode` and every generated table: slice 5. All of `UCD.zip` beyond
the files listed. Unicode 18.0.0. `NOTICE` (the coordinator decides whether the vendored data needs
a line there). Any edit of the design: the coordinator writes what this story found into it.
