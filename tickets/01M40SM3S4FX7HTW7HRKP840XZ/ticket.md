+++
id = "01M40SM3S4FX7HTW7HRKP840XZ"
title = "JSON output of the report"
type = "story"
category = "todo"
priority = "medium"
points = 2
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:36Z"
updated = "2026-10-03T11:50:36Z"
persona = "repository owner"
capability = "get the same report as JSON"
outcome_text = "scripts can consume it"
scope = ["crates/clocx/**", "docs/**"]

[[acceptance]]
text = "Given --json, When clocx runs, Then a single JSON document with totals, activity and worktrees is printed"
bound = false

[[acceptance]]
text = "Given --json, When stdout is a terminal, Then no ANSI escapes appear in the output"
bound = false
+++
