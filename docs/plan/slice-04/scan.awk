# scan.awk: the failing cases of slice 4 by the story of slice 4 that is the first to hold every part of
# slice 4 a case needs, and the parts of other slices that it also needs.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-04/scan.awk <first failures>
# where <first failures> is "path<TAB>detail" of the first failing run of each failing case of slice 4:
#   awk -F"\t" '$2 == "04" && $5 != "Passed" && !($3 in s) { s[$3] = 1; print $3 "\t" $6 }' <runs.tsv> | LC_ALL=C sort
# The text scanned is the case and the harness files it includes but propertyHelper.js, with comments taken
# out. Output, tab separated: path, key, evidence, later parts ("-" for none), the other keys of this slice that
# the case also reads (the stories its story depends on; "-" for none).
# Keys, the story of slice 4 that completes a case (the case takes the highest rank of the parts it needs):
#   math-exact        Math constants and the functions IEEE 754 makes exact (abs ceil floor trunc round sign fround
#                     clz32 imul min max sqrt) and the object itself
#   number-functions  Number.isFinite, isInteger, isNaN, isSafeInteger, parseFloat, parseInt and the globals
#                     parseFloat and parseInt
#   date-clock        Date with no calendar function: the call and new with no argument, a Number or a Date, now,
#                     getTime, valueOf, setTime, the properties of Date and Date.prototype, the class tag
#   date-zone         the constructor with components, Date.UTC, the getters but getTime and valueOf, getTimezoneOffset
#   date-setters      the setters but setTime (setDate, setFullYear, setHours, setMilliseconds, setMinutes, setMonth,
#                     setSeconds and the UTC forms)
#   date-text         Date.parse, a string argument, toString and its family, toISOString, toJSON, toLocale*String,
#                     Symbol.toPrimitive, Date() as text (a case that reads or prints the text of a date)
#   number-format     Number.prototype.toFixed, toExponential, toPrecision and toLocaleString
#   math-functions    Math.acos acosh asin asinh atan atan2 atanh cbrt cos cosh exp expm1 hypot log log10 log1p
#                     log2 pow sin sinh tan tanh
#   math-random       Math.random
#   exponentiation    the operator ** and **=
#   math-sum-precise  Math.sumPrecise
# The keys are in the order the stories are merged: a case that reads parts of two stories takes the later one.
# The cases of close-joint.cases.txt need two stories that no edge orders (a case that reads parts of two stories
# takes the later key only when an edge orders them): they carry the key "joint" after the command, and the
# conformance close of slice 4 checks them (its list, with the two stories each needs).
# A case with none of these tokens, no directory of them and no refusal of slice 4 takes the key "other" (it fails
# on something else: the evidence says what). Later parts, "later <n>": a refusal of a slice other than 4 in the
# first failure or a token of a later slice in the text: class, super, Proxy, Reflect, Map, Set, a BigInt
# literal, binary data, Atomics, Iterator helpers, WeakRef, import, Temporal, Intl; Math.f16round is "later 13".
# The list of a story, sorted (LC_ALL=C sort), read from scan.tsv:
#   LC_ALL=C awk -F"\t" '$2 == "<key>" && $4 == "-" { print $1 }' docs/plan/slice-04/scan.tsv
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6]; feat[m[1]] = m[3] }
  nrank = split("math-exact date-clock date-zone number-functions date-setters date-text number-format math-functions math-random math-sum-precise exponentiation", name, " ")
  for (i = 1; i <= nrank; i++) rank[name[i]] = i }
