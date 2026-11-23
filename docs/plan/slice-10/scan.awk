# scan.awk: the failing cases of slice 10 by the story of slice 10 that is the first to hold every part a
# case needs, or "later <n>" for a part of slice <n>, delivered after slice 10.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-10/scan.awk <first failures>
# where <first failures> is "path<TAB>detail" of the first failing run of each failing case of slice 10:
#   awk -F"\t" '$2 == "10" && $5 != "Passed" && !($3 in s) { s[$3] = 1; print $3 "\t" $6 }' <runs.tsv> | LC_ALL=C sort
# The text scanned is the case and the harness files it includes but propertyHelper.js, with comments taken
# out. Output: path, key, evidence, the open stories of slice 7 the case also needs ("-" for none).
# Keys, a case taking the first that matches: later <n> (a refusal of a slice other than 7 and 10 in the
# first failure, or a token of a later slice in the text: class, super, Proxy, Reflect, Map, Set, a BigInt
# literal, binary data, Atomics, Iterator helpers, DisposableStack, await using, WeakRef,
# FinalizationRegistry, Date, Math but max, parseInt, parseFloat, import); then the flag of highest rank
# among statics (Promise.all, allSettled, any, race, withResolvers, try, .finally, AggregateError, or their
# directories), async-methods (an async method of an object literal), function-constructor (AsyncFunction
# by name, its directory, getPrototypeOf of an async function), async-generators (async function*, async *,
# for await, Symbol.asyncIterator, the async iteration objects, their directories) and from-async
# (Array.fromAsync), ranked in that order; a case with none takes async-functions when its text has async
# (any use of the word async) or await, or its directory is one of async functions, async arrows or await, and
# promise otherwise. Field 4 names yield-star (yield then *), methods (a generator method of an object
# literal) and function-constructor (GeneratorFunction by name): the stories of slice 7 open on the base.
# The lists of the stories, each sorted (LC_ALL=C sort), read from scan.tsv and meta.tsv:
#   story-promise.cases.txt, the key promise:
#     LC_ALL=C awk -F'\t' '$2 == "promise" { print $1 }' docs/plan/slice-10/scan.tsv
#   story-async-syntax.cases.txt, the key async-functions whose negative phase (field 4 of meta.tsv) is parse:
#     LC_ALL=C awk -F'\t' 'NR == FNR { if ($2 == "async-functions") w[$1] = 1; next } ($1 in w) && $4 == "parse" { print $1 }' docs/plan/slice-10/scan.tsv .cache/inventory/meta.tsv
#   story-async-functions.cases.txt, the rest of the key async-functions:
#     LC_ALL=C awk -F'\t' 'NR == FNR { if ($4 == "parse") neg[$1] = 1; next } $2 == "async-functions" && !($1 in neg) { print $1 }' .cache/inventory/meta.tsv docs/plan/slice-10/scan.tsv
#   guard-async-token.cases.txt, the cases with a passing run in the base run whose text has the word async or
#   await (the guard of the parser's change of story:async-function-syntax-and-early-errors):
#     LC_ALL=C awk -F'\t' '$5 == "Passed" { print $3 }' <runs.tsv> | LC_ALL=C sort -u | sed "s|^|<checkout>/|" | xargs grep -lE '\b(async|await)\b' | sed "s|^<checkout>/||" | LC_ALL=C sort
#   guard-promise.cases.txt, the cases whose every run passes in the base run under the directories of the
#   global object, the builtin function objects, the well-known symbols and @@toStringTag (the guard of
#   story:promise-constructor-then-and-jobs):
#     LC_ALL=C awk -F'\t' '{ n[$3]++; if ($5 == "Passed") p[$3]++ } END { for (k in n) if (p[k] == n[k]) print k }' <runs.tsv> | LC_ALL=C sort | LC_ALL=C awk '/^test\/(built-ins\/global|language\/global-code|built-ins\/Object\/prototype\/toString|built-ins\/Function\/prototype\/(bind|call|apply|toString)|built-ins\/Symbol|built-ins\/Object\/getOwnPropertyNames)\//'
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6] }
  nrank = split("promise async-functions statics async-methods function-constructor async-generators from-async", name, " ")
  for (i = 1; i <= nrank; i++) rank[name[i]] = i }
{ p = $1; d = $2; t = slurp(root "/" p); n = split(inc[p], a, ","); for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  k = ""; e = ""; c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  if (match(d, /not supported before slice [0-9]+/)) { s = substr(d, RSTART + 27, RLENGTH - 27)
    if (s + 0 != 7 && s + 0 != 10) { k = "later " s; e = substr(d, 1, 80) } }
  if (k == "" && match(c, /(\<class +[A-Za-z_$][A-Za-z0-9_$]* *(extends|\{)|\<class *\{|\<super\>|\<Proxy\>|\<Reflect\.|new (Weak)?(Map|Set)\>|\<(Weak)?(Map|Set)(\[|\.|\()|\<[0-9]+n\>|ArrayBuffer|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>|DataView|\<Date\>|\<Math\.[a-z0-9]+|parseInt|parseFloat|\<Atomics\>|\<WeakRef\>|FinalizationRegistry|\<Iterator\.(from|prototype\.[a-z])|DisposableStack|\<await +using\>|\<import *\(|\<import\.meta\>)/)) {
    x = substr(c, RSTART, RLENGTH)
    if (x != "Math.max") { s = (x ~ /class|super/) ? 8 : (x ~ /Proxy|Reflect|Map|Set/) ? 9 : (x ~ /^[0-9]+n$/) ? 12 : (x ~ /Atomics/) ? 14 : (x ~ /Iterator\.|Disposable|using/) ? 15 : (x ~ /WeakRef|Finalization/) ? 16 : (x ~ /import/) ? 11 : (x ~ /Date|Math|parse/) ? 4 : 13; k = "later " s; e = x } }
  s7 = ""
  if (c ~ /\<yield[ \t\n]*\*/) s7 = s7 "yield-star "
  if (c ~ /[{,][ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) s7 = s7 "methods "
  if (c ~ /\<GeneratorFunction\>/) s7 = s7 "function-constructor "
  if (s7 == "") s7 = "-"
  if (k == "") {
    best = 0; ev = ""
    if (c ~ /Promise\.(all|allSettled|any|race|withResolvers|try)\>|\.finally\>|\<AggregateError\>/ || p ~ /\/built-ins\/(Promise\/(all|allSettled|any|race|withResolvers|try|prototype\/finally)|AggregateError)\//) { ev = ev "statics "; if (rank["statics"] > best) best = rank["statics"] }
    if (c ~ /[{,][ \t\n]*async[ \t]+([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) { ev = ev "async-methods "; if (rank["async-methods"] > best) best = rank["async-methods"] }
    if (c ~ /\<AsyncFunction\>|getPrototypeOf\(async/ || p ~ /\/built-ins\/AsyncFunction\//) { ev = ev "function-constructor "; if (rank["function-constructor"] > best) best = rank["function-constructor"] }
    if (c ~ /\<async[ \t\n]+function[ \t\n]*\*|\<async[ \t\n]+\*|\<for[ \t\n]+await\>|Symbol\.asyncIterator|AsyncGenerator|AsyncIterator|AsyncFromSyncIterator/ || p ~ /\/(built-ins\/(AsyncGenerator[A-Za-z]*|AsyncIteratorPrototype|AsyncFromSyncIteratorPrototype)|statements\/for-await-of|expressions\/async-generator|statements\/async-generator)\//) { ev = ev "async-generators "; if (rank["async-generators"] > best) best = rank["async-generators"] }
    if (c ~ /Array\.fromAsync/ || p ~ /\/built-ins\/Array\/fromAsync\//) { ev = ev "from-async "; if (rank["from-async"] > best) best = rank["from-async"] }
    if (best == 0) {
      if (c ~ /\<async\>|\<await\>/ || p ~ /\/(expressions\/(async-function|async-arrow-function|await)|statements\/async-function)\//) { best = rank["async-functions"]; ev = "async function or await" }
      else { best = rank["promise"]; ev = "Promise and jobs" }
    }
    k = name[best]; e = ev
  }
  print p, k, e, s7 }
