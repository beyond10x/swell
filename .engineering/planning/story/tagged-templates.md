---
format: aep.planning-md/3
id: story:tagged-templates
kind: story
status: draft
title: 'Tagged templates: GetTemplateObject and the template objects a realm keeps per site'
summary: 50 failing cases of slice 6 on its list; waits for the design of where template objects are kept and charged.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:array-destructuring-and-spread
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/interp/templates.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/tests/language/tagged-template-identity.js
- confidence: cited
  path: crates/swell/tests/language/tagged-template-raw.js
- confidence: cited
  path: crates/swell/tests/tagged_templates.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lexer.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/template-literal.js
revision: 2
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`). None of the 191 is a tagged template; this story serves the cases of test262 and `String.raw` (`story:string-access-and-search-methods` of slice 3 delivers the function; its cases call it with a tag). After `story:template-literals` the parser refuses a template after a MemberExpression, a CallExpression or `new` as `tagged template is not supported before slice 6`; on `main` (`6a43e18`) every template is refused by the lexer as `template literal` (`crates/syntax/src/lexer.rs` line 259). The fixture `crates/syntax/tests/sources/refused/template-literal.js`, as `story:template-literals` leaves it for a tagged template, is removed.

The list of this story is `docs/plan/slice-06/story-tagged-templates.cases.txt`: 50 failing cases (96 runs) of slice 6 and of the six directories `story:slice-03-conformance-close` moves to slice 6 (`test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`, `entries/`, `Object/fromEntries/`, `Object/groupBy/`), on no list of the first five stories of the slice, whose text and harness files use a tagged template, no other part of slice 6 than those of the stories this one depends on, and no part of a later slice. Found by the token scan `docs/plan/slice-06/scan.awk` (run as `story:let-const-and-block-scopes` gives it, but with a copy of `meta.tsv` whose includes leave out `propertyHelper.js`, whose three later tokens are a branch behind `typeof`, a case of a bigint value and the name `checkSet`; the comment at the head of `lists-b.awk` gives the command; over the first failing run of each failing case of slice 6 and of those six directories on `main`, `6a43e18`) and the assignment `docs/plan/slice-06/lists-b.awk` (`gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-06/lists-b.awk <scan output>`, the cases of the five lists of the first part taken out), which puts a case on the list of the first story of the slice that holds every part the case needs and whose own construct the case uses. By directory: `language/expressions/tagged-template` 23, `language/expressions/template-literal` 16, `language/expressions/optional-chaining` and `language/expressions/assignmenttargettype` 4 each, `built-ins/String/raw` 3. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, a precondition (named by the decomposition for a design author, before this story starts): `docs/design/engine.md`, decision 3 and the section on the heap: where the [[TemplateMap]] of a realm (`#sec-gettemplateobject`) is kept and charged: the key of a site (the compiled code of a script or function and the index of the site in it, so that one site gives one object for the life of the realm and two sites with the same text give two), the record that holds the objects (on the realm record, traced by the collector, `crates/swell/src/realm/mod.rs` line 75), when an entry is freed (with the realm, or with the compiled code if the design lets code be freed earlier), and the charge of the template array, its `raw` array and the strings in `heap_bytes()` (`crates/swell/src/host/agent.rs` line 211) when GetTemplateObject first makes them.

## Domain relations

- The realm record holds a template object per template site of the code it ran (`#sec-gettemplateobject`, [[TemplateMap]]); the relation is added to `docs/design/engine.md` by the design change above, not by this story.

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses a tagged template).

