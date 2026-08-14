# T-069-01-04 — lisa reads its channel from the package that installed it

`lisa upgrade` used to look at a Homebrew or apt path and refuse, printing the
command that would have worked. It now reads the channel off the package —
`lisa-nightly`, or the suite word in `/etc/apt/sources.list.d/lisa.list` — and
hands the move to `brew` or `apt-get`, keeping the parts no package manager has:
the live-run refusal, the before-and-after report, the alarm, and `doctor`
naming the gap.

## What changed

| file | what |
| --- | --- |
| `crates/lisa-cli/src/upgrade/install_channel.rs` | new. Derives the channel from the formula or the suite, builds the package-manager commands as argv, and rewrites an apt sources line. |
| `crates/lisa-cli/src/upgrade.rs` | the package-managed path: derive, plan, guard, delegate, report. The old refusal is gone; the download-and-swap path is untouched. |
| `crates/lisa-cli/src/freshness.rs` | `Setting::Package` — the channel, where it was read from, and the config field a package-managed box does not read. |
| `crates/lisa-cli/src/doctor.rs` | the `lisa` row derives its channel; a new `lisa install` row reports a box carrying two lisas. |
| `crates/lisa-cli/src/nightly.rs` | the cycle's mover on a package box is `brew`/`apt-get`; `nightly install` checks the package instead of writing a channel it would not read. |
| `crates/lisa-cli/tests/package_channel_cli.rs` | new. Runs the real binary from a directory shaped like a Homebrew Cellar. |
| `crates/lisa-cli/data/json-guide.md`, `crates/lisa-cli/src/main.rs`, `docs/knowledge/mac-mini-nightly.md`, `README.md` | which source wins where, and what rollback costs on each platform. |

Commits: `3833a19`, `9e86575`, `3964ca3`, `5c81fec`, `5deb7fa`, plus the README
commit below.

### Which source wins where, in one sentence

**A package-managed box reads its channel off the package; every other box reads
it out of the machine config.** Not a precedence rule between two answers — the
install decides which of the two is even consulted. On a brew or apt box the
`channel` field is inert, and `doctor` reports a field that disagrees
(`channel_conflict` in `--json`) rather than letting it look load-bearing.
`lisa upgrade --channel <name>` on such a box switches *packages* and writes
nothing to the config file, so the box cannot grow a second answer.

The derivation is path-based and needs no `brew` or `dpkg` call:
`<prefix>/Cellar/<formula>/<version>/bin/lisa` is Homebrew's own layout, and
`upgrade` already canonicalises before classifying. apt is read from the sources
files (both the one-line and deb822 grammars), matched on the archive URI.

### What `upgrade` runs now

- **brew, same channel** — `brew update`, `brew upgrade <formula>`.
- **brew, `--channel`** — `brew fetch <tap>/<formula>`, `brew uninstall <old>`,
  `brew install <tap>/<new>`. The bottle is fetched first so the window with no
  `lisa` on the box is as short as a local copy, and the way back is printed
  before anything is removed.
- **apt, same channel** — `apt-get update`, `apt-get install --only-upgrade -y
  lisa lisa-runtime-zellij`, elevated with `sudo` when the process is not root.
- **apt, `--channel`** — Lisa rewrites the suite word in the sources file (whole
  file, from a computed value, through `sudo tee` when not root), then the two
  commands above. **Coming back down a channel is printed, not run**: it is a
  downgrade, and half-doing one on a machine's behalf is worse than naming it.
- **apt, `--tag`** — `apt-get install --allow-downgrades -y lisa=<deb>
  lisa-runtime-zellij=<deb>`, the version derived by nfpm's semver rule
  (`0.5.0-rc.2` → `0.5.0~rc.2-1`).
- **brew, `--tag`** — Lisa's own installer into `~/.local/bin`, because
  `brew switch` is gone. It says so, says it leaves two lisas with PATH order
  deciding, and `doctor` reports the pair until one is removed.

The mid-run guard applies to every one of these before the mover starts, and
`--dry-run` prints the plan and runs nothing.

### The two-lisa box

`doctor` grew a `lisa install` row: `ok` with one lisa, `unsupported` (required
`false`, so it never fails the verdict) when a packaged lisa and
`~/.local/bin/lisa` are both present, naming both paths, which one `command -v`
finds right now, and `rm` as the remedy. Reporting is the whole fix — nothing
removes anything.

## How it is tested

- `cargo test --workspace --no-fail-fast` — every target passes except one
  pre-existing failure that is not mine (below).
- **32 new tests.** 24 unit tests in `install_channel.rs` and `upgrade.rs`
  (formula and suite derivation, both apt grammars, a commented-out line, two
  lines naming two channels, sources rewriting that leaves unrelated lines byte
  for byte, plan construction for all four movers, sudo elevation, the deb
  version transform, `package_lisa_path`), 5 in `freshness.rs` (formula and
  suite rows, the disagreement sentence, agreement staying quiet, a source-built
  box having no package setting), and 8 CLI tests running the real binary from a
  fake Cellar.
- The CLI tests cover the delegation end to end short of running `brew`: every
  upgrade case is `--dry-run`, which prints the plan and touches nothing.
  `LISA_RELEASES_URL` points at a dead port, so nothing reaches the network.
- `cargo clippy --workspace --all-targets` — clean. `cargo fmt --all` — applied.

## Concerns

1. **No `brew` or `apt-get` has actually been run by this code.** The commands
   are asserted as argv and rehearsed with `--dry-run`; the first real
   `brew upgrade lisa-nightly` will be on the mini. The failure mode I would
   watch for is `brew fetch <tap>/<formula>` on a machine that has not run
   `brew trust johnhkchen/lisa` — the switch would stop at the fetch, before
   anything is uninstalled, which is the safe end of it, but the message the
   operator sees will be Homebrew's, not Lisa's.

2. **The apt half has no CLI-level test.** `/usr/bin` cannot be faked in a
   temporary directory the way a Cellar can, so apt derivation, the sources
   rewrite and the pin are unit-tested only. `LISA_APT_SOURCES_DIR` exists so a
   future Docker-based verifier can point Lisa at a real sources tree.

3. **`lisa upgrade --tag` on a brew box deliberately creates the two-lisa
   state.** It is the only rollback Homebrew has, and it is what
   `S-069-01` asked to keep. `doctor` reports it and the README and the runbook
   both say how to undo it, but a box left in that state will not follow its
   formula, and only the `doctor` row says so.

4. **Concurrent-thread file sharing.** `T-069-01-03` was working on the same
   branch throughout. Its `main.rs` and `promote.rs` landed in `a30b6dc` before
   my `main.rs` help-text commit, so that one is clean. `README.md` was not: my
   README commit carries one paragraph of `T-069-01-03`'s prose about the
   promotion pointer, which was sitting uncommitted in the worktree. The content
   is correct and complete; the attribution is not. Same hazard
   `T-069-01-02` reported — two tickets with no dependency edge editing one
   file.

5. **One pre-existing test failure on the branch, not from this ticket.**
   `flag_audit_covers_live_cli_config_and_prompts` reports five missing rows,
   all `flag:lisa/promote-nightly:*` — `T-069-01-03`'s new command, whose flag
   audit rows it has not written yet. Nothing in this ticket adds or removes a
   flag.

6. **Worth deciding: `soak_hours` on a package box.** It is in the same config
   file as `channel`, and on a package-managed box nothing reads it either — the
   publisher decides soak now (`T-069-01-03`). This ticket did not touch that
   field or its comment, so the file still describes it as if the machine
   applies it. That is honest on a curl-installed box and stale on a brew one.
