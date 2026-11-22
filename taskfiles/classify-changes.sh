#!/bin/sh
# Classify the files a change touches as planning-only or code (story local-gate-equals-forge,
# point 3; the organization's rule "Planning-only pull requests skip the tests").
#
#   sh taskfiles/classify-changes.sh [-C <repository>] [-l <path list>] <base> [<head>]
#   sh taskfiles/classify-changes.sh [-l <path list>] -f <file of paths, one per line>
#
# The changed files are those of "git diff --name-only --no-renames <base> <head>" in the
# repository (default .; head default HEAD; a base or head may be a commit or a tree), so a
# rename counts with its old and its new path. The path list (.github/planning-paths.txt; its
# head says its form) is read from the base revision, so that a change cannot classify itself
# by changing the list; -l names a file to read instead. A change of the list itself is code.
#
# Prints one line and exits 0:
#   classify-changes: planning-only: <n> changed file(s), each a planning or prose file
#   classify-changes: code: <n> changed file(s), <k> of them code, the first <path>
#   classify-changes: code: no base revision (a first push); every file counts as code
#   classify-changes: code: the base <base> cannot be read; every file counts as code
#   classify-changes: code: the base <base> has no <list>; every file counts as code
#   classify-changes: code: no changed file
# and, when a changed file is one of a built site, "classify-changes: site: <m> changed
# file(s) of a built site". An empty base or one of zeros is a first push. When GITHUB_OUTPUT
# names a file, appends kind=<planning-only|code>, count=<n> and site=<true|false> to it.
# Exits 2, naming the line, when the path list holds a line it cannot read, and on a wrong
# call.
set -u

list_name=.github/planning-paths.txt
repo=.
list_file=
files_from=
usage() {
  echo "classify-changes: usage: classify-changes.sh [-C <repository>] [-l <path list>] <base> [<head>] | [-l <path list>] -f <file>" >&2
  exit 2
}
while getopts C:l:f: opt; do
  case $opt in
    C) repo=$OPTARG ;;
    l) list_file=$OPTARG ;;
    f) files_from=$OPTARG ;;
    *) usage ;;
  esac
done
shift $((OPTIND - 1))

kind=code
count=0
site=0
out() {
  echo "classify-changes: $kind: $1"
  if [ "$site" -gt 0 ]; then echo "classify-changes: site: $site changed file(s) of a built site"; fi
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    if [ "$site" -gt 0 ]; then s=true; else s=false; fi
    printf 'kind=%s\ncount=%s\nsite=%s\n' "$kind" "$count" "$s" >> "$GITHUB_OUTPUT"
  fi
  exit 0
}

if [ -n "$files_from" ]; then
  [ $# -eq 0 ] || usage
  [ -r "$files_from" ] || { echo "classify-changes: cannot read $files_from" >&2; exit 2; }
  changed=$(cat "$files_from")
  [ -n "$list_file" ] || list_file=$list_name
  list=$(cat "$list_file" 2>/dev/null) || { echo "classify-changes: cannot read $list_file" >&2; exit 2; }
  where=$list_file
else
  [ $# -ge 1 ] && [ $# -le 2 ] || usage
  base=$1
  head=${2:-HEAD}
  case $base in
    *[!0]*) ;;
    *) out "no base revision (a first push); every file counts as code" ;;
  esac
  git -C "$repo" rev-parse --verify --quiet "$base^{tree}" > /dev/null 2>&1 ||
    out "the base $base cannot be read; every file counts as code"
  git -C "$repo" rev-parse --verify --quiet "$head^{tree}" > /dev/null 2>&1 ||
    { echo "classify-changes: the head $head cannot be read" >&2; exit 2; }
  if [ -n "$list_file" ]; then
    list=$(cat "$list_file" 2>/dev/null) || { echo "classify-changes: cannot read $list_file" >&2; exit 2; }
    where=$list_file
  else
    list=$(git -C "$repo" show "$base:$list_name" 2>/dev/null) ||
      out "the base $base has no $list_name; every file counts as code"
    where="$base:$list_name"
  fi
  changed=$(git -C "$repo" diff --name-only --no-renames "$base" "$head") ||
    { echo "classify-changes: git diff $base $head failed" >&2; exit 2; }
fi

# The path list, checked whole before any file is classified: "<class> <pattern>" per line.
patterns=$(printf '%s\n' "$list" | awk -v where="$where" '
  /^[ \t]*#/ || /^[ \t]*$/ { next }
  NF != 2 || ($1 != "prose" && $1 != "code" && $1 != "site") {
    print "classify-changes: " where ":" NR ": not \"<prose|code|site> <pattern>\": " $0 > "/dev/stderr"
    bad = 1
    next
  }
  { print $1 " " $2; n++ }
  END {
    if (bad) exit 2
    if (!n) { print "classify-changes: " where " holds no pattern" > "/dev/stderr"; exit 2 }
  }
') || exit 2

first=
codes=0
# Word splitting of the patterns is wanted; a pattern holds no blank. Globbing is off.
set -f
while IFS= read -r path; do
  [ -n "$path" ] || continue
  count=$((count + 1))
  prose=0
  code=0
  built=0
  if [ "$path" = "$list_name" ]; then code=1; fi
  while read -r class pattern; do
    # shellcheck disable=SC2254
    case $path in
      $pattern)
        case $class in
          prose) prose=1 ;;
          code) code=1 ;;
          site) code=1; built=1 ;;
        esac
        ;;
    esac
  done <<EOF
$patterns
EOF
  site=$((site + built))
  if [ "$code" -eq 1 ] || [ "$prose" -eq 0 ]; then
    codes=$((codes + 1))
    [ -n "$first" ] || first=$path
  fi
done <<EOF
$changed
EOF

if [ "$count" -eq 0 ]; then out "no changed file"; fi
if [ "$codes" -eq 0 ]; then
  kind=planning-only
  out "$count changed file(s), each a planning or prose file"
fi
out "$count changed file(s), $codes of them code, the first $first"
