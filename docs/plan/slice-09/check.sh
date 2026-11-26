#!/bin/sh
# The checks of a story of slice 9 over run lines (taskfiles/test262.yml: "test262-run", the
# slice, the path, the mode, the outcome, the cause; separated by tabs).
# Usage: sh docs/plan/slice-09/check.sh <case list> <runs before> <runs after>
# Over the cases that <runs after> holds (a part of the checkout or the whole run), it prints
#   failing <path> <mode> <cause>     each run of a case of the list that does not pass after;
#   regressed <path> <mode> <cause>   each run that passed before and does not pass after;
#   missing <path>                    each case of the list that <runs after> does not hold;
#   passed <P> bound <B+F-R> B <B> F <F> R <R>
# P the cases whose every run passes after, B the same before, F the cases of the list with a
# run that does not pass before, R those with a run that does not pass after; and it exits 1
# when it printed a line "failing", "regressed" or "missing", when P is below the bound, or when
# the list holds no case ("check.sh: the case list is empty" on standard error), so an empty
# list or a part that lacks a case of the list cannot pass. POSIX sh and awk.
# Self-tests, the refusals (each exits 1; <runs> any run file that holds the list's cases):
#   sh docs/plan/slice-09/check.sh /dev/null <runs> <runs copy>         the case list is empty
#   sh docs/plan/slice-09/check.sh <list> <runs> /dev/null              every case "missing"
# Owner: the decomposition of slice 9 (docs/plan/slice-09/); a unit reads it and does not
# change it.
set -eu
if [ $# -ne 3 ]; then
  echo "usage: check.sh <case list> <runs before> <runs after>" >&2
  exit 2
fi
if [ "$2" = "$3" ]; then
  echo "check.sh: <runs before> and <runs after> are the same file" >&2
  exit 2
fi
LC_ALL=C awk -F '\t' '
  FILENAME == ARGV[1] { if ($1 != "") { want[$1] = 1; n++ }; next }
  FILENAME == ARGV[2] {
    bn[$3]++
    if ($5 == "Passed") { bp[$3]++; was[$3 SUBSEP $4] = 1 } else if ($3 in want) f[$3] = 1
    next
  }
  {
    an[$3]++
    if ($5 == "Passed") { ap[$3]++; next }
    if ($3 in want) { r[$3] = 1; print "failing", $3, $4, $6; bad = 1 }
    if (($3 SUBSEP $4) in was) { print "regressed", $3, $4, $6; bad = 1 }
  }
  END {
    if (n == 0) { print "check.sh: the case list is empty" > "/dev/stderr"; bad = 1 }
    for (k in want) if (!(k in an)) { print "missing", k; bad = 1 }
    for (k in an) {
      if (ap[k] == an[k]) P++
      if ((k in bn) && bp[k] == bn[k]) B++
      if (k in f) F++
      if (k in r) R++
    }
    printf "passed %d bound %d B %d F %d R %d\n", P, B + F - R, B, F, R
    if (P < B + F - R) bad = 1
    exit bad
  }' "$1" "$2" "$3"
