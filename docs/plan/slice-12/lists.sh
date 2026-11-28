#!/bin/sh
# The lists of slice 12 partition the slice: every case the run counts in slice 12 is on exactly one
# list of docs/plan/slice-12/ (the story lists story-*.cases.txt and the two lists of the close,
# close-slice-11.cases.txt and close-later.tsv, whose first field is the path), and every case of a
# list is counted in slice 12, or is a line of incoming.tsv (a case another slice's close is to move
# into slice 12; the decomposition writes none, and a close of slice 11 that moves cases into slice
# 12 writes the file in the commit that moves them).
# close-slice-11.cases.txt holds the cases that read a part of slice 11 (import(), import.meta),
# which the close of slice 12 checks once slice 11 is delivered.
# close-later.tsv holds the decomposition's lines (path, the later slice, the later parts; none on
# the plan's base, so the file is empty there) and the
# lines the coordinator adds when it takes a case off a story list (path, the slice or the story,
# story:<name>, that takes it, one reason), in the commit that takes it off; both count as a list.
# Usage: sh docs/plan/slice-12/lists.sh <runs> [<directory of the lists>]
# <runs> holds run lines (taskfiles/test262.yml: "test262-run", the slice, the path, the mode, the
# outcome, the cause; separated by tabs) of a full run. Prints one line per list, "<list> <cases>",
# then
#   missing <path>     a case of slice 12 on no list;
#   twice <path>       a case on two lists, or twice on one;
#   outside <path>     a case of a list that the run does not count in slice 12 and incoming.tsv
#                      does not name;
#   incoming <path>    a case of incoming.tsv the run does not count in slice 12 yet (no failure);
#   slice <n> lists <m> incoming <k>
# with n the cases of slice 12 in the run, m the cases on the lists and k the lines of incoming.tsv
# the run does not count in slice 12; and exits 1 when it printed a line "missing", "twice" or
# "outside", or when n + k differs from m. Reads nothing but its arguments and the lists. POSIX sh
# and awk.
set -eu
if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: lists.sh <runs> [<directory of the lists>]" >&2
  exit 2
fi
runs=$1
dir=${2:-docs/plan/slice-12}
set -- "$dir"/story-*.cases.txt "$dir/close-slice-11.cases.txt" "$dir/close-later.tsv"
for f in "$@"; do
  if [ ! -f "$f" ]; then
    echo "lists.sh: $f is not a file" >&2
    exit 2
  fi
done
if [ ! -s "$runs" ]; then
  echo "lists.sh: $runs is empty or missing" >&2
  exit 2
fi
incoming="$dir/incoming.tsv"
[ -f "$incoming" ] || incoming=/dev/null
LC_ALL=C awk -F '\t' -v runs="$runs" -v incoming="$incoming" '
  BEGIN {
    while ((getline line < runs) > 0) {
      split(line, f, "\t")
      if (f[1] == "test262-run" && f[2] == "12") slice[f[3]] = 1
    }
    close(runs)
    while ((getline line < incoming) > 0) {
      split(line, f, "\t")
      if (f[1] != "") come[f[1]] = 1
    }
    close(incoming)
  }
  FNR == 1 { name = FILENAME; sub(/^.*\//, "", name); order[++lists] = name }
  $1 == "" { next }
  {
    count[name]++
    m++
    if ($1 in on) { print "twice", $1; bad = 1 }
    on[$1] = name
    if (!($1 in slice) && !($1 in come)) { print "outside", $1; bad = 1 }
  }
  END {
    for (i = 1; i <= lists; i++) print order[i], count[order[i]] + 0
    n = 0
    for (k in slice) { n++; if (!(k in on)) { print "missing", k; bad = 1 } }
    k = 0
    for (c in come) if (!(c in slice)) { k++; print "incoming", c }
    printf "slice %d lists %d incoming %d\n", n, m, k
    if (n == 0 || n + k != m) bad = 1
    exit bad
  }' "$@"
