#!/bin/sh
# Planted defect: a checkout of the right commit with one file edited on disk.
# task test262-fetch must fail, print the `changed path(s)` line and leave the edit in place.
# Run from the repository root: sh tests/gate/test262-fetch-refuses-a-changed-working-tree.sh

name=test262-fetch-refuses-a-changed-working-tree
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

work=$sandbox/cache/work
git init -q "$work" &&
	mkdir -p "$work/test" &&
	printf 'one\n' >"$work/test/one.js" &&
	git -C "$work" add -A &&
	git -C "$work" commit -q -m one ||
	fail "cannot make the local repository"
commit=$(git -C "$work" rev-parse HEAD) || fail "no commit in the local repository"
tree=$(git -C "$work" rev-parse "$commit^{tree}") || fail "no tree for $commit"
sum=$(git -C "$work" ls-tree -r "$commit" | sha256sum | cut -d ' ' -f 1)
dir=$sandbox/cache/$commit
mv "$work" "$dir" || fail "cannot move the checkout to $dir"
printf 'edited\n' >"$dir/test/one.js" || fail "cannot edit $dir/test/one.js"

out=$(task -d . -t taskfiles/test262-fetch.yml test262-fetch \
	TEST262_CACHE="$sandbox/cache" TEST262_COMMIT="$commit" TEST262_TREE="$tree" \
	TEST262_LS_TREE_SHA256="$sum" TEST262_URL=file:///nonexistent 2>&1)
status=$?
printf '%s\n' "$out" >"$sandbox/task.log"

[ "$status" -ne 0 ] || fail "task exited 0 on a changed working tree (log: $sandbox/task.log)"
printf '%s\n' "$out" | grep -qF "$dir: 1 changed path(s) in the working tree" ||
	fail "no changed path(s) line for $dir (log: $sandbox/task.log)"
[ "$(cat "$dir/test/one.js" 2>/dev/null)" = edited ] ||
	fail "the edited file in $dir was changed or removed"

echo "$name: ok"
