+++
id = "01M40SM3V0681C0SZ4TYTN81Y7"
title = "Remove the Perl cloc at parity"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:36Z"
updated = "2026-10-03T13:03:06Z"
persona = "repository owner"
capability = "keep only the Rust tool once it covers what I use"
outcome_text = "the repository has one tool to maintain"
scope = ["cloc", "Unix/**", "tests/**", "sqlite_formatter", "Dockerfile*", "README.md", "changelog.d/**", ".dockerignore", ".devcontainer/**", ".github/**", ".vscode/**", "img/**", ".gitignore", "docs/**", "frob.lock", "Cargo.toml"]

[[acceptance]]
text = "Given the owner confirms parity, When this ticket lands, Then the Perl cloc, its tests and packaging are removed and README describes clocx"
bound = true
+++
