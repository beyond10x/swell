# Sourced by the planted scripts of tests/gate/ that run the test262 runner on the counting side
# (TEST262_RUN=none): `test262_checkout <dir>` makes a small checkout of their own in place of
# the pinned one, so that a script reads a few dozen files instead of the 53,801 cases of the
# pinned checkout and test262-fetch's ls-tree and status of it (story
# gate-test-steps-within-budget, decision 2 of the coordinator). Not a script of gate-selftest:
# it lives outside tests/gate/*.sh.
#
# The checkout, <dir>/checkout/<commit>/, a git repository of one commit:
# - features.txt declaring every feature conformance/test262/features.txt lists, so that the
#   unchanged list matches it;
# - one case, front matter only, under each path of conformance/test262/sections.txt, so that
#   every section matches a case;
# - one case naming the feature generators, which the scripts plant a change of.
# Then one run of task test262 on it against an empty expected file, which must fail; its lines
# are <dir>/checkout-lines.txt, the expected file of the counting mode. <dir>/checkout-engine.md
# is docs/design/engine.md with the slice lines after "Its output on this document:" made those
# of the checkout, so that the gate's check of them passes.
# Sets checkout_vars, the task variables that point test262-fetch and test262 at the checkout
# (TEST262_CACHE, TEST262_COMMIT, TEST262_TREE, TEST262_LS_TREE_SHA256), to give on each call
# (task lets an exported variable win over a task's env, so they are not exported), and
# checkout_design, the path of the design copy. Calls fail (of the sourcing script) on an error.

test262_checkout() {
  root=$1/checkout-src
  rm -rf "$1/checkout-src" "$1/checkout" && mkdir -p "$root" "$1/checkout" ||
    fail "cannot make $root"
  awk '$1 == "run" || $1 == "exclude" { print $2 }' conformance/test262/features.txt \
    > "$root/features.txt" || fail "cannot write $root/features.txt"
  [ -s "$root/features.txt" ] || fail "conformance/test262/features.txt lists no feature"
  for prefix in $(awk '$1 == "path" { print $3 }' conformance/test262/sections.txt | LC_ALL=C sort -u); do
    mkdir -p "$root/$prefix" &&
      printf '/*---\ndescription: a case of the planted checkout\n---*/\n' > "$root/${prefix}checkout-case.js" ||
      fail "cannot write a case under $prefix"
  done
  mkdir -p "$root/test/language/statements/generators" &&
    printf '/*---\ndescription: a case naming a feature\nfeatures: [generators]\n---*/\n' \
      > "$root/test/language/statements/generators/checkout-generators.js" ||
    fail "cannot write the case naming generators"
  git init -q "$root" &&
    git -C "$root" add -A &&
    git -C "$root" -c user.name=gate -c user.email=gate@invalid -c commit.gpgsign=false \
      commit -q -m "the planted checkout" ||
    fail "cannot commit the planted checkout"
  commit=$(git -C "$root" rev-parse HEAD) || fail "no commit in the planted checkout"
  mv "$root" "$1/checkout/$commit" || fail "cannot move the planted checkout"
  tree=$(git -C "$1/checkout/$commit" rev-parse "HEAD^{tree}")
  sum=$(git -C "$1/checkout/$commit" ls-tree -r HEAD | sha256sum | cut -d ' ' -f 1)
  checkout_vars="TEST262_CACHE=$1/checkout TEST262_COMMIT=$commit TEST262_TREE=$tree TEST262_LS_TREE_SHA256=$sum"

  : > "$1/checkout-empty.txt"
  # shellcheck disable=SC2086
  if TEST262_RUN=none task -d . test262 $checkout_vars TEST262_EXPECTED="$1/checkout-empty.txt" \
    TEST262_OUT="$1/checkout-mode" > "$1/checkout-mode.out" 2>&1; then
    fail "task test262 exited 0 against an empty expected file ($1/checkout-mode.out)"
  fi
  grep -q '^test262 total ' "$1/checkout-mode/lines.txt" ||
    fail "task test262 printed no total line on the planted checkout ($1/checkout-mode.out)"
  cp "$1/checkout-mode/lines.txt" "$1/checkout-lines.txt" || fail "cannot copy the checkout's lines"
  awk '$1 == "test262-slice" { print $2, $3, $4 }' "$1/checkout-lines.txt" > "$1/checkout-slices.txt"
  [ -s "$1/checkout-slices.txt" ] || fail "no slice line on the planted checkout ($1/checkout-mode.out)"
  checkout_design=$1/checkout-engine.md
  awk -v slices="$1/checkout-slices.txt" '
    $0 == "Its output on this document:" { f = 1; print; next }
    f == 1 && $0 == "```" { print; while ((getline line < slices) > 0) print line; f = 2; next }
    f == 2 { if ($0 == "```") { print; f = 3 } next }
    { print }
  ' docs/design/engine.md > "$checkout_design" || fail "cannot write $checkout_design"
  [ "$(awk '$0 == "Its output on this document:" { n++ } END { print n + 0 }' "$checkout_design")" = 1 ] ||
    fail "docs/design/engine.md has not one line \"Its output on this document:\""
}
