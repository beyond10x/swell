# scan.awk: the failing cases of slice 5 by the story of slice 5 that is the first to hold every part a
# case needs, or "later <n>" for a part of slice <n>, delivered after slice 5.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-05/scan.awk <first failures>
# where <first failures> is "path<TAB>detail" of the first failing run of each failing case of slice 5:
#   awk -F"\t" '$2 == "05" && $5 != "Passed" && !($3 in s) { s[$3] = 1; print $3 "\t" $6 }' .out/test262/runs.tsv | LC_ALL=C sort
# The later slices are looked for with comments taken out. The text scanned is the case and the harness files it includes but propertyHelper.js (its later tokens are
# behind a typeof test: _isTypedArray). Output: path, key, evidence.
# Keys, latest first (a case takes the first that matches): later <n>; unicode-sets (the v flag);
# property-escapes (\p{ or \P{, the feature); case-insensitive (an i flag of a literal or a RegExp call,
# an i of a modifier); identifiers (a refused identifier outside ASCII, a group name outside ASCII);
# case-mapping (the four case methods); normalize-compare (normalize, localeCompare); escape
# (RegExp.escape); match-all (matchAll, %RegExpStringIteratorPrototype%); symbol-methods (the RegExp
# methods keyed by a well-known symbol and the String methods that call them); string-replace-split
# (replace, replaceAll and split of String without a regular expression, in their directories or called); literal-early-errors (a
# negative parse case); objects-exec (every other case).
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); feat[m[1]] = "," m[3] ","; neg[m[1]] = m[4]; inc[m[1]] = m[6] } }
{ p = $1; d = $2; t = slurp(root "/" p); n = split(inc[p], a, ","); for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  f = feat[p]; k = ""; e = ""; c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  if (match(d, /not supported before slice [0-9]+/)) { s = substr(d, RSTART + 27, RLENGTH - 27); if (s + 0 != 5) { k = "later " s; e = substr(d, 1, 80) } }
  if (k == "" && match(c, /(\<Date\>|Math\.pow|parseInt|parseFloat)/)) { k = "later 4"; e = substr(c, RSTART, RLENGTH) }
  if (k == "" && match(c, /(\<class +[A-Za-z_$][A-Za-z0-9_$]* *(extends|\{)|\<class *\{|\<async +function|\<async *\(|function *\* *[A-Za-z_$(]|\<Proxy\>|\<Reflect\.|new (Weak)?(Map|Set)\>|\<Promise\>|\<[0-9]+n\>|ArrayBuffer|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>|DataView)/)) {
    x = substr(c, RSTART, RLENGTH); s = (x ~ /class/) ? 8 : (x ~ /async|Promise/) ? 10 : (x ~ /\*/) ? 7 : (x ~ /Proxy|Reflect|Map|Set/) ? 9 : (x ~ /n$/) ? 12 : 13; k = "later " s; e = x }
  if (k == "" && (f ~ /,regexp-v-flag,/ || p ~ /unicodeSets/)) { k = "unicode-sets"; e = "v flag" }
  if (k == "" && (f ~ /,regexp-unicode-property-escapes,/ || match(t, /\\[pP]\{/))) { k = "property-escapes"; e = "\\p{" }
  if (k == "" && (match(t, /[^*\/\\ \t]\/[dgmsuy]*i[dgmsuy]*[.,;) \]\n]/) || match(t, /RegExp\([^)]*, *["'][dgmsuy]*i[dgmsuy]*["']/) || match(t, /\(\?[ms]*i[ms]*(-[ims]*)?:|\(\?[ims]*-[ms]*i[ms]*:/))) { k = "case-insensitive"; e = substr(t, RSTART, RLENGTH) }
  if (k == "" && (d ~ /identifier with .* outside ASCII/ || match(t, /\(\?<[^=!>]*([\200-\377]|\\u)/))) { k = "identifiers"; e = (d ~ /outside ASCII/) ? "refused identifier" : substr(t, RSTART, RLENGTH) }
  if (k == "" && (p ~ /String\/prototype\/to(Locale)?(Lower|Upper)Case\// || d ~ /case mapping outside ASCII/)) { k = "case-mapping"; e = "case methods" }
  if (k == "" && (p ~ /String\/prototype\/(normalize|localeCompare)\// || d ~ /comparison outside ASCII/ || t ~ /\.normalize\(/)) { k = "normalize-compare"; e = "normalize or localeCompare" }
  if (k == "" && (f ~ /,RegExp\.escape,/ || p ~ /RegExp\/escape\//)) { k = "escape"; e = "RegExp.escape" }
  if (k == "" && (f ~ /,String\.prototype\.matchAll,/ || p ~ /(matchAll|RegExpStringIteratorPrototype)/ || t ~ /matchAll/)) { k = "match-all"; e = "matchAll" }
  rx = (d ~ /counted refusal|RegExp is not defined/ || t ~ /RegExp/)
  if (k == "" && (p ~ /RegExp\/prototype\/Symbol\.(match|replace|search|split)\// || p ~ /String\/prototype\/(match|search)\// || t ~ /\.(match|search)\(/ || (rx && t ~ /\.(replace|replaceAll|split)\(/))) { k = "symbol-methods"; e = "@@match, @@replace, @@search or @@split" }
  if (k == "" && !rx && (p ~ /String\/prototype\/(replace|replaceAll|split)\// || c ~ /\.(replace|replaceAll|split)\(/)) { k = "string-replace-split"; e = "no regular expression" }
  if (k == "" && neg[p] == "parse" && d ~ /SyntaxError was expected at parse/) { k = "literal-early-errors"; e = "negative parse" }
  if (k == "") { k = "objects-exec"; e = substr(d, 1, 60) }
  gsub(/[\n\t]/, " ", e); print p, k, e }
