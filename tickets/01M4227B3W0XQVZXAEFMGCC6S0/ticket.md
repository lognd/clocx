+++
id = "01M4227B3W0XQVZXAEFMGCC6S0"
title = "Live progress readout while a one-shot report is computed"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M422757XQAA9PJ8601NCEV1X"
reporter = "lognd"
created = "2026-10-03T23:40:09Z"
updated = "2026-10-03T23:40:11Z"
persona = "clocx owner"
capability = "a progress line that overwrites itself while files are counted, with an ETA"
outcome_text = "I am not staring at a blank screen on a large tree"
scope = ["crates/clocx/**", "docs/clocx.md", "changelog.d/**", "README.md"]

[[acceptance]]
text = "Given stderr is a terminal and the run takes longer than a short delay, When clocx computes a one-shot report, Then a progress line showing the phase, files counted out of the total, rate and ETA overwrites itself in place and is erased before the report is written"
bound = false

[[acceptance]]
text = "Given stderr is not a terminal, or --no-progress, or --live, When clocx runs, Then no progress output is written"
bound = false

[[acceptance]]
text = "Given the progress line is showing, When a tracing event is logged, Then the line is cleared before the log line is written so the two do not interleave"
bound = false
+++

One-shot runs print nothing until the report is ready. Show a single self-overwriting progress line on stderr (phase, bar, done/total, rate, ETA) while walking, counting, reading history and reading worktrees. Compute reports progress through shared atomic counters; only the renderer draws. Log lines must not tear the progress line.
