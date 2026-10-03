+++
id = "01M40YS7JK42X1Q5E75V9PSN9E"
title = "CI matrix and tagged releases to GitHub and crates.io"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M40YS7F24KWB0TN8WFMBY2D8"
reporter = "lognd"
created = "2026-10-03T13:20:46Z"
updated = "2026-10-03T13:31:42Z"
persona = "repository owner"
capability = "get every push checked on three platforms and every version tag released as binaries and a crate"
outcome_text = "users can install clocx without building it and breakage shows before release"
scope = [".github/workflows/**", ".github/dependabot.yml", "rust-toolchain.toml", "Cargo.toml", "Cargo.lock", "crates/clocx/**", "docs/**", "changelog.d/**", "CHANGELOG.md"]

[[acceptance]]
text = "Given a push or pull request, When CI runs, Then formatting, clippy with warnings denied, the MSRV build and the tests on Linux, macOS and Windows must all pass"
bound = true

[[acceptance]]
text = "Given a tag vX.Y.Z matching the crate version on a commit with green CI, When the release workflow runs, Then archives with checksums for five targets are attached to a GitHub release"
bound = true

[[acceptance]]
text = "Given the same tag, When the release workflow runs, Then the crate is published to crates.io after the GitHub release"
bound = true

[[acceptance]]
text = "Given the crate package, When cargo package runs, Then it builds with complete crates.io metadata and the README"
bound = true
+++
