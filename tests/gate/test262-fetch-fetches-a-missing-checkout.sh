#!/bin/sh
# A missing checkout is fetched: with an empty cache and a local source repository whose tip
# is the commit, task test262-fetch must exit 0, print `fetched and checked` and leave the
# files of that commit on disk.
# Run from the repository root: sh tests/gate/test262-fetch-fetches-a-missing-checkout.sh

name=test262-fetch-fetches-a-missing-checkout
root=$(pwd)
sandbox=$root/.out/gate-tests/$name

fail() {
	echo "$name: FAIL $*"
	exit 1
}

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
export GIT_AUTHOR_NAME=gate-test GIT_AUTHOR_EMAIL=gate-test@example.invalid
export GIT_COMMITTER_NAME=gate-test GIT_COMMITTER_EMAIL=gate-test@example.invalid
export GIT_AUTHOR_DATE=2026-01-01T00:00:00Z GIT_COMMITTER_DATE=2026-01-01T00:00:00Z

rm -rf "$sandbox" && mkdir -p "$sandbox/cache" || fail "cannot make $sandbox"

source=$sandbox/source
git init -q "$source" &&
	mkdir -p "$source/test/one" "$source/harness" &&
	printf 'one\n' >"$source/test/one/one.js" &&
	printf 'two\n' >"$source/harness/two.js" &&
	git -C "$source" add -A &&
	git -C "$source" commit -q -m one &&
	printf 'three\n' >"$source/harness/two.js" &&
	git -C "$source" commit -q -a -m two ||
	fail "cannot make the source repository"
commit=$(git -C "$source" rev-parse HEAD) || fail "no commit in the source repository"
tree=$(git -C "$source" rev-parse "$commit^{tree}") || fail "no tree for $commit"
sum=$(git -C "$source" ls-tree -r "$commit" | sha256sum | cut -d ' ' -f 1)
dir=$sandbox/cache/$commit

out=$(task -d . -t taskfiles/test262-fetch.yml test262-fetch \
	TEST262_CACHE="$sandbox/cache" TEST262_COMMIT="$commit" TEST262_TREE="$tree" \
	TEST262_LS_TREE_SHA256="$sum" TEST262_URL="file://$source" 2>&1)
status=$?
printf '%s\n' "$out" >"$sandbox/task.log"

[ "$status" -eq 0 ] || fail "task exited $status on a missing checkout (log: $sandbox/task.log)"
printf '%s\n' "$out" | grep -qxF "test262 $commit: fetched and checked" ||
	fail "no fetched and checked line (log: $sandbox/task.log)"
[ "$(git -C "$dir" rev-parse HEAD 2>/dev/null)" = "$commit" ] ||
	fail "HEAD of $dir is not $commit"
[ "$(cat "$dir/test/one/one.js" 2>/dev/null)" = one ] ||
	fail "$dir/test/one/one.js is not the file of $commit"
[ "$(cat "$dir/harness/two.js" 2>/dev/null)" = three ] ||
	fail "$dir/harness/two.js is not the file of $commit"

echo "$name: ok"
