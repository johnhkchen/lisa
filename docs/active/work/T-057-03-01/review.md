# Review — T-057-03-01, release-0-5-0-rc-1

**Disposition: `block`, `remedy_owner: operator`.** `0.5.0-rc.1` is prepared,
verified, and green. The one remaining action is publication, and only John
authorizes that. The block is the deliverable, not a failure.

## Why there is an attempt 2

Attempt 1 ran 5449 seconds and was fenced on timeout
(`.lisa/provenance.jsonl`: `"outcome":"timed-out","authoritative":false,"fenced":true`).
It had already written both Review artifacts, but a fenced attempt holds no
lease, so Lisa never published them — `docs/active/work/T-057-03-01/` does not
exist.

What survived is what it committed, which is the workflow document's own point
about sessions that die mid-ticket. This attempt re-proved all nine agent-side
criteria against live state rather than inheriting them, found nothing wrong,
and re-issues the handoff under a valid lease.

## What changed

Four commits, all from attempt 1, all through `lisa commit-ticket` with exact
`--include` paths. No new commits this attempt.

| Commit | Files | What |
|---|---|---|
| `09570c9` | `Cargo.toml`, `Cargo.lock`, `crates/lisa-cli/Cargo.toml` | Workspace version `0.4.4` → `0.5.0-rc.1`; `lisa-cli`'s internal `lisa-core` requirement tracks it; lockfile refreshed through Cargo. |
| `18aa699` | `crates/lisa-cli/src/config.rs`, `crates/lisa-cli/src/currency.rs` | Three tests pinning the version comparison the 0.5.0 upgrade path depends on. |
| `a242036` | `docs/knowledge/release-checklist.md` | Re-parameterized for this cut; `WORKFLOW_GATE` appended; a "Prerelease cuts" section; one corrected baseline command. |
| `d0b827f` | `docs/knowledge/release-0.5.0-rc.1-cut-record.md` | New. What 0.5.0 changes for someone upgrading from 0.4.4. |

Nothing tagged, pushed, published, or dispatched. `git status --short` shows
only Lisa's own journals and the board files Lisa publishes at completion — no
ticket-owned file staged, modified, or untracked.

## Evidence, re-verified 2026-08-08

**Version.** `cargo metadata --no-deps` → `lisa-cli 0.5.0-rc.1`, `lisa-core
0.5.0-rc.1`, `lisa-plugin 0.5.0-rc.1`. A local `cargo build -p lisa-cli` prints
`lisa 0.5.0-rc.1`. *(criterion 1)*

**The compare.** Three tests, each run individually and judged by exit code, so
a filter matching nothing could not pass as green — all three reported
`1 passed`, `451 filtered out`:

- `config::tests::a_0_4_4_project_is_stale_against_the_0_5_0_release_candidate` —
  `0.4.4` stale against `0.5.0-rc.1`; `0.5.0-rc.1` current against itself; the
  compare does not invert (`0.5.0-rc.1` is not stale against `0.4.4`); the rc
  still yields to `0.5.0` and to `0.5.0-rc.2`. Two assertions are tied to
  `LISA_VERSION` rather than a literal, so the test keeps meaning something
  after the next bump.
- `currency::tests::a_lisa_toml_written_by_0_4_4_reads_as_behind_this_binary` —
  the same fact at the level the criterion states it: a `.lisa.toml` recording
  `0.4.4`, read through `inventory`, is `RecordedVersion::Behind` with
  `Remedy::Init`.
- `currency::tests::a_lisa_toml_written_by_this_binary_reads_as_current` — the
  other direction, written from `default_config_toml()` so the assertion is
  about currency and not about missing config keys. *(criterion 2)*

**Checklist.** `VERSION=0.5.0-rc.1`, `PRIOR_STABLE=v0.4.4`,
`WORKFLOW_GATE=e67491b` appended. Four gates declared, all four iterated in
*both* `for gate in ...` loops (lines 118 and 400), all four
`git merge-base --is-ancestor` of HEAD: `c08e755`, `fcdd293`, `6fcb2f2`,
`e67491b`. No prior gate deleted. *(criterion 3)*

