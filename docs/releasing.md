# Releasing swell

A release of swell is a version of its source. swell is a library: a release holds no binary and
no archive of its own, and nothing is uploaded to a package registry. The release is a tag on
the public repository and a GitHub Release whose text is a short summary that links to the
version's section of [`CHANGELOG.md`](../CHANGELOG.md).

- The tag is annotated, made by the organization bot (`b10x-bot[bot]`), and named exactly as
  the workspace version: `0.1.0`, no prefix.
- It is on a commit of `main` whose `check` run on the forge is green.
- The tag message and the release notes are one text, the notes file: a lead of one sentence
  (at most 25 words); `**Breaking**`, with `- None.` when nothing breaks; `**Changed**`, with
  at most five bullets of one clause each; and as its last line a link to the version's
  section of `CHANGELOG.md`, absolute at the tag
  (`https://github.com/beyond10x/swell/blob/<version>/CHANGELOG.md#<anchor>`). It has at most
  900 characters without the link line and states nothing the changelog section does not
  state. The changelog section keeps its full form, with the gate's lines. Since `0.2.0`;
  release `0.1.0` carries its whole changelog section and is not rewritten.
- Every step on the forge goes through `b10x-gates` as the bot and is done by the session that
  decides for the operator. The coordinating session prepares the commit and makes no call to
  the forge ([`AGENTS.md`](../AGENTS.md), "Publishing").
- No workflow builds anything for a release. `.github/workflows/check.yml` runs the gate on
  the commit; that run is the release's evidence.

## Versions

One version for the whole workspace: `[workspace.package] version` in the root `Cargo.toml`;
every crate takes it with `version.workspace = true`. Versions follow Semantic Versioning.
Before `1.0.0` the host interface may change in any release, and the changelog says where.

A release is cut from a green `main`. It does not wait for a delivery slice to end, and work of
an open wave is not hurried onto `main` for it. `CHANGELOG.md` states for each release what
works and what does not, with the gate's own lines.

## Cutting a release

Steps 1 to 4 are the coordinating session's, on a branch from `main`; steps 5 to 8 are the
deciding session's, from a checkout of the repository:

    gates="b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/swell"

1. **Version.** Set `[workspace.package] version` in the root `Cargo.toml`, then
   `cargo generate-lockfile --offline`. `Cargo.lock` holds the workspace's own packages and
   nothing else. The workspace of `conformance/ess` has its own manifest and lock: set its
   `version` to the same and let cargo rewrite that lock
   (`cargo metadata --offline --format-version 1` in `conformance/ess`); only the version
   lines of the six packages of this repository change there.
2. **Changelog.** In `CHANGELOG.md`, rename `## [Unreleased]` to `## [<version>] - <date>`,
   put an empty `## [Unreleased]` above it, and set the two link lines at the end. Every number
   in the section is copied from the gate's output on this commit (step 3); a number that
   cannot be copied from there is not written.
3. **Gate.** One commit by the bot holds steps 1 and 2. Run the gate on it as the forge's
   runner does and compare its lines with the changelog section:

       CI=true GITHUB_ACTIONS=true CARGO_TERM_COLOR=always CARGO_INCREMENTAL=0 \
         RUSTC_WRAPPER="" slot task check

4. **Notes.** Write the notes file, `.out/release-<version>-notes.md`, in the form the
   introduction gives: the lead, `**Breaking**`, `**Changed**`, the link line. Count its
   characters without the link line (at most 900) and check each statement against the
   changelog section. The same file is the tag's message (step 6) and the release's notes
   (step 7). The branch tip, the path of the notes file, its character count and the
   gate's lines are sent to the deciding session; on its answer `main` is moved to the commit
   (fast-forward).
5. **Publish.** The deciding session checks and verifies the commit, pushes `main`, and waits
   for the `check` run on it to be green:

       $gates gh -- run list --workflow check.yml --branch main --limit 1
       $gates gh -- run watch <run-id> --exit-status

6. **Tag.** Scan the notes file, then tag the commit with it as the message and push the tag.
   `--cleanup=whitespace` keeps a line of the notes that starts with `#`, should one do so;
   without it git removes such a line:

       $gates scan-text .out/release-<version>-notes.md
       $gates bot -- tag --cleanup=whitespace -a <version> \
         -F .out/release-<version>-notes.md <commit>
       $gates bot -- push origin <version>

7. **Release.** Create the release as the bot, with no asset, and view it:

       $gates gh -- release create <version> --repo beyond10x/swell --verify-tag \
         --title "swell <version>" \
         --notes-file .out/release-<version>-notes.md
       $gates gh -- release view <version> --repo beyond10x/swell \
         --json author,tagName,isDraft

   The release is done when it is owned by `b10x-bot[bot]`, names the tag `<version>` and is
   not a draft.
8. **Report.** The deciding session reports the version, the tag's commit and the release's
   address to the coordinating session, which merges `main` into the open wave branch.

A tag is never moved and a release is never rewritten: a defect found after step 6 is fixed on
`main` and released as the next patch version.

## When swell ships a binary

`crates/cli` (the binary `swell`) is planned and not written. From the release that holds it, a
release workflow builds the binary per target with `--locked`, the archives and a `SHA256SUMS`
file are scanned and attached to the release, and this document gains the steps for it, as
[surf's](https://github.com/beyond10x/surf/blob/main/docs/releasing.md) has them.
