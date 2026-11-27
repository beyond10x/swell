#!/bin/sh
# Write website/data/engines.json (format swell-engines/1), the compare page's only data, from
# the engine catalog (SITE_DATA_CATALOG), the published counts (SITE_DATA_PUBLISHED/*.tsv, each
# held by SITE_DATA_PUBLISHED/SHA256SUMS), our result files (SITE_DATA_RESULTS/*.json, format
# swell-bench/1; a figure carries its row's loaded and loaded_reason) and swell's own committed
# files: SITE_DATA_EXPECTED, SITE_DATA_PIN, SITE_DATA_MANIFEST, SITE_DATA_REQUIREMENTS,
# SITE_DATA_LIMITS, SITE_DATA_INTERRUPT and the status file site-data has just written. Called by the task site-data-write of
# taskfiles/site.yml: sh taskfiles/site-engines.sh <engines.json to write> <status.json written>.
# POSIX sh, awk and jq; no network, no Node, no date of its own.
set -eu
export LC_ALL=C
target=$1
status=$2
here=$(dirname "$0")
work=${SITE_DATA_OUT:-.out/site-data}/engines-work
catalog=$SITE_DATA_CATALOG
published=$SITE_DATA_PUBLISHED
results=$SITE_DATA_RESULTS

fail() { echo "site-data: $*" >&2; exit 1; }

for input in "$catalog" "$status" "$SITE_DATA_EXPECTED" "$SITE_DATA_PIN" "$SITE_DATA_MANIFEST" "$SITE_DATA_REQUIREMENTS" "$SITE_DATA_LIMITS" "$SITE_DATA_INTERRUPT"; do
  [ -s "$input" ] || fail "$input is missing or empty"
done
rm -rf "$work" && mkdir -p "$work"

# The published files: each one listed in SHA256SUMS with the digest it has.
if [ -d "$published" ]; then
  sums=$published/SHA256SUMS
  [ -s "$sums" ] || fail "$sums is missing or empty; run sh taskfiles/bench-import-zoo.sh"
  for file in "$published"/*; do
    name=${file##*/}
    [ "$name" = SHA256SUMS ] && continue
    grep -q "  $name\$" "$sums" || fail "$file is not listed in $sums"
  done
  if ! (cd "$published" && sha256sum --check --strict --quiet SHA256SUMS) > "$work/sums.out" 2>&1; then
    bad=$(grep -m 1 'FAILED\|No such file' "$work/sums.out" | sed 's/:.*//')
    fail "$published/$bad does not match its sha256 in $sums: a published line was edited by hand; run sh taskfiles/bench-import-zoo.sh"
  fi
fi

