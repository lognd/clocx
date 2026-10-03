+++
id = "01M40SKJNHMPP34A591K4MKVXS"
title = "clocx: modern code-progress tool in Rust"
type = "epic"
category = "todo"
priority = "medium"
points = 13
reporter = "lognd"
created = "2026-10-03T11:50:18Z"
updated = "2026-10-03T11:50:18Z"
persona = "repository owner"
capability = "glance at where code is, how much, and where work is happening now"
outcome_text = "I can follow coding progress across many agent worktrees"
scope = ["Cargo.toml", "crates/**", "docs/**"]

[[acceptance]]
text = "Given the clocx binary, When run in a git repository, Then it prints totals, recent activity and worktree status in one report"
bound = false
+++

Rust Cargo workspace in this fork; tokei for counting, gix for git, ratatui for the live view, notify for watching. GPL-2.0. Binary name clocx (owner decision 2026-10-03).
