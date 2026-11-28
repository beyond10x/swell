#!/bin/sh
# The part of the pinned test262 checkout on which a unit of slice 11 runs the runner (the
# acceptance of each story of docs/plan/slice-11/: the full run is made in parts by the
# coordinator on the join, never by a unit).
# Usage: sh docs/plan/slice-11/part.sh <checkout> <new directory> <case list> [<directory> ...]
# <checkout> is .cache/test262/<commit of conformance/test262/PIN>; the list holds one path per
# line, starting with test/, and each <directory> starts with test/. Copies features.txt,
# harness/, each case of the list, and each directory named, whole. A module case loads the
# files beside it (design, section 11, "Modules": "./name" is the file beside the case), so the
# part holds, beside each case, every file of the case's directory whose name holds _FIXTURE
# (the runner counts none of them as a case) and every file a copied file names by a string
# literal "./<name>", until no copied file names one the part lacks; such a file that is a
# case of the checkout is run by the runner too, and check.sh reads only the cases it is given.
# Prints "part: <n> files" (the .js files under test/ that are not fixtures) and
# "part: <m> fixtures". <new directory> must not exist; nothing outside it is written. POSIX
# sh, sed and grep.
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
queue="$out/.queue"
: > "$queue"
while IFS= read -r path; do
  [ -n "$path" ] || continue
  dir=$(dirname "$path")
  mkdir -p "$out/$dir"
  cp "$root/$path" "$out/$path"
  printf '%s\n' "$path" >> "$queue"
  for fx in "$root/$dir"/*_FIXTURE*; do
    [ -f "$fx" ] || continue
    name=$(basename "$fx")
    if [ ! -e "$out/$dir/$name" ]; then
      cp "$fx" "$out/$dir/$name"
      printf '%s\n' "$dir/$name" >> "$queue"
    fi
  done
done < "$list"
# The files named by "./<name>" in a copied file, followed until none is missing.
done_list="$out/.done"
: > "$done_list"
while :; do
  next=$(grep -vxF -f "$done_list" "$queue" | head -n 1 || true)
  [ -n "$next" ] || break
  printf '%s\n' "$next" >> "$done_list"
  dir=$(dirname "$next")
  sed -n "s/.*[\"']\.\/\([^\"']*\)[\"'].*/\1/p" "$out/$next" | while IFS= read -r name; do
    [ -n "$name" ] || continue
    if [ -f "$root/$dir/$name" ] && [ ! -e "$out/$dir/$name" ]; then
      mkdir -p "$(dirname "$out/$dir/$name")"
      cp "$root/$dir/$name" "$out/$dir/$name"
      printf '%s\n' "$dir/$name" >> "$queue"
    fi
  done
done
rm -f "$queue" "$done_list"
for d in "$@"; do
  d=${d%/}
  mkdir -p "$out/$d"
  cp -R "$root/$d/." "$out/$d"
done
printf 'part: %s files\n' "$(find "$out/test" -type f -name '*.js' ! -name '*_FIXTURE*' | wc -l | tr -d ' ')"
printf 'part: %s fixtures\n' "$(find "$out/test" -type f -name '*_FIXTURE*' | wc -l | tr -d ' ')"
