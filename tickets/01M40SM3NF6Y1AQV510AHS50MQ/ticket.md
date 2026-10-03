+++
id = "01M40SM3NF6Y1AQV510AHS50MQ"
title = "Worktree status: branch, uncommitted, ahead, vs base, last activity"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:35Z"
updated = "2026-10-03T12:20:23Z"
persona = "repository owner"
capability = "see every worktree of the repository with its in-progress work"
outcome_text = "work by me and agents is visible before it is merged"
scope = ["crates/clocx/**", "docs/**", "Cargo.lock", "frob.lock", "tests/inputs/issues/280/L/locale_facets.h", "tests/inputs/issues/280/R/locale_facets.h", "tests/inputs/issues/513/L/hello_1.c", "tests/inputs/issues/513/L/locale_facets.h", "tests/inputs/issues/513/R/hello_2.c", "tests/inputs/issues/513/R/locale_facets.h", "changelog.d/**"]

[[acceptance]]
text = "Given several worktrees, When clocx runs, Then each is listed with its branch and time of last activity"
bound = false

[[acceptance]]
text = "Given uncommitted edits in a worktree, When clocx runs, Then lines added and removed against its HEAD are shown"
bound = false

[[acceptance]]
text = "Given a branch with commits beyond the base, When clocx runs, Then commits ahead and total lines added and removed against the base are shown"
bound = false
+++
