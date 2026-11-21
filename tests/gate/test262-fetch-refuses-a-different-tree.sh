#!/bin/sh
# Planted defect: a checkout whose tree is not the pinned tree.
# task test262-fetch must fail, print the `tree` line and leave the directory in place.
# Run from the repository root: sh tests/gate/test262-fetch-refuses-a-different-tree.sh

name=test262-fetch-refuses-a-different-tree
root=$(pwd)
sandbox=$root/.out/gate-tests/$name
pinned_tree=70ab70455f270f26e92d097d863a0b08ad020b94

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
sum=$(git -C "$work" ls-tree -r "$commit" | sha256sum | cut -d ' ' -f 1)
dir=$sandbox/cache/$commit
mv "$work" "$dir" || fail "cannot move the checkout to $dir"
tree=$(git -C "$dir" rev-parse "$commit^{tree}") || fail "no tree for $commit"
[ "$tree" != "$pinned_tree" ] || fail "the planted tree equals the pinned tree"

out=$(task -d . -t taskfiles/test262-fetch.yml test262-fetch \
	TEST262_CACHE="$sandbox/cache" TEST262_COMMIT="$commit" TEST262_TREE="$pinned_tree" \
	TEST262_LS_TREE_SHA256="$sum" TEST262_URL=file:///nonexistent 2>&1)
status=$?
printf '%s\n' "$out" >"$sandbox/task.log"

[ "$status" -ne 0 ] || fail "task exited 0 on a different tree (log: $sandbox/task.log)"
printf '%s\n' "$out" | grep -qF "$dir: tree $tree, pinned $pinned_tree" ||
	fail "no tree line for $dir (log: $sandbox/task.log)"
[ "$(git -C "$dir" rev-parse HEAD 2>/dev/null)" = "$commit" ] ||
	fail "the checkout $dir was changed or removed"

echo "$name: ok"
