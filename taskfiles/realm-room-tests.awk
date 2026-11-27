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
# does, a function of its own file or of the helper files: by its name followed by "(" (a
# call), or by its bare name where a value stands, after "=", "(" or "," and before ")", ","
# or ";" (a function pointer, as in on_bounded_stack(..., body) or let f: fn() = run;), unless
# the function binds that name itself (let, for, a match pattern, a parameter, a closure's
# parameter). A path names the function of the module it names (crate::other::run() is run of
# other.rs); otherwise a function of its own file is taken before a helper of the same name. A
# name defined more than once in a file (a nested fn and a later top-level fn, a fn in a mod and
# one outside) is one function whose text is that of every definition, so the rule marks it
# when any of them reads the room, and a test is printed once. The default budget that
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
# opt-level 1.
function flush() {
  if (cur != "") {
    occ++
    occtext[occ] = body
    occname[occ] = cur
    occparent[occ] = curparent
    occtest[occ] = istest
  }
  cur = ""; body = ""; istest = 0; curparent = 0
}
function indent(line) {
  match(line, /^[ \t]*/)
  return RLENGTH
}
# bound(t, n): the text t binds the name n itself (a let, a for or a match pattern, a parameter
# or a typed name, a closure's parameter), so a bare n in t is that binding, not the function n.
function bound(t, n,   w) {
  w = "[^A-Za-z0-9_]"
  return t ~ ("(^|" w ")let" w "([^=;]*" w ")?" n w "[^=;]*=") ||
    t ~ ("(^|" w ")for" w "([^;{]*" w ")?" n w "([^;{]*" w ")?in" w) ||
    t ~ (w n w "[^=;{}]*=>") ||
    t ~ ("(^|[^A-Za-z0-9_:])" n "[ \t]*:([^:]|$)") ||
    t ~ ("\\|([^|]*[^A-Za-z0-9_|])?" n "[ \t]*(:[^|]*)?\\|")
}
FNR == 1 {
  flush()
  scope = FILENAME
  if (scope ~ /(^|\/)support\/[^\/]*$/) scope = "support"
  else { sub(/^.*\//, "", scope); sub(/\.rs$/, "", scope) }
  pending = 0
  depth = 0
}
/^[ \t]*}/ {
  at = indent($0)
  while (depth > 0 && openind[depth] >= at) depth--
}
/^[ \t]*#\[test\]/ { pending = 1 }
/^[ \t]*(pub(\([a-z]+\))? )?(const )?(async )?(unsafe )?fn [A-Za-z_]/ {
  flush()
  name = $0
  sub(/^[ \t]*(pub(\([a-z]+\))? )?(const )?(async )?(unsafe )?fn /, "", name)
  sub(/[^A-Za-z0-9_].*$/, "", name)
  cur = scope ":" name
  istest = pending
  pending = 0
  at = indent($0)
  while (depth > 0 && openind[depth] >= at) depth--
  curparent = (depth > 0) ? openocc[depth] : 0
  depth++
  openind[depth] = at
  openocc[depth] = occ + 1
}
/^[ \t]*(pub(\([a-z]+\))? )?fn \$/ { flush(); pending = 0 }
{ if (cur != "") body = body "\n" $0 }
END {
  flush()
  for (o = occ; o >= 1; o--) if (occparent[o]) occtext[occparent[o]] = occtext[occparent[o]] occtext[o]
  for (o = 1; o <= occ; o++) {
    text[occname[o]] = text[occname[o]] occtext[o]
    if (occtest[o] && !(occname[o] in listed)) { listed[occname[o]] = 1; ntest++; tests[ntest] = occname[o] }
  }
  for (f in text) {
    s = substr(f, 1, index(f, ":") - 1)
    if (rule == "stack" ? text[f] ~ /stack_with_room\(|\.stack_size\(/ : text[f] ~ /heap_bytes\(|live_bytes\(/ || (s != "support" && text[f] ~ /memory_bytes|MemoryBudget/)) {
      marked[f] = 1
      queue[++tail] = f
    }
    rest = text[f]
    gsub(/(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z_][A-Za-z0-9_]*/, " ", rest)
    seen = " "
    while (match(rest, /([A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*(::<[^>]*>)?\(|[=(,][ \t\n]*([A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*[ \t\n]*[),;]/)) {
      name = substr(rest, RSTART, RLENGTH)
      rest = substr(rest, RSTART + RLENGTH - 1)
      call = name ~ /\($/
      sub(/^[=(,][ \t\n]*/, "", name)
      sub(/(::<.*)?[ \t\n]*[),;(]$/, "", name)
      if (index(seen, " " call name " ")) continue
      seen = seen call name " "
      k = split(name, seg, "::")
      callee = seg[k]
      if (k > 1 && (seg[k - 1] ":" callee) in text) target = seg[k - 1] ":" callee
      else if ((s ":" callee) in text) target = s ":" callee
      else if (("support:" callee) in text) target = "support:" callee
      else continue
      if (target == f || (!call && k == 1 && bound(text[f], callee))) continue
      callers[target] = callers[target] " " f
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
    print ((f in marked) ? (rule == "stack" ? "stack " : "room ") : "other ") substr(f, 1, index(f, ":") - 1) "::" substr(f, index(f, ":") + 1)
  }
}
