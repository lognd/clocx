+++
id = "01M40X2WJN2EYZ0KPNQCD6C3VR"
title = "Count tracked dotfiles and Python docstrings like cloc"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T12:51:05Z"
updated = "2026-10-03T12:59:52Z"
scope = ["crates/clocx/**", "docs/**", "changelog.d/**", "frob.lock"]

[[acceptance]]
text = "Given a tree with source files under a hidden directory such as .github, When clocx runs, Then they are counted while .git and gitignored paths stay excluded"
bound = true

[[acceptance]]
text = "Given a Python file with docstrings, When clocx counts it, Then docstring lines count as comments, as cloc does"
bound = true

[[acceptance]]
text = "Given a count cache written by an earlier version, When clocx runs, Then the stale cache is discarded and files are recounted"
bound = false
+++

Found in the parity check against Perl cloc (--vcs=git) on typani, mdcat, lograder and frob: YAML under .github was missing and Python code was 26-43% above cloc because docstrings counted as code.
