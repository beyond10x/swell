# scan.awk: the cases of slice 8 by the story of slice 8 that is the first to hold every part of slice 8 a case
# needs, or by the list of the close when the case needs a part of another slice or passes already.
# Usage: LC_ALL=C gawk -v root=<test262 checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-08/scan.awk <cases>
# where <cases> is "path<TAB>state" of each case of slice 8, state "pass" when every run of the case passes and
# "fail" otherwise:
#   awk -F"\t" '$2 == "08" { n[$3]++; if ($5 == "Passed") p[$3]++ } END { for (k in n) print k "\t" (p[k] == n[k] ? "pass" : "fail") }' <runs.tsv> | LC_ALL=C sort
# The text scanned is the case and the harness files it includes but propertyHelper.js, with comments taken out.
# Output, tab separated: path, key, the parts the case needs (space separated), the negative phase ("-" for none).
# Parts, from the features of meta.tsv (field 3) and the text:
#   NT  new.target (the feature new.target, or new . target in the text)
#   SP  a super property (super then . or [)
#   B   a class (the feature class, or class then an optional name then extends or {)
#   C   a derived class or a super call (extends, or super then ()
#   D   a public field (class-fields-public, class-static-fields-public)
#   E   a static block (class-static-block, or static then {)
#   PM  a private method or accessor (class-methods-private, class-static-methods-private)
#   PF  a private field or #x in (class-fields-private, class-static-fields-private, class-fields-private-in)
#   EV  a call of eval (eval then (, or (0, eval))
#   A   super inside an arrow function (=> then super, with no } between them): the derived story lifts its
#       refusal, so a case with A is on the derived list or on the list of a story after it
#   L9  a part of slice 9 (Proxy, Reflect., Map, Set, WeakMap, WeakSet in the text, extends of one of the four
#       collections, or the features Map, Set, WeakMap, WeakSet, Proxy, Reflect, Reflect.construct)
#   L<n> a part of a later slice: a BigInt literal (12), binary data (13), Atomics (14), Iterator helpers or
#       DisposableStack (15), WeakRef or FinalizationRegistry (16), import (11), __lookupGetter__,
#       __lookupSetter__, __defineGetter__ or __defineSetter__ (18, Annex B)
# Keys, a case taking the first rule that matches:
#   later <n>         a part of slice <n> delivered after 8 (the close moves it)
#   slice-9           a part of slice 9 (the close checks it once slice 9 is delivered)
#   pass              every run passes on the base run (the close checks it still does)
#   negative parse cases (field 4 of meta.tsv is parse):
#     element-syntax  D, E, PM or PF, or a private name (# then a letter, $ or _) in the text
#     derived         a class (B, or class in the path) and the word super (super in a class body)
#     class-syntax    a class (B, or class in the path)
#     super           SP or C
#     new-target      NT, or new.target in the path
#     class-syntax    any other (the block redeclarations of a class name)
#   the other cases without B:
#     element-syntax  D, E, PM or PF, or a private name in the text (eval and Function given a private name)
#     derived         EV with SP or C (super in eval code: the flags of PerformEval, step 7, are the new.target
#                     story's and the rules that read them for super the derived story's), or SP with A
#                     (the arrow takes [[HomeObject]] through the new.target story's box)
#     super           SP or C (super in functions and object literals)
#     new-target      NT
#     classes         none of these and class in the path (a class name written with an escape)
#   the other cases with B:
#     eval-private    EV with PM or PF
#     eval-fields     EV with D
#     then the first of these keys, in the order of the edges, whose story holds every part of the case (its
#     closure: its own parts and those of every story it depends on, directly or through other stories):
#     classes {B NT SP}, derived {B NT SP C A}, public-fields {B NT SP D}, private-methods {B NT SP PM},
#     static-blocks {B NT SP C D PM E A} (after public-fields, derived and private-methods by edges),
#     private-fields {B NT SP C D PM E PF A}
# The list of a story, sorted (LC_ALL=C sort), read from scan.tsv:
#   LC_ALL=C awk -F"\t" '$2 == "<key>" { print $1 }' docs/plan/slice-08/scan.tsv
function slurp(f,   s, save) { save = RS; RS = "^$"; s = ""; if ((getline s < f) <= 0) s = ""; close(f); RS = save; return s }
BEGIN { FS = "\t"; OFS = "\t"
  while ((getline line < meta) > 0) { split(line, m, "\t"); inc[m[1]] = m[6]; feat[m[1]] = "," m[3] ","; neg[m[1]] = m[4] }
  cl["classes"] = " B NT SP "
  cl["derived"] = " B NT SP C A "
  cl["public-fields"] = " B NT SP D "
  cl["static-blocks"] = " B NT SP C D PM E A "
  cl["private-methods"] = " B NT SP PM "
  cl["private-fields"] = " B NT SP C D PM E PF A "
  no = split("classes derived public-fields private-methods static-blocks private-fields", ord, " ")
}
{ p = $1; st = $2; f = feat[p]; ph = neg[p]
  t = slurp(root "/" p); n = split(inc[p], a, ",")
  for (i = 1; i <= n; i++) if (a[i] != "" && a[i] != "propertyHelper.js") t = t "\n" slurp(root "/harness/" a[i])
  c = t; gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", c); gsub(/(^|\n)[ \t]*\/\/[^\n]*/, "\n", c)
  s = " "
  if (f ~ /,new\.target,/ || c ~ /\<new[ \t\n]*\.[ \t\n]*target\>/) s = s "NT "
  if (c ~ /\<class\>[ \t\n]*([A-Za-z_$][A-Za-z0-9_$]*[ \t\n]*)?(extends\>|\{)/ || f ~ /,class,/) s = s "B "
  if (c ~ /\<extends\>/ || c ~ /\<super[ \t\n]*\(/) s = s "C "
  if (c ~ /\<super[ \t\n]*(\.|\[)/) s = s "SP "
  if (f ~ /,class-fields-public,|,class-static-fields-public,/) s = s "D "
  if (f ~ /,class-static-block,/ || c ~ /\<static[ \t\n]*\{/) s = s "E "
  if (f ~ /,class-methods-private,|,class-static-methods-private,/) s = s "PM "
  if (f ~ /,class-fields-private,|,class-static-fields-private,|,class-fields-private-in,/) s = s "PF "
  if (c ~ /\<eval[ \t\n]*\(|\(0, *eval\)/) s = s "EV "
  if (c ~ /=>[ \t\n]*(\{[^}]*)?\<super\>/) s = s "A "
  if (c ~ /\<Proxy\>|\<Reflect\.|new (Weak)?(Map|Set)\>|\<(Weak)?(Map|Set)(\[|\.|\()|\<extends[ \t\n]+(Weak)?(Map|Set)\>/ || f ~ /,(Map|Set|WeakMap|WeakSet|Proxy|Reflect|Reflect\.construct),/) s = s "L9 "
  later = ""
  if (match(c, /\<[0-9]+n\>|ArrayBuffer|\<(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\>|DataView|\<Atomics\>|\<WeakRef\>|FinalizationRegistry|\<Iterator\.(from|prototype\.[a-z])|DisposableStack|\<import *\(|\<import\.meta\>|__(lookup|define)(Getter|Setter)__/)) {
    x = substr(c, RSTART, RLENGTH)
    later = (x ~ /^[0-9]+n$/) ? 12 : (x ~ /Atomics/) ? 14 : (x ~ /Iterator\.|Disposable/) ? 15 : (x ~ /WeakRef|Finalization/) ? 16 : (x ~ /import/) ? 11 : (x ~ /^__/) ? 18 : 13
    s = s "L" later " " }
  parts = substr(s, 2, length(s) - 2); if (parts == "") parts = "-"
  if (later != "") k = "later " later
  else if (s ~ / L9 /) k = "slice-9"
  else if (st == "pass") k = "pass"
  else if (ph == "parse") {
    if (s ~ / (D|E|PM|PF) / || c ~ /#[A-Za-z_$]/) k = "element-syntax"
    else if ((s ~ / B / || p ~ /\/class\//) && c ~ /\<super\>/) k = "derived"
    else if (s ~ / B / || p ~ /\/class\//) k = "class-syntax"
    else if (s ~ / (SP|C) /) k = "super"
    else if (s ~ / NT / || p ~ /new\.target/) k = "new-target"
    else k = "class-syntax"
  } else if (s !~ / B /) {
    if (s ~ / (D|E|PM|PF) / || c ~ /#[A-Za-z_$]/) k = "element-syntax"
    else if ((s ~ / EV / && s ~ / (SP|C) /) || (s ~ / SP / && s ~ / A /)) k = "derived"
    else if (s ~ / (SP|C) /) k = "super"
    else if (s ~ / NT /) k = "new-target"
    else if (p ~ /\/class\//) k = "classes"
    else k = "other"
  } else if (s ~ / EV / && s ~ / (PM|PF) /) k = "eval-private"
  else if (s ~ / EV / && s ~ / D /) k = "eval-fields"
  else {
    k = "other"; m2 = split(parts, q, " ")
    for (i = 1; i <= no; i++) {
      ok = 1
      for (j = 1; j <= m2; j++) if (q[j] != "EV" && index(cl[ord[i]], " " q[j] " ") == 0) { ok = 0; break }
      if (ok) { k = ord[i]; break }
    }
  }
  print p, k, parts, (ph == "" ? "-" : ph)
}
