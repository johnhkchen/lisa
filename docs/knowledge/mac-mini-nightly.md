# The Mac mini runs nightly

The mini's job in the fleet is to **meet the next release before a user does**.
It runs background work on a schedule — real work, nothing whose failure costs
anything in front of anyone — so a bug in a release candidate becomes a broken
Mac mini on a Tuesday morning instead of a broken install after we promote it.

That job only works if the mini actually moves. A box that waits for someone to
remember `lisa upgrade` is a stable box with extra steps, and it drifts exactly
the way the curl-installed machines drifted four weeks behind without anyone
deciding they should.

This is the whole arrangement: what is installed, where it lives, what it does
each night, what it refuses to do, how it shouts, and how to put it back.

---

## Setting it up (two commands on the mini)

```bash
lisa nightly install --project ~/path/to/a/board \
                     --alert '<the command that reaches you>'
lisa nightly status
```

`install` does four things and prints all of them:

1. records `channel = "nightly"` in the machine config,
2. records the board a new release is checked against (`nightly_project`) and
   the alarm that leaves the machine (`alert_command`),
3. writes the launchd job, and
4. loads it with `launchctl bootstrap gui/<uid>`.

`lisa nightly install --dry-run` prints the job and changes nothing, which is
the way to read it before it exists.

**The mini must be on the shell-installer Lisa** (`~/.local/bin/lisa`, from the
one-command install in the README). `install` refuses on a Homebrew- or
apt-managed box, because one formula and one apt suite carry one version each
and cannot follow a channel.

---

## Where the schedule lives

`~/Library/LaunchAgents/io.johnhkchen.lisa.nightly.plist`

```
Label                   io.johnhkchen.lisa.nightly
ProgramArguments        ~/.local/bin/lisa nightly run
StartCalendarInterval   04:30, 05:30, 06:30
RunAtLoad               false
EnvironmentVariables    PATH=~/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
StandardOutPath         <machine config dir>/nightly/launchd.out
StandardErrorPath       <machine config dir>/nightly/launchd.err
```

Three times, not one, because this machine's job is running work: a cycle that
finds a live run skips, and the two later tries give a busy box another chance
before anyone looks at it. `RunAtLoad` is off on purpose — an upgrade is not
something a login should trigger. The `PATH` is written down rather than
inherited because launchd hands a job `/usr/bin:/bin:/usr/sbin:/sbin`, which has
neither `lisa` nor the Homebrew `zellij` on it.

Useful launchctl lines:

```bash
launchctl list | grep io.johnhkchen.lisa.nightly       # is it loaded?
launchctl kickstart -k gui/$(id -u)/io.johnhkchen.lisa.nightly   # run it now
launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/io.johnhkchen.lisa.nightly.plist
```

---

## What one cycle does

`lisa nightly run`, in order, stopping at the first line that applies:

| step | what happens | outcome recorded |
| --- | --- | --- |
| package-managed lisa | nothing can move; the schedule says so every night until it is fixed | `failed` |
| a Zellij session is up | **nothing is touched** | `skipped` |
| release list unreachable | nothing moved, and it says so | `failed` |
| newest tag has not soaked | nothing moved; names the tag and the wait left | `waiting` |
| already on the tag | nothing moved | `level` |
| a soaked newer tag | download, install, then check | `moved` or `failed` |

**It never lands mid-run.** An upgrade swaps the binary a running loop is
calling. Being one release behind is the cheaper mistake, every time, so a cycle
that finds any running Zellij session records `skipped` and ends.

**A failed move leaves the working version in place.** The move is the release's
own shell installer writing a new file into `~/.local/bin`; nothing deletes or
truncates the running binary first. A failed download, checksum or installer
ends with the Lisa that was working still being the Lisa on the box.

**A move is not believed until it is checked.** After installing, the cycle runs
the newly installed binary: `lisa --version` must be the version that was asked
for, and `lisa doctor --json --path <nightly_project>` must pass on the board
this machine actually works. That second check is the one that matters on macOS
— Zellij comes from Homebrew here rather than the pinned `/usr/libexec/lisa/zellij`
an apt box gets, so a Lisa upgrade can move out of `SUPPORTED_ZELLIJ_RANGE` in a
way a Linux box cannot. A `doctor` that fails under the new release fails the
cycle, loudly, with the rollback attached.

---

## How a broken nightly reaches us

Four ways, from the quietest to the one that leaves the machine:

1. **stderr**, filed by launchd into `nightly/launchd.err`.
2. **The system log** — `logger -t lisa-nightly -p user.err`, readable with
   `log show --predicate 'process == "logger"' --last 1d`.
3. **A desktop notification** on the mini itself.
4. **`alert_command`**, run with the whole health record on stdin and
   `LISA_NIGHTLY_OUTCOME` / `LISA_NIGHTLY_DETAIL` in its environment. This is
   the only one that leaves the box, so set it to something actually read — a
   webhook, a mail command, a push service:

   ```toml
   alert_command = "curl -sS -X POST -H 'Content-Type: application/json' -d @- https://example.invalid/hook"
   ```

   With none set, the record says so in as many words: *nothing left this
   machine: no alert_command is set in the machine config*.

And the pull side, for the fleet:

```bash
lisa nightly status          # exits non-zero when the arrangement is not working
lisa nightly status --json   # data.state is "ok" or "finding"
```

`status` fails on three different silences, not only on a failed upgrade:

- the record is **stale** (nothing has run for 36 hours — the schedule itself is
  broken),
- the box has **skipped three cycles in a row** because it is always working, so
  it is not moving at all,
- **nothing has ever run** here.

---

## Rolling back

One command, from the mini:

```bash
lisa upgrade --tag v0.4.4    # or whichever tag was working
```

It names both versions before it moves, refuses while a run is live (`--anyway`
overrides), and leaves the current binary in place if anything fails. Every
failing cycle's record and alarm already carry this line with the right tag
filled in — the version the machine was on before it moved.

After a rollback, confirm the work runs again:

```bash
lisa --version
lisa doctor --path ~/path/to/a/board
lisa status --path ~/path/to/a/board
```

To stop the machine upgrading itself without forgetting anything it knows:

```bash
lisa nightly uninstall   # removes the job; keeps the channel and the record
```

---

## What to write down as this runs

The point of the mini is evidence, so the two things worth keeping are:

- **How often nightly actually catches something.** `nightly/history.jsonl` is
  one line per cycle and is the raw material. If it catches nothing over several
  releases, the honest conclusion is that the soak window is the wrong length or
  that the workloads are not exercising Lisa hard enough — not that the releases
  were fine.
- **Every `failed` cycle, and what the rollback cost.** A rollback that took one
  command and five minutes is the arrangement working; anything more is the
  thing to fix next.

The rest of the fleet stays where it is until this has proven itself here.
Moving every machine at once would remove the control group.
