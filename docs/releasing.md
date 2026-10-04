# Releasing clocx

How-to for maintainers: cut a version and publish it to GitHub releases,
crates.io and PyPI. Pushing a tag does everything after step 4; the workflow
is `.github/workflows/release.yml`. Release tags are `clocx-vX.Y.Z` (also
`[release] tag` in `frob.toml`); `v0.1.0` and `v0.1.1` predate that.

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
4. On pypi.org, add a trusted publisher for the project `clocx` (a
   pending publisher before the first upload): owner `lognd`, repository
   `clocx`, workflow `release.yml`, environment `pypi`. In the GitHub
   repository, create the environment `pypi` (no secrets needed; a
   required reviewer is optional, as for `crates-io`).

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

       git tag -a clocx-vX.Y.Z -m "clocx X.Y.Z"
       git push origin clocx-vX.Y.Z

   Then record the cut in the ledger with
   `frob release adopt X.Y.Z --reason "tagged by the release how-to"`.

The release workflow then:

- checks the tag matches the crate version and that CI passed on that
  exact commit (it waits for a running CI and refuses a red one);
- builds `clocx` for Linux (x86_64, arm64; static musl binaries that
  run on any distribution, checked with `file`), macOS (arm64, x86_64)
  and Windows (x86_64), runs each binary it can, and packages
  `clocx-X.Y.Z-<target>.tar.gz` (`.zip` on Windows) with a `.sha256`
  file; each archive holds the binary, README, LICENSE, CHANGELOG and the
  reference (`clocx.md`);
- creates the GitHub release `vX.Y.Z` with those archives;
- publishes the crate to crates.io (skipped if that version is already
  there, so a re-run is safe);
- builds PyPI wheels with maturin (`pyproject.toml`, `bindings = "bin"`:
  each wheel holds just the binary): manylinux and musllinux for x86_64
  and arm64, macOS arm64 and x86_64, Windows x86_64, plus an sdist; installs
  and runs each wheel its runner can, and uploads them all to PyPI by
  trusted publishing (files already there are skipped). The PyPI version is
  the crate version, spelled the PEP 440 way for pre-releases
  (`0.3.0-rc.1` is `0.3.0rc1`).

A tag with a pre-release suffix (`clocx-v0.3.0-rc.1`) makes a GitHub
pre-release; crates.io treats the version as a pre-release too.

## Dry run

Run the Release workflow by hand from the Actions tab (Run workflow, on
`master`) to exercise everything short of publishing: the CI gate, the
five builds with their smoke tests, the archives, and the wheels and sdist
(all downloadable from the run). It creates no release and publishes nothing.

## If a release fails

Re-run the failed jobs from the Actions tab: every job is safe to repeat
(the release job uploads missing assets to an existing release, the
crates.io job skips a version crates.io already has, and the PyPI job skips
files PyPI already has). A version that reached crates.io or PyPI cannot be
replaced, only yanked; fix forward with a new patch version.
