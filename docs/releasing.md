# Releasing swell

A release of swell is a version of its source and the shell binary `swell` (`crates/cli`) built
from it for five targets. Nothing is uploaded to a package registry. The release is a tag on
the public repository and a GitHub Release whose text is a short summary that links to the
version's section of [`CHANGELOG.md`](../CHANGELOG.md). Its assets are the five archives of the
binary and a `SHA256SUMS` file ("The binary of a release" below).

- The tag is annotated, made by the organization bot (`b10x-bot[bot]`), and named exactly as
  the workspace version: `0.1.0`, no prefix.
- It is on a commit of `main` whose `check` run on the forge is green.
- The tag message and the release notes are one text, the notes file. Its parts:

  | part | form |
  |---|---|
  | lead | one sentence, at most 25 words |
  | `**Breaking**` | `- None.` when nothing breaks |
  | `**Changed**` | at most five bullets of one clause each |
  | last line | a link to the version's section of `CHANGELOG.md`, absolute at the tag: `https://github.com/beyond10x/swell/blob/<version>/CHANGELOG.md#<anchor>` |

  The file has at most 900 characters without the link line and states nothing the changelog
  section does not state. The changelog section keeps its full form, with the gate's lines.
  This form holds since `0.2.0`. Release `0.1.0` carries its whole changelog section and is
  not rewritten.
- Every step on the forge goes through `b10x-gates` as the bot and is done by the session that
  decides for the operator. The coordinating session prepares the commit and makes no call to
  the forge ([`AGENTS.md`](../AGENTS.md), "Publishing").
- `.github/workflows/check.yml` runs the gate on the commit when `main` is pushed, and that
  run is the release's evidence. The push of the tag starts one run of
  `.github/workflows/shared-gates.yml` and one run of `.github/workflows/release-build.yml`,
  which builds and signs the binary, and no run of `check.yml`.

## Versions

One version for the whole workspace: `[workspace.package] version` in the root `Cargo.toml`.
All crates take it with `version.workspace = true`. Versions follow Semantic Versioning.
Before `1.0.0` the host interface may change in any release, and the changelog says where.

A release is cut from a green `main`. It does not wait for a delivery slice to end, and work of
an open wave is not hurried onto `main` for it. `CHANGELOG.md` states for each release what
works and what does not, with the gate's own lines.

## Cutting a release

Steps 1 to 4 are the coordinating session's, on a branch from `main`. Steps 5 to 8 are the
deciding session's, from a checkout of the repository:

    gates="b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/swell"

1. **Version.** Set `[workspace.package] version` in the root `Cargo.toml`, then
   `cargo generate-lockfile --offline`. `Cargo.lock` holds the workspace's own packages and
   nothing else. The workspace of `conformance/ess` has its own manifest and lock: set its
   `version` to the same and let cargo rewrite that lock with
   `cargo metadata --offline --format-version 1` in `conformance/ess`. Only the version
   lines of the six packages of this repository change there.
2. **Changelog.** In `CHANGELOG.md`, rename `## [Unreleased]` to `## [<version>] - <date>`,
   put an empty `## [Unreleased]` above it, and set the two link lines at the end. `<date>`
   is the day the tag is made (step 6); when the tag comes on a later day than this commit,
   a commit before the tag sets the heading and the link line of the notes file to that day.
   Every number
   in the section is copied from the gate's output on this commit (step 3). A number that
   cannot be copied from that output is not written.
   The section keeps its `### Host interface` heading, with `- None.` when the host interface
   did not change. After step 1, `slot --local task host-surface-snapshot` writes the scan of
   the host interface into `conformance/host/surface.txt`, headed `# swell <version>`; the
   file is committed with steps 1 and 2. The notes' `**Breaking**` part (step 4) lists every
   removed or changed name of that heading, and every variant added to a public enum that is
   not `non_exhaustive` (a `match` of an embedder without a wildcard arm stops compiling). The
   names have the form of "The host interface
   in the changelog" below.
3. **Gate.** One commit by the bot holds steps 1 and 2. Run the gate on it as the forge's
   runner does and compare its lines with the changelog section:

       slot --timeout 30m task check-runner

   The gate does not run test262 in full; it compares the digest of the run's sources with
   `conformance/test262/expected.digest`. The version change is a source: run
   `slot --timeout 30m task test262-expected` on the commit first (or, where the run does not
   fit one pool job, `slot --timeout 30m task test262-part TEST262_PART=<k>/<n>` for each part
   and then `slot task test262-expected TEST262_FROM_PARTS=.out/test262/parts`), and
   `slot --local task site-data`, and commit the files they write with steps 1 and 2.

