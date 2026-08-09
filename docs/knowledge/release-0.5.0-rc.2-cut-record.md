# Release-candidate cut record — v0.5.0-rc.2

Field report for the checklist in [release-checklist.md](release-checklist.md).
Every value below is from live evidence gathered during or after the cut; none
are assumed. Unlike [the rc.1 record](release-0.5.0-rc.1-cut-record.md), this
one has no `PENDING` values, because this release was **prepared and published
in the same pass** — John authorized publication while the cut was in hand.

```text
release: v0.5.0-rc.2
prepared_at: 2026-08-09
prepared_by: Claude — preparation and, on explicit authorization, publication
cut_at: 2026-08-09T06:00:15Z
operator: johnhkchen (authorized in session; the push was the publishing action)
release_commit: 2b73a4e957e8b264dac3e930756c63c2411545a8  ("Set the workspace to 0.5.0-rc.2")
release_run_url: https://github.com/johnhkchen/lisa/actions/runs/31297671933
tag_api_prerelease: true
latest_api_tag: v0.4.4              # confirmed — latest does not move for a prerelease
ancestry_gates: e045=ancestor / musl=ancestor / seal=ancestor / workflow=ancestor  (re-proven against the public tag object 2b73a4e, not just pre-push HEAD)
asset_audit: 18 assets — 4 target tarballs + 4 .sha256, installer, lisa.rb, dist-manifest.json, sha256.sum, source.tar.gz(+.sha256), 4 .deb (lisa amd64/arm64, lisa-runtime-zellij amd64/arm64)
aarch64_musl_bullseye_step: success  ("Verify static musl artifact on Debian bullseye")
x86_64_musl_bullseye_step: success   ("Verify static musl artifact on Debian bullseye")
readme_installer_path: n/a for a prerelease — releases/latest resolves to v0.4.4; install through the tagged asset
installed_version: 0.5.0-rc.2        # /Users/johnchen/.local/bin/lisa
homebrew_version: 0.5.0-rc.2         # the acceptance value, confirmed in the tap formula
apt_fresh_version: n/a — publish-apt-repository is stable-only and was correctly skipped
apt_upgrade_from_prior: n/a — same reason
channel_skew: deliberate
```

## Why this cut exists

rc.1 was prepared on 2026-08-07 and never published. Before it was, a field run
of 0.4.4 on `tabular-recipes` produced a failure that made the release worth
re-cutting rather than shipping as prepared: **Lisa never delivered a ticket's
prompt to a recycled pane.** Two Claude sessions were recovered whose only user
message was Lisa's own shell-readiness probe, with the agent replying that no
task had arrived. rc.2 is rc.1 plus that fix.

The chain, and where rc.2 breaks it:

- A recycled pane was declared empty on a stopwatch — eight seconds after
  `/exit`, two of them the deferred Enter — so the next launch line was typed
  into a TUI that had not left, and landed in its chat box. A pane now holds
  past the exit grace while it is still emitting provider hooks, which only a
  running process can produce. That evidence decays after a grace of renewed
  silence, and `AGENT_EXIT_CEILING_SECS` caps the hold so a noisily-dying
  provider can never strand a seat.
- With no process started, nothing announced itself, so the seat timed out into
  the same-pane reset — which deleted the very lease marker `on-start.sh`
  requires and then probed for a shell. A live Claude answers that probe,
  because a probe is only a command. The reset now retains the marker and the
  attempt, readiness admits a late `.started` from inside the window, and a pane
  that still proves residency is re-exited rather than interrupted and probed.
- Separately: every assignment named a ticket file that does not exist.
  `ticket_prompt` scans for the real filename but was handed the path with the
  `/host` mount stripped, which WASI cannot read, so the naive `<id>.md`
  fallback always won. Projects that name tickets descriptively — which is most
  of them — got a path to nowhere.

Readiness widened; **ownership did not**. Every lease, ticket and currency check
stands where it stood.

## What is still open against this line

`ON_HEARTBEAT_HOOK` copies the pane lease marker without checking who is
calling it, unlike `ON_START_HOOK`. rc.2 makes the exposure window longer (the
reset now retains that marker) and contains the consequence in the scheduler — a
heartbeat inside the reset window proves residency and nothing else — but the
signal itself is still forgeable. Tracked as **S-058-01 / T-058-01-01**, held
back from this cut deliberately so the two changes stay independently
revertable.

## Gates

`just check` green at exit 0 before the bump and again after it. Workspace tests
1494 passed / 0 failed (1474 before this line's work; +20 tests). CI on `main`
succeeded in 1m58s, which is what triggered Auto Release; Auto Release created
and pushed the tag and dispatched the release workflow. Every required
cargo-dist job succeeded — plan, four `build-local-artifacts`,
`build-global-artifacts`, `host`, `publish-homebrew-formula`, `announce` — and
`publish-apt-repository` was skipped, which is correct for a prerelease and is
not a failure.

## Note on the publication route

This cut went through **Route A**: pushing the version-bump commit to `main` is
itself the publishing action, because `auto-release.yml` reads the workspace
version from `cargo metadata` on green main CI, tags, and dispatches. `just
release` was not run — the two routes are mutually exclusive for one commit.

The push carried 55 commits, because `origin/main` had not been updated since
2026-08-03 (`3c5dc69`). It was a clean fast-forward, zero behind. Worth noting
for the next cut: check `git log --oneline origin/main..main` *after* a fetch
before treating a release push as a two-commit operation.
