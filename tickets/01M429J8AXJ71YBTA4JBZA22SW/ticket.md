+++
id = "01M429J8AXJ71YBTA4JBZA22SW"
title = "Release with frob release cut"
type = "chore"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M422757XQAA9PJ8601NCEV1X"
reporter = "lognd"
created = "2026-10-04T01:48:26Z"
updated = "2026-10-04T01:49:37Z"
scope = [".github/**", "docs/releasing.md", ".gitattributes", "changelog.d/**"]

[[acceptance]]
text = "Given a CHANGELOG with a frob-written '## X.Y.Z - date' section or a hand-written '## [X.Y.Z] - date' section, When the release workflow extracts the notes, Then it gets that section's body without the frob integrity marker"
bound = true

[[acceptance]]
text = "Given a release commit and tag pushed together, When the plan job finds no CI run for the commit yet, Then it waits a few minutes for one to appear before failing"
bound = true

[[acceptance]]
text = "Given core.autocrlf=true, When frob release cut runs in a fresh checkout, Then it commits and tags without reporting its own edits as local edits"
bound = true

[[acceptance]]
text = "Given docs/releasing.md, When a maintainer cuts X.Y.Z, Then the steps are frob milestone new, frob release status and frob release cut --push"
bound = true
+++

Releases are tagged by hand and adopted because the workflow reads notes only from '## [X.Y.Z]' headings, while frob release cut writes '## X.Y.Z - date' sections (with #### kinds and an integrity marker). Make the workflow read both styles, give the plan job a grace period for a CI run that has not started yet (cut --push pushes the release commit and the tag together), pin text files to LF so frob's checks agree with git under core.autocrlf=true (otherwise cut trips over its own edits), and document frob release cut as the way to release.
