+++
id = "01M40SM3KHP7KNC8JZ7NDZ230M"
title = "Recent change activity from git history"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:35Z"
updated = "2026-10-03T11:50:35Z"
persona = "repository owner"
capability = "see lines added and removed and files touched over the last hour, day, 7 and 30 days by directory"
outcome_text = "I see where work is happening right now"
scope = ["crates/clocx/**", "docs/**", "Cargo.lock"]

[[acceptance]]
text = "Given commits at known times, When clocx runs, Then added, removed, files and commits are reported for 1h, 24h, 7d and 30d windows"
bound = false

[[acceptance]]
text = "Given commits touching several directories, When clocx runs, Then activity is broken down per directory at the configured depth"
bound = false

[[acceptance]]
text = "Given activity in a window, When clocx renders, Then a trend sparkline of bucketed changes is shown"
bound = false
+++