4. **Notes.** Write the notes file, `.out/release-<version>-notes.md`, in the form the
   introduction gives: the lead, `**Breaking**`, `**Changed**`, the link line. Count its
   characters without the link line (at most 900) and check each statement against the
   changelog section. `**Changed**` has one bullet that says whether the macOS and Windows
   binaries are signed: signed when the twelve signing secrets ("Signing" below) are set,
   unsigned otherwise. The same file is the tag's message (step 6) and the release's notes
   (step 7). The branch tip, the path of the notes file, its character count and the
   gate's lines are sent to the deciding session. On its answer `main` is moved to the commit
   (fast-forward).
5. **Publish.** The deciding session checks and verifies the commit, pushes `main`, and waits
   for the `check` run and the `slow` run on it to be green. The lane `slow` makes the full
   test262 run and the full counts of the heavy tests; a release is not tagged on a commit
   whose `slow` run is red or missing:

       $gates gh -- run list --workflow check.yml --branch main --limit 1
       $gates gh -- run watch <run-id> --exit-status
       $gates gh -- run list --workflow slow.yml --branch main --limit 1
       $gates gh -- run watch <run-id> --exit-status

   The lane starts on a push only when the push changes a path of its list. When the push of
   the tip changed no such path (a tip that changes prose only), the deciding session starts
   the lane by hand on `main` at that tip and waits for that run as for one a push started;
   a run of the lane on an earlier commit does not stand in for it. A run started by hand
   takes some seconds to appear in the list; wait on the run whose `headSha` is the tip, and
   a run with another `headSha` is not the one to wait for:

       $gates gh -- workflow run slow.yml --ref main
       $gates gh -- run list --workflow slow.yml --branch main --event workflow_dispatch \
         --limit 5 --json databaseId,headSha,event,status
       $gates gh -- run watch <run-id> --exit-status

6. **Tag.** Scan the notes file, then tag the commit with it as the message and push the
   tag. `secret list` is for information: it prints the names and dates of the secrets that
   are set, never values, and shows whether the run will sign; when it disagrees with the
   notes' signing bullet, the bullet is corrected before the tag. `--cleanup=whitespace`
   keeps a line of the notes that starts with `#`. Without it git removes such a line:

       $gates gh -- secret list --repo beyond10x/swell
       $gates scan-text .out/release-<version>-notes.md
       $gates bot -- tag --cleanup=whitespace -a <version> \
         -F .out/release-<version>-notes.md <commit>
       $gates bot -- push origin <version>

   The push starts one run of "Shared source gates" and one run of "Release build" on the
   tag. Wait until both end with success before step 7:

       $gates gh -- run list --workflow shared-gates.yml --limit 1
       $gates gh -- run list --workflow release-build.yml --branch <version> --limit 1 \
         --json databaseId,headSha,status,conclusion
       $gates gh -- run watch <run-id> --exit-status

   With the secrets set, the run signs the macOS and Windows binaries before it packages
   them. Each of the two macOS jobs prints the output of `codesign --verify --strict` and
   `codesign -dv` and a notarization status of `Accepted`; the Windows job prints an
   Authenticode status of `Valid`. Without them, each of those jobs prints one warning,
   `<target> is not signed in this run`, with the names of the secrets that are not set, and
   goes on unsigned. A red job of "Release build" holds the release. Its cause is fixed on
   `main` and released as the next patch version; the tag is not moved. A job that failed
   because a signing secret was wrong or had expired is run again
   (`$gates gh -- run rerun <run-id> --failed`) once the secret is fixed; the tag is not
   moved.

7. **Release.** Download the five archives of the tag's "Release build" run, write
   `SHA256SUMS`, create the release as the bot with the six files as assets, and view it
   ("The binary of a release" names the archives):

       rm -rf .out/release-<version> && mkdir -p .out/release-<version>
       $gates gh -- run download <run-id> --repo beyond10x/swell \
         --dir .out/release-<version>
       cd .out/release-<version>
       mv */swell-<version>-* . && rmdir swell-<version>-*/
       ls swell-<version>-* | wc -l
       sha256sum swell-<version>-* > SHA256SUMS
       $gates gh -- release create <version> --repo beyond10x/swell --verify-tag \
         --title "swell <version>" \
         --notes-file ../release-<version>-notes.md \
         swell-<version>-* SHA256SUMS
       $gates gh -- release view <version> --repo beyond10x/swell \
         --json author,tagName,isDraft,assets --jq '{author: .author.login, tagName, isDraft, assets: [.assets[].name]}'

   `ls` counts five archives. The release is done when its author is `b10x-bot[bot]`, its tag
   is `<version>`, it is not a draft and its assets are six: the five archives and
   `SHA256SUMS`.
8. **Report.** The deciding session reports the version, the tag's commit and the release's
   address to the coordinating session, which merges `main` into the open wave branch.

