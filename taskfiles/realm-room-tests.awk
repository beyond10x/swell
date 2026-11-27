# The tests of swell's integration-test binary that read the realm's room, for the task
# test-realm-sizes (Taskfile.yml; story realm-size-lane-within-its-share, point 3).
#
# Usage: awk -f taskfiles/realm-room-tests.awk crates/swell/tests/support/*.rs <test files>
# with every crates/swell/tests/<module>.rs but main.rs. A file under a directory named support
# is a helper file; any other file is the module named by its stem. It prints one line per
# #[test] function it finds, "room <module>::<test>" or "other <module>::<test>", in the order
# of the files, and nothing else.
#
# The rule. A function reads the realm's room when its text names heap_bytes( or live_bytes(,
# or, in a test file (not a helper file), memory_bytes or MemoryBudget: the readings a room is
# taken from and the budget a room is cut from. A function also reads it when it names one that
# does, a function of its own file or of the helper files: by its name followed by "(" or "::<"
# (a call, with a turbofish of any depth), or by its bare name where a value stands, after "=",
# "(", ",", "[" or a single ":" and before ")", ",", ";", "]" or "}" (a function pointer, as in
# on_bounded_stack(..., body), let f: fn() = run;, [run, plain] or Case { body: run }), unless
# the function binds that name itself (let, for, a match pattern, a parameter of a fn, a
# closure's parameter; a field of a struct and a comment bind nothing), and not inside a string
# literal or a comment. A path names the function of the module it names (crate::other::run()
# is run of other.rs, crate::support::host::run() is run of the helper files); otherwise a
# function of its own file is taken before a helper of the same name. A fn may be pub, pub(...),
# const, async, unsafe or extern "...". A name defined more than once in a file (a nested fn and
# a later top-level fn, a fn in a mod and one outside) is one function whose text is that of
# every definition, so the rule marks it when any of them reads the room, and a test is printed
# once. Each definition's lines are read once and count for every function that holds it.
#
# Forms the rule does not read: a function reached only through one of them is not marked.
# - a helper imported by use: use crate::other::run; and then run();
# - a helper imported under another name: use ... as go; and then go();
# - a const or static function pointer: const DEEP: fn() = run; and then DEEP();
# - a function pointer made by a cast: &(run as fn()).
# For the stack rule the guard of slow-tests catches each of them at run time: the run at
# opt-level 1 sets SWELL_NO_STACK_TESTS, and stack_with_room and stack_thread of
# crates/swell/tests/support (the only ways to a bounded stack) panic when it is set.
#
# The default budget that
# support::host::limits sets for every agent (memory_bytes 64 MiB) does not count by itself: it
# is far above any realm, and the padding moves it only as it moves every reading, which a test
# sees only through one of the names above. A function's text is its lines up to the next line
# that declares a function, so an item between two functions counts for the one before it.
#
# What a test of the kind the task exists for does (a test that compares with a room or a
# heap_bytes() reading fitted to one size of the realm; story
# tests-hold-at-any-size-of-the-realm) needs one of these names, in its own text or in a
# function it calls. A test this rule does not see at all, one that a macro writes (its fn line
# has no literal name), is not printed: the task runs every test of the binary that is not
# printed as "other", so such a test runs at every size.
#
# A function declared inside another (its fn line indented deeper than the open function's, before
# the line "}" that closes that one at its own indent) is a function of its own and also part of
# the text of the function that holds it, so a test whose body is a nested fn passed by value
# (on_bounded_stack(..., body)) is read with that body.
#
# With -v rule=stack the same walk reads another name, for the task slow-tests (story
# realm-size-lane-within-its-share): a function runs on a bounded stack when its text names
# stack_with_room( or .stack_size(, and the lines are "stack <module>::<test>" or
# "other <module>::<test>". A stack test proves the stack the limits promise with frames whose
# size depends on the optimization level, so slow-tests runs the tests printed "stack" (and
# those not printed at all) in the test profile at opt-level 0, and those printed "other" at
# opt-level 1. A test whose attributes hold #[cfg(not(feature = "collect-every-safe-point"))]
# is not in the collect build; its line ends in " not-in-collect", so that slow-tests counts
# what runs.
function flush() {
  if (cur != "") {
    occ++
    occtext[occ] = body
    occname[occ] = cur
    occparent[occ] = curparent
    occtest[occ] = istest
    occgone[occ] = curgone
  }
  cur = ""; body = ""; istest = 0; curparent = 0; curgone = 0
}
function indent(line) {
  match(line, /^[ \t]*/)
  return RLENGTH
}
# bound(t, n): the text t (its comments left out) binds the name n itself (a let, a for or a
# match pattern, a parameter of a fn, a closure's parameter), so a bare n in t is that binding,
# not the function n. A field n: of a struct is no binding.
function bound(t, n,   w) {
  w = "[^A-Za-z0-9_]"
  return t ~ ("(^|" w ")let" w "([^=;]*" w ")?" n w "[^=;]*=") ||
    t ~ ("(^|" w ")for" w "([^;{]*" w ")?" n w "([^;{]*" w ")?in" w) ||
    t ~ (w n w "[^=;{}]*=>") ||
    t ~ ("(^|" w ")fn[ \t]+[A-Za-z_][A-Za-z0-9_]*[^(]*\\(([^)]*" w ")?" n "[ \t]*:") ||
    t ~ ("\\|([^|]*[^A-Za-z0-9_|])?" n "[ \t]*(:[^|]*)?\\|")
}
FNR == 1 {
  flush()
  scope = FILENAME
  if (scope ~ /(^|\/)support\/[^\/]*$/) scope = "support"
  else { sub(/^.*\//, "", scope); sub(/\.rs$/, "", scope) }
  pending = 0
  gone = 0
  depth = 0
}
/^[ \t]*}/ {
  at = indent($0)
  while (depth > 0 && openind[depth] >= at) depth--
}
/^[ \t]*#\[test\]/ { pending = 1 }
/^[ \t]*#\[cfg\(not\(feature = "collect-every-safe-point"\)\)\]/ { gone = 1 }
!/^[ \t]*(#\[|\/\/|$)/ && !/^[ \t]*(pub(\([^)]*\))? )?(const )?(async )?(unsafe )?(extern ("[^"]*" )?)?fn [A-Za-z_]/ { gone = 0 }
/^[ \t]*(pub(\([^)]*\))? )?(const )?(async )?(unsafe )?(extern ("[^"]*" )?)?fn [A-Za-z_]/ {
  flush()
  name = $0
  sub(/^[ \t]*(pub(\([^)]*\))? )?(const )?(async )?(unsafe )?(extern ("[^"]*" )?)?fn /, "", name)
  sub(/[^A-Za-z0-9_].*$/, "", name)
  cur = scope ":" name
  istest = pending
  curgone = gone
  pending = 0
  gone = 0
  at = indent($0)
  while (depth > 0 && openind[depth] >= at) depth--
  curparent = (depth > 0) ? openocc[depth] : 0
  depth++
  openind[depth] = at
  openocc[depth] = occ + 1
}
/^[ \t]*(pub(\([^)]*\))? )?fn \$/ { flush(); pending = 0; gone = 0 }
{ if (cur != "") body = body "\n" $0 }
END {
  flush()
  for (o = 1; o <= occ; o++) own[o] = occtext[o]
  for (o = occ; o >= 1; o--) if (occparent[o]) occtext[occparent[o]] = occtext[occparent[o]] occtext[o]
  for (o = 1; o <= occ; o++) {
    text[occname[o]] = text[occname[o]] occtext[o]
    if (occtest[o] && !(occname[o] in listed)) { listed[occname[o]] = 1; ntest++; tests[ntest] = occname[o]; testgone[ntest] = occgone[o] }
  }
  for (f in text) {
    s = substr(f, 1, index(f, ":") - 1)
    if (rule == "stack" ? text[f] ~ /stack_with_room\(|\.stack_size\(/ : text[f] ~ /heap_bytes\(|live_bytes\(/ || (s != "support" && text[f] ~ /memory_bytes|MemoryBudget/)) {
      marked[f] = 1
      queue[++tail] = f
    }
  }
  # Each definition's own lines are read once; a function they name counts for the definition's
  # function and for every function that holds it. A line is read with the last character of the
  # line before it and the first of the line after it, so that a value whose "," or ")" stands
  # on another line is read as on one line.
  for (o = 1; o <= occ; o++) fname[substr(occname[o], index(occname[o], ":") + 1)] = 1
  for (o = 1; o <= occ; o++) {
    s = substr(occname[o], 1, index(occname[o], ":") - 1)
    all = own[o]
    gsub(/(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z_][A-Za-z0-9_]*/, " ", all)
    nl = split(all, line, "\n")
    seen = " "
    for (j = 1; j <= nl; j++) {
      ln = line[j]
      # the words of the line (w) and what stands between them (sep: sep[q] before word q)
      nw = split(ln, sep, /[A-Za-z0-9_]+/)
      split(ln, w, /[^A-Za-z0-9_]+/)
      off = (ln ~ /^[^A-Za-z0-9_]/) ? 1 : 0
      for (q = 1; q < nw; q++) {
        if (!(w[q + off] in fname)) continue
        p = q
        name = w[q + off]
        while (p > 1 && sep[p] == "::") { p--; name = w[p + off] "::" name }
        after = sep[q + 1]
        sub(/^[ \t]+/, "", after)
        if (after == "" && q + 1 == nw && j < nl) after = substr(line[j + 1], match(line[j + 1], /[^ \t]/), 1)
        call = (after ~ /^(\(|::<)/)
        if (!call) {
          before = sep[p]
          sub(/[ \t]+$/, "", before)
          if (before == "" && p == 1 && j > 1) before = substr(line[j - 1], length(line[j - 1]))
          if (!(before ~ /[=(,[]$/ || (before ~ /:$/ && before !~ /::$/)) || after !~ /^[]),;}]/) continue
          # a value inside a string literal ("{ n: garbage }") or a comment names no function
          quotes = 0
          for (x = 1; x <= p; x++) { t = sep[x]; gsub(/\\"/, "", t); quotes += gsub(/"/, "", t); if (sep[x] ~ /\/\//) break }
          if (quotes % 2 || x <= p) continue
        }
        if (index(seen, " " call name " ")) continue
        seen = seen call name " "
        k = split(name, seg, "::")
        callee = seg[k]
        if (k > 1 && (seg[k - 1] ":" callee) in text) target = seg[k - 1] ":" callee
        else if (k > 1 && name ~ /(^|::)support::/ && ("support:" callee) in text) target = "support:" callee
        else if ((s ":" callee) in text) target = s ":" callee
        else if (("support:" callee) in text) target = "support:" callee
        else continue
        for (r = o; r; r = occparent[r]) {
          f = occname[r]
          if (target == f || ((f, target) in edge)) continue
          if (!call && k == 1) {
            if (!(f in nocom)) { t = text[f]; gsub(/\/\/[^\n]*/, "", t); nocom[f] = t }
            if (bound(nocom[f], callee)) continue
          }
          edge[f, target] = 1
          callers[target] = callers[target] " " f
        }
      }
    }
  }
  for (head = 1; head <= tail; head++) {
    n = split(callers[queue[head]], list, " ")
    for (i = 1; i <= n; i++) {
      if (!(list[i] in marked)) { marked[list[i]] = 1; queue[++tail] = list[i] }
    }
  }
  for (i = 1; i <= ntest; i++) {
    f = tests[i]
    print ((f in marked) ? (rule == "stack" ? "stack " : "room ") : "other ") substr(f, 1, index(f, ":") - 1) "::" substr(f, index(f, ":") + 1) ((rule == "stack" && testgone[i]) ? " not-in-collect" : "")
  }
}
