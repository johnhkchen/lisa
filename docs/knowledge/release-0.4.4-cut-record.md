# Stable cut record — v0.4.4

Field report for the checklist in [release-checklist.md](release-checklist.md).
Evidence gathered live during the cut; every value below is from the recorded
evidence files, none assumed.

```text
release: v0.4.4
cut_at: 2026-07-19T18:15:42Z
operator: John Chen (authorization) / Claude (checklist walk)
release_commit: 9f21d0aaf18aaaaa3f78c721122be351d6d7797a
release_run_url: https://github.com/johnhkchen/lisa/actions/runs/29698178112
latest_api_tag: v0.4.4
latest_prerelease: false
ancestry_gates: e045=ancestor / musl=ancestor / seal=ancestor  (pre-push and re-proven on the public tag)
asset_audit: 18 assets — all four platform archives + sha256s, installer, lisa.rb, all four .deb packages, sha256.sum, source tarball; zero unknown-linux-gnu
aarch64_musl_bullseye_step: success
x86_64_musl_bullseye_step: success
readme_installer_path: releases/latest/download/lisa-cli-installer.sh (isolated $HOME, no ~/.cargo)
installed_version: lisa 0.4.4
homebrew_version: 0.4.4
apt_fresh_version: 0.4.4 (bookworm container, keyring + pinned source, runtime at /usr/libexec/lisa/zellij)
apt_upgrade_from_prior: before=lisa 0.4.3 after=lisa 0.4.4  (true prior-stable upgrade — the audit item deferred from the 0.4.3 cut is closed with a real transition, not the equal-to-fresh fallback)
channel_skew: eliminated
```

All ten required Release jobs succeeded, including `publish-apt-repository`
(stable-only; a skip would have failed the cut) and `announce`.

## What this stable carries beyond v0.4.3

The 0.4.4 line was hardened across eleven release candidates in ~48 hours,
every one falsified or confirmed against real emulated-Chromebook legs:

- **Seal ladder + park machinery** (E-048/E-049): journal-tier completion for
  history-less projects, level-triggered parking, bounded completion failures,
  plain-language asks — five consecutive clean field legs, zero stuck-at-REV.
- **Common-sense defaults** (E-050): decide-don't-ask init, config upsert,
  client autodetect, the flag audit.
- **Session-per-ticket** (rc.8–rc.10): both native clients end their session at
  each ticket boundary; fresh per-ticket identity every launch; finished
  sessions rest before retirement so no exit can destroy an in-flight usage
  capture.
- **True token accounting** (E-051, rc.11): captures durable before the stop
  signal, late-joined to the ledger as append-only corrections,
  latest-snapshot-per-session (never the double-counted sum), surfaced
  per-ticket by `lisa status` with an honest not-yet-joined gap line.
- **Honest gates** (E-051): `just check` runs the fmt + clippy + WASM gates CI
  enforces; the bounded-runner flake is dead; the in-place `/clear` machinery
  is retired with field evidence cited.

Known open, deliberately not stable-blocking: T-051-01-03 (checksum-test
flake under parallel load — test-only).
