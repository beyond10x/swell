# scan-later.awk: the cases of the later keys of scan.tsv by the story of the later waves of slice 10 that holds them.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-10/scan-later.awk docs/plan/slice-10/scan.tsv
# Reads the case and the harness files it includes but propertyHelper.js, with comments taken out, as scan.awk does.
# Output: path, story key, what else the case needs ("-" for nothing): async-functions (the word async or await in a
# case of the key statics or from-async: story:async-functions-and-await), and the field 4 of scan.tsv.
# Story keys, from the key of scan.tsv (field 2):
#   statics -> statics; async-methods -> async-methods; function-constructor -> function-constructor;
#   from-async -> from-async; later <n> -> later <n> (the close);
#   async-generators -> the first that matches: for a negative parse case (field 4 of meta.tsv is parse), ag-methods (an
#   async generator method, the pattern of ag-methods below), ag-yield-star (an async generator and yield then * on one
#   line: the 36 for-await-of/async-func-dstr-* cases, whose parse reaches a yield* in an async function*),
#   ag-functions (an async generator and for await in its text), ag-syntax (an async generator without for await),
#   for-await otherwise; for the other cases,
#   ag-constructor (AsyncGeneratorFunction by name or its directory, or .constructor of an async generator function,
#   the function agctor below: 2 cases), ag-yield-star (yield then * in the text; ag-cross
#   when an async generator method is in it too, a case the conformance close checks),
#   ag-methods (an async generator method of an object literal: async then * then a property name then "("),
#   ag-functions (async function* or async * in the text, or the directories of async generators and of
#   %AsyncGeneratorPrototype%), and for-await otherwise (for await, Symbol.asyncIterator, %AsyncIteratorPrototype%,
#   %AsyncFromSyncIteratorPrototype% with no async generator).
# The lists of the later stories, each sorted, read from its output (later.tsv):
#   story-<key>.cases.txt: LC_ALL=C awk -F'\t' '$2 == "<key>" { print $1 }' docs/plan/slice-10/later.tsv
#   but story-from-async.cases.txt, which leaves out the 2 cases that need yield* in an async generator:
#     LC_ALL=C awk -F'\t' '$2 == "from-async" && $3 !~ /yield-star/ { print $1 }' docs/plan/slice-10/later.tsv
#   close-cross.cases.txt, those 2 cases and the key ag-cross, which the conformance close of slice 10 checks:
#     LC_ALL=C awk -F'\t' '($2 == "from-async" && $3 ~ /yield-star/) || $2 == "ag-cross" { print $1 }' docs/plan/slice-10/later.tsv | LC_ALL=C sort
#   close-later.tsv, the cases of a later slice: LC_ALL=C awk -F'\t' '$2 ~ /^later / { print $1 "\t" $2 }' docs/plan/slice-10/later.tsv
# agctor: the case reaches %AsyncGeneratorFunction% through .constructor of an async generator function, written inline
# ((async function * () {}).constructor) or by the name of an async generator declaration (f.constructor).
function agctor(c,   s, m) { if (c ~ /\<async[ \t\n]*function[ \t\n]*\*[ \t\n]*[A-Za-z0-9_$]*[ \t\n]*\([^)]*\)[ \t\n]*\{[ \t\n]*\}[ \t\n]*\)[ \t\n]*\.[ \t\n]*constructor\>/) return 1
  s = c; while (match(s, /\<async[ \t\n]+function[ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*)/, m)) { if (c ~ ("(^|[^A-Za-z0-9_$.])" m[1] "[ \t\n]*\\.[ \t\n]*constructor\\>")) return 1; s = substr(s, RSTART + RLENGTH) }
  return 0 }
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"; while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6]; neg[m[1]] = m[4] } }
$2 == "promise" || $2 == "async-functions" { next }
{ p = $1; k = $2; s7 = $4; t = slurp(root "/" p); n = split(inc[p], a, ","); for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  need = ""
  if (k == "async-generators") {
    if (neg[p] == "parse") {
      if (c ~ /[{,][ \t\n]*async[ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) k = "ag-methods"
      else if (c ~ /\<async[ \t\n]+function[ \t\n]*\*|\<async[ \t\n]*\*/) k = (c ~ /\<yield[ \t]*\*/) ? "ag-yield-star" : (c ~ /\<for[ \t\n]+await\>/) ? "ag-functions" : "ag-syntax"
      else k = "for-await"
    }
    else if (c ~ /\<AsyncGeneratorFunction\>/ || p ~ /\/built-ins\/AsyncGeneratorFunction\// || agctor(c)) k = "ag-constructor"
    else if (c ~ /\<yield[ \t\n]*\*/) k = (c ~ /[{,][ \t\n]*async[ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) ? "ag-cross" : "ag-yield-star"
    else if (c ~ /[{,][ \t\n]*async[ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) k = "ag-methods"
    else if (c ~ /\<async[ \t\n]+function[ \t\n]*\*|\<async[ \t\n]*\*/ || p ~ /\/(built-ins\/AsyncGeneratorPrototype|built-ins\/AsyncGeneratorFunction|expressions\/async-generator|statements\/async-generator)\//) k = "ag-functions"
    else k = "for-await"
  }
  if ((k == "statics" || k == "from-async") && c ~ /\<async\>|\<await\>/) need = need "async-functions "
  if (s7 != "-") need = need s7
  if (need == "") need = "-"
  print p, k, need }
