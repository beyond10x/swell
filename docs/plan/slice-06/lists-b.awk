# lists-b.awk: the failing cases of slice 6 (and of the six directories slice 3 moves here) by the
# story of slice 6 that is the first to hold every part of slice 6 a case needs, after scan.awk.
# Usage: gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f lists-b.awk <scan output>..., where the scan
# output is that of scan.awk run with a copy of meta.tsv whose includes leave out propertyHelper.js:
# awk 'BEGIN { FS = OFS = "\t" } { gsub(/(^|,)propertyHelper\.js/, "", $6); sub(/^,/, "", $6); print }' meta.tsv
# (its later tokens Uint8Array, BigInt and Set are a typeof-guarded branch, a bigint case and checkSet).
# Placed by hand after the run, in hand-placed.tsv (path, list, reason): 16 cases the scan marked as of a later
# slice only for a later name in a string or a local ("Array Iterator", OArray). The keys, values and entries
# of Object are not iterators: Object.keys( and the like are taken out before the test for ITER.
# Output: path, story key (a story, "later" for a part of a later slice, "close" when no story
# holds the needs), needs, detail. The keys of the first five stories are those of lists.awk.
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
function add(k, list,   n, a, i) { n = split(list, a, " "); for (i = 1; i <= n; i++) clos[k, a[i]] = 1; order[++nk] = k }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); feat[m[1]] = m[3] }
  A = "LC TPL ARROW PARAM OBJPAT"
  add("let-const", "LC"); add("template", "TPL"); add("arrow", "LC TPL ARROW"); add("parameters", "LC TPL ARROW PARAM"); add("object-pattern", A)
  add("computed-and-object-spread", A " COMPUTED OBJREST"); add("block-functions", A " COMPUTED OBJREST BLOCKFN")
  add("symbol", A " COMPUTED OBJREST SYMBOL"); add("symbol-hooks", A " COMPUTED OBJREST SYMBOL S3"); add("iterators", A " COMPUTED OBJREST SYMBOL S3 ITER")
  add("builtins", A " COMPUTED OBJREST SYMBOL S3 ITER BUILTIN"); add("for-of", A " COMPUTED BLOCKFN OBJREST SYMBOL S3 ITER FOROF")
  add("array-patterns-and-spread", A " COMPUTED BLOCKFN OBJREST SYMBOL S3 ITER FOROF ARRPAT SPREAD"); add("tagged-templates", A " COMPUTED BLOCKFN OBJREST SYMBOL S3 ITER FOROF ARRPAT SPREAD TAG")
  split("let-const:LC template:TPL arrow:ARROW parameters:PARAM object-pattern:OBJPAT computed-and-object-spread:COMPUTED,OBJREST block-functions:BLOCKFN symbol:SYMBOL symbol-hooks:S3 iterators:ITER builtins:BUILTIN for-of:FOROF array-patterns-and-spread:ARRPAT,SPREAD tagged-templates:TAG", ow, " "); for (i in ow) { split(ow[i], kv, ":"); n = split(kv[2], ws, ","); for (j = 1; j <= n; j++) own[kv[1], ws[j]] = 1 }
}
{ p = $2; f = " " $3 " "; d = $4; ft = feat[p]; delete need; later = 0
  if (f ~ / LATER /) later = 1
  if (f ~ / LC /) need["LC"] = 1; if (f ~ / TPL /) need["TPL"] = 1; if (f ~ / ARROW /) need["ARROW"] = 1
  if (f ~ / (DEFAULT|REST) /) need["PARAM"] = 1; if (f ~ / OBJPAT /) need["OBJPAT"] = 1; if (f ~ / COMPUTED /) need["COMPUTED"] = 1
  if (f ~ / SYMBOL /) need["SYMBOL"] = 1; if (f ~ / FOROF /) need["FOROF"] = 1; if (f ~ / ARRPAT /) need["ARRPAT"] = 1; if (f ~ / TAG /) need["TAG"] = 1; if (f ~ / S3 /) need["S3"] = 1
  if (ft ~ /(^|,)object-(spread|rest)(,|$)/) need["OBJREST"] = 1; else if (f ~ / SPREAD /) need["SPREAD"] = 1
  if (d ~ /(let|const) declaration is not supported/) need["LC"] = 1; if (d ~ /arrow function is not/) need["ARROW"] = 1; if (d ~ /template literal is not/) need["TPL"] = 1
  if (d ~ /(default|rest) parameter is not/) need["PARAM"] = 1; if (d ~ /CoverInitializedName is not/) need["OBJPAT"] = 1
  if (d ~ /computed property name is not/) need["COMPUTED"] = 1; if (d ~ /function declaration inside a block/) need["BLOCKFN"] = 1
  if (d ~ /for-of statement is not/) need["FOROF"] = 1; if (d ~ /Symbol is not defined/) need["SYMBOL"] = 1
  if (d ~ /spread element is not/ && (!("OBJREST" in need) || p ~ /^test\/language\/expressions\/(call|new|array)\//)) need["SPREAD"] = 1; if (d ~ /destructuring pattern is not/ && f !~ / OBJPAT /) need["ARRPAT"] = 1
  if (f ~ / B6 /) { t = slurp(root "/" p); if (t ~ /Array\.from|Object\.fromEntries|Object\.groupBy/) need["BUILTIN"] = 1; if (t ~ /String\.raw/) need["TAG"] = 1; u = t; gsub(/Object\.(values|keys|entries)\(/, "", u); if (u ~ /\.(values|keys|entries)\(/) need["ITER"] = 1 }
  if (p ~ /^test\/built-ins\/(Array\/from|Object\/fromEntries|Object\/groupBy)\//) need["BUILTIN"] = 1
  if (p ~ /^test\/built-ins\/(Array\/prototype\/(keys|values|entries)|ArrayIteratorPrototype|StringIteratorPrototype|Array\/prototype\/Symbol\.iterator|String\/prototype\/Symbol\.iterator)\// || ft ~ /(^|,)Symbol\.iterator(,|$)/) { need["ITER"] = 1; need["SYMBOL"] = 1 }
  if (p ~ /\/(dstr|destructuring)\// && p ~ /\/(ary|array)-|-ary-|-array-|iterator|iter-/) need["ARRPAT"] = 1; if (d ~ /^counted refusal/ && p ~ /\/(block-decl|switch-(case|dflt)-decl)/) need["BLOCKFN"] = 1
  if ("FOROF" in need || "ARRPAT" in need || "SPREAD" in need) { need["ITER"] = 1; need["SYMBOL"] = 1 }
  if ("ITER" in need || "BUILTIN" in need) need["SYMBOL"] = 1
  k = asorti(need, ks); list = ""; for (i = 1; i <= k; i++) list = list (i > 1 ? "," : "") ks[i]
  if (later) { print p, "later", list, d; next }
  key = "close"; for (j = 1; j <= nk; j++) { ok = 1; mine = 0; for (i = 1; i <= k; i++) { if (!((order[j], ks[i]) in clos)) { ok = 0; break } if ((order[j], ks[i]) in own) mine = 1 } if (ok && mine) { key = order[j]; break } }
  if (k == 0) key = "none"
  if (key ~ /^(let-const|template|arrow|parameters|object-pattern|none)$/) key = "close"
  print p, key, list, d }
