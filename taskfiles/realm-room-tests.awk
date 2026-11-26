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
# taken from and the budget a room is cut from. A function also reads it when it calls one that
# does: a function of its own file or of the helper files, by its name followed by "(" (a
# function of its own file is taken before a helper of the same name). The default budget that
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
function flush() {
  if (cur != "" && !(cur in text)) {
    text[cur] = body
    if (istest) { ntest++; tests[ntest] = cur }
  }
  cur = ""; body = ""; istest = 0
}
FNR == 1 {
  flush()
  scope = FILENAME
  if (scope ~ /(^|\/)support\/[^\/]*$/) scope = "support"
  else { sub(/^.*\//, "", scope); sub(/\.rs$/, "", scope) }
  pending = 0
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
}
/^[ \t]*(pub(\([a-z]+\))? )?fn \$/ { flush(); pending = 0 }
{ if (cur != "") body = body "\n" $0 }
END {
  flush()
  for (f in text) {
    s = substr(f, 1, index(f, ":") - 1)
    if (text[f] ~ /heap_bytes\(|live_bytes\(/ || (s != "support" && text[f] ~ /memory_bytes|MemoryBudget/)) {
      marked[f] = 1
      queue[++tail] = f
    }
    rest = text[f]
    seen = " "
    while (match(rest, /[A-Za-z_][A-Za-z0-9_]*(::<[^>]*>)?\(/)) {
      callee = substr(rest, RSTART, RLENGTH - 1)
      sub(/::<.*$/, "", callee)
      rest = substr(rest, RSTART + RLENGTH)
      if (index(seen, " " callee " ")) continue
      seen = seen callee " "
      if ((s ":" callee) in text) target = s ":" callee
      else if (("support:" callee) in text) target = "support:" callee
      else continue
      if (target == f) continue
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
    print ((f in marked) ? "room " : "other ") substr(f, 1, index(f, ":") - 1) "::" substr(f, index(f, ":") + 1)
  }
}
