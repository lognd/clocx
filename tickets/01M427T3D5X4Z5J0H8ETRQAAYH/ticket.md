+++
id = "01M427T3D5X4Z5J0H8ETRQAAYH"
title = "Publish clocx to PyPI as binary wheels"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M422757XQAA9PJ8601NCEV1X"
reporter = "lognd"
created = "2026-10-04T01:17:46Z"
updated = "2026-10-04T01:38:27Z"
persona = "clocx user with Python but no Rust toolchain"
capability = "uvx clocx / uv tool install clocx / pipx install clocx"
outcome_text = "I get a prebuilt clocx without compiling it"
scope = [".github/**", "pyproject.toml", "docs/**", "README.md", "changelog.d/**", ".gitignore", "frob.toml"]

[[acceptance]]
text = "Given pyproject.toml, When maturin builds the project, Then it produces a py3-none wheel holding the clocx binary whose version equals the crate version"
bound = true

[[acceptance]]
text = "Given a pushed tag vX.Y.Z, When the release workflow runs, Then it builds wheels for every release platform and an sdist, smoke-tests the native wheels, and publishes them to PyPI by trusted publishing, skipping files already there"
bound = true

[[acceptance]]
text = "Given docs/releasing.md and README.md, When a maintainer or user reads them, Then they describe the one-time PyPI setup and the uv/pipx install"
bound = true
+++

Package the clocx binary as wheels with maturin (bindings = bin), version taken from Cargo.toml. The release workflow builds manylinux and musllinux (x86_64, aarch64), macOS (x86_64, arm64) and Windows x86_64 wheels plus an sdist, smoke-tests the native ones, and publishes to PyPI by trusted publishing (environment pypi) after the GitHub release. The PyPI pending-publisher setup is an owner action.
