# The filter of the stub cargo of tests/gate/fixtures/cargo-stub.sh: which tests of a planted
# tree a "cargo nextest list" or "cargo nextest run" takes. Not a script of gate-selftest: it
# lives outside tests/gate/*.sh.
#
# Usage: awk -v mode=<oneline|json|run> -v expr=<the -E expression or empty> -v pkgs=<the
# packages of -p, blank-separated, or empty> -v ws=<1 for --workspace> -v targets=<"test:main",
# "lib", blank-separated, or empty> -v feat=<--features> -v ignore=<1 for
# --ignore-default-filter> -f nextest-filter.awk <config file or /dev/null> <tests file>
#
# The tests file holds one line per test of the planted workspace, "<binary-id> <test>
# [default-only]": the package is the binary id up to "::", a binary id with "::" is the
# integration-test target named after it, one without is the library's unit tests, and a line
# marked default-only is a test behind #[cfg(not(feature = "collect-every-safe-point"))]. A test
# is considered when its package is selected (every package for --workspace or no -p), its
# target is selected (every target when neither --test nor --lib is given) and its feature
# allows it; it matches when the default-filter line of the config file (unless ignore) and the
# expression both take it. Both are nextest filtersets in the forms the task files write: |, &,
# not, parentheses, all(), none(), test(), binary_id(), package() with =exact, /regex/ or a
# bare value (contains for test, exact for the others). An alternative of test(=...) terms is
# read into a table, so a line naming 40,000 tests costs one lookup per test.
#
# mode oneline prints "<binary-id> <test>" for each test that matches; json prints the form of
# "cargo nextest list --message-format json-pretty" the task test-split reads, every test
# considered with the status "matches" or "mismatch"; run prints the matching tests as
# oneline does. A filterset it cannot read ends it with exit 2 and a line on stderr.
function die(m) { printf "error: the stub's filter cannot read the filterset: %s\n", m > "/dev/stderr"; failed = 1; exit 2 }
function newnode(t) { nodes++; type[nodes] = t; nk[nodes] = 0; return nodes }
function tokens(s,   n, i, t) {
  gsub(/\(/, "( ", s); gsub(/\)/, " )", s)
  n = split(s, t, /[ \t]+/)
  ntok = 0
  for (i = 1; i <= n; i++) if (t[i] != "") T[++ntok] = t[i]
  pos = 1
}
function parse(s,   n) {
  tokens(s)
  if (ntok == 0) die("an empty filterset")
  n = p_or()
  if (pos <= ntok) die("a token after the end: " T[pos])
  return n
}
function p_or(   n, c, first) {
  first = p_and()
  if (pos > ntok || (T[pos] != "|" && T[pos] != "or")) return first
  n = newnode("or")
  addor(n, first)
  while (pos <= ntok && (T[pos] == "|" || T[pos] == "or")) { pos++; addor(n, p_and()) }
  return n
}
function addor(n, c) {
  if (type[c] == "atom" && af[c] == "test" && substr(aa[c], 1, 1) == "=") { inset[n, substr(aa[c], 2)] = 1; hasset[n] = 1; return }
  nk[n]++; kid[n, nk[n]] = c
}
function p_and(   n, first) {
  first = p_not()
  if (pos > ntok || (T[pos] != "&" && T[pos] != "and")) return first
  n = newnode("and")
  nk[n] = 1; kid[n, 1] = first
  while (pos <= ntok && (T[pos] == "&" || T[pos] == "and")) { pos++; nk[n]++; kid[n, nk[n]] = p_not() }
  return n
}
function p_not(   n) {
  if (pos <= ntok && T[pos] == "not") { pos++; n = newnode("not"); nk[n] = 1; kid[n, 1] = p_not(); return n }
  return p_atom()
}
function p_atom(   n, t) {
  if (pos > ntok) die("the filterset ends early")
  t = T[pos++]
  if (t == "(") {
    n = p_or()
    if (pos > ntok || T[pos] != ")") die("no ) closes a (")
    pos++
    return n
  }
  if (t ~ /^[a-z_]+\($/) {
    n = newnode("atom")
    af[n] = substr(t, 1, length(t) - 1)
    aa[n] = ""
    if (pos <= ntok && T[pos] != ")") aa[n] = T[pos++]
    if (pos > ntok || T[pos] != ")") die("no ) closes " t)
    pos++
    if (af[n] != "test" && af[n] != "binary_id" && af[n] != "package" && af[n] != "all" && af[n] != "none") die("an unknown predicate " af[n])
    return n
  }
  die("an unexpected token " t)
}
function matches(v, a, contains,   c) {
  c = substr(a, 1, 1)
  if (c == "=") return v == substr(a, 2)
  if (c == "/") return v ~ substr(a, 2, length(a) - 2)
  if (c == "~") return index(v, substr(a, 2)) > 0
  if (contains) return index(v, a) > 0
  return v == a
}
function ev(n,   i, t) {
  t = type[n]
  if (t == "or") {
    if (hasset[n] && ((n, name) in inset)) return 1
    for (i = 1; i <= nk[n]; i++) if (ev(kid[n, i])) return 1
    return 0
  }
  if (t == "and") {
    for (i = 1; i <= nk[n]; i++) if (!ev(kid[n, i])) return 0
    return 1
  }
  if (t == "not") return !ev(kid[n, 1])
  if (af[n] == "all") return 1
  if (af[n] == "none") return 0
  if (af[n] == "test") return matches(name, aa[n], 1)
  if (af[n] == "binary_id") return matches(bin, aa[n], 0)
  return matches(pkg, aa[n], 0)
}
FILENAME == ARGV[1] {
  if ($0 ~ /^default-filter = /) { d = $0; sub(/^default-filter = '/, "", d); sub(/'[ \t]*$/, "", d); dflt = d }
  next
}
NF >= 2 {
  b = $1; p = b; sub(/::.*$/, "", p)
  k = (index(b, "::") ? "test:" substr(b, index(b, "::") + 2) : "lib")
  if (!ws && pkgs != "" && !index(" " pkgs " ", " " p " ")) next
  if (targets != "" && !index(" " targets " ", " " k " ")) next
  if ($3 == "default-only" && feat ~ /collect-every-safe-point/) next
  n_++; tb[n_] = b; tn[n_] = $2; tp[n_] = p
}
END {
  if (failed) exit 2
  droot = (dflt != "" && !ignore) ? parse(dflt) : 0
  eroot = (expr != "") ? parse(expr) : 0
  if (mode == "json") printf "{\n  \"test-count\": %d,\n  \"rust-suites\": {\n", n_
  last = ""
  for (i = 1; i <= n_; i++) {
    bin = tb[i]; name = tn[i]; pkg = tp[i]
    m = (!droot || ev(droot)) && (!eroot || ev(eroot))
    if (mode == "json") {
      if (bin != last) printf "    \"%s\": {\n      \"binary-id\": \"%s\",\n      \"testcases\": {\n", bin, bin
      last = bin
      printf "        \"%s\": {\n          \"kind\": \"test\",\n          \"ignored\": false,\n          \"filter-match\": {\n            \"status\": \"%s\"\n          }\n        },\n", name, (m ? "matches" : "mismatch")
    } else if (m) print bin " " name
  }
}