A tag is never moved and a release is never rewritten: a defect found after step 6 is fixed on
`main` and released as the next patch version.

## The host interface in the changelog

Every change of the host interface since the last release is named under `### Host interface`
of `## [Unreleased]` in `CHANGELOG.md`; `task host-surface-check`, a step of `check`, fails
until it is, one line per name. The host interface is every public item of
`crates/swell/src/value.rs` and `crates/swell/src/host/*.rs` and every `**HOST-<n>**` line of
`docs/design/host-interface.md`. `conformance/host/surface.txt` holds it as it was at the last
release; `task host-surface-scan` prints it for the tree. An item changes when it is added or
removed or its signature or its attributes `derive`, `cfg`, `non_exhaustive` or `repr` change;
a requirement changes when its line does. A doc comment, a private item and a body change
nothing. A rename is a removal and an addition, and both names are written.

Each name stands in backticks, in this form and no other; another form of the same item is not
found:

- a function or method: its bare name (`charge_steps`); where two types of the interface have
  a method of that name, with its type before it (`Agent::host`);
- a method of `scoped_methods!`, which `Cx` and `HookCx` both have: once, by its bare name
  (`host`, `get`);
- a variant or a field: `Type::name` (`HostError::StaleHandle`, `ErrorSite::line`);
- a type, trait, constant or type alias: its name (`HostBytes`);
- a requirement: `HOST-<n>`.

## The binary of a release

`.github/workflows/release-build.yml` runs on every pushed tag of the form
`<major>.<minor>.<patch>`, with anything after it, on a pull request that changes the
workflow, and when started by hand. It has one job per target, each on a runner of that
system, with the compiler of `rust-toolchain.toml`:

| target | runner | archive |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | `swell-<tag>-x86_64-unknown-linux-gnu.tar.gz` |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | `swell-<tag>-aarch64-unknown-linux-gnu.tar.gz` |
| `x86_64-apple-darwin` | `macos-15-intel` | `swell-<tag>-x86_64-apple-darwin.tar.gz` |
| `aarch64-apple-darwin` | `macos-15` | `swell-<tag>-aarch64-apple-darwin.tar.gz` |
| `x86_64-pc-windows-msvc` | `windows-2022` | `swell-<tag>-x86_64-pc-windows-msvc.zip` |

Each job:

1. Builds `cargo build --release --locked -p swell-cli --target <target>`, with the runner's
   home directory and the checkout remapped by `--remap-path-prefix`.
2. On macOS and Windows, signs the binary ("Signing" below).
3. Fails when the binary still holds either path.
4. Packages the binary with `README.md`, `LICENSE` and `NOTICE` in a directory
   `swell-<tag>-<target>/`. The archive and `SHA256SUMS` hold the signed binary.
5. Unpacks the archive in a fresh directory and runs the smoke test there, on the binary as
   packaged, signed or not:
   - `swell --version` prints `swell <version>`, where the tag is `<version>` or
     `<version>-<suffix>`;
   - `swell -e "print(1 + 1)"` prints `2`;
   - `swell --steps 10000 -e "for (;;) {}"` exits 3, the step budget.
6. Uploads the archive as a workflow artifact named `swell-<tag>-<target>`.

A tag with a suffix, `<version>-<suffix>` on a branch, tests the workflow without a release.
On a pull request or a run started by hand on
a branch, `<tag>` is the workspace version and the first seven characters of the commit
(`0.12.0-1a2b3c4`).

## Signing

The macOS and Windows binaries of a release are signed when the signing secrets are set and
unsigned otherwise; the release notes say which (step 4). Signing comes before Package, so
the archive, its smoke test and `SHA256SUMS` all see the signed binary.

- **macOS** (both jobs): the job imports the Developer ID Application certificate into a
  temporary keychain with a random password, finds the identity of the team
  `APPLE_TEAM_ID` names in it, and runs
  `codesign --force --options runtime --timestamp` on `swell` (the hardened runtime and a
  secure timestamp). It prints `codesign --verify --strict --verbose=2` and
  `codesign -dv --verbose=2` and fails unless the authority is a Developer ID Application,
  the flags hold `runtime` and the signature has a timestamp. It then zips the binary with
  `ditto` and sends it to Apple with `xcrun notarytool submit --wait` and the App Store
  Connect API key, prints the submission id and its status, and fails on any status but
  `Accepted` (it prints the notary log first). A bare binary cannot be stapled, so
  Gatekeeper looks the ticket up online on first run. The last step of the job deletes the
  keychain and the key files, also when an earlier step failed.
