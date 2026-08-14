# T-068-01-03 — the Mac mini runs nightly and can be put back

The arrangement is built, tested, and rehearsed for real against the live
release list. **It is not on the Mac mini**, and it cannot be yet: no published
release carries `lisa upgrade` or `lisa nightly` — both were written this story
and exist only on `main` — and the mini turns out to have no Lisa and no Zellij
on it at all. That is the block; the details and the exact way out are at the
bottom.

## What changed

**New: `crates/lisa-cli/src/busy.rs`** — is any Zellij session still running on
this machine? Deliberately blunt: not "is it one of Lisa's", because a
machine-level command has no project to compare against and the two mistakes do
not cost the same. Refusing an upgrade over an unrelated Zellij costs a skipped
cycle; upgrading into a live run costs the run. Both Zellijs a Lisa box can have
are asked — the one on `PATH` and the Debian package's pinned
`/usr/libexec/lisa/zellij`.

**New: `crates/lisa-cli/src/nightly.rs`** — `lisa nightly install | run | status
| uninstall`.

- `install` records `channel = "nightly"`, the board to check against
  (`nightly_project`) and the alarm that leaves the machine (`alert_command`),
  writes `~/Library/LaunchAgents/io.johnhkchen.lisa.nightly.plist`, and loads it
  with `launchctl bootstrap gui/<uid>`. `--dry-run` prints the job and changes
  nothing. It refuses a brew/apt-managed Lisa, and refuses a `--project` that is
  not a board — an alarm that fires every morning about a directory `doctor`
  cannot answer for is worse than no alarm.
- `run` is one cycle: skip while the machine is working → fail loudly if the
  release list is unreachable → hold if the newest tag has not soaked → do
  nothing if already level → otherwise install, then **check the new release
  against this machine's own board** with the newly installed binary's
  `--version` and `doctor --json`. Every outcome is written to
  `nightly/health.json` and appended to `nightly/history.jsonl`, next to the
  channel.
- `status` is the question you ask a box you are not sitting at, prose or
  `--json`, and it exits non-zero on three different silences: the last cycle
  failed, the record is more than 36 hours old (so the schedule itself has
  stopped), or the machine has been too busy to move three nights running.
- `uninstall` removes the job and keeps the channel and the record.

**Changed: `crates/lisa-cli/src/upgrade.rs`** — two things.

1. **The guard.** `lisa upgrade` now refuses while any Zellij session is up,
   names the sessions, and offers `--anyway`. The refusal happens after the plan
   is printed and before anything is downloaded, so a `--dry-run` still reports
   (and says a real run would stop).
2. **`installed_lisa()`** — an upgrade is about the lisa in `~/.local/bin`, not
   about the process asking. On a box where the running `lisa` came from
   somewhere else, comparing against the *runner's* version made `--tag` decide
   the machine was already where it needed to be and move nothing. That is the
   rollback path, so it is the worst place to get this wrong. Found by the
   rehearsal below, not by reading.

**Changed: `crates/lisa-cli/src/channel.rs`** — `nightly_project` and
`alert_command` in the machine config, rendered as commented-out examples when
unset and preserved when `lisa upgrade --channel` rewrites the file (a channel
change silently dropping the alarm would be a quiet way to go deaf). Plus
`format_rfc3339_utc`, the inverse of the parser already there, so a record a
person opens carries a time and not a number.

**Changed: `crates/lisa-cli/src/session_name.rs`** — `running_session_names()`,
so the `EXITED` grammar has one reader shared with the loop.

**Changed: `crates/lisa-cli/src/main.rs`** — the `nightly` command group and
`upgrade --anyway`.

**Docs:** `docs/knowledge/mac-mini-nightly.md` (new) is the runbook — the two
setup commands, where the schedule lives and what is in it, what a cycle does
and refuses to do, the four ways a failure reaches us, rolling back, and what to
record as it runs. README gains *A machine that upgrades itself*; `lisa
json-guide` gains `lisa nightly status --json`; `docs/knowledge/flag-audit.md`
gains the five new flag rows the audit test requires.

## How it is tested

`just check` is green end to end (exit 0): wasm check, `cargo fmt --all --check`,
clippy `-D warnings` on all three crates, `cargo test --workspace`.

**Automated** — 19 unit tests in `nightly.rs`, 4 in `busy.rs`, 4 added to
`channel.rs`, 12 black-box tests in `tests/nightly_cli.rs`, 1 added to
`tests/upgrade_cli.rs`. The CLI tests run the real binary against a local
stand-in for the releases API, a throwaway `HOME` and machine config, and a
stand-in `zellij` that reports whatever the case needs, so nothing reaches the
network, installs an artifact, or touches this machine's own channel or launch
agents.

| what | test |
| --- | --- |
| an upgrade refuses under a live run, names it, and names `--anyway` | `an_upgrade_does_not_land_under_a_live_run` |
| a cycle skips a working machine and touches nothing | `a_cycle_never_lands_under_a_live_run` |
| a tag too new to trust is seen and not taken | `a_tag_that_has_not_soaked_is_seen_and_not_taken` |
| an unreachable release list fails loudly, moves nothing, and the alarm carries the record | `a_cycle_that_cannot_read_the_release_list_fails_loudly_and_moves_nothing` |
| a release that did not actually land is not called a move | `a_release_that_did_not_actually_land_is_caught_before_it_is_called_a_move` |
| a Zellij the new release cannot use fails the cycle by name | `a_zellij_the_new_release_cannot_use_fails_the_cycle_by_name` |
| three skipped nights stop reading as healthy | `a_machine_that_is_always_working_stops_reading_as_healthy` |
| a stale record says the schedule is not running | `a_record_older_than_a_night_says_the_schedule_is_not_running` |
| the alarm with nowhere to go says so instead of reading as sent | `an_alarm_with_nowhere_to_go_says_so_rather_than_reading_as_sent` |
| the job's shape: absolute lisa, three tries, its own PATH, never at load | `the_job_runs_lisa_by_absolute_path_and_carries_a_path_of_its_own` |
| install writes the job and uninstall keeps what the box knows (macOS) | `install_puts_the_machine_on_nightly_and_uninstall_leaves_what_it_knows` |
| the settings the arrangement needs survive a channel change | `what_the_nightly_arrangement_records_survives_the_next_channel_change` |

