#!/bin/sh
# Build the benchmark image: the pinned engines of bench/image/engines.tsv and the pinned suite
# files of bench/suites/, each checked against its sha256, in one directory per pin set.
#
# Usage, from the repository root: sh bench/image/build.sh [ROOT]
# ROOT defaults to $HOME/.cache/swell-bench. The image is ROOT/image-<digest of the pin files>;
# an image that exists is kept. Prints the image's directory as its last line. The image's
# manifest.tsv lists every file with its sha256; its own sha256 is the image digest.
#
# Downloads happen here and nowhere else: a measurement runs in a network namespace without a
# network (bench/run.sh). A container runtime is not used: on the build node the docker daemon
# does not run for the pool's user and rootless podman is refused (newuidmap has no setuid).
set -eu

pins="bench/image/engines.tsv bench/suites/suites.tsv bench/suites/files.tsv"
for pin in $pins; do
  [ -f "$pin" ] || { echo "image: $pin is missing; run from the repository root" >&2; exit 1; }
done
digest=$(cat $pins | sha256sum | cut -c1-16)
root=${1:-$HOME/.cache/swell-bench}
dest=$root/image-$digest
if [ -f "$dest/manifest.tsv" ]; then
  echo "image: $dest exists"
  echo "$dest"
  exit 0
fi

mkdir -p "$root"
tmp=$root/tmp-$digest-$$
rm -rf "$tmp"
mkdir -p "$tmp/dl" "$tmp/engines" "$tmp/suites"
trap 'rm -rf "$tmp"' EXIT

check() {
  got=$(sha256sum "$1" | cut -d' ' -f1)
  if [ "$got" != "$2" ]; then
    echo "image: $3: sha256 $got, pinned $2" >&2
    exit 1
  fi
}

tail -n +2 bench/image/engines.tsv > "$tmp/engines.list"
while read -r engine version archive sha binary loads url; do
  [ -n "$engine" ] || continue
  curl -fsSL --retry 3 -o "$tmp/dl/$engine" "$url"
  check "$tmp/dl/$engine" "$sha" "$engine $version ($url)"
  mkdir -p "$tmp/engines/$engine"
  case $archive in
    zip) unzip -q -d "$tmp/engines/$engine" "$tmp/dl/$engine" ;;
    tar.gz) tar -xzf "$tmp/dl/$engine" -C "$tmp/engines/$engine" ;;
    bin) cp "$tmp/dl/$engine" "$tmp/engines/$engine/$binary" ;;
    *) echo "image: $engine: unknown archive kind $archive" >&2; exit 1 ;;
  esac
  [ -f "$tmp/engines/$engine/$binary" ] || { echo "image: $engine: no $binary in $url" >&2; exit 1; }
  chmod +x "$tmp/engines/$engine/$binary"
  echo "image: $engine $version: sha256 $sha"
done < "$tmp/engines.list"

tail -n +2 bench/suites/suites.tsv > "$tmp/suites.list"
while read -r suite repo commit dir; do
  [ -n "$suite" ] || continue
  raw=https://raw.githubusercontent.com/${repo#https://github.com/}/$commit/$dir
  mkdir -p "$tmp/suites/$suite"
  awk -v s="$suite" 'NR > 1 && $1 == s { print $2, $3 }' bench/suites/files.tsv > "$tmp/files.list"
  [ -s "$tmp/files.list" ] || { echo "image: $suite: no file in bench/suites/files.tsv" >&2; exit 1; }
  while read -r file sha; do
    # A name with a leading / is a path from the suite repository's root (a licence file
    # beside the suite's directory); it is stored under its base name.
    case $file in
      /*) url=https://raw.githubusercontent.com/${repo#https://github.com/}/$commit$file ;;
      *) url=$raw/$file ;;
    esac
    curl -fsSL --retry 3 -o "$tmp/suites/$suite/${file##*/}" "$url"
    check "$tmp/suites/$suite/${file##*/}" "$sha" "$suite $file ($url)"
  done < "$tmp/files.list"
  echo "image: $suite at $commit: $(wc -l < "$tmp/files.list") file(s)"
done < "$tmp/suites.list"

rm -rf "$tmp/dl" "$tmp/engines.list" "$tmp/suites.list" "$tmp/files.list"
(
  cd "$tmp"
  find engines suites -type f | LC_ALL=C sort | while read -r file; do sha256sum "$file"; done
  find engines suites -type l | LC_ALL=C sort | while read -r link; do echo "link $link -> $(readlink "$link")"; done
) > "$tmp/manifest.tsv.part"
mv "$tmp/manifest.tsv.part" "$tmp/manifest.tsv"
trap - EXIT
if ! mv -T "$tmp" "$dest" 2>/dev/null; then
  rm -rf "$tmp"
fi
[ -f "$dest/manifest.tsv" ] || { echo "image: $dest has no manifest" >&2; exit 1; }
echo "image: $dest, digest sha256:$(sha256sum "$dest/manifest.tsv" | cut -d' ' -f1)"
echo "$dest"