- **Windows**: the job signs `swell.exe` with Azure Artifact Signing (formerly Trusted
  Signing) through the action `azure/artifact-signing-action`, pinned by commit (`v2.0.0`),
  SHA-256 file digest and the service's RFC 3161 timestamp
  (`http://timestamp.acs.microsoft.com`). The service's certificates are valid for three
  days, so the timestamp is what keeps the signature valid. The job then reads the signature
  with `Get-AuthenticodeSignature`, prints its status, signer and timestamp, and fails unless
  the status is `Valid` and a timestamp is present.

Which jobs sign is decided first in each macOS and Windows job:

- All six secrets of the job's system set: the job signs, on any event. A signing step that
  fails (a wrong or expired secret, a notarization status other than `Accepted`, a signature
  that is not `Valid`) fails the job.
- One or more not set, on any event, tags included: the signing steps do nothing. The job
  prints one line, `<target> is not signed in this run; these signing secrets are not set:`
  and the names (names only; a warning on a tag), and builds, packages and smoke-tests the
  unsigned binary. A pull request from a fork never sees the secrets.

The Linux jobs do not sign.

### The secrets

Each value is a GitHub Actions secret of the repository (Settings, Secrets and variables,
Actions), set by the owner. None is in the repository. The decision step sees only whether
each is set. The signing steps get the values as environment variables or action inputs,
write the `.p12` and the `.p8` only into a directory of mode 700 with files of mode 600
(removed by the step, and by the last step of the job, which runs also after a failure), and
print none of them: no step traces its commands, and the `codesign -dv` output shows the
team ID as `<APPLE_TEAM_ID>`. GitHub also masks every secret in the log.

| secret | holds |
|---|---|
| `APPLE_DEVELOPER_ID_P12_BASE64` | the Developer ID Application certificate with its private key, exported from Keychain Access as `.p12`, encoded with `base64` |
| `APPLE_DEVELOPER_ID_P12_PASSWORD` | the password of that `.p12` |
| `APPLE_TEAM_ID` | the ten-character Team ID of the Apple Developer account; the job signs with the identity of this team |
| `APPLE_NOTARY_KEY_ID` | the key ID of an App Store Connect API key (role Developer) |
| `APPLE_NOTARY_ISSUER_ID` | the issuer ID of the App Store Connect API keys of the account |
| `APPLE_NOTARY_KEY_P8` | the text of that key's `.p8` file, `-----BEGIN PRIVATE KEY-----` to `-----END PRIVATE KEY-----` |
| `AZURE_TENANT_ID` | the Microsoft Entra tenant (directory) ID |
| `AZURE_CLIENT_ID` | the client (application) ID of the app registration that signs |
| `AZURE_CLIENT_SECRET` | a client secret of that app registration |
| `AZURE_SIGNING_ENDPOINT` | the endpoint of the Artifact Signing account's region, such as `https://weu.codesigning.azure.net/` |
| `AZURE_SIGNING_ACCOUNT` | the name of the Artifact Signing account |
| `AZURE_SIGNING_CERTIFICATE_PROFILE` | the name of its Public Trust certificate profile |

The app registration has the role "Artifact Signing Certificate Profile Signer" on the
certificate profile and no other role. Set a secret with `gh secret set <name>
--repo beyond10x/swell` and the value on standard input, never as an argument; for the
`.p12`, `base64 < identity.p12 | gh secret set APPLE_DEVELOPER_ID_P12_BASE64 --repo
beyond10x/swell`.

### Rotating them

A rotation sets the new value before the old one is revoked, and the next tag's run is the
check. Between releases, a run by hand on `main` signs with the new values without a
release: `$gates gh -- workflow run release-build.yml --ref main`.

- **Developer ID certificate** (valid five years): create a new Developer ID Application
  certificate in the Apple Developer account, export it with its key as `.p12` with a new
  password, set `APPLE_DEVELOPER_ID_P12_BASE64` and `APPLE_DEVELOPER_ID_P12_PASSWORD`, and
  delete the exported file. Revoke the old certificate only when it leaked: after a
  revocation Gatekeeper can refuse the binaries of earlier releases signed with it.
- **App Store Connect API key**: create a new key in App Store Connect (Users and Access,
  Integrations), set `APPLE_NOTARY_KEY_ID` and `APPLE_NOTARY_KEY_P8`, then revoke the old
  key. The issuer ID does not change.
- **Azure client secret**: add a new client secret to the app registration in Microsoft
  Entra ID, set `AZURE_CLIENT_SECRET`, then delete the old secret. A client secret expires
  at the date chosen when it is made, at most two years; rotate before that date.
- **Artifact Signing account or profile**: when either is replaced, set
  `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT` and `AZURE_SIGNING_CERTIFICATE_PROFILE`
  and give the app registration the signer role on the new profile. The profile's
  short-lived certificates are renewed by the service and need nothing.
- **Team ID and tenant ID** change only with the account or tenant.

A leaked secret is rotated at once, without waiting for a release.
