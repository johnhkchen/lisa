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

1. records `channel = "nightly"` in the machine config — **unless a package
   manager owns this Lisa**, in which case the package already says it and the
   config field is not read at all,
2. records the board a new release is checked against (`nightly_project`) and
   the alarm that leaves the machine (`alert_command`),
3. writes the launchd job, and
4. loads it with `launchctl bootstrap gui/<uid>`.

`lisa nightly install --dry-run` prints the job and changes nothing, which is
the way to read it before it exists.

**Either Lisa works here, and which one you have decides what "nightly" means:**

| how the mini has Lisa | what puts it on nightly | what the 04:30 job runs |
| --- | --- | --- |
| `brew install johnhkchen/lisa/lisa-nightly` | the formula name | `brew update && brew upgrade lisa-nightly` |
| the one-command install (`~/.local/bin/lisa`) | `channel = "nightly"` in the machine config | the release's own installer |

On a package-managed box `install` checks the package really is the nightly one
and refuses if it is not — a job that quietly followed `stable` while calling
itself nightly is the failure this catches. `lisa upgrade --channel nightly`
moves it, by swapping formulae.

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

`ProgramArguments` names whichever lisa the mover keeps current — Homebrew's
`<brew prefix>/bin/lisa` on a formula box, `~/.local/bin/lisa` on a
curl-installed one — so the job always runs the binary that is actually being
upgraded.

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

**Rollback is the one thing the two package managers do not agree on, and this
is the paragraph to have read before an incident rather than during one.**

| the box | the way back | why |
| --- | --- | --- |
| Homebrew (the mini) | `lisa upgrade --tag v0.4.4` | `brew switch` was removed and a formula carries one version, so there is no `lisa=0.4.4` to ask brew for |
| apt | `sudo apt-get install --allow-downgrades lisa=0.4.4-1 lisa-runtime-zellij=0.4.4-1` | the pool keeps every version any suite has carried |
| the one-command install | `lisa upgrade --tag v0.4.4` | the installer writes whichever release you name |

One command, from the mini:

```bash
lisa upgrade --tag v0.4.4    # or whichever tag was working
```

It names both versions before it moves, refuses while a run is live (`--anyway`
overrides), and leaves the current binary in place if anything fails. Every
failing cycle's record and alarm already carry this line with the right tag
filled in — the version the machine was on before it moved.

**On a Homebrew mini, that pin is the shell installer, and it leaves two lisas
on the box.** Nothing else can do it: the pinned release lands in
`~/.local/bin/lisa`, Homebrew's stays where it was, and PATH order decides which
one your shell — and the nightly job — finds. `lisa doctor` reports the pair as
its own row for as long as it lasts. When the release that broke has been
replaced, put the box back on its formula:

```bash
rm ~/.local/bin/lisa
brew upgrade lisa-nightly
```

After a rollback, confirm the work runs again:

```bash
lisa --version
lisa doctor --path ~/path/to/a/board
lisa status --path ~/path/to/a/board
```

### On a Linux box, apt does this instead

The apt repository keeps every version it has ever carried in the pool, so
rolling back is naming one:

```bash
apt-cache madison lisa                    # every version this channel offers
sudo apt-get install --allow-downgrades \
  lisa=0.4.4-1 lisa-runtime-zellij=0.4.4-1
```

`--allow-downgrades` is not optional and it is not only about rollback. **A box
that has been on `nightly` or `canary` and moves back to `stable` is asking for
an older version than the one it is running**, so `apt-get upgrade` will leave
it where it is and say nothing useful. Changing the suite in
`/etc/apt/sources.list.d/lisa.list` is only half the move; the other half is the
line above, naming the version the new channel offers.

This is the one thing apt does better than Homebrew. `brew switch` was removed,
so a Mac has no equivalent — on the mini, `lisa upgrade --tag` above *is* the
rollback path, and that is why it survives on brew boxes.

`lisa upgrade --tag v0.4.4` on an apt box runs exactly the command above, with
the version derived from the tag. `lisa upgrade --channel stable` changes the
suite word and runs the update, then prints the `--allow-downgrades` line rather
than running it: coming back down a channel is a downgrade, and Lisa does not
half-do one on a machine's behalf.

To stop the machine upgrading itself without forgetting anything it knows:

```bash
lisa nightly uninstall   # removes the job; keeps the channel and the record
```

---

## Recording that it works

The mini's own answer is the evidence, so keep it where the rest of us can read
it. From the mini, after the first cycle has run:

```bash
lisa nightly status --json > docs/knowledge/mac-mini-nightly-record.json   # in this repo
```

That file is what says the arrangement is real rather than described: it carries
the channel the machine is on, the last cycle it ran, and when. Refresh it after
anything interesting — the first move, the first failure, the first rollback.

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
