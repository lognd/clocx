+++
id = "01M40SM3QAK31AZ48JG48EWRC6"
title = "Live dashboard view"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:36Z"
updated = "2026-10-03T12:26:56Z"
persona = "repository owner"
capability = "leave a full-screen dashboard open that refreshes on file and git changes"
outcome_text = "I can watch progress on a second screen"
scope = ["crates/clocx/**", "docs/**", "Cargo.lock", "frob.lock", "changelog.d/**"]

[[acceptance]]
text = "Given --live, When clocx starts, Then a full-screen dashboard of the same report is drawn and q quits"
bound = false

[[acceptance]]
text = "Given a file edit or new commit, When the debounce elapses, Then the dashboard refreshes"
bound = false

[[acceptance]]
text = "Given a refresh, When most files are unchanged, Then only changed files are recounted"
bound = false
+++
