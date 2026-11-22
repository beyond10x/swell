# scan.awk: the constructs of slice 6 and the parts of other slices that a test262 case and the
# harness files it includes use, found by tokens in the text with comments removed.
# Usage: gawk -v root=<checkout> -v later=<later-parts.tsv> -f scan.awk <cases> <meta.tsv>
# <cases>: slice, path, first-failure detail (tab separated). Output: slice, path, flags, detail.
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
function strip(s) {
  gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", s)
  s = gensub(/(^|[^:"\047\\])\/\/[^\n]*/, "\\1", "g", s)
  return s
}
function count(s, re,   n) { n = gsub(re, "&", s); return n }
function flags(s,   f, t, d, r, nokw, q, i) {
  f = ""
  if (s ~ /(^|[^A-Za-z0-9_$.])(let|const)[ \t\n]+[A-Za-z_$\[{]/) f = f " LC"
  if (s ~ /=>/) f = f " ARROW"
  q = gensub(/"([^"\\\n]|\\.)*"|\047([^\047\\\n]|\\.)*\047/, "S", "g", s)
  if (q ~ /`/) {
    f = f " TPL"
    t = gensub(/`([^`\\]|\\.)*`/, "`T`", "g", q)
    t = gensub(/(return|typeof|case|void|delete|throw|in|of|yield|await|else|do)[ \t\n]*`T`/, " `T`", "g", t)
    if (t ~ /[A-Za-z0-9_$)\]][ \t\n]*`T`/) f = f " TAG"
  }
  d = count(s, "\\.\\.\\.")
  r = count(s, "\\.\\.\\.[A-Za-z_$][A-Za-z0-9_$]*[ \t\n]*\\)[ \t\n]*(\\{|=>)")
  if (r > 0) f = f " REST"
  if (d > r) f = f " SPREAD"
  nokw = gensub(/(if|while|for|switch|with|catch|return|typeof)[ \t\n]*\(/, "#(", "g", s)
  if (nokw ~ /[A-Za-z0-9_$][ \t\n]*\(([^();]|\([^();]*\))*[A-Za-z0-9_$\]}][ \t\n]*=[^=>]([^();]|\([^();]*\))*\)[ \t\n]*(\{|=>)/ || nokw ~ /\(([^();]|\([^();]*\))*[A-Za-z0-9_$\]}][ \t\n]*=[^=>]([^();]|\([^();]*\))*\)[ \t\n]*=>/) f = f " DEFAULT"
  if (s ~ /(var|let|const)[ \t\n]+\{/ || s ~ /function[ \t\n*]*[A-Za-z0-9_$]*[ \t\n]*\([^)]*\{/ || s ~ /\([ \t\n]*\{([^()]|\([^()]*\))*\)[ \t\n]*=>/ || s ~ /\(([^()]|\([^()]*\))*,[ \t\n]*\{([^()]|\([^()]*\))*\)[ \t\n]*=>/ || s ~ /catch[ \t\n]*\([ \t\n]*\{/ || s ~ /for[ \t\n]*\([ \t\n]*((var|let|const)[ \t\n]+)?\{/ || s ~ /(^|[;{}(,=\n])[ \t]*\(?[ \t]*\{[^{}]*\}[ \t\n]*\)?[ \t\n]*=[^=>]/) f = f " OBJPAT"
  if (s ~ /(var|let|const)[ \t\n]+\[/ || s ~ /catch[ \t\n]*\([ \t\n]*\[/ || s ~ /for[ \t\n]*\([ \t\n]*((var|let|const)[ \t\n]+)?\[/ || s ~ /function[ \t\n*]*[A-Za-z0-9_$]*[ \t\n]*\([^)]*\[/ || s ~ /\([ \t\n]*\[([^()]|\([^()]*\))*\)[ \t\n]*=>/ || s ~ /\(([^()]|\([^()]*\))*,[ \t\n]*\[([^()]|\([^()]*\))*\)[ \t\n]*=>/ || s ~ /(^|[;{}(,=\n])[ \t]*\(?\[[^\[\]]*\][ \t\n]*\)?[ \t\n]*=[^=>]/ || s ~ /:[ \t\n]*\[[^\]]*\][ \t\n]*(=[^=>]|[,}])/) f = f " ARRPAT"
  if (s ~ /for[ \t\n]*\([^;]*[ \t\n\])}]of[ \t\n(\[]/) f = f " FOROF"
  if (s ~ /[{,][ \t\n]*((get|set|static|async)[ \t\n]+|\*[ \t\n]*)?\[[^\]\n]*\][ \t\n]*[:(]/) f = f " COMPUTED"
  if (s ~ /(^|[^A-Za-z0-9_$.])Symbol([^A-Za-z0-9_$]|$)/) f = f " SYMBOL"
  if (s ~ /(^|[^A-Za-z0-9_$.])class[ \t\n]+[A-Za-z_${]/ || s ~ /function[ \t\n]*\*/ || s ~ /(^|[^A-Za-z0-9_$.])(async|yield|await|super)([^A-Za-z0-9_$]|$)/ || s ~ /new\.target/ || s ~ /[{,][ \t\n]*\*[ \t\n]*[A-Za-z_$\[]/ || s ~ /\*\*/ || s ~ /(^|[^A-Za-z0-9_$.])(Reflect|Proxy|Map|Set|WeakMap|WeakSet|WeakRef|FinalizationRegistry|Promise|BigInt|RegExp|Date|Math|parseInt|parseFloat|ArrayBuffer|SharedArrayBuffer|DataView|Atomics|Intl|Temporal|Iterator|AsyncIterator|[A-Z][a-z0-9]*Array)([^A-Za-z0-9_$]|$)/ || s ~ /[0-9]n([^A-Za-z0-9_$]|$)/ || s ~ /[=(,:!&|?;{}][ \t\n]*\/[^\/*\n][^\/\n]*\/[dgimsuyv]*[ \t\n]*[,;).\n]/) f = f " LATER"
  for (i = 1; i <= ntok; i++) if (index(s, tok[i]) && s ~ tokre[i]) { f = f " " (tokslice[i] == 3 ? "S3" : "LATER") }
  return f
}
function esc(s) { gsub(/[.[\]()*+?^$|\\{}]/, "\\\\&", s); return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < later) > 0) { split(line, a, "\t"); if (a[2] == 6 || a[1] == "valueOf" || a[1] ~ /^var/) continue; ntok++; tok[ntok] = a[1]; tokslice[ntok] = a[2]; tokre[ntok] = esc(a[1]) (a[1] ~ /[A-Za-z0-9_$]$/ ? "([^A-Za-z0-9_$]|$)" : "") }
  close(later)
  n6 = split("Object.getOwnPropertySymbols Array.from Object.fromEntries Object.groupBy String.raw .values( .keys( .entries(", t6, " ")
}
FNR == NR { want[$2] = $1; detail[$2] = $3; next }
($1 in want) {
  p = $1; text = strip(slurp(root "/" p))
  inc = ($2 ~ /raw/ ? "" : "assert.js,sta.js") ($2 ~ /async/ ? ",doneprintHandle.js" : "") ($6 != "" ? "," $6 : ""); sub(/^,/, "", inc)
  m = split(inc, h, ",")
  f = flags(text); b6 = 0; for (j = 1; j <= n6; j++) if (index(text, t6[j])) b6 = 1
  for (i = 1; i <= m; i++) { if (!(h[i] in hflags)) { ht = strip(slurp(root "/harness/" h[i])); hflags[h[i]] = flags(ht); hb6[h[i]] = 0; for (j = 1; j <= n6; j++) if (index(ht, t6[j])) hb6[h[i]] = 1 } f = f hflags[h[i]]; if (hb6[h[i]]) b6 = 1 }
  if (b6) f = f " B6"
  delete seenf; nf = split(f, fa, " "); f = ""; for (j = 1; j <= nf; j++) if (fa[j] != "" && !(fa[j] in seenf)) { seenf[fa[j]] = 1; f = f " " fa[j] }
  if (index($2, "raw")) f = f " RAW"
  sub(/^ /, "", f)
  print want[p], p, f, detail[p]
}