function note(key, ev) { seen[key] = 1; if (rank[key] > best) best = rank[key]; evid = evid key " " ev ";" }
{ p = $1; t = slurp(root "/" p); n = split(inc[p], a, ","); for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  best = 0; evid = ""; delete seen
  if (match(p, /^test\/built-ins\/Math\/[A-Za-z0-9_]+/)) { f = substr(p, RSTART + 20, RLENGTH - 20)
    if (f ~ /^(abs|ceil|floor|trunc|round|sign|fround|clz32|imul|min|max|sqrt|E|LN10|LN2|LOG10E|LOG2E|PI|SQRT1_2|SQRT2)$/) note("math-exact", "dir " f)
    else if (f ~ /^(acos|acosh|asin|asinh|atan|atan2|atanh|cbrt|cos|cosh|exp|expm1|hypot|log|log10|log1p|log2|pow|sin|sinh|tan|tanh)$/) note("math-functions", "dir " f)
    else if (f == "random") note("math-random", "dir random")
    else if (f == "sumPrecise") note("math-sum-precise", "dir sumPrecise") }
  if (p ~ /^test\/built-ins\/(parseInt|parseFloat)\//) note("number-functions", "dir " p)
  if (p ~ /^test\/built-ins\/Number\/(isFinite|isInteger|isNaN|isSafeInteger|parseFloat|parseInt)\//) note("number-functions", "dir Number static")
  if (p ~ /^test\/built-ins\/Number\/prototype\/(toFixed|toExponential|toPrecision|toLocaleString)\//) note("number-format", "dir Number.prototype format")
  if (p ~ /^test\/built-ins\/Date\//) {
    if (p ~ /\/Date\/prototype\/(toString|toDateString|toTimeString|toUTCString|toISOString|toJSON|toLocaleString|toLocaleDateString|toLocaleTimeString|Symbol\.toPrimitive)\//) note("date-text", "dir Date text")
    else if (p ~ /\/Date\/prototype\/set(UTC)?(Date|FullYear|Hours|Milliseconds|Minutes|Month|Seconds)\//) note("date-setters", "dir Date setter")
    else if (p ~ /\/Date\/parse\//) note("date-text", "dir Date.parse")
    else if (p ~ /\/Date\/(UTC\/|prototype\/get(UTC)?(Date|Day|FullYear|Hours|Milliseconds|Minutes|Month|Seconds|TimezoneOffset)\/)/) note("date-zone", "dir Date components")
    else note("date-clock", "dir Date") }
  s = c
  while (match(s, /\<Math\.[A-Za-z0-9_]+/)) { f = substr(s, RSTART + 5, RLENGTH - 5); s = substr(s, RSTART + RLENGTH)
    if (f ~ /^(abs|ceil|floor|trunc|round|sign|fround|clz32|imul|min|max|sqrt|E|LN10|LN2|LOG10E|LOG2E|PI|SQRT1_2|SQRT2)$/) note("math-exact", "Math." f)
    else if (f ~ /^(acos|acosh|asin|asinh|atan|atan2|atanh|cbrt|cos|cosh|exp|expm1|hypot|log|log10|log1p|log2|pow|sin|sinh|tan|tanh)$/) note("math-functions", "Math." f)
    else if (f == "random") note("math-random", "Math.random")
    else if (f == "sumPrecise") note("math-sum-precise", "Math.sumPrecise") }
  if (c ~ /\<Math\>/ && best == 0) note("math-exact", "Math")
  if (c ~ /parseInt|parseFloat/) note("number-functions", "parseInt or parseFloat")
  if (c ~ /Number\.(isFinite|isInteger|isNaN|isSafeInteger)/) note("number-functions", "Number static")
  if (c ~ /toFixed|toExponential|toPrecision/) note("number-format", "toFixed, toExponential or toPrecision")
  if (c ~ /Number\.prototype/ && c ~ /toLocaleString/) note("number-format", "Number.prototype.toLocaleString")
  if (c ~ /\<Date\>|assertRelativeDateMs/) {
    note("date-clock", "Date")
    if (c ~ /\<get(UTC)?(Date|Day|FullYear|Hours|Milliseconds|Minutes|Month|Seconds)\>|\<getTimezoneOffset\>|Date\.UTC|(new Date|Date) *\([^)]*,/) note("date-zone", "Date components, getter or UTC")
    if (c ~ /\.set(UTC)?(Date|FullYear|Hours|Milliseconds|Minutes|Month|Seconds|Year)\(/) note("date-setters", "Date setter")
    if (c ~ /Date\.parse|(new Date|Date) *\( *(\047|"|`|String)/) note("date-text", "Date.parse or a string argument")
    if (c ~ /toISOString|toUTCString|toDateString|toTimeString|toJSON|toGMTString|toLocale(Date|Time)?String|Symbol\.toPrimitive|\<toString\>|JSON\.stringify|\<String\(|[+] *(new Date|\047\047|"")|(\047\047|"") *[+]|\$\{/) note("date-text", "Date to text") }
  if (feat[p] ~ /exponentiation/ || c ~ /[A-Za-z0-9_)\]] *\*\*=? *[-+(A-Za-z0-9_.]/ || p ~ /\/expressions\/exponentiation\//) note("exponentiation", "feature or text")
  if ($2 ~ /exponentiation operator is not supported before slice 4/) note("exponentiation", "refusal")
  later = ""
  if (match($2, /not supported before slice [0-9]+/)) { s = substr($2, RSTART + 27, RLENGTH - 27); if (s + 0 != 4) later = later "later " s " (refusal) " }
  if (c ~ /(\<class +[A-Za-z_$][A-Za-z0-9_$]* *(extends|\{)|\<class *\{|\<super\>)/) later = later "later 8 (class or super) "
  if (c ~ /\<Proxy\>|\<Reflect\./) later = later "later 9 (Proxy or Reflect) "
  if (c ~ /\<(Weak)?(Map|Set)\>/) later = later "later 9 (Map or Set) "
  if (c ~ /\<[0-9][0-9_]*n\>|\<BigInt\>/) later = later "later 12 (BigInt) "
  if (c ~ /ArrayBuffer|DataView|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>|testTypedArray|testBigIntTypedArray/ || p ~ /\/TypedArray/) later = later "later 13 (typed arrays) "
  if (c ~ /\<Atomics\>/) later = later "later 14 (Atomics) "
  if (c ~ /\<WeakRef\>|FinalizationRegistry/) later = later "later 16 (WeakRef) "
  if (c ~ /\<Iterator\.|\<DisposableStack/) later = later "later 15 (Iterator or disposable) "
  if (c ~ /\<import *\(|\<import\.meta\>/ || feat[p] ~ /dynamic-import/) later = later "later 11 (import) "
  if (c ~ /\<Temporal\>/ || feat[p] ~ /Temporal/) later = later "later 20 (Temporal) "
  if (c ~ /\<Intl\>/) later = later "later 21 (Intl) "
  if (c ~ /Math\.f16round/ || p ~ /Math\/f16round/) later = later "later 13 (f16round) "
  if (later == "") later = "-"
  key = (best == 0) ? "other" : name[best]
  if (best == 0) evid = substr($2, 1, 80)
  also = ""; for (i = 1; i <= nrank; i++) if ((name[i] in seen) && name[i] != key) also = also (also == "" ? "" : ",") name[i]
  print p, key, evid, later, (also == "" ? "-" : also) }
