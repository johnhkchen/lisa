# T-068-01-02 — doctor names the gap between a box and its channel

`lisa doctor` now reports Lisa itself as its first row: the channel this machine
is on, the version installed, and the version that channel resolves to right
now. When those differ the row says `behind` and carries the command that
settles it. `lisa doctor --json` carries the same fields, so the fleet can be
asked from a script instead of read on each box in turn.

What one machine looks like today, against a served release list:

```
  lisa         behind
    channel unset (treated as stable), installed 0.5.0-rc.2, stable resolves to v0.9.0 — this machine is behind its channel
    Remedy: Say which Lisa this machine wants, and move to it in the same command:
    lisa upgrade --channel <name>    (canary, nightly, stable)
  zellij       mode system, version 0.44.3, supported >= 0.43.0, path … OK
```

## What changed

**`crates/lisa-cli/src/freshness.rs` (new).** The comparison itself, kept pure:
given the installed version, what the machine recorded, a release list and a
clock, it returns one of five states — `level`, `behind`, `ahead`, `waiting`,
`unresolved` — plus the row's sentence, the remedy when there is one, and the
JSON object. `ahead` exists because the dev desk builds `main` and is routinely
newer than every channel; calling that drift would have been wrong. `unresolved`
is deliberately not `level`: a machine that could not look has not been told it
is current.

**`crates/lisa-cli/src/doctor.rs`.** A new `CheckResult::Behind` variant renders
`behind` + description + `Remedy:` in the existing row shape (the Zellij row is
the model; `unsupported` would have been a different and wrong claim). The
`lisa` row is built first, ahead of `zellij`. Everything `doctor` looks at is
now gathered once by `gather()`, so the prose report and the `--json` document
cannot disagree about what was found. `run_doctor_json` emits the document.

**`crates/lisa-cli/src/upgrade.rs`.** `fetch_releases_within(timeout)` so
`doctor` can read the same release list on a shorter budget (8s) than `upgrade`
uses (30s). `upgrade`'s behaviour is unchanged.

**`crates/lisa-cli/src/main.rs`.** `lisa doctor --json`, gated and emitted the
way `status --json` is.

**Docs.** `lisa json-guide` gained a `lisa doctor --json` section naming every
field and what each `state` means; `docs/knowledge/flag-audit.md` gained the
`--json` row; `README.md` says in both the channels section and the `lisa
doctor` section that doctor is where you find out where a machine stands.

## How it is tested

- `crates/lisa-cli/src/freshness.rs` — 9 unit tests over the states: behind,
  level, ahead, unset-and-behind, unset-and-level, offline, mid-soak, an
  unreadable machine config, and the JSON field set.
- `crates/lisa-cli/src/doctor.rs` — 5 unit tests over the row: what it names,
  that `behind` carries its command and is not counted a failure, that unset
  reads as unset, that an unreachable list is not an OK row, and the report's
  closing line.
- `crates/lisa-cli/tests/doctor_channel_cli.rs` (new) — 8 tests running the real
  binary against a local stand-in for the releases API and a throwaway
  `LISA_CONFIG_DIR`: a box behind its channel, a box level with it, a box with
  no channel set, the offline case, the `--json` document for each of those, and
  that the guide names the fields. Nothing reaches the network or touches the
  operator's real channel.
- `cargo test -p lisa-cli`: 621 unit + all integration tests pass.
  `cargo test -p lisa-core -p lisa-plugin`: pass. `cargo fmt --check` clean,
  `cargo clippy -p lisa-cli --all-targets -D warnings` clean.

## What still concerns me

1. **`lisa doctor` now touches the network on every prose run.** Reading what a
   channel resolves to means reading the release list; there is no cache. The
   budget is 8 seconds, and a machine with no network gets the honest
   `could not be resolved` row rather than a hang — but every `doctor` run is
   now that much slower offline, and unauthenticated GitHub allows 60 requests
   an hour per IP, so a box that runs `doctor` in a tight loop will start
   reading `unresolved`. A short-lived on-disk cache of the release list is the
   obvious next move if that bites. `--json` does not add side effects: unlike
   the prose run it does not clean Zellij's plugin cache or seed Codex trust.
2. **Being behind is informational, not a failure.** `doctor`'s exit code is
   unchanged, on the reasoning that every machine in the fleet is behind the
   moment a release is cut — including the desk that cut it — and a `doctor`
   that starts refusing on drift is a `doctor` people stop running. The row is
   loud and `data.lisa.state` is the field to alert on. If the intent was for
   drift to fail the command, that is a one-line change to the row's `required`
   flag plus `has_failures`.
3. **Pre-existing, unrelated:** `cargo clippy --workspace --all-targets` fails on
   three lints in `crates/lisa-plugin` (`ui.rs`, `tests/operator_recovery_matrix.rs`)
   under the local toolchain (clippy 1.97). Those files are untouched by this
   ticket and the failures reproduce without it.
