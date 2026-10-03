+++
id = "01M40SM3HJ3HY9DZ27AHYFZM5W"
title = "Totals by language and directory with change since last run"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T11:50:35Z"
updated = "2026-10-03T12:06:36Z"
persona = "repository owner"
capability = "see code, comment and blank lines per language and per top-level directory"
outcome_text = "I know where code is and how much changed since I last looked"
scope = ["crates/clocx/**", "docs/**", "Cargo.lock", "frob.lock", "tests/inputs/issues/280/L/locale_facets.h", "tests/inputs/issues/280/R/locale_facets.h", "tests/inputs/issues/513/L/hello_1.c", "tests/inputs/issues/513/L/locale_facets.h", "tests/inputs/issues/513/R/hello_2.c", "tests/inputs/issues/513/R/locale_facets.h", "changelog.d/**"]

[[acceptance]]
text = "Given a directory tree, When clocx runs, Then it reports files, code, comment and blank lines per language and per directory at the configured depth"
bound = true

[[acceptance]]
text = "Given a previous run, When clocx runs again after edits, Then each row shows the code-line change since that run"
bound = true

[[acceptance]]
text = "Given unchanged files, When clocx refreshes, Then their counts come from the content-hash cache instead of being recounted"
bound = true
+++
