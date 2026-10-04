+++
id = "01M42BBZH92CZ22CEYJAPPNKCW"
title = "Release commit from frob release cut --push gets no CI run"
type = "bug"
category = "todo"
priority = "medium"
points = 1
parent = "01M422757XQAA9PJ8601NCEV1X"
reporter = "lognd"
created = "2026-10-04T02:19:58Z"
updated = "2026-10-04T02:19:58Z"
scope = [".github/workflows/ci.yml", "docs/releasing.md"]

[[acceptance]]
text = "Given a pushed clocx-vX.Y.Z tag whose commit is not a branch head, When the tag is pushed, Then CI runs on the tagged commit and the release plan job finds it"
bound = false
+++

frob release cut --push pushes the release commit followed by ledger commits, so the pushed branch head is a ledger commit and GitHub runs CI only there. The release plan job needs CI on the tagged commit and gave up after its grace period; 0.2.1 went out only because CI was dispatched by hand on the tag. Run CI on clocx-v* tag pushes too, so the tagged commit always has its own run.
