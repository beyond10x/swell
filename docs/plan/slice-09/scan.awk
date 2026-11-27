# scan.awk: every case of slice 9 by the story of slice 9 that is the first to hold every part of slice 9 the
# case reads, "joint" when no one story holds them all, or "later <n>" for a failing case that needs a part of
# slice <n>, delivered after slice 9.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-09/scan.awk <cases>
# where <cases> is "path<TAB>detail" of each case of slice 9, the detail the cause of its first failing run or
# "passed" when every run passes, from the run lines of a full run (taskfiles/test262.yml, "test262-run"):
#   awk -F'\t' '$2 == "09" { if (!($3 in s)) { s[$3] = "passed"; o[++n] = $3 } if ($5 != "Passed" && s[$3] == "passed") s[$3] = $6 } END { for (i = 1; i <= n; i++) print o[i] "\t" s[o[i]] }' <runs.tsv> | LC_ALL=C sort
# The text scanned is the case and the harness files it includes, with comments taken out and the contents of
# quoted strings and template literals blanked; propertyHelper.js, compareArray.js, nativeFunctionMatcher.js, asyncHelpers.js and
# wellKnownIntrinsicObjects.js are not scanned (their names of later builtins are strings or guarded by typeof),
# deepEqual.js counts as the part map-and-set (it calls new Map() on every comparison of two objects) and
# proxyTrapsHelper.js as the part proxy-traps-of-shape (its handler defines every trap).
# Parts, each read from the directory of the case, its features (field 3 of meta.tsv) and tokens of its text:
#   reflect           the Reflect namespace: Reflect, the directory Reflect, the features Reflect, Reflect.construct,
#                     Reflect.set, Reflect.setPrototypeOf (isConstructor.js calls Reflect.construct)
#   map-and-set       Map and Set with their iterators: Map or Set as a word, the directories Map, Set,
#                     MapIteratorPrototype, SetIteratorPrototype, the features Map, Set, deepEqual.js
#   weak              WeakMap and WeakSet: either as a word, their directories, the features WeakMap, WeakSet,
#                     symbols-as-weakmap-keys, and the feature upsert under the directory WeakMap
#   set-methods       Set.prototype union, intersection, difference, symmetricDifference, isSubsetOf, isSupersetOf,
#                     isDisjointFrom: their directories, the feature set-methods, a call .<name>(
#   map-upsert        Map.prototype.getOrInsert and getOrInsertComputed, Map.groupBy: their directories, the
#                     feature upsert outside the directory WeakMap, the token getOrInsert, Map.groupBy
#   proxy             Proxy: Proxy as a word, the directory Proxy, the features Proxy, proxy-missing-checks
#   proxy-traps-of-shape  the traps getPrototypeOf, setPrototypeOf, isExtensible, preventExtensions,
#                     getOwnPropertyDescriptor, defineProperty, ownKeys of a Proxy: the directories of these traps
#                     under Proxy, one of these names followed by ":" or "(" and not preceded by "." in a case
#                     that has the part proxy, proxyTrapsHelper.js
# Keys, the stories of slice 9 in the order of their edges (a story holds the parts of the stories it depends on):
#   reflect               story:reflect-namespace                              holds reflect
#   map-and-set           story:map-and-set                                    holds reflect, map-and-set
#   weak                  story:weak-map-and-weak-set                          holds reflect, map-and-set, weak
#   set-methods           story:set-methods                                    holds reflect, map-and-set, set-methods
#   map-upsert            story:map-upsert-and-group-by                        holds reflect, map-and-set, weak,
#                                                                              map-upsert
#   proxy                 story:proxy-objects-and-access-traps                 holds reflect, proxy
#   proxy-traps-of-shape  story:proxy-descriptor-prototype-and-extensibility-traps  holds reflect, proxy,
#                                                                              proxy-traps-of-shape
# story:keyed-entry-table and story:host-object-overrides have no case of test262; story:slice-09-conformance-close
# has close-later.tsv.
# A case takes the key of the highest rank among its parts (the order above); when that story does not hold every
# part the case reads, the key is "joint" and field 5 names the parts (the conformance close checks these cases).
# Later parts, "later <n>", for a case with a failing run only: a refusal of slice 8 or of a slice above 10 in its
# first failure (a refusal of slice 4, which wave big-1 delivers before 9, is no later part), or a token of a
# later slice in its text: class or super (8), a BigInt literal or BigInt (12), binary data (13), Atomics (14),
# Iterator helpers or DisposableStack (15), WeakRef, FinalizationRegistry or $262.gc (16), import (11). The slice
# is the one delivered last among those found (the order 9, 8, 11, 12, ...).
# A case with no part and no later part takes the key "other" (the evidence is its first failure).
# Output, tab separated: path, key, evidence (the parts with the token that gave each), later parts ("-" for
# none), the parts of the case ("-" for none), "passed" or "failed".
# scan.tsv is this output over the 1,785 cases of slice 9 in a full run on the code of wave big-1 before the close
# of slice 4 (2026-11-26; its line "test262-slice 09 1785 3541 0 0 1785 267 1518 0" equals the line of
# conformance/test262/expected.txt at 62c7cf59) and over the 2 cases of incoming.tsv (path, the slice the case
# leaves, the token of slice 9 that the scan of docs/plan/slice-04/ found), which the close of slice 4 moves into
# slice 9, with their first failure in the same run: 1,787 lines.
# The lists, each sorted (LC_ALL=C sort), read from scan.tsv; lists.sh checks that they partition the slice:
#   story-<key>.cases.txt, for each key above:
#     LC_ALL=C awk -F'\t' '$2 == "<key>" { print $1 }' docs/plan/slice-09/scan.tsv
#   close-later.tsv, path, the later slice and the later parts, the cases the close moves or checks:
#     LC_ALL=C awk -F'\t' '$2 ~ /^later / { s = $2; sub(/^later /, "", s); print $1 "\t" s "\t" $4 }' docs/plan/slice-09/scan.tsv
# The guards, the cases outside slice 9 whose every run passes in that run (<passing>: LC_ALL=C awk -F'\t'
# '$2 != "09" { n[$3]++; if ($5 == "Passed") p[$3]++ } END { for (k in n) if (p[k] == n[k]) print k }' <runs.tsv>
# | LC_ALL=C sort), under the directories whose code the stories change:
#   guard-reflect.cases.txt (249), the operations Reflect reuses and the global object:
#     grep -E '^test/built-ins/(Object/(getPrototypeOf|setPrototypeOf|isExtensible|preventExtensions|keys)|Function/prototype/apply|global)/' <passing>
#   guard-collections.cases.txt (156), the iteration of entries, the object kinds and the global object:
#     grep -E '^test/built-ins/(Object/(fromEntries|groupBy|prototype/toString)|Array/from|ArrayIteratorPrototype|global|Symbol/species)/' <passing>
#   guard-proxy.cases.txt (1,081), the internal methods and the walks of the prototype chain:
#     grep -E '^test/(language/statements/(with|for-in)|language/expressions/(property-accessors|in|delete|instanceof)|built-ins/Array/isArray|built-ins/JSON/stringify|built-ins/Object/(keys|getPrototypeOf|setPrototypeOf|isExtensible|preventExtensions|getOwnPropertyDescriptor)|built-ins/Function/prototype/bind)/' <passing>
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
function note(part, ev) { if (!(part in has)) { has[part] = 1; evid = evid part " " ev ";" } }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6]; feat[m[1]] = m[3] }
  nrank = split("reflect map-and-set weak set-methods map-upsert proxy proxy-traps-of-shape", name, " ")
  for (i = 1; i <= nrank; i++) rank[name[i]] = i
  holds["reflect"] = "reflect"
  holds["map-and-set"] = "reflect map-and-set"
  holds["weak"] = "reflect map-and-set weak"
  holds["set-methods"] = "reflect map-and-set set-methods"
  holds["map-upsert"] = "reflect map-and-set weak map-upsert"
  holds["proxy"] = "reflect proxy"
  holds["proxy-traps-of-shape"] = "reflect proxy proxy-traps-of-shape"
  skip["propertyHelper.js"] = 1; skip["compareArray.js"] = 1; skip["nativeFunctionMatcher.js"] = 1
  skip["asyncHelpers.js"] = 1; skip["wellKnownIntrinsicObjects.js"] = 1; skip["deepEqual.js"] = 1
  skip["proxyTrapsHelper.js"] = 1 }
{ p = $1; failed = ($2 != "passed"); t = slurp(root "/" p); delete has; evid = ""
  n = split(inc[p], a, ",")
  for (i = 1; i <= n; i++) {
    if (a[i] == "") continue
    if (a[i] == "deepEqual.js") note("map-and-set", "deepEqual.js")
    if (a[i] == "proxyTrapsHelper.js") { pth = 1 } else pth = pth
    if (!(a[i] in skip)) t = t "\n" slurp(root "/harness/" a[i]) }
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  gsub(/"([^"\\\n]|\\.)*"/, "\"\"", c); gsub(/\047([^\047\\\n]|\\.)*\047/, "\047\047", c); gsub(/`[^`]*`/, "``", c)
  f = "," feat[p] ","
  if (p ~ /\/Reflect\//) note("reflect", "dir Reflect")
  if (f ~ /,Reflect(\.construct|\.set|\.setPrototypeOf)?,/) note("reflect", "feature")
  if (c ~ /\<Reflect\>/) note("reflect", "Reflect")
  if (p ~ /^test\/built-ins\/(Map|Set|MapIteratorPrototype|SetIteratorPrototype)\//) note("map-and-set", "dir")
  if (f ~ /,(Map|Set),/) note("map-and-set", "feature")
  if (c ~ /(^|[^A-Za-z0-9_$])(Map|Set)\>/) note("map-and-set", "Map or Set")
  if (p ~ /^test\/built-ins\/Weak(Map|Set)\//) note("weak", "dir")
  if (f ~ /,(WeakMap|WeakSet|symbols-as-weakmap-keys),/) note("weak", "feature")
  if (c ~ /\<Weak(Map|Set)\>/) note("weak", "WeakMap or WeakSet")
  if (p ~ /^test\/built-ins\/Set\/prototype\/(union|intersection|difference|symmetricDifference|isSubsetOf|isSupersetOf|isDisjointFrom)\//) note("set-methods", "dir")
  if (f ~ /,set-methods,/) note("set-methods", "feature")
  if (c ~ /\.(union|intersection|difference|symmetricDifference|isSubsetOf|isSupersetOf|isDisjointFrom)\(/) note("set-methods", "call")
  if (p ~ /^test\/built-ins\/Map\/(groupBy|prototype\/getOrInsert)/) note("map-upsert", "dir")
  if (f ~ /,upsert,/ && p !~ /^test\/built-ins\/WeakMap\//) note("map-upsert", "feature")
  if (f ~ /,upsert,/ && p ~ /^test\/built-ins\/WeakMap\//) note("weak", "feature upsert")
  if (p !~ /^test\/built-ins\/WeakMap\// && c ~ /getOrInsert|Map\.groupBy/) note("map-upsert", "getOrInsert or Map.groupBy")
  if (p ~ /^test\/built-ins\/Proxy\//) note("proxy", "dir")
  if (f ~ /,(Proxy|proxy-missing-checks),/) note("proxy", "feature")
  if (c ~ /\<Proxy\>/) note("proxy", "Proxy")
  if ("proxy" in has) {
    if (p ~ /^test\/built-ins\/Proxy\/(getPrototypeOf|setPrototypeOf|isExtensible|preventExtensions|getOwnPropertyDescriptor|defineProperty|ownKeys)\//) note("proxy-traps-of-shape", "dir")
    if (pth) note("proxy-traps-of-shape", "proxyTrapsHelper.js")
    if (c ~ /(^|[^.A-Za-z0-9_$])(getPrototypeOf|setPrototypeOf|isExtensible|preventExtensions|getOwnPropertyDescriptor|defineProperty|ownKeys) *(:|\()/) note("proxy-traps-of-shape", "trap") }
  pth = 0
  late = 0; later = ""
  if (failed) {
    if (match($2, /not supported before slice [0-9]+/)) { s = substr($2, RSTART + 27, RLENGTH - 27) + 0; if (s == 8 || s > 10) { later = later "later " s " (refusal) "; if (s > late) late = s } }
    if (c ~ /(\<class +[A-Za-z_$][A-Za-z0-9_$]* *(extends|\{)|\<class *\{|\<super\>)/) { later = later "later 8 (class or super) "; if (8 > late) late = 8 }
    if (c ~ /\<import *\(|\<import\.meta\>/) { later = later "later 11 (import) "; if (11 > late) late = 11 }
    if (c ~ /\<[0-9][0-9_]*n\>|\<BigInt\>/) { later = later "later 12 (BigInt) "; if (12 > late) late = 12 }
    if (c ~ /ArrayBuffer|DataView|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>/) { later = later "later 13 (binary data) "; if (13 > late) late = 13 }
    if (c ~ /\<Atomics\>/) { later = later "later 14 (Atomics) "; if (14 > late) late = 14 }
    if (c ~ /\<Iterator\.|DisposableStack/) { later = later "later 15 (Iterator or disposable) "; if (15 > late) late = 15 }
    if (c ~ /\<WeakRef\>|FinalizationRegistry|\$262\.gc/) { later = later "later 16 (WeakRef) "; if (16 > late) late = 16 } }
  best = 0; for (k in has) if (rank[k] > best) best = rank[k]
  parts = ""; for (i = 1; i <= nrank; i++) if (name[i] in has) parts = parts (parts == "" ? "" : ",") name[i]
  if (late > 0) key = "later " late
  else if (best == 0) { key = "other"; evid = substr($2, 1, 80) }
  else { key = name[best]; split(holds[key], hl, " "); delete held; for (i in hl) held[hl[i]] = 1
    for (k in has) if (!(k in held)) key = "joint" }
  print p, key, evid, (later == "" ? "-" : later), (parts == "" ? "-" : parts), (failed ? "failed" : "passed") }
