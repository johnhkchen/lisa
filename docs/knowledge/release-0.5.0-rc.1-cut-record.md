# Release-candidate cut record — v0.5.0-rc.1

> **Superseded, and never published.** `v0.5.0-rc.1` was prepared on 2026-08-07
> and then overtaken: a 0.4.4 field run on `tabular-recipes` found that Lisa
> never delivered a ticket's prompt to a recycled pane, and that fix went out as
> [`v0.5.0-rc.2`](release-0.5.0-rc.2-cut-record.md) on 2026-08-09 instead. No
> `v0.5.0-rc.1` tag or release exists anywhere and none will be made.
>
> The `PENDING` values below are therefore **closed, not owed** — they were
> waiting on a publication that will never happen. Everything above them is
> still a true record of the preparation pass, and rc.2 is rc.1 plus the pane
> fix, so the preparation evidence carried forward rather than being redone.

Field report for the checklist in [release-checklist.md](release-checklist.md).
The preparation values below are from live evidence gathered during the cut;
none are assumed. The publication values are `PENDING` because **this release
was prepared but not published** — only John authorizes publication, and the
agent that prepared it stopped at that line. Whoever publishes fills them in.

```text
release: v0.5.0-rc.1
prepared_at: 2026-08-07
prepared_by: Claude (T-057-03-01) — preparation only, no publishing action taken
cut_at: PENDING
operator: PENDING
release_commit: PENDING
release_run_url: PENDING
tag_api_prerelease: PENDING         # prerelease cut: expect true
latest_api_tag: PENDING             # expect v0.4.4 — latest does not move for a prerelease
ancestry_gates: e045=ancestor / musl=ancestor / seal=ancestor / workflow=ancestor  (proven pre-push against HEAD; re-prove on the public tag)
asset_audit: PENDING
aarch64_musl_bullseye_step: PENDING
x86_64_musl_bullseye_step: PENDING
readme_installer_path: n/a for a prerelease — releases/latest resolves to v0.4.4; install through the tagged asset
installed_version: PENDING
homebrew_version: PENDING           # the acceptance value: 0.5.0-rc.1
apt_fresh_version: n/a — publish-apt-repository is stable-only and is correctly skipped
apt_upgrade_from_prior: n/a — same reason
channel_skew: deliberate
```

`channel_skew: deliberate` is the intended disposition for this cut, not a
defect: the tap moves to `0.5.0-rc.1` while `releases/latest` and the apt
repository stay on `v0.4.4`. That is `dist-workspace.toml` doing exactly what it
says (`publish-prereleases = true`, apt publishing stable-only). It resolves at
the stable 0.5.0 cut.

## Channel baseline, captured live 2026-08-07

Read before any change, with the commands in the checklist's "Channel baseline"
section:

```text
releases/latest:       v0.4.4   prerelease=false  published 2026-07-19T18:15:42Z
newest release:        v0.4.4   (same release — no prerelease published since the 0.4.4 stable cut)
homebrew tap formula:  version "0.4.4"
apt (binary-amd64):    lisa 0.4.3-1, lisa 0.4.4-1
v0.5.0-rc.1:           absent locally, absent on the remote, no GitHub release
```

Two notes on that baseline. The apt repository serves both 0.4.3 and 0.4.4; the
checklist's old one-stanza `awk` stopped at the first `Version:` and reported
`0.4.3-1` as the current apt version, which is the oldest one published. That
snippet is corrected in this cut's checklist edit. And `releases/latest` equals
the newest release of any kind, which is why the "newest prerelease" line of the
prior baseline no longer describes reality.

## What 0.5.0 changes for someone who ran 0.4.4 yesterday

This is the breaking cut. Five changes, and one regression that is accepted
rather than fixed.

**1. Four phases are gone. A ticket now runs `ready → implement → review → done`.**
`research`, `design`, `structure` and `plan` are retired. An agent writes no
document before the work exists; it does the work, commits it through
`lisa commit-ticket`, and writes `review.md`. The only artifacts a ticket
produces are `review.md` and `review-disposition.json`.

