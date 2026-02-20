---
id: S-016
title: Package manager distribution
type: story
status: Active
created: 2026-02-20
---

# S-016: Package manager distribution

## Problem

After the launch-phase distribution (GitHub Releases + cargo install) is solid, Lisa needs to be available through the package managers that the target audience actually uses: Homebrew for macOS users, Nix for NixOS/Nix users, and AUR for Arch Linux users.

## Goal

Publish Lisa through Homebrew tap, Nix flake, and AUR. Each entry should reference prebuilt GitHub Release binaries (not compile from source) and declare Claude Code and Zellij as dependencies where the format supports it.

## Tickets

- **T-016-01:** Create Homebrew tap (task)
- **T-016-02:** Add Nix flake to repository (task)
- **T-016-03:** Publish AUR package (task)

## Dependencies

```
S-014 (distribution infrastructure) must be complete first.

T-016-01 (Homebrew tap)
T-016-02 (Nix flake)
T-016-03 (AUR package)
```

All three tickets are independent of each other but all depend on S-014 being done (they need working GitHub Releases to point at).

## Success Criteria

1. `brew tap johnhkchen/lisa && brew install lisa` works on macOS
2. `nix profile install github:johnhkchen/lisa` works
3. AUR `-bin` package installs from GitHub Release binary
4. All three declare Claude Code and Zellij as dependencies where possible
5. Version updates are straightforward (ideally automated via cargo-dist or CI)
