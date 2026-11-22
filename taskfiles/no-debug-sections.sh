#!/bin/sh
# Fail when a binary carries debug information: a section whose name starts with .debug_.
# Usage: sh taskfiles/no-debug-sections.sh <binary>...
# Reads the section headers of each binary with readelf -S -W. A binary with one or more .debug_
# sections is named as "stack-profiles: <binary> has debug sections", followed by
# "stack-profiles: <binary>: <n> debug section(s): <names>"; a clean one as
# "stack-profiles: <binary>: 0 debug sections among <k> sections". It fails when one binary has
# them, when no binary is named, and when readelf cannot read a binary or finds no section in it
# (a check that reads nothing fails). The task test-stack-profiles
# (taskfiles/stack-profiles.yml) and the planted script
# tests/gate/stack-profiles-refuses-a-binary-with-debug-sections.sh call this file.
if [ "$#" -eq 0 ]; then
  echo "stack-profiles: no binary named; usage: sh taskfiles/no-debug-sections.sh <binary>..."
  exit 1
fi
bad=0
for binary in "$@"; do
  if ! headers=$(LC_ALL=C readelf -S -W "$binary" 2>&1); then
    echo "stack-profiles: readelf cannot read $binary: $headers"
    bad=1
    continue
  fi
  result=$(printf '%s\n' "$headers" | LC_ALL=C awk '
    /^ *\[ *[0-9]+\]/ {
      sections++
      for (i = 1; i <= NF; i++) if ($i ~ /^\.debug_/) { debug++; names = names " " $i; break }
    }
    END { printf "%d %d%s\n", sections, debug, names }
  ')
  sections=${result%% *}
  rest=${result#* }
  debug=${rest%% *}
  names=${rest#"$debug"}
  if [ "$sections" -eq 0 ]; then
    echo "stack-profiles: readelf finds no section in $binary"
    bad=1
  elif [ "$debug" -gt 0 ]; then
    echo "stack-profiles: $binary has debug sections"
    echo "stack-profiles: $binary: $debug debug section(s):$names"
    bad=1
  else
    echo "stack-profiles: $binary: 0 debug sections among $sections sections"
  fi
done
exit "$bad"
