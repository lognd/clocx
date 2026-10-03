+++
id = "01M40YS7NDDKEDSJ42SQDPA4CM"
title = "Project front page: banner, badges and community files"
type = "docs"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M40YS7F24KWB0TN8WFMBY2D8"
reporter = "lognd"
created = "2026-10-03T13:20:46Z"
updated = "2026-10-03T13:32:07Z"
scope = ["README.md", "docs/**", "CONTRIBUTING.md", "SECURITY.md", "CODE_OF_CONDUCT.md", "CHANGELOG.md", ".github/ISSUE_TEMPLATE/**", ".github/PULL_REQUEST_TEMPLATE.md", "changelog.d/**"]

[[acceptance]]
text = "Given the README, When rendered on GitHub or crates.io, Then it opens with a banner, badges for CI, crates.io, license and MSRV, an install section and a rendered sample report"
bound = false

[[acceptance]]
text = "Given a new contributor, When they open the repository, Then CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, issue forms and a pull request template are present and describe this project"
bound = false
+++
