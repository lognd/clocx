+++
id = "01M40WNC6J55Z9D0EFTNPA6CW6"
title = "Remove frob:accept PROC001 workarounds now that frob scopes the rule"
type = "chore"
category = "todo"
priority = "medium"
points = 1
parent = "01M40SKJNHMPP34A591K4MKVXS"
reporter = "lognd"
created = "2026-10-03T12:43:43Z"
updated = "2026-10-03T12:43:43Z"
scope = ["crates/clocx/**", "frob.lock"]

[[acceptance]]
text = "Given frob with PROC001 scoped to declaring repositories, When check --ticket runs on crates/clocx, Then no frob:accept PROC001 directive remains and the check passes with no PROC001 or EXC finding"
bound = false
+++

frob ~XGS6PK2 made PROC001 apply only to repositories declaring [check] process_spawners; the accepts added on 2026-10-03 are dead.
