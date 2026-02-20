---
id: T-016-01
title: Create Homebrew tap
type: task
phase: done
status: done
priority: medium
story: S-016
created: 2026-02-20
depends_on:
  - T-014-03
---

# T-016-01: Create Homebrew tap

## Objective

Publish Lisa as a Homebrew formula via a dedicated tap so macOS users can install with `brew install`.

## Requirements

### Tap repository

Create a new GitHub repo: `johnhkchen/homebrew-lisa`

### Formula

Write a formula that:
- Downloads the prebuilt binary from GitHub Releases (not source build)
- Selects the correct binary for the platform (Intel vs Apple Silicon)
- Declares dependencies: `depends_on "zellij"`
- Claude Code can't be expressed as a Homebrew dependency — add a caveat instead
- Includes a test block that runs `lisa --help`

### Install flow

```sh
brew tap johnhkchen/lisa
brew install lisa
```

### Automation

Consider using cargo-dist's Homebrew integration if available, or set up a GitHub Action that updates the formula on each release.

## Acceptance Criteria

- [ ] `homebrew-lisa` tap repo exists on GitHub
- [ ] `brew tap johnhkchen/lisa && brew install lisa` works on macOS
- [ ] Formula points to prebuilt release binaries
- [ ] Zellij declared as dependency
- [ ] `lisa --help` works after install