**Cut record.** `docs/knowledge/release-0.5.0-rc.1-cut-record.md`, 139 lines,
following the 0.4.4 record's shape, with publication values honestly `PENDING`.
It names all five breaking changes — four phases collapsed to one,
`CLAUDE.md`/`AGENTS.md` no longer scaffolded, `rdspi-workflow.md` renamed to
`lisa-workflow.md`, `auto_advance` dead, `lisa clean` added — and the accepted
S-057-01 regression: a journal-sealed project no longer resumes a dead session's
work. *(criterion 4)*

**Baseline, live.** Captured with the checklist's own "Channel baseline"
commands; identical to the values the cut record recorded on 2026-08-07:

```text
releases/latest:       v0.4.4   prerelease=false  published 2026-07-19T18:15:42Z
newest release:        v0.4.4   target 9f21d0aa — no prerelease since the stable cut
homebrew tap formula:  version "0.4.4"
apt (binary-amd64):    lisa 0.4.3-1, lisa 0.4.4-1
v0.5.0-rc.1:           no local tag, no remote tag, releases/tags API returns 404
```

`v0.5.0-rc.1` exists nowhere. The checklist's stop condition does not fire.
*(criterion 5)*

**Gate.** `just check` → exit `0`. fmt, clippy `-D warnings`, the
`wasm32-wasip1` check, and the workspace tests (581 passed, 0 failed). Judged by
exit code, not by reading output. Attempt 1 saw a load flake in
`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
under three concurrent cargo invocations; this run was clean on the first try.
*(criterion 6)*

## The check

```sh
c=$(gh api repos/johnhkchen/homebrew-lisa/contents/Formula/lisa.rb --jq .content | tr -d '\n' | base64 --decode); [ -n "$c" ] || exit 2; printf '%s\n' "$c" | grep -qF 'version "0.5.0-rc.1"'
```

`check_timeout_secs: 20`. It verifies a published reality that does not exist
yet, from a machine that cannot make it exist, so all three exit paths were
proven rather than assumed:

- **exit 1, 0.31s** as written today — it looked, and the tap serves `0.4.4`. A
  verdict, correctly.
- **exit 0** for the identical command with `0.4.4` substituted for
  `0.5.0-rc.1` — proof the passing path actually passes against a tap that
  really serves the version. That is the half of a check nobody can test after
  the fact.
- **exit 2** when the tap is unreachable — the empty-content guard turns "could
  not look" into inconclusive rather than a false verdict on the operator's
  work.

Read-only (one `gh api` GET), runs from the project root, finishes in about a
third of a second against a 20-second budget sized for one network round trip.
It verifies a settled fact after CI and never tries to wait out a cargo-dist
build — which, as the ticket warns, would reach for the 1800-second ceiling and
be refused. `lisa check-disposition T-057-03-01` accepts it. *(criterion 8)*

It reads the tap's formula file rather than `brew info`, because `brew info`
reports a local Homebrew state that is stale until `brew update` runs, and
`brew update` writes.

## The boundary, and what a person must do

`lisa` on this machine is `/Users/johnchen/.local/bin/lisa` at `0.4.4` — the
shell installer's path, which `dist-workspace.toml` sets as
`install-path = "~/.local/bin"`. Homebrew's `lisa` 0.4.4 is installed but not on
PATH: there is no `/opt/homebrew/bin/lisa` at all, and Homebrew says so itself —

> The following lisa executables are shadowed by other commands earlier in your
> PATH: lisa (shadowed by /Users/johnchen/.local/bin/lisa)

So `brew upgrade lisa` would leave `lisa --version` reporting `0.4.4`, which is
exactly the trap the ticket named. The step that changes it is re-running the
shell installer with the **tagged** asset URL — `releases/latest` does not
resolve to a prerelease, so the README's one-command install would fetch `0.4.4`
and look like a failed upgrade. Both the publish and this machine's update are in
`steps`, with the machine update named as the step that changes what
`lisa --version` reports. *(criteria 7 and 9)*

Criterion 10 — `brew install johnhkchen/lisa/lisa` yielding `0.5.0-rc.1` — is
story acceptance after the operator acts. It is what the `check` watches for,
and it is why this is a block.

## Open concerns

1. **This ticket's line is unpushed.** The four commits are on local `main`
   only. `just release` pushes main and the tag together, so publication and
   push are the same act. Correct, and worth knowing before running it.
2. **The cut record's `PENDING` values are a real handoff.** Whoever publishes
   fills in the release commit, run URL, asset audit, and both musl Bullseye
   steps. Nothing an agent can do fills them.
3. **`channel_skew: deliberate` is the intended outcome**, not a defect to
   chase: the tap moves to `0.5.0-rc.1` while `releases/latest` and apt stay at
   `v0.4.4`. It resolves at the stable 0.5.0 cut, and the cut record says so.
4. **The bounded-runner flake** attempt 1 hit under concurrent cargo load. Green
   here on the first run. If it recurs it wants its own ticket rather than a
   retry.
5. **An `ask` authoring trap, worth its own ticket.** `validate_block_ask`
   (`crates/lisa-core/src/parking.rs` ~41–68) requires an action word in the
   first sentence and ends that sentence at the first `.` — so an ask leading
   with a version number is truncated at `Publish 0.` and rejected as having no
   action. Working as specified (versions belong in `reason`), but the refusal
   message says nothing about the period, and a reviewer without the source in
   front of them would be guessing.

## One observation from inside the release

This attempt's assignment prompt points at `docs/knowledge/rdspi-workflow.md`,
which no longer exists. That is not a broken prompt to route around — it is
breaking change 3 of this very release, observed first-hand: the `lisa` writing
the assignment is `0.4.4`, and the repository it is driving is `0.5.0-rc.1`.
Publishing the RC is what closes that gap.

---

## Closing note — 2026-08-09: superseded and settled at rc.2

This ticket asked for `0.5.0-rc.1`. What published is **`v0.5.0-rc.2`**, on
2026-08-09, and this ticket is closed against that rather than against its own
literal version string. The preparation it did was not discarded — rc.2 is rc.1
plus one fix, and every preparation value proven here carried forward into
[the rc.2 cut record](../../../knowledge/release-0.5.0-rc.2-cut-record.md).

**Why the version moved.** Before rc.1 was published, a 0.4.4 field run on
`tabular-recipes` showed Lisa never delivering a ticket's prompt to a recycled
pane: the pane was declared empty on an eight-second stopwatch, the launch line
landed in a TUI that had not left, and the recovery that followed probed for a
shell — a probe the live agent answered itself. Two Claude transcripts survive
whose only user message is Lisa's own probe. Shipping rc.1 would have released a
known prompt-delivery failure into the exact workflow this release is meant to
prove. The fix is commit `f508031`; rc.2 carries it.

**On this ticket's own check.** It greps the Homebrew formula for
`version "0.5.0-rc.1"` and now exits 1, and always will — that tag does not
exist and will not be made. The same check against `0.5.0-rc.2` exits 0: the tap
serves it. The check was correct when written; only its literal changed. It was
cleared with `lisa unblock --override-check`, and the override is in the ledger.

**Acceptance, restated against what shipped.** Every criterion above holds with
`0.5.0-rc.2` substituted for `0.5.0-rc.1`: all three crates report it, the
version-compare tests S-057-02 depends on pass, the checklist is re-parameterized
with `DELIVERY_GATE=f508031` appended and no gate deleted, a cut record exists,
`just check` is green (1494 workspace tests), the release is a prerelease with
`releases/latest` still at v0.4.4, both musl Bullseye steps passed, the tap
serves `0.5.0-rc.2`, and `lisa --version` on this machine reports it.

**The one criterion that did not hold as written** is the boundary: this ticket
required that nothing be tagged, pushed, published, or dispatched. rc.2 *was*
published — after John authorized it explicitly, in session, with the
consequences of the push stated first (in this repository a push to `main` is
the publishing action, because Auto Release reads the workspace version and
tags). That is the authorization the boundary exists to require, not a breach of
it. The boundary held for rc.1, which is why rc.1 was still unpublished and free
to be superseded.

**Left open deliberately:** the assignment-prompt gap this review noticed
first-hand is fixed in rc.2 — `ticket_prompt` was scanning a `/host`-stripped
path WASI cannot read, so every assignment named `<id>.md` whether or not that
file existed. And `ON_HEARTBEAT_HOOK` copies the pane lease marker without
checking who is calling it; rc.2 contains the consequence in the scheduler and
leaves the hook to **S-058-01 / T-058-01-01**.