- `swell-syntax`. `parser/expression.rs`: a TemplateLiteral after a MemberExpression, a CallExpression and `new` MemberExpression (`#sec-tagged-templates`, MemberExpression TemplateLiteral and CallExpression TemplateLiteral); an OptionalChain followed by a template stays the early error `story:template-literals` gives it; `lexer.rs`: in a tagged template a NotEscapeSequence is allowed and its cooked value is `undefined` (`#sec-static-semantics-template-early-errors` applies to untagged templates only, `#sec-static-semantics-tv`). `ast.rs`: the tagged template (a variant of `Expr` with the tag, the cooked strings, each `None` for a NotEscapeSequence, the raw strings and the substitutions), with its arm in `Expr::children` (line 511, no wildcard). `early.rs`: the arm of the tagged template where the rules of `Expr` read it. `crates/syntax/tests/sources.rs`: the kind `tagged_template` (`t`${t`${ ... }`}``) added to `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` in the form the file has on the base of the unit (on `wave/s3-1` an arm of `nested`, line 417, and an entry of the array at line 462); the unit times `cargo nextest run --locked -p swell-syntax --test sources` before and after its change (29.6 s on `wave/s3-1`; `story:gate-test-steps-within-budget` changes how the binary runs) and reports both times. Fixtures `crates/syntax/tests/sources/valid/tagged-template-*.js` (new; one with a NotEscapeSequence) and `early-errors/tagged-template-optional-chain.js`; the refused fixture removed.
- `swell-compiler`. `expression.rs`: the evaluation of a tagged template (`#sec-tagged-templates-runtime-semantics-evaluation`): the tag and its `this` as for a call (a member gives its base, `#sec-evaluatecall`), then GetTemplateObject of the site, then the substitutions in order (ArgumentListEvaluation of a TemplateLiteral, `#sec-template-literals-runtime-semantics-argumentlistevaluation`), then the call; the site index of each template in the code it compiles to; `resolve.rs`: the arm in the match over `Expr` (line 498); compiler `expression.rs`: the arm in the match over `Expr` (line 210); `bytecode.rs`: the instruction that pushes the template object of a site, documented with its stack effect and its arm in `Instruction::target` (line 339, no wildcard), and the site table of the compiled code; `disasm.rs`: its text and the site table. Fixtures `crates/compiler/tests/sources/tagged-template-*.js` and `.disasm` (new).
- `swell`. `interp/templates.rs` (new): GetTemplateObject in a function of its own (`#sec-gettemplateobject`: the template array and its `raw` array, their `length`, the cooked and raw strings as own properties not writable, not configurable, `raw` not enumerable, both frozen by SetIntegrityLevel of `crates/swell/src/ops/objects.rs` line 234), the lookup and insert in the record of the design, called from one arm of `interp/dispatch.rs` (line 237, no wildcard); `interp/mod.rs`: the `mod` line; `realm/mod.rs`: the record of template objects of the design and its trace. Nothing added to the frame of `Agent::execute`; GetTemplateObject charges one step and the bytes of the design.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/tagged_templates.rs` (new): `a_site_gives_the_same_template_object_on_each_evaluation`, `two_sites_with_the_same_text_give_two_objects`, `the_template_object_and_its_raw_are_frozen_with_the_attributes_of_the_spec`, `cooked_is_undefined_for_a_not_escape_sequence_and_raw_keeps_its_text`, `a_member_tag_is_called_with_its_base_as_this`, `the_substitutions_are_evaluated_after_the_tag_and_in_order`, `a_template_object_is_charged_once_per_site` (`heap_bytes()` after a script that evaluates one site 1,000 times in a loop minus that after the same script with 1 iteration, both measured after `collect`, is less than the bytes of one template object), `a_tag_re_enters_on_the_promised_stack` (`reach_through` of `tests/support/stack.rs`, on seven eighths of `min_stack_bytes()`); `crates/swell/tests/language/` (new files): `tagged-template-identity.js`, `tagged-template-raw.js` (over `String.raw`).
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the test steps run a process per test); no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Run loop: GetTemplateObject does its work in a function of its own in `interp/templates.rs`, called from one arm of `interp/dispatch.rs`; the call of the tag goes through the path of a call; nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts.rs`, `budgets.rs`, `eval_stack.rs`) do not change and pass on seven eighths of `min_stack_bytes()`; a path that does not fit is taken apart and reported with its bytes per re-entry before and after.

Writes: `crates/syntax/src/{ast,early,lexer}.rs`, `crates/syntax/src/parser/expression.rs`, `crates/syntax/tests/sources.rs` (one nesting kind), `crates/syntax/tests/sources/valid/tagged-template-*.js` and `early-errors/tagged-template-optional-chain.js` (new), `crates/syntax/tests/sources/refused/template-literal.js` (removed), `crates/compiler/src/{expression,resolve,bytecode,disasm}.rs`, `crates/compiler/tests/sources/tagged-template-*.js` and `.disasm` (new), `crates/swell/src/interp/templates.rs` (new), `crates/swell/src/interp/{mod,dispatch}.rs`, `crates/swell/src/realm/mod.rs`, `crates/swell/tests/tagged_templates.rs` (new), `crates/swell/tests/language/{tagged-template-identity,tagged-template-raw}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges), the list files under `docs/plan/slice-06/` (the coordinator), `docs/design/engine.md` (the design change lands before this story). The matches with no wildcard this story adds arms to are all in files above: `Expr` (`ast.rs` line 511, `resolve.rs` line 498, compiler `expression.rs` line 210), `Instruction` (`bytecode.rs` line 339, `dispatch.rs` line 237). Shared: none with a story that may run beside it; it lands after `story:array-destructuring-and-spread`, which writes the same files of `crates/syntax` and `crates/compiler`.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test tagged_templates --test language --test limits_scripts --test budgets --test eval_stack` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-tagged-templates.cases.txt" > .out/story-tagged-templates.cases.txt` (`WAVE` the wave branch the unit merges into; the coordinator takes a case off the list by a commit there whose body names the part it ends on), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-tagged-templates.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 96 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/expressions\/tagged-template\// && $6 ~ /(template literal|tagged template) is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `42`), (c) the lines `test262 language/expressions/tagged-template` and `test262 language/expressions/template-literal` of `.out/test262/lines.txt` each read at least B + L - R passed in the seventh field, where B is the passes of that line on the base of the unit (0 and 0 on `6a43e18`), L the cases of the copy under that directory (23 and 16 on the list of today) and R the cases under it among the runs (a) prints; it is a bound and not the count because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

Template objects of code that is freed before its realm, if the design keeps them with the realm (then they live as long as the realm); the moves of cases and the slice lines of the design (`story:slice-06-conformance-close`).
