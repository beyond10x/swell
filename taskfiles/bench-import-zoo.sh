#!/bin/sh
# bench-import-zoo: fetch javascript-zoo-data's test262 results for the engine configurations of
# bench/engines.jsonl that name a zoo file, and write their counts per top directory to
# bench/published/zoo-test262-<test262 revision, 7>.tsv, the MIT notice to
# bench/published/LICENSE-javascript-zoo-data, and the sha256 of both to bench/published/SHA256SUMS.
# Manual and networked (curl); never part of the gate. The fetched files are data: jq reads them,
# nothing runs them. They are kept under .cache/zoo/<data commit>/.
# Usage, from the repository root: sh taskfiles/bench-import-zoo.sh [<data commit>] [<test262 revision>]
set -eu
export LC_ALL=C

commit=${1:-450da5030317276c38ae6601c85a737a1a3c8b52}
revision=${2:-4a1e96230008fe27367233f7e4947ace176aac5a}
catalog=${BENCH_CATALOG:-bench/engines.jsonl}
published=${BENCH_PUBLISHED:-bench/published}
cache=${BENCH_ZOO_CACHE:-.cache/zoo}/$commit
raw=https://raw.githubusercontent.com/ivankra/javascript-zoo-data/$commit
short=$(printf '%s' "$revision" | cut -c1-7)
tsv=$published/zoo-test262-$short.tsv

fail() { echo "bench-import-zoo: $*" >&2; exit 1; }
case "$commit" in *[!0-9a-f]*|'') fail "data commit $commit is not a hex commit id" ;; esac
[ "${#commit}" -eq 40 ] || fail "data commit $commit is not 40 hex digits"
case "$revision" in *[!0-9a-f]*|'') fail "test262 revision $revision is not a hex commit id" ;; esac
[ -s "$catalog" ] || fail "$catalog is missing or empty"
command -v curl >/dev/null || fail "no curl"
command -v jq >/dev/null || fail "no jq"
command -v sha256sum >/dev/null || fail "no sha256sum"

mkdir -p "$cache/test262" "$published"
date=$(date -u +%Y-%m-%d)

fetch() {
  # $1 path in the data repository, $2 local file. A file already fetched is kept.
  if [ ! -s "$2" ]; then
    curl --fail --silent --show-error --location --proto '=https' --max-filesize 67108864 \
      --output "$2.part" "$raw/$1" || { rm -f "$2.part"; return 1; }
    mv "$2.part" "$2"
  fi
}

fetch LICENSE "$cache/LICENSE" || fail "cannot fetch $raw/LICENSE"
grep -q '^MIT License' "$cache/LICENSE" || fail "$raw/LICENSE is not the MIT licence"

pairs=$(jq -r '.configurations[] | select(.zoo_test262 != null) | "\(.id) \(.zoo_test262)"' "$catalog")
[ -n "$pairs" ] || fail "no configuration in $catalog names a zoo file"

tmp=$tsv.tmp
printf 'config\tzoo_file\tdirectory\tpassed\tfailed\ttotal\ttest262_revision\tengine_revision\tengine_flags\trunner_revision\trun_date\tdata_commit\tsha256\tfetch_date\turl\n' > "$tmp"
echo "$pairs" > "$cache/pairs.txt"
while read -r config stem; do
  case "$stem" in *[!A-Za-z0-9._-]*|'') fail "zoo file name $stem of $config is not a plain name" ;; esac
  file=$cache/test262/$stem.json
  url=$raw/test262/$stem.json
  if ! fetch "test262/$stem.json" "$file"; then
    echo "bench-import-zoo: $config: no $url"
    continue
  fi
  jq -e --arg rev "$revision" '.test262.revision == $rev and (.tests | type == "object")' "$file" >/dev/null ||
    fail "$file: not a test262 result at $revision"
  sum=$(sha256sum "$file" | cut -d' ' -f1)
  jq -r --arg config "$config" --arg stem "$stem" --arg commit "$commit" --arg sum "$sum" \
    --arg date "$date" --arg url "$url" '
    def clean: tostring | gsub("[\t\r\n]"; " ");
    (.binary.revision // .binary.version // "unknown" | clean) as $engine
    | ([.flags[]? | strings] | join(" ") | clean) as $flags
    | (.test262.revision | clean) as $rev
    | (.runner.revision // "unknown" | clean) as $runner
    | (.rusage.started_at // "unknown" | clean | .[0:10]) as $run
    | [.tests | to_entries[] | {dir: (.key | split("/")[1]), pass: (.value == "PASS")}] as $t
    | if ((.summary.all.passed // 0) != ([$t[] | select(.pass)] | length)) then
        error("\($stem): \([$t[] | select(.pass)] | length) passing tests counted, summary.all says \(.summary.all.passed)")
      else . end
    | ([$t | group_by(.dir)[] | {dir: .[0].dir, passed: ([.[] | select(.pass)] | length), total: length}]
       + [{dir: "all", passed: ([$t[] | select(.pass)] | length), total: ($t | length)}])[]
    | [$config, $stem, .dir, .passed, (.total - .passed), .total, $rev, $engine, $flags, $runner, $run, $commit, $sum, $date, $url]
    | map(tostring) | join("\t")
  ' "$file" >> "$tmp" || fail "$file: cannot count"
  echo "bench-import-zoo: $config: $stem.json, sha256 $sum"
done < "$cache/pairs.txt"
[ "$(wc -l < "$tmp")" -gt 1 ] || fail "no engine imported"
mv "$tmp" "$tsv"
cp "$cache/LICENSE" "$published/LICENSE-javascript-zoo-data"
(cd "$published" && find . -maxdepth 1 -type f ! -name SHA256SUMS ! -name '*.tmp' | sed 's|^\./||' | sort | xargs sha256sum) > "$published/SHA256SUMS"
echo "bench-import-zoo: wrote $tsv ($(($(wc -l < "$tsv") - 1)) line(s)), $published/LICENSE-javascript-zoo-data and $published/SHA256SUMS"
