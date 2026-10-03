+++
id = "01M411DA13HQ7BGWX2JXW8GMB7"
title = "Linux release binaries need glibc 2.39"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M40YS7F24KWB0TN8WFMBY2D8"
reporter = "lognd"
created = "2026-10-03T14:06:41Z"
updated = "2026-10-03T14:14:30Z"
scope = [".github/workflows/release.yml", "README.md", "docs/**"]

[[acceptance]]
text = "Given the Linux release archives, When the binary is inspected, Then it is statically linked (musl) and the release job fails if it is not"
bound = true

[[acceptance]]
text = "Given a Linux system with an older glibc such as Ubuntu 22.04, When the downloaded clocx runs, Then it starts and prints a report"
bound = true
+++

Found verifying the v0.1.0 release: clocx-0.1.0-aarch64-unknown-linux-gnu fails on Ubuntu 22.04 with GLIBC_2.39 not found, because it was built on an ubuntu-24.04 runner. Build the Linux targets for musl instead, statically linked.
