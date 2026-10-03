# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | yes       |

Only the latest release receives security fixes. Upgrade before filing a
report if you are on an older version.

## Reporting a vulnerability

Please do not file a public GitHub issue for a suspected vulnerability.

Use one of the following, in order of preference:

1. GitHub private vulnerability reporting: open the "Security" tab on
   [lognd/clocx](https://github.com/lognd/clocx), then "Report a
   vulnerability". This opens a private advisory thread that only the
   maintainer and you can see.
2. Email logan@logand.app if you cannot use GitHub's reporting flow.

### What to include

- A minimal reproduction: the command, and a small repository or file
  layout that triggers it.
- The clocx version (`clocx --version`) and how you installed it.
- Your operating system and terminal.
- What you expected versus what happened, and why you believe it is a
  security issue.

## What counts

clocx reads files and git data from the directory you point it at and
writes only to its cache directory (`<user cache dir>/clocx`, or
`--cache-dir`). In scope, for example:

- Reading or writing outside those places.
- Terminal escape injection: a crafted file or directory name changing
  what the terminal shows (names are sanitized in tables; a bypass is a
  bug).
- A crafted repository making clocx run code, follow links out of the
  tree, or exhaust memory or CPU disproportionately.

## Response

You will get an acknowledgement within a week. Fixes are released as a
patch version, with an advisory crediting the reporter unless you ask not
to be named.