**Rehearsed for real**, against the live GitHub release list, inside a throwaway
`HOME` on this desk (script and full transcript: `rehearsal.sh`,
`rehearsal.txt`, in this work directory). This machine had two live Zellij
sessions throughout, which is what made steps 1b and 2 real rather than staged:

1. **The guard, live.** `lisa upgrade --tag v0.4.4` exited 1 with *not moving
   lisa while this machine is working: 2 Zellij sessions are still running on
   this machine: overseer, lisa-6* and `lisa 0.5.0-rc.2 … is unchanged`.
2. **A real move.** `--tag v0.4.4 --anyway` downloaded and ran v0.4.4's own
   installer; the sandbox `lisa --version` reported `0.4.4`.
3. **The rollback, for real and in one command.** `lisa upgrade --tag
   v0.5.0-rc.2` moved it back — *Moving lisa 0.4.4 → 0.5.0-rc.2* — and the
   workload ran again under it: `lisa status --path <this repo>` printed the
   board (215 tickets, 217 edges) from the rolled-back binary.
4. **The soak window, exercised.** A release list served with a tag published 60
   seconds earlier: the cycle recorded `waiting` — *v0.9.9 is the newest release
   and has 23h of its 24h soak window left; every older release has been
   superseded by it* — and moved nothing.
5. **A whole cycle against the live list**, on a machine reported idle: `level`,
   recorded, and `lisa nightly status` / `--json` read it back the same way.

## Why this is blocked, and on what

Two facts, both checked rather than assumed:

**No published release has these commands.** `lisa upgrade` (T-068-01-01) and
`lisa nightly` (this ticket) exist only on `main`. The newest tags are
v0.5.0-rc.2 (2026-08-09) and v0.4.4 (2026-07-19); neither has `upgrade`, so
neither can put a machine on a channel. Until a release carries this work, the
mini cannot install the arrangement, and it cannot be *level with nightly* in
`lisa doctor` either — the nearest thing available would be a hand-copied
unreleased binary, which `doctor` would honestly report as `ahead`, and which is
the exact "freshness is a property of install history, not of intent" failure
this story exists to end.

**The mini is not a Lisa machine yet.** `johns-mac-mini.local` (192.168.4.33,
macOS 26.6.1, arm64) answers, and a read-only look at it found: no `lisa`, no
`zellij`, no `brew`, no Lisa machine config, no launch agent, and `claude` in
`~/.local/bin`. It has been up 7 days with a load average around 1.6, so it is
doing background work — but not Lisa's, and there is no board on it for a
nightly cycle to check a release against.

So the three criteria that need the machine — the mini actually on nightly and
level, its next run recorded, and a rollback performed on the mini itself —
cannot be met from here, in this order. Everything else is done: the schedule,
the mid-run guard, the failed-upgrade safety, the signal, a rollback tested for
real, and the soak window exercised.

The way out is in the disposition and is two steps: publish a release with this
work in it, then run the one-command install and `lisa nightly install` on the
mini and save what it reports into `docs/knowledge/mac-mini-nightly-record.json`.
The runbook (`docs/knowledge/mac-mini-nightly.md`) is written for exactly that.

## What still concerns me

1. **The alarm is only as loud as `alert_command`.** Unset, a failure reaches
   stderr, the system log and a desktop notification — all of them on the mini
   itself, which is a box nobody is sitting at. The record says so in as many
   words (*nothing left this machine*), and `lisa nightly status` fails from the
   dev desk when the box goes quiet, but the "reaches us the same morning"
   criterion genuinely depends on the operator naming a destination at install
   time. I could not pick one for you.
2. **Busy means "any Zellij session", including one that is nothing to do with
   Lisa.** On the mini that is right. On a box where someone keeps a personal
   Zellij open for weeks, the nightly cycle would skip forever — which `status`
   reports after three nights, and which the operator then has to clear by hand.
   The safe direction, but not a free one.
3. **The post-move check needs a board.** With no `nightly_project`, a cycle
   verifies only that the version landed and says so plainly rather than
   implying more. The Zellij-range check the ticket asks about only happens with
   a board named.
4. **`--anyway` exists.** It has to — an operator who knows the running session
   is idle needs a way past — but it is a foot-gun on a shared box, and nothing
   stops a future script from reaching for it.
5. **Rollback across a version that predates `upgrade`.** Step 3 of the
   rehearsal was driven by the built binary, because v0.4.4 has no `upgrade`
   subcommand to roll itself forward with. On the mini this will not arise once
   the arrangement ships (every release from here has `upgrade`), but a machine
   that rolls back *past* the first release carrying `upgrade` will need the
   one-command install to come back, not `lisa upgrade`.
6. **The three run times are a guess.** 04:30, 05:30, 06:30 fits "before anyone
   looks" on this desk's clock. If the mini's real work runs overnight, all
   three may land on a busy machine and the box will skip; `status` will say so
   after three nights, and the times are a one-line change in `RUN_TIMES`.
