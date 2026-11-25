#!/bin/sh
# Planted defect of taskfiles/site.yml, task site-data (and so site-data-check): a line of a
# published table under bench/published edited by hand. Each file there is listed with its
# sha256 in bench/published/SHA256SUMS, which taskfiles/bench-import-zoo.sh writes. From a copy
# of bench/published, site-data must pass on the unchanged copy and must exit non-zero, naming
# the file and its sha256, when one count of the table is one higher, and naming the file when
# a file is not listed in SHA256SUMS.
# Run from the repository root: sh tests/gate/site-data-refuses-a-published-line-edited-without-its-sha256.sh
# Works only under .out/gate-tests/<name>/; prints "<name>: ok" or "<name>: FAIL <why>".

# The same output from task on any machine: no colour, and no "::error" line that go-task adds
# under GITHUB_ACTIONS (gate-selftest-is-colour-proof.sh checks this).
unset CI GITHUB_ACTIONS FORCE_COLOR CLICOLOR_FORCE
export NO_COLOR=1 CARGO_TERM_COLOR=never
name=site-data-refuses-a-published-line-edited-without-its-sha256
dir=.out/gate-tests/$name
published=bench/published
unset SITE_DATA_FILE SITE_DATA_OUT SITE_DATA_EXPECTED SITE_DATA_DELIVERED SITE_DATA_REQUIREMENTS \
  SITE_DATA_MOVES SITE_DATA_DESIGN SITE_DATA_MANIFEST SITE_DATA_LOCK SITE_DATA_CRATES \
  SITE_DATA_ENGINES SITE_DATA_CATALOG SITE_DATA_PUBLISHED SITE_DATA_RESULTS SITE_DATA_PIN \
  SITE_DATA_LIMITS SITE_DATA_INTERRUPT
fail() { echo "$name: FAIL $*"; exit 1; }

rm -rf "$dir" && mkdir -p "$dir" || fail "cannot make $dir"
[ -s "$published/SHA256SUMS" ] || fail "no $published/SHA256SUMS"
table=$(ls "$published"/*.tsv 2>/dev/null | head -n 1)
[ -n "$table" ] || fail "no table under $published"
base=${table##*/}

write() {
  # $1 the copy of bench/published to read.
  task -d . -t taskfiles/site.yml site-data SITE_DATA_OUT="$dir/out" SITE_DATA_FILE="$dir/status.json" \
    SITE_DATA_ENGINES="$dir/engines.json" SITE_DATA_PUBLISHED="$1"
}

cp -R "$published" "$dir/unplanted" || fail "cannot copy $published"
write "$dir/unplanted" > "$dir/unplanted.out" 2>&1 ||
  fail "site-data refused the unchanged copy of $published ($dir/unplanted.out)"
grep -q '"status": "published"' "$dir/engines.json" ||
  fail "site-data wrote no published value from the unchanged copy ($dir/engines.json)"

# A count edited by hand: field 4 (passed) of line 2, one higher; SHA256SUMS left as it was.
cp -R "$published" "$dir/edited" || fail "cannot copy $published"
awk -F '\t' -v OFS='\t' 'NR == 2 { $4 = $4 + 1 } { print }' "$table" > "$dir/edited/$base" ||
  fail "cannot plant the edited line"
cmp -s "$table" "$dir/edited/$base" && fail "the planted table equals $table"
if write "$dir/edited" > "$dir/edited.out" 2>&1; then
  fail "site-data accepted $base with a count edited by hand ($dir/edited.out)"
fi
grep -q "site-data: $dir/edited/$base does not match its sha256 in $dir/edited/SHA256SUMS" "$dir/edited.out" ||
  fail "site-data did not name $base and its sha256 ($dir/edited.out)"

# A file the digest list does not name.
cp -R "$published" "$dir/unlisted" || fail "cannot copy $published"
cp "$table" "$dir/unlisted/extra.tsv" || fail "cannot plant the unlisted file"
if write "$dir/unlisted" > "$dir/unlisted.out" 2>&1; then
  fail "site-data accepted a file SHA256SUMS does not list ($dir/unlisted.out)"
fi
grep -q "site-data: $dir/unlisted/extra.tsv is not listed in $dir/unlisted/SHA256SUMS" "$dir/unlisted.out" ||
  fail "site-data did not name the unlisted file ($dir/unlisted.out)"

echo "$name: ok"
