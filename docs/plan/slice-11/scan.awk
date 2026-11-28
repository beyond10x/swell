# scan.awk: every case of slice 11 by the story of slice 11 that is the first, in the order of the edges, to hold
# every part of slice 11 the case needs ("joint" when no story holds them all, none on f962c324); "later <n>" for a
# failing case that needs a part of slice <n>, delivered after slice 11; a case with no part goes to the first
# story, module-syntax-and-early-errors, whose Module goal and await rules it guards.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -f docs/plan/slice-11/scan.awk <cases>
# where <cases> is "path<TAB>detail" of each case of slice 11, the detail the cause of its first failing run or
# "passed" when every run passes, from the run lines of a full run (taskfiles/test262.yml, "test262-run"):
#   LC_ALL=C awk -F'\t' '$2 == "11" { if (!($3 in s)) { s[$3] = "passed"; o[++n] = $3 } if ($5 != "Passed" && s[$3] == "passed") s[$3] = $6 } END { for (i = 1; i <= n; i++) print o[i] "\t" s[o[i]] }' <runs.tsv> | LC_ALL=C sort
# The front matter of the case (flags, features, negative phase) is read from the case itself. The text scanned is
# the code of the case after its front matter and the code of each module it names by a string literal that starts
# with "./" (one level: a fixture's own imports are not followed), with comments taken out and the contents of
# quoted strings and template literals blanked after the names were read.
# Parts, the work of one story each:
#   msyn   the module goal of the parser: the flag module, or a script case whose negative phase is parse and whose
#          code holds an import or an export declaration (a word import or export at the start of a statement,
#          not followed by "(" or ".")
#   dsyn   the grammar of ImportCall and import.meta: the features dynamic-import and import.meta, or a token
#          import( or import.meta in the code
#   mrun   a module record, its loading and the runner's module run: the flag module
#   mlink  linking and synchronous evaluation: the flag module and a negative phase other than parse
#   ns     module namespace objects: a module case of mlink with import * as or export * as in its code or in a
#          fixture it names, the directory module-code/namespace, or the feature
#          export-star-as-namespace-from-module
#   tla    top-level await and the asynchronous evaluation of modules: a module case of mlink with the feature
#          top-level-await
#   meta   import.meta evaluated: a module case of mlink with the feature import.meta or a token import.meta in its
#          code or in a fixture it names
#   json   JSON modules: the feature json-modules, or a specifier ending in .json
#   dimp   ImportCall evaluated: a case whose negative phase is not parse with the feature dynamic-import or a token
#          import( in its code or in a fixture it names
# Keys, the stories in the order of their edges (a story holds the parts of the stories it depends on); the key
# printed is the name of the story, the part beside it is the part the story brings:
#   msyn   story:module-syntax-and-early-errors                holds msyn
#   dsyn   story:import-call-and-import-meta-syntax            holds msyn dsyn
#   mrun   story:module-records-loading-and-module-runs        holds msyn dsyn mrun
#   mlink  story:module-linking-and-evaluation                 holds msyn dsyn mrun mlink
#   ns     story:module-namespace-objects                      holds msyn dsyn mrun mlink ns
#   tla    story:top-level-await                               holds msyn dsyn mrun mlink tla
#   meta   story:import-meta                                   holds msyn dsyn mrun mlink meta
#   json   story:json-modules                                  holds msyn dsyn mrun mlink json
#   dimp   story:dynamic-import                                holds msyn dsyn mrun mlink ns tla meta json dimp
# story:modules-design-text, story:module-code-compiles, story:ess-target-module-commands,
# story:module-determinism-and-emulated-loading and story:cli-module-loader have no case of test262;
# story:slice-11-conformance-close has close-later.tsv.
# A case takes the key of the first story, from the highest rank among its parts on in the order above, whose
# closure holds every part the case needs (json-via-namespace.js needs ns and json: dynamic-import); when none
# does, the key is "joint" and field 5 names the parts (no such case on f962c324).
# Later parts, "later <n>", for a case with a failing run only: a refusal of a slice delivered after 11 in its first
# failure, or a token of a later slice in its text: a BigInt literal or BigInt (12), binary data (13), Atomics or
# SharedArrayBuffer (14), Iterator helpers or DisposableStack (15), WeakRef, FinalizationRegistry or $262.gc (16),
# __defineGetter__, __defineSetter__, __lookupGetter__ or __lookupSetter__ (18; not __proto__: the three cases of
# slice 11 that read ns.__proto__ read the [[Get]] of a namespace object, which reaches no prototype). The slice is
# the one delivered last among those found (the order 11, 12, ...). No case of slice 11 has a later part in the
# run on f962c324.
# Output, tab separated: path, key, evidence (the parts with what gave each), later parts ("-" for none), the parts
# of the case ("-" for none), "passed" or "failed".
# scan.tsv is this output over the 1,281 cases of slice 11 in the full run on f962c324 (its line
# "test262-slice 11 1281 1833 0 0 1281 23 1258 0" equals the line of conformance/test262/expected.txt at f962c324).
# The lists, each sorted (LC_ALL=C sort), read from scan.tsv; lists.sh checks that they partition the slice:
#   story-<key>.cases.txt, for each key above (the name of the story):
#     LC_ALL=C awk -F'\t' '$2 == "<key>" { print $1 }' docs/plan/slice-11/scan.tsv
#   close-later.tsv, path, the later slice and the later parts, the cases the close moves or checks:
#     LC_ALL=C awk -F'\t' '$2 ~ /^later / { s = $2; sub(/^later /, "", s); print $1 "\t" s "\t" $4 }' docs/plan/slice-11/scan.tsv
# The guard lists, cases outside slice 11 whose every run passes in the full run on f962c324 (37,074 cases), each a
# set of cases a story's part must not break; with <passing> the output of
#   LC_ALL=C awk -F'\t' '$2 != "11" { n[$3]++; if ($5 == "Passed") p[$3]++ } END { for (k in n) if (p[k] == n[k]) print k }' <runs.tsv> | LC_ALL=C sort
#   guard-syntax.cases.txt (985), the Script goal the parser's new goal and holds must not change:
#     grep -E '^test/language/(reserved-words|comments|identifiers|keywords|white-space|line-terminators|expressions/(await|call|new|assignmenttargettype)|statements/(labeled|expression))/' <passing>
#   guard-async.cases.txt (298), promises, await and async functions, which a module run's checkpoint must not change:
#     grep -E '^test/(built-ins/Promise/(all|resolve|prototype/then)|language/(expressions/await|statements/async-function))/' <passing>
#   guard-internals.cases.txt (764), the readers of the internal methods a new object kind gets an arm in:
#     grep -E '^test/(built-ins/Object/(keys|getOwnPropertyNames|getOwnPropertyDescriptor|isFrozen|isSealed|prototype/toString)|built-ins/Reflect/(ownKeys|getOwnPropertyDescriptor|defineProperty|deleteProperty|get|set|has|isExtensible|preventExtensions)|language/statements/for-in)/' <passing>
#   guard-json.cases.txt (77), the JSON parser a JSON module reuses:
#     grep -E '^test/built-ins/JSON/parse/' <passing>
# Owner: the decomposition of slice 11 (docs/plan/slice-11/); a unit reads it and does not change it. POSIX awk with
# the word boundaries of gawk (\< and \>), as docs/plan/slice-09/scan.awk.
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
function note(part, ev) { if (!(part in has)) { has[part] = 1; evid = evid part " " ev ";" } }
function listval(rest) { sub(/^[^\[]*\[/, "", rest); sub(/\].*$/, "", rest); gsub(/[ \t]/, "", rest); return rest }
# Reads the front matter of text t into fm_flags, fm_features and fm_phase, and answers the code after it.
function front(t,   start, stop, head, n, ln, i, key, rest, lkey) {
  fm_flags = ""; fm_features = ""; fm_phase = ""
  start = index(t, "/*---"); stop = index(t, "---*/")
  if (start == 0 || stop == 0) return t
  head = substr(t, start + 5, stop - start - 5)
  n = split(head, ln, "\n"); lkey = ""
  for (i = 1; i <= n; i++) {
    if (match(ln[i], /^[a-zA-Z_]+:/)) {
      key = substr(ln[i], 1, RLENGTH - 1); rest = substr(ln[i], RLENGTH + 1); gsub(/^[ \t]+|[ \t]+$/, "", rest); lkey = ""
      if (key == "flags" || key == "features") {
        if (rest ~ /\[/) { if (key == "flags") fm_flags = listval(rest); else fm_features = listval(rest) }
        else if (rest == "") lkey = key
      } else if (key == "negative") lkey = "negative"
      continue }
    if (lkey == "negative" && match(ln[i], /^[ \t]+phase:/)) { rest = substr(ln[i], RLENGTH + 1); gsub(/[ \t]/, "", rest); fm_phase = rest; continue }
    if ((lkey == "flags" || lkey == "features") && match(ln[i], /^[ \t]*-[ \t]*/)) {
      rest = substr(ln[i], RLENGTH + 1); gsub(/[ \t]/, "", rest)
      if (lkey == "flags") fm_flags = fm_flags (fm_flags == "" ? "" : ",") rest
      else fm_features = fm_features (fm_features == "" ? "" : ",") rest } }
  return substr(t, stop + 5) }
# The code of text t with comments taken out and strings blanked.
function code(t,   c) {
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  gsub(/"([^"\\\n]|\\.)*"/, "\"\"", c); gsub(/\047([^\047\\\n]|\\.)*\047/, "\047\047", c); gsub(/`[^`]*`/, "``", c)
  return c }
BEGIN { FS = "\t"; OFS = "\t"
  nrank = split("msyn dsyn mrun mlink ns tla meta json dimp", name, " ")
  nl = split("module-syntax-and-early-errors import-call-and-import-meta-syntax module-records-loading-and-module-runs module-linking-and-evaluation module-namespace-objects top-level-await import-meta json-modules dynamic-import", slug, " ")
  for (i = 1; i <= nl; i++) label[name[i]] = slug[i]
  for (i = 1; i <= nrank; i++) rank[name[i]] = i
  holds["msyn"] = "msyn"
  holds["dsyn"] = "msyn dsyn"
  holds["mrun"] = "msyn dsyn mrun"
  holds["mlink"] = "msyn dsyn mrun mlink"
  holds["ns"] = "msyn dsyn mrun mlink ns"
  holds["tla"] = "msyn dsyn mrun mlink tla"
  holds["meta"] = "msyn dsyn mrun mlink meta"
  holds["json"] = "msyn dsyn mrun mlink json"
  holds["dimp"] = "msyn dsyn mrun mlink ns tla meta json dimp" }
{ p = $1; failed = ($2 != "passed"); delete has; evid = ""
  body = front(slurp(root "/" p)); flags = "," fm_flags ","; f = "," fm_features ","; phase = fm_phase
  module = (flags ~ /,module,/); c = code(body)
  # The modules the case names by a string literal "./...", read before the strings are blanked; one level.
  dir = p; sub(/\/[^\/]*$/, "", dir); fx = ""; jsonspec = 0; s = body
  while (match(s, /["\047]\.\/[^"\047\n]+["\047]/)) {
    spec = substr(s, RSTART + 3, RLENGTH - 4); s = substr(s, RSTART + RLENGTH)
    if (spec ~ /\.json$/) jsonspec = 1
    else if (!((dir "/" spec) in seen)) { seen[dir "/" spec] = 1; fx = fx "\n" code(slurp(root "/" dir "/" spec)) } }
  delete seen
  cf = c fx
  if (module) note("msyn", "flag module")
  else if (phase == "parse" && c ~ /(^|[;{}\n])[ \t]*(import|export)\>[ \t]*[^ \t(.]/) note("msyn", "declaration in a script")
  if (f ~ /,(dynamic-import|import\.meta),/) note("dsyn", "feature")
  if (c ~ /\<import[ \t\n]*(\(|\.[ \t\n]*meta\>)/) note("dsyn", "import( or import.meta")
  if (module) note("mrun", "flag module")
  if (module && phase != "parse") {
    note("mlink", "flag module, phase " (phase == "" ? "none" : phase))
    if (cf ~ /\<(import|export)[ \t\n]*\*[ \t\n]*as\>/) note("ns", "* as")
    if (p ~ /^test\/language\/module-code\/namespace\//) note("ns", "dir namespace")
    if (f ~ /,export-star-as-namespace-from-module,/) note("ns", "feature")
    if (f ~ /,top-level-await,/) note("tla", "feature")
    if (f ~ /,import\.meta,/) note("meta", "feature")
    if (cf ~ /\<import[ \t\n]*\.[ \t\n]*meta\>/) note("meta", "import.meta") }
  if (f ~ /,json-modules,/) note("json", "feature")
  if (jsonspec) note("json", "specifier .json")
  if (phase != "parse") {
    if (f ~ /,dynamic-import,/) note("dimp", "feature")
    if (cf ~ /\<import[ \t\n]*\(/) note("dimp", "import(") }
  late = 0; later = ""
  if (failed) {
    if (match($2, /not supported before slice [0-9]+/)) { sl = substr($2, RSTART + 27, RLENGTH - 27) + 0; if (sl > 11) { later = later "later " sl " (refusal) "; if (sl > late) late = sl } }
    if (cf ~ /\<[0-9][0-9_]*n\>|\<BigInt\>/) { later = later "later 12 (BigInt) "; if (12 > late) late = 12 }
    if (cf ~ /ArrayBuffer|DataView|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>/) { later = later "later 13 (binary data) "; if (13 > late) late = 13 }
    if (cf ~ /\<(Atomics|SharedArrayBuffer)\>/) { later = later "later 14 (Atomics) "; if (14 > late) late = 14 }
    if (cf ~ /\<Iterator\.|DisposableStack/) { later = later "later 15 (Iterator or disposable) "; if (15 > late) late = 15 }
    if (cf ~ /\<WeakRef\>|FinalizationRegistry|\$262\.gc/) { later = later "later 16 (WeakRef) "; if (16 > late) late = 16 }
    if (cf ~ /__defineGetter__|__defineSetter__|__lookupGetter__|__lookupSetter__/) { later = later "later 18 (annex B) "; if (18 > late) late = 18 } }
  best = 0; for (k in has) if (rank[k] > best) best = rank[k]
  parts = ""; for (i = 1; i <= nrank; i++) if (name[i] in has) parts = parts (parts == "" ? "" : ",") name[i]
  if (late > 0) key = "later " late
  else if (best == 0) { key = name[1]; evid = "no part: " substr($2, 1, 70) }
  else { key = ""
    for (j = best; j <= nrank && key == ""; j++) {
      split(holds[name[j]], hl, " "); delete held; for (i in hl) held[hl[i]] = 1
      ok = 1; for (k in has) if (!(k in held)) ok = 0
      if (ok) key = name[j] }
    if (key == "") key = "joint" }
  if (key in label) key = label[key]
  print p, key, evid, (later == "" ? "-" : later), (parts == "" ? "-" : parts), (failed ? "failed" : "passed") }
