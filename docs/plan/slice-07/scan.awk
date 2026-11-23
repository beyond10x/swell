# scan.awk: the failing cases of slice 7 by the story of slice 7 that is the first to hold every part a
# case needs, or "later <n>" for a part of slice <n>, delivered after slice 7.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-07/scan.awk <first failures>
# where <first failures> is "path<TAB>detail" of the first failing run of each failing case of slice 7:
#   awk -F"\t" '$2 == "07" && $5 != "Passed" && !($3 in s) { s[$3] = 1; print $3 "\t" $6 }' .out/test262/runs.tsv | LC_ALL=C sort
# The text scanned is the case and the harness files it includes but propertyHelper.js (its later tokens
# are behind a typeof test: _isTypedArray), with comments taken out. Output: path, key, evidence.
# Keys, a case taking the first that matches: later <n> (a refusal of a later slice in the first failure,
# but the refusal of slice 18 of a generator declaration as an if clause or as a labelled statement,
# which is an early error of slice 7; or a token of a later slice in the text: class, super, async,
# await, Promise, Proxy, Reflect, Map, Set, a BigInt literal, binary data, Date, Math but max, parseInt,
# parseFloat, Atomics, WeakRef, FinalizationRegistry, Iterator helpers); then the flags of the three
# stories after the first: function-constructor (GeneratorFunction by name, the files of
# built-ins/GeneratorFunction/ but its prototype/, or the constructor of a generator function read
# from its text), yield-star (yield followed by *), methods (a generator method of an object literal:
# * then a name, a computed name, a string or a number, then "(", after "{" or ","); a case with two or
# more flags takes the key cross; a case with none takes generators (the first story of the slice).
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6] } }
{ p = $1; d = $2; t = slurp(root "/" p); n = split(inc[p], a, ","); for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  k = ""; e = ""; c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  if (match(d, /not supported before slice [0-9]+/)) { s = substr(d, RSTART + 27, RLENGTH - 27)
    if (s + 0 != 7 && !(s + 0 == 18 && p ~ /\/statements\/(if|labeled)\//)) { k = "later " s; e = substr(d, 1, 80) } }
  if (k == "" && match(c, /(\<class +[A-Za-z_$][A-Za-z0-9_$]* *(extends|\{)|\<class *\{|\<super\>|\<async +function|\<async *\(|\<async +\*|\<await\>|Symbol\.asyncIterator|\<Proxy\>|\<Reflect\.|new (Weak)?(Map|Set)\>|\<Promise\>|\<[0-9]+n\>|ArrayBuffer|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>|DataView|\<Date\>|\<Math\.[a-z0-9]+|parseInt|parseFloat|\<Atomics\>|\<WeakRef\>|FinalizationRegistry|\<Iterator\.(from|prototype\.[a-z]))/)) {
    x = substr(c, RSTART, RLENGTH)
    if (x != "Math.max") { s = (x ~ /class|super/) ? 8 : (x ~ /async|await|Promise/) ? 10 : (x ~ /Proxy|Reflect|Map|Set/) ? 9 : (x ~ /^[0-9]+n$/) ? 12 : (x ~ /Atomics/) ? 14 : (x ~ /Iterator\./) ? 15 : (x ~ /WeakRef|Finalization/) ? 16 : (x ~ /Date|Math|parse/) ? 4 : 13; k = "later " s; e = x } }
  if (k == "") {
    f = 0; ev = ""
    if (c ~ /\<GeneratorFunction\>/ || p ~ /\/built-ins\/GeneratorFunction\/[^\/]*$/ || c ~ /function *\* *[A-Za-z_$]* *\([^)]*\) *\{[^}]*\}\)? *\.constructor/) { f++; ev = ev "function-constructor "; kk = "function-constructor" }
    if (c ~ /\<yield[ \t\n]*\*/) { f++; ev = ev "yield-star "; kk = "yield-star" }
    if (c ~ /[{,][ \t\n]*\*[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*|\[[^\]\n]*\]|"[^"\n]*"|\047[^\047\n]*\047|[0-9][0-9A-Za-z._]*)[ \t\n]*\(/) { f++; ev = ev "methods "; kk = "methods" }
    if (f == 0) { k = "generators"; e = "function* and yield" } else if (f == 1) { k = kk; e = ev } else { k = "cross"; e = ev }
  }
  print p, k, e }