# Published rows, one object per line of each TSV, with the file and its line number.
: > "$work/published.jsonl"
for file in "$published"/*.tsv; do
  [ -f "$file" ] || continue
  jq -R -s -c --arg file "$file" '
    split("\n") | map(select(length > 0))
    | (.[0] | split("\t")) as $cols
    | if ($cols[0:4] != ["config", "zoo_file", "directory", "passed"]) then error("\($file): line 1 is not the header config, zoo_file, directory, passed, ...") else . end
    | to_entries[1:][]
    | (.value | split("\t")) as $v
    | if ($v | length) != ($cols | length) then error("\($file): line \(.key + 1) has \($v | length) fields, the header \($cols | length)") else . end
    | (.key + 1) as $line
    | reduce range(0; $cols | length) as $i ({file: $file, line: $line}; .[$cols[$i]] = $v[$i])
  ' "$file" >> "$work/published.jsonl" || fail "$file: not a published table"
done
jq -s '.' "$work/published.jsonl" > "$work/published.json" || fail "cannot read the published rows"

# Result files of our runs.
: > "$work/results.jsonl"
for file in "$results"/*.json; do
  [ -f "$file" ] || continue
  jq -c --arg file "$file" '
    if .format != "swell-bench/1" then error("\($file): format is \(.format), not swell-bench/1")
    elif (.rows | type) != "array" then error("\($file): no rows")
    else {file: $file} + . end
  ' "$file" >> "$work/results.jsonl" || fail "$file: not a result file of format swell-bench/1"
done
jq -s 'sort_by(.date, .file)' "$work/results.jsonl" > "$work/results.json" || fail "cannot read the result files"

# Input digests, so the data file names the exact inputs it was made from.
: > "$work/inputs.txt"
for file in "$catalog" "$published"/*.tsv "$results"/*.json; do
  [ -f "$file" ] && printf '%s %s\n' "$file" "$(sha256sum "$file" | cut -d' ' -f1)" >> "$work/inputs.txt"
done

# swell's own values: the total line and the sums per top directory of expected.txt, with lines.
awk '
  $1 == "test262" && $2 == "total" { printf "{\"kind\":\"total\",\"line\":%d,\"exist\":%d,\"excluded\":%d,\"not_applicable\":%d,\"run\":%d,\"passed\":%d}\n", FNR, $3, $4, $5, $6, $7; next }
  $1 == "test262" && NF == 9 {
    split($2, p, "/"); d = p[1]
    if (!(d in first)) { first[d] = FNR; order[++n] = d }
    last[d] = FNR; exist[d] += $3; run[d] += $6; passed[d] += $7
  }
  END { for (i = 1; i <= n; i++) { d = order[i]; printf "{\"kind\":\"dir\",\"dir\":\"%s\",\"first\":%d,\"last\":%d,\"exist\":%d,\"run\":%d,\"passed\":%d}\n", d, first[d], last[d], exist[d], run[d], passed[d] } }
' "$SITE_DATA_EXPECTED" | jq -s '.' > "$work/expected.json" || fail "$SITE_DATA_EXPECTED: cannot sum"
jq -e 'map(select(.kind == "total")) | length == 1' "$work/expected.json" >/dev/null ||
  fail "$SITE_DATA_EXPECTED: not one test262 total line"
jq -e 'map(select(.kind == "dir")) | length > 0' "$work/expected.json" >/dev/null ||
  fail "$SITE_DATA_EXPECTED: no directory line"

anchor() {
  # $1 file, $2 extended regular expression: the line number of its first match, or a failure.
  n=$(grep -n -E -m 1 "$2" "$1" | cut -d: -f1) || true
  [ -n "$n" ] || fail "$1: no line matches /$2/, which a value of engines.json cites"
  echo "$n"
}
pin_commit=$(awk '$1 == "commit" { print $2 }' "$SITE_DATA_PIN")
[ -n "$pin_commit" ] || fail "$SITE_DATA_PIN: no commit line"
jq -n \
  --arg expected "$SITE_DATA_EXPECTED" --arg pin "$SITE_DATA_PIN" --arg pin_commit "$pin_commit" \
  --arg pin_line "$(anchor "$SITE_DATA_PIN" '^commit ')" \
  --arg manifest "$SITE_DATA_MANIFEST" \
  --arg version_line "$(anchor "$SITE_DATA_MANIFEST" '^version = "')" \
  --arg license_line "$(anchor "$SITE_DATA_MANIFEST" '^license = "')" \
  --arg license "$(sed -n 's/^license = "\(.*\)"$/\1/p' "$SITE_DATA_MANIFEST" | head -n 1)" \
  --arg edition_line "$(anchor "$SITE_DATA_MANIFEST" '^edition = "')" \
  --arg edition "$(sed -n 's/^edition = "\(.*\)"$/\1/p' "$SITE_DATA_MANIFEST" | head -n 1)" \
  --arg requirements "$SITE_DATA_REQUIREMENTS" \
  --arg determinism_line "$(anchor "$SITE_DATA_REQUIREMENTS" '^HOST-133 test ')" \
  --arg determinism_test "$(awk '$1 == "HOST-133" && $2 == "test" { print $3 }' "$SITE_DATA_REQUIREMENTS")" \
  --arg limits "$SITE_DATA_LIMITS" \
  --arg steps_line "$(anchor "$SITE_DATA_LIMITS" '^ *pub steps_per_entry: ')" \
  --arg memory_line "$(anchor "$SITE_DATA_LIMITS" '^ *pub memory_bytes: ')" \
  --arg frames_line "$(anchor "$SITE_DATA_LIMITS" '^ *pub frame_depth: ')" \
  --arg interrupt "$SITE_DATA_INTERRUPT" \
  --arg interrupt_line "$(anchor "$SITE_DATA_INTERRUPT" '^ *pub fn interrupt\(&self\)')" \
  '$ARGS.named' > "$work/swell.json" || fail "cannot collect swell's values"
empty=$(jq -r 'to_entries | map(select(.value == "") | .key) | join(", ")' "$work/swell.json")
[ -z "$empty" ] || fail "no value for $empty among swell's own files"

tmp=$target.tmp
jq --slurpfile catalog_lines "$catalog" \
  --slurpfile published "$work/published.json" \
  --slurpfile results "$work/results.json" \
  --slurpfile status "$status" \
  --slurpfile expected "$work/expected.json" \
  --slurpfile swell "$work/swell.json" \
  --rawfile inputs "$work/inputs.txt" \
  --arg catalog_path "$catalog" \
  -n -f "$here/site-engines.jq" > "$tmp" || { rm -f "$tmp"; fail "cannot write $target"; }
mv "$tmp" "$target"
