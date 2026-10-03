+++
id = "01M40SM3FPZD9RMETEZ8GM2FQ6"
title = "Workspace, CLI and renderer skeleton"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:35Z"
updated = "2026-10-03T11:57:23Z"
persona = "repository owner"
capability = "run clocx with a single rendering module that owns all output"
outcome_text = "output stays consistent and no stray prints creep in"
scope = ["Cargo.toml", "Cargo.lock", ".gitignore", "crates/clocx/**", "docs/**", "frob.lock"]

[[acceptance]]
text = "Given the workspace, When cargo build runs, Then a clocx binary is produced"
bound = true

[[acceptance]]
text = "Given any module outside render, When clippy runs, Then print_stdout and print_stderr are denied and a test fails on print macros outside render"
bound = true

[[acceptance]]
text = "Given stdout is not a terminal or NO_COLOR is set, When clocx renders, Then no ANSI escapes are written"
bound = true
+++
