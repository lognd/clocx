# Releasing clocx

How-to for maintainers: cut a version and publish it to GitHub releases and
crates.io. Pushing a tag does everything after step 4; the workflow is
`.github/workflows/release.yml`.

## One-time setup

1. On crates.io, create an API token with the `publish-new` and
   `publish-update` scopes for the `clocx` crate.
2. In the GitHub repository, open Settings, Environments, create an
   environment named `crates-io`, and add the token as the secret
   `CARGO_REGISTRY_TOKEN`. Optionally add yourself as a required reviewer,
   so every publish waits for your approval.
3. After the first publish, crates.io trusted publishing (OIDC) can replace
   the token: add this repository and `release.yml` as a trusted publisher
   on the crate's settings page, then switch the `crates-io` job to
   `rust-lang/crates-io-auth-action` and delete the secret.

## Cutting version X.Y.Z

1. Compile the fragments in `changelog.d/` into a new section of
   `CHANGELOG.md`, directly under `## [Unreleased]`:

       ## [X.Y.Z] - YYYY-MM-DD

       ### Added
       - ...

   Group entries by the fragment kind in the file name (`added`,
   `changed`, `fixed`, `removed`, `deprecated`, `security`), then delete
   the compiled fragments. The release notes are taken from this section,
   and the release fails without it.
2. Set `version = "X.Y.Z"` under `[workspace.package]` in `Cargo.toml` and
   run `cargo check` so `Cargo.lock` follows.
3. Commit as `chore(release): X.Y.Z`, push to `master`, and let CI finish.
4. Tag the commit and push the tag:

       git tag -a vX.Y.Z -m "clocx X.Y.Z"
       git push origin vX.Y.Z

The release workflow then:

- checks the tag matches the crate version and that CI passed on that
  exact commit (it waits for a running CI and refuses a red one);
- builds `clocx` for Linux (x86_64, arm64), macOS (arm64, x86_64) and
  Windows (x86_64), runs each binary it can, and packages
  `clocx-X.Y.Z-<target>.tar.gz` (`.zip` on Windows) with a `.sha256`
  file; each archive holds the binary, README, LICENSE, CHANGELOG and the
  reference (`clocx.md`);
- creates the GitHub release `vX.Y.Z` with those archives;
- publishes the crate to crates.io (skipped if that version is already
  there, so a re-run is safe).

A tag with a pre-release suffix (`v0.2.0-rc.1`) makes a GitHub
pre-release; crates.io treats the version as a pre-release too.

## If a release fails

Re-run the failed jobs from the Actions tab: every job is safe to repeat
(the release job uploads missing assets to an existing release, and the
publish job skips a version crates.io already has). A version that reached
crates.io cannot be replaced, only yanked; fix forward with a new patch
version.
