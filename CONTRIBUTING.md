# Contributing to clocx

Thanks for considering a contribution. This document has two paths
through it: a short summary if you already know how open-source
contribution works, and a full walkthrough if this is your first time.
Both point at the same commands.

## TL;DR for experienced contributors

- Fork the repo, clone your fork, add `lognd/clocx` as `upstream`.
- Install Rust with [rustup](https://rustup.rs); the toolchain is pinned in
  `rust-toolchain.toml` and installs itself on the first `cargo` command.
- The gate, green before you open a PR:

  ```bash
  cargo fmt --all --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo test --locked
  ```

- Commit format: `<type>(<scope>): <imperative summary, 72 chars max>`.
  Types: `feat`, `fix`, `chore`, `refactor`, `test`, `docs`, `perf`,
  `ci`, `build`. No trailing period. One logical change per commit.
- ASCII only in every file, no exceptions (CI checks it).
- All output goes through `crates/clocx/src/render`. Clippy denies
  `print!`/`println!`/`eprintln!` everywhere else, and a test fails on them.
  Diagnostics go through `tracing`.
- Fallible operations return a typed `Result` with a `thiserror` error;
  `unwrap`/`expect` are for tests and impossible states only.
- Every public item gets a one-line doc comment saying why it exists or
  what it is for, not a restatement of its name.
- User-visible changes get a changelog fragment in `changelog.d/` and an
  update to `docs/clocx.md` in the same change.
- Fill in every box of `.github/PULL_REQUEST_TEMPLATE.md`.

## Your first contribution (step by step)

If you have never opened a pull request against someone else's project
before, this section is for you. None of these steps are specific to
clocx; they are the standard GitHub flow, spelled out.

### 1. Fork the repository

On [github.com/lognd/clocx](https://github.com/lognd/clocx), click "Fork".
You get your own copy under your account, which you can push to.

### 2. Clone your fork

```bash
git clone https://github.com/<you>/clocx
cd clocx
git remote add upstream https://github.com/lognd/clocx
```

### 3. Install Rust

Install rustup from [rustup.rs](https://rustup.rs). You do not need to
pick a version: the first `cargo` command in the checkout installs the
toolchain named in `rust-toolchain.toml`, with rustfmt and clippy.

### 4. Build and run the tests once, to see green

```bash
cargo test
```

The first build downloads and compiles the dependencies; later builds are
fast. Everything should pass before you change anything. If it does not,
open an issue with the output: that is a bug in itself.

### 5. Try it on a repository you know

```bash
cargo run -- ~/some/repo
cargo run -- --live ~/some/repo
```

### 6. Make a small change

Create a branch, make the change, and add a test that fails before it and
passes after it.

```bash
git switch -c fix/short-description
```

### 7. Run the gate

```bash
cargo fmt --all
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

### 8. Commit your change

```bash
git commit -m "fix(totals): count files in hidden directories"
```

Follow the format above. If the change is user-visible, add a fragment
named `changelog.d/<anything-unique>.<kind>.md`, where kind is `added`,
`changed`, `fixed`, `removed`, `deprecated` or `security`, holding one
sentence about the change from a user's point of view.

### 9. Push a branch and open the PR

```bash
git push -u origin fix/short-description
```

GitHub prints a link to open a pull request against `lognd/clocx`. Fill in
the template.

### 10. What review looks like

Expect questions and requests for changes; they are about the code, not
about you. Push new commits to the same branch to answer them. Do not
force-push over commits someone has already reviewed unless asked to.

### 11. If CI is red

Click the failed job to read its log. CI runs the same commands as the
gate above, plus a build with the minimum supported Rust version and a
crates.io package dry run, on Linux, macOS and Windows. A failure on one
platform only is usually a path or line-ending assumption.

## What makes a good change

- **Small scope.** One logical change per PR.
- **A test that fails before and passes after.** Unit tests sit next to
  the code; end-to-end tests that run the binary live in
  `crates/clocx/tests/`. Tests that need git history build it with fixed
  dates (see `git::testrepo`).
- **Docs in the same change.** If behaviour a user can see changes, update
  `docs/clocx.md` (and the README if it is mentioned there).
- **Output stays in one place.** New output means new data in
  `crates/clocx/src/model.rs` and new drawing in `render/sections.rs`;
  the text, JSON and live views all build from those.
- **No new dependencies without discussion.** Open an issue first.
- **Performance claims come with numbers.** Time `clocx` on a large
  repository before and after, cold (`--no-cache`) and warm.

## How this repo is tracked: frob

The maintainer manages work in this repository with frob, a ticket and
gate tool: tickets live in `tickets/` (a git-tracked ledger), and the
`frob:` comments in the source (for example `frob:tests`) bind tests to
code. You do not need frob to contribute: leave existing `frob:` comments
intact, and the maintainer files and links tickets for your change.

## AI-assisted contributions

AI-assisted contributions are welcome under the following policy, which
is deliberately strict. The rules exist so that AI assistance moves work
off reviewers rather than onto them: a generated diff nobody has read is
review work handed to the maintainer, not a contribution.

**(a) Disclosure.** Say in the PR description which tool you used and
roughly how (drafted the patch, reviewed and edited an AI-drafted patch,
wrote tests with AI assistance, and so on). The PR template has a line for
this; fill it in honestly.

**(b) You are the author.** You are responsible for every line, whether
you or a model typed it, and must be able to explain any part of the diff
in review. "The model said so" is not an answer to a review question.

**(c) Not accepted:**

- An unreviewed, unedited AI-generated PR submitted as-is.
- A PR description that was clearly not written for this repository
  (generic boilerplate, wrong project name, invented capabilities).
- A bulk or sweeping change with no prior issue discussing it.
- Generated tests that only assert the current behaviour of the code (a
  test that would pass on a buggy implementation just as easily as on a
  correct one is not a test).

**(d) If you are an AI agent working in this repository:**

- Run the gate above before claiming a task is done. A gate you did not
  run is not a gate you can claim passed.
- Use frob when it is available on the host: work inside a ticket's
  declared scope, record test evidence against its acceptance criteria,
  and land through it.
- Keep every `frob:` comment you encounter intact.
- Never leave a bare `TODO`; describe deferred work in the PR instead.
- ASCII only, in every file you touch.
- Never read or write `.env` files, directly or indirectly.
- Never amend a commit that has already been pushed.
- Follow the commit format exactly. Never add a `Co-Authored-By` trailer
  or any other attribution line to a commit message.

## Reporting bugs and proposing features

Use the issue forms:

- [Bug report](.github/ISSUE_TEMPLATE/bug_report.yml)
- [Feature request](.github/ISSUE_TEMPLATE/feature_request.yml)

For a counting difference, include the file (or a minimal one that shows
it) and both numbers. For a git or worktree problem, include
`git worktree list` and `git status` from the affected worktree.

## Release process (maintainers)

Releases are cut by pushing a `vX.Y.Z` tag; the release workflow checks
that CI passed on the tagged commit, builds binaries for five targets,
creates the GitHub release and publishes the crate. The steps, the
one-time crates.io setup and a dry run are in
[docs/releasing.md](docs/releasing.md).

## License

By contributing you agree that your contribution is licensed under
GPL-2.0-only, the license of this repository.
