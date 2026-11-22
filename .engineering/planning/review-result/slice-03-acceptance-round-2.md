---
format: aep.planning-md/3
id: review-result:slice-03-acceptance-round-2
kind: review-result
status: active
title: acceptance critic, slice 3 stories, round 2
relations:
- reviews: story:builtin-group-modules
- reviews: story:property-index-and-dense-elements
- reviews: story:object-integrity-and-prototype-methods
- reviews: story:object-enumeration-and-copy-methods
- reviews: story:function-apply-and-source-text
- reviews: story:global-this-uri-functions-and-is-error
- reviews: story:json-parse-and-raw-json
- reviews: story:array-iteration-methods
- reviews: story:array-search-and-copy-methods
- reviews: story:array-mutation-and-sort-methods
- reviews: story:string-access-and-search-methods
- reviews: story:string-transform-and-compare-methods
- reviews: story:slice-03-conformance-close
revision: 1
---
needs-revision
story:slice-03-conformance-close — the check "every case moved out of slice 3 has a failed run on the base, the cases of the seven directories excepted" prints `test/built-ins/Function/prototype/apply/this-not-callable.js`, a case that passes on the base and that `story:function-apply-and-source-text` hands to the close ("fails here on the refusal of slice 5 and moves at the close"), so the close can neither move it (that check prints it) nor leave it (the final `$2 == "03" && $5 != "Passed"` count stops being `0`); the same holds for any other case that passes on the base and ends on a later part after the wave, so the close must name an exception for it or the function story must stop sending it — .engineering/planning/story/slice-03-conformance-close.md:69, .engineering/planning/story/function-apply-and-source-text.md:50
story:builtin-group-modules — the acceptance requires `grep -rl 'reason = "story:' crates/swell/src` to print `crates/swell/src/builtins/string/mod.rs` and `crates/swell/src/ops/objects.rs`, but Change fixes the text of a reason only for a group file, and every mark on the base reads `reason = "<what>: story:..."` (`crates/swell/src/ops/testing.rs:74`, `crates/swell/src/object/mod.rs:259`), so a unit that follows Change and the repository's form fails its own check; the Change must say that a reason starts with `story:` — .engineering/planning/story/builtin-group-modules.md:94, .engineering/planning/story/builtin-group-modules.md:109

What I read: 14 artifacts, each in full with `aep plan artifact show` (the epic and the 13 stories), the four round-1 results, `docs/plan/slice-03/`, `taskfiles/test262.yml`, `taskfiles/host-requirements.yml` and the base `runs.tsv`. I ran every "prints N on the base" count of the ten wave 2 acceptances against the base `runs.tsv`. All match (36, 14, 154, 6, 34, 26, 18, 44, 30; 2,201, 252, 110, 250, 134, 2,512, 1,022, 391, 618, 542). The ten lists total 4,050 distinct cases, all failing on the base in slice 3. I also checked the floors against `expected.txt`, the 38 and 188 and 17 counts, the totals 44931 85794 and 53801 103360, the task and test names the acceptances use, and the three `pending slice 3` lines byte for byte. Scratch under `.out/critic-acceptance/` is removed.

Round-1 findings: all twelve of mine are met. The `.out` files are now in `docs/plan/slice-03/`, the probes are files of the story, and the own-property patterns are scoped by slice and directory. I also ran each of those patterns on the base for the cases they match outside their own list. Those still-failing runs (`flat/symbol-object-create-null-depth-throws.js`, `trimStart/this-value-symbol-typeerror.js` and the like) end on `Symbol` after the story, so they leave the pattern. The design, scope and parallel-safety findings are met too.

What I could not establish:
- The floors of the form "at least N passed" are the base passes plus the whole list, but each story lets the coordinator take cases off its list in the wave record. A floor then needs to follow the list. I did not file this as ten findings.
- I did not run `run.sh` of `story:property-index-and-dense-elements`: `Write` is not available to me, and the script cannot go through a heredoc. I read its options against `crates/test262/src/main.rs` (`--sections` and `--expected` are optional) and its `test262-time` and `test262-steps` parsing against the base `count.log`.
- Whether wave 2 stories actually remove the marks they name is not observable in their acceptances: `cargo test` here does not deny warnings, so an unfulfilled `expect(dead_code)` shows only in the wave's `slot task check`. This is not filed.
- Out of my lane: the three requirement lines and four `omit` lines moved to the close (item 3a), and the shared mark lines of `ops/objects.rs` and `builtins/string/mod.rs`, are parallel-safety and design calls. Judged only as acceptance, the copy checks of the three stories and the close's `grep -c` checks are observable, and none depends on the omissions.

```findings
- file: .engineering/planning/story/slice-03-conformance-close.md
  line: 69
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the check \"every case moved out of slice 3 has a failed run on the base, the cases of the seven directories excepted\" prints test/built-ins/Function/prototype/apply/this-not-callable.js, a case that passes on the base and that story:function-apply-and-source-text hands to the close to move, so the close can neither move it (that check prints it) nor leave it (the final count of non-Passed runs of slice 3 stops being 0); the same holds for any case that passes on the base and ends on a later part after the wave, so the close must name an exception for it or the function story must stop sending it"
- file: .engineering/planning/story/builtin-group-modules.md
  line: 109
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires grep -rl 'reason = \"story:' crates/swell/src to print builtins/string/mod.rs and ops/objects.rs, but Change fixes the text of a reason only for a group file and every mark on the base reads reason = \"<what>: story:...\" (ops/testing.rs:74, object/mod.rs:259), so a unit that follows Change and the repository's form fails its own check; Change must say that a reason starts with story:"
```
