#!/bin/sh
# The part of the pinned test262 checkout on which a unit of slice 10 runs the runner (the
# acceptance of each story of docs/plan/slice-10/: the full run is made in parts by the
# coordinator on the join, never by a unit).
# Usage: sh docs/plan/slice-10/part.sh <checkout> <new directory> <case list> [<directory> ...]
# <checkout> is .cache/test262/<commit of conformance/test262/PIN>; the list holds one path per
# line, starting with test/, and each <directory> starts with test/. Copies features.txt,
# harness/, each case of the list and each directory named, whole, and prints
# "part: <n> files". <new directory> must not exist; nothing outside it is written. POSIX sh.
set -eu
if [ $# -lt 3 ]; then
  echo "usage: part.sh <checkout> <new directory> <case list> [<directory> ...]" >&2
  exit 2
fi
root=$1
out=$2
list=$3
shift 3
if [ -e "$out" ]; then
  echo "part.sh: $out exists" >&2
  exit 2
fi
mkdir -p "$out/test"
cp "$root/features.txt" "$out/features.txt"
cp -R "$root/harness" "$out/harness"
while IFS= read -r path; do
  [ -n "$path" ] || continue
  mkdir -p "$out/$(dirname "$path")"
  cp "$root/$path" "$out/$path"
done < "$list"
for d in "$@"; do
  d=${d%/}
  mkdir -p "$out/$d"
  cp -R "$root/$d/." "$out/$d"
done
printf 'part: %s files\n' "$(find "$out/test" -type f -name '*.js' | wc -l | tr -d ' ')"