You do not have to hand-edit your board. `Phase` deserializes all four retired
names as `implement` (`crates/lisa-core/src/types.rs` ~140–146), so a ticket
whose frontmatter still says `phase: structure` loads and resolves to
`implement`, and a completion-journal line written by a 0.4 board still replays.
Only `implement` is ever written back. `lisa doctor` will point at those tickets
so you know they exist; nothing is rewritten under you.

**2. `lisa init` no longer writes `CLAUDE.md` or `AGENTS.md`, and `lisa loop` no
longer refuses to start without one.** A context file is where your project
states its own standing intentions to every model that reads it, and Lisa has no
business seeding it. The assignment prompt names no context file either — Claude
Code loads `CLAUDE.md` natively and Codex loads `AGENTS.md` natively, so naming
it added nothing. If you have a `CLAUDE.md` you wrote, it is untouched and keeps
working. If you have the one Lisa generated, `lisa doctor` reports it and
`lisa init` or `lisa clean` retires it — see change 5.

**3. `docs/knowledge/rdspi-workflow.md` is now `docs/knowledge/lisa-workflow.md`.**
Lisa keeps exactly one document of its own, and it had to change name because it
no longer describes RDSPI. An unmodified copy of the old file is migrated for
you by `lisa init`. One you edited is preserved and reported, because your edits
are yours; `lisa clean` is how you say to remove it.

**4. `scheduling.auto_advance` in `.lisa.toml` means nothing now.** It is a dead
key. Lisa reports it and can lift the line out of your config; it does not
silently rewrite the file around it.

**5. Three commands now know what "out of date" means, in escalating consent
order.** `lisa doctor` gained an opinion about the project it stands in and
tells you what is stale. `lisa init` brings forward what it safely can — it acts
only where the bytes prove nobody edited the file. `lisa clean` is new and
removes what init deliberately would not; it lists by default, and only
`lisa clean --remove` removes anything.

### The accepted regression: journal-sealed projects no longer resume

Naming this here so it is found in the release notes rather than in the field.

The four retired artifacts were insurance. A session that died mid-ticket was
reseeded at the right phase because `plan.md` was on disk. Collapsing to one
working phase gives that up, and the replacement is commits: `lisa commit-ticket`
is already per-ticket, already serialized, already exact-`--include`, and unlike
`plan.md` a commit is a surface a machine can read.

That replacement is real in a commit-sealed project and **absent in a
journal-sealed one**, which has no commits to resume from. A journal-sealed
ticket whose session dies mid-work now restarts from the beginning. This is a
known, accepted regression for this cut (S-057-01); closing it is out of that
slice. It is also the strongest practical argument for letting Lisa keep some
history.

## Why a release candidate

The claim 0.5.0 makes is that a coding agent handed a well-specified ticket, a
`lisa commit-ticket` contract, and a disposition schema does better work than one
marched through six artifact phases. Nothing in the test suite can settle that.
It gets settled by a released binary running a real board — which is what this RC
exists to make possible, and why the version is a candidate rather than a stable.

## Preparation evidence

- Workspace version `0.5.0-rc.1`; all three `lisa-*` packages report it
  (`cargo metadata`), and `Cargo.lock` was refreshed through Cargo.
- The version comparison S-057-02's upgrade path depends on is asserted rather
  than reasoned about: `0.4.4` is stale against `0.5.0-rc.1`, `0.5.0-rc.1` is
  current against itself, the comparison does not invert, and the rc still
  yields to `0.5.0` and to `0.5.0-rc.2`
  (`crates/lisa-cli/src/config.rs`, `a_0_4_4_project_is_stale_against_the_0_5_0_release_candidate`).
  The same reading is asserted through `inventory` at the `.lisa.toml` level
  (`crates/lisa-cli/src/currency.rs`).
- `just check` green — fmt, clippy, the `wasm32-wasip1` check, and the workspace
  tests.
- Nothing tagged, pushed, published, or dispatched.
