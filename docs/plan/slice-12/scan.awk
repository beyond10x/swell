# scan.awk: every case of slice 12 by the story of slice 12 that is the first to hold every part of slice 12 the
# case reads, "joint" when no one story of order 3 holds them all, "slice-11" for a case that reads a part of
# slice 11 (delivered before 12, decomposed beside it), or "later <n>" for a case that needs a part of slice <n>,
# delivered after slice 12.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-12/scan.awk <cases>
# where <cases> is "path<TAB>detail" of each case of slice 12, the detail the cause of its first failing run or
# "passed" when every run passes, from the run lines of a full run (taskfiles/test262.yml, "test262-run"):
#   awk -F'\t' '$2 == "12" { if (!($3 in s)) { s[$3] = "passed"; o[++n] = $3 } if ($5 != "Passed" && s[$3] == "passed") s[$3] = $6 } END { for (i = 1; i <= n; i++) print o[i] "\t" s[o[i]] }' <runs.tsv> | LC_ALL=C sort
# The text scanned is the case and the harness files it includes, with comments taken out and the contents of
# quoted strings and template literals blanked; propertyHelper.js, compareArray.js, asyncHelpers.js,
# isConstructor.js, deepEqual.js, temporalHelpers.js and testIntl.js are not scanned (they name no BigInt
# operation a case of slice 12 reaches beyond the value itself: deepEqual.js compares a BigInt with ===, formats
# it with String and reads typeof, and names BigInt and BigInt64Array behind typeof ... === 'function';
# temporalHelpers.js and testIntl.js hold BigInt literals and calls in functions that the cases of slice 12 which
# include them do not call, so those cases need the literal to parse and nothing more of slice 12: the 9 cases
# of built-ins/Array/fromAsync whose first failure is "temporalHelpers.js: parse: refused" and
# test/harness/testIntl-getLocaleBaseName.js).
# A literal L below is a BigInt literal, a word of digits, letters and _ that starts with a digit and ends in n
# (1n, 0x1Fn, 1_000n).
# Parts, each read from the directory of the case and tokens of its text:
#   values    the BigInt value: every case of slice 12 has it (a literal, typeof, ===, SameValue,
#             SameValueZero, ToString, ToPropertyKey, unary - and !, ToBoolean, a BigInt as a key of Map or Set)
#   compare   <, >, <=, >=, == and != with a BigInt operand: the directories less-than, greater-than,
#             less-than-or-equal, greater-than-or-equal, equals and does-not-equals under language/expressions,
#             or one of these operators next to a literal L
#   arith     +, -, *, /, %, ** and the bitwise and shift operators on BigInts, ++ and --, and their compound
#             assignments: the directories addition, subtraction, multiplication, division, modulus,
#             exponentiation, left-shift, right-shift, unsigned-right-shift, bitwise-and, bitwise-or,
#             bitwise-xor, bitwise-not, prefix-increment, prefix-decrement, postfix-increment and
#             postfix-decrement under language/expressions, or a binary operator of these next to a literal L
#   builtin   %BigInt% and %BigInt.prototype% with their functions: the directory built-ins/BigInt, the word
#             BigInt, Object.prototype.toString (its tag is BigInt.prototype[@@toStringTag]), a property read on a
#             literal (L then .), a call .valueOf( or .toLocaleString(, or ToObject of a BigInt: Object( with a
#             literal L as its argument, or Object.create( with a literal L as its second argument (the wrapper
#             object and its [[BigIntData]], whose ToPrimitive reads BigInt.prototype.valueOf)
#   others    the BigInt arms of builtins of earlier slices: JSON.stringify or JSON.rawJSON, or Number( (a call
#             or new Number)
#   slice-11  a part of slice 11: import then (, or import.meta
# Keys, the stories of slice 12 in the order of their edges (a story holds the parts of the stories it depends
# on; story:swell-num-bigint-core and story:swell-num-bigint-powers-shifts-and-bits have no case of test262):
#   values    story:bigint-values-and-literals                      holds values
#   compare   story:bigint-comparison-and-loose-equality            holds values, compare
#   arith     story:bigint-arithmetic-and-bitwise-operators         holds values, arith
#   builtin   story:bigint-constructor-and-prototype                holds values, builtin
#   joint     story:bigint-json-number-and-joined-cases             holds values, compare, arith, builtin, others
# A case takes the key of the one part of compare, arith and builtin it reads, "values" when it reads none of
# them, and "joint" when it reads two or more of them or the part others. A case that reads a part of slice 11
# takes "slice-11" before any
# other key but "later <n>".
# Later parts, "later <n>": binary data (13), Atomics (14), Iterator helpers or DisposableStack (15), WeakRef,
# FinalizationRegistry or $262.gc (16), a token of a slice delivered after 12 in the text.
# Output, tab separated: path, key, evidence (the parts with the token that gave each), later parts ("-" for
# none), the parts of the case, "passed" or "failed".
# scan.tsv is this output over the 381 cases of slice 12 in the full run on f962c324 (wave big-2 after its
# fourth part; its line "test262-slice 12 381 760 0 0 381 39 342 0" equals the line of
# conformance/test262/expected.txt at f962c324).
# The lists, each sorted (LC_ALL=C sort), read from scan.tsv; lists.sh checks that they partition the slice:
#   story-<key>.cases.txt, for each key values, compare, arith, builtin, joint:
#     LC_ALL=C awk -F'\t' '$2 == "<key>" { print $1 }' docs/plan/slice-12/scan.tsv
#   close-slice-11.cases.txt, the cases that need slice 11 (the close checks them once slice 11 is delivered):
#     LC_ALL=C awk -F'\t' '$2 == "slice-11" { print $1 }' docs/plan/slice-12/scan.tsv
#   close-later.tsv, path, the later slice and the later parts, the cases the close moves:
#     LC_ALL=C awk -F'\t' '$2 ~ /^later / { s = $2; sub(/^later /, "", s); print $1 "\t" s "\t" $4 }' docs/plan/slice-12/scan.tsv
# The guards, the cases outside slice 12 whose every run passes in that run (<passing>: LC_ALL=C awk -F'\t'
# '$2 != "12" { n[$3]++; if ($5 == "Passed") p[$3]++ } END { for (k in n) if (p[k] == n[k]) print k }' <runs.tsv>
# | LC_ALL=C sort), under the directories whose code the stories change:
#   guard-literals.cases.txt (246), the numeric literals and the lexer's tokens next to them:
#     grep -E '^test/language/(literals/numeric|expressions/(typeof|logical-not|unary-minus|strict-equals|strict-does-not-equals))/' <passing>
#   guard-operators.cases.txt (1,233), the operators whose Number arm the stories share with a BigInt arm:
#     grep -E '^test/language/expressions/(addition|subtraction|multiplication|division|modulus|exponentiation|left-shift|right-shift|unsigned-right-shift|bitwise-(and|or|xor|not)|(prefix|postfix)-(increment|decrement)|compound-assignment|less-than|greater-than|less-than-or-equal|greater-than-or-equal|equals|does-not-equals)/' <passing>
#   guard-builtins.cases.txt (298), the conversions and builtins whose BigInt arm the stories write:
#     grep -E '^test/built-ins/(Number/(S9|S15\.7\.1|string-)|Object/prototype/toString|JSON/stringify|Map/prototype/(set|get|has)|Set/prototype/(add|has))' <passing>
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
function note(part, ev) { if (!(part in has)) { has[part] = 1; evid = evid part " " ev ";" } }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6]; feat[m[1]] = m[3] }
  skip["propertyHelper.js"] = 1; skip["compareArray.js"] = 1; skip["asyncHelpers.js"] = 1
  skip["isConstructor.js"] = 1; skip["deepEqual.js"] = 1; skip["temporalHelpers.js"] = 1; skip["testIntl.js"] = 1
  L = "(^|[^A-Za-z0-9_$.])[0-9][0-9A-Za-z_]*n($|[^A-Za-z0-9_$])"
  Lr = "[0-9][0-9A-Za-z_]*n($|[^A-Za-z0-9_$])"
  Ll = "(^|[^A-Za-z0-9_$.])[0-9][0-9A-Za-z_]*n"
  cmpdir = "^test/language/expressions/(less-than|greater-than|less-than-or-equal|greater-than-or-equal|equals|does-not-equals)/"
  aridir = "^test/language/expressions/(addition|subtraction|multiplication|division|modulus|exponentiation|left-shift|right-shift|unsigned-right-shift|bitwise-(and|or|xor|not)|(prefix|postfix)-(increment|decrement))/" }
{ p = $1; failed = ($2 != "passed"); t = slurp(root "/" p); delete has; evid = ""
  n = split(inc[p], a, ",")
  for (i = 1; i <= n; i++) if (a[i] != "" && !(a[i] in skip)) t = t "\n" slurp(root "/harness/" a[i])
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  gsub(/[ \t]\/\/[^\n]*/, " ", c)
  gsub(/"([^"\\\n]|\\.)*"/, "\"\"", c); gsub(/\047([^\047\\\n]|\\.)*\047/, "\047\047", c); gsub(/`[^`]*`/, "``", c)
  note("values", "slice 12")
  if (p ~ cmpdir) note("compare", "dir")
  if (c ~ (Ll "[ \t]*(<[^<]|>[^>]|==[^=]|!=[^=])") || c ~ ("([^=!<>]==|[^!=]!=|[^=<]<|[^=>-]>|<=|>=)[ \t]*[-(]*" Lr)) note("compare", "operator next to a literal")
  if (p ~ aridir) note("arith", "dir")
  if (c ~ (Ll "[ \t]*(\\*\\*|\\*|\\/|%|\\+|-|&[^&]|\\|[^|]|\\^|<<|>>)") || c ~ ("(\\*\\*|[^*]\\*|[^/]\\/|%|[^&]&|[^|]\\||\\^|<<|>>|[A-Za-z0-9_$)\\]][ \t]*[-+])=?[ \t]*[(]*" Lr)) note("arith", "operator next to a literal")
  if (p ~ /^test\/built-ins\/BigInt\//) note("builtin", "dir")
  if (c ~ /(^|[^A-Za-z0-9_$])BigInt($|[^A-Za-z0-9_$])/) note("builtin", "BigInt")
  if (c ~ /Object\.prototype\.toString/) note("builtin", "Object.prototype.toString")
  if (c ~ (Ll "\\.") || c ~ /\.(valueOf|toLocaleString)\(/) note("builtin", "method of a BigInt")
  if (c ~ ("(^|[^A-Za-z0-9_$.])Object *\\([ \t]*[-(]*" Lr) || c ~ ("Object\\.create\\([^,()]*,[ \t]*" Lr)) note("builtin", "ToObject")
  if (c ~ /JSON\.(stringify|rawJSON)|(^|[^A-Za-z0-9_$.])Number *\(/) note("others", "JSON or Number")
  if (c ~ /(^|[^A-Za-z0-9_$.])import *\(|import\.meta/) note("slice-11", "import")
  late = 0; later = ""
  if (c ~ /ArrayBuffer|DataView|(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array/) { later = later "later 13 (binary data) "; if (13 > late) late = 13 }
  if (c ~ /(^|[^A-Za-z0-9_$])Atomics($|[^A-Za-z0-9_$])/) { later = later "later 14 (Atomics) "; if (14 > late) late = 14 }
  if (c ~ /(^|[^A-Za-z0-9_$])Iterator\.|DisposableStack/) { later = later "later 15 (Iterator or disposable) "; if (15 > late) late = 15 }
  if (c ~ /(^|[^A-Za-z0-9_$])WeakRef($|[^A-Za-z0-9_$])|FinalizationRegistry|\$262\.gc/) { later = later "later 16 (WeakRef) "; if (16 > late) late = 16 }
  k = 0; key = "values"
  if ("compare" in has) { k++; key = "compare" }
  if ("arith" in has) { k++; key = "arith" }
  if ("builtin" in has) { k++; key = "builtin" }
  if (k > 1 || ("others" in has)) key = "joint"
  if ("slice-11" in has) key = "slice-11"
  if (late > 0) key = "later " late
  parts = "values"; split("compare arith builtin others slice-11", pn, " ")
  for (i = 1; i <= 5; i++) if (pn[i] in has) parts = parts "," pn[i]
  print p, key, evid, (later == "" ? "-" : later), parts, (failed ? "failed" : "passed") }
