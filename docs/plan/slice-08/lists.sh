#!/bin/sh
# Proves that the case lists of slice 8 partition the slice: every case the runner counts in slice 8
# is on exactly one list, and every line of a list is a case of slice 8.
# Usage: sh docs/plan/slice-08/lists.sh <runs.tsv>
# <runs.tsv> holds the run lines of a full run (taskfiles/test262.yml: "test262-run", the slice, the
# path, the mode, the outcome, the cause; separated by tabs) made before the close of slice 8 moves a
# case. The lists are the files story-*.cases.txt, close-*.cases.txt and close-later.tsv (its first
# field) beside this script. Prints "missing <path>" for a case of the slice on no list, "twice <path>
# <list> <list>" for a case on two lists, "foreign <path> <list>" for a listed path that is not a case
# of slice 8, then "lists: <c> cases of slice 8, <l> lines in <n> lists"; exits 1 when it printed any
# other line or read no case. Reads and writes nothing else. POSIX sh and awk.
set -eu
if [ $# -ne 1 ]; then
  echo "usage: lists.sh <runs.tsv>" >&2
  exit 2
fi
dir=$(dirname "$0")
set -- "$1" "$dir"/story-*.cases.txt "$dir"/close-*.cases.txt "$dir"/close-later.tsv
LC_ALL=C awk -F '\t' '
  FNR == 1 { files++ }
  FILENAME == ARGV[1] { if ($2 == "08") slice[$3] = 1; next }
  $1 == "" { next }
  {
    lines++
    name = FILENAME; sub(/.*\//, "", name)
    if ($1 in on) { print "twice", $1, on[$1], name; bad = 1 } else on[$1] = name
    if (!($1 in slice)) { print "foreign", $1, name; bad = 1 }
  }
  END {
    for (k in slice) { c++; if (!(k in on)) { print "missing", k; bad = 1 } }
    printf "lists: %d cases of slice 8, %d lines in %d lists\n", c, lines, files - 1
    if (c == 0) bad = 1
    exit bad
  }' "$@"
