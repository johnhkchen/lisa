# T-068-01-03 — the Mac mini runs nightly and can be put back

The first attempt blocked on two things that were not this desk's to do:
no published release carried `lisa upgrade` or `lisa nightly`, and the mini
was not a Lisa machine. **Both are now done.** v0.5.0 shipped on 2026-08-14
carrying all of it, and the mini is on nightly and moved itself onto v0.5.0
without anyone touching it.

This attempt did not redo that work. It went and looked at the machine, found
one real gap between what this repo says and what the box does, fixed it, and
is blocking on the two criteria that still need a person at the machine.

## What changed this attempt

One commit, `5e71ff1`, both files docs.

**`docs/knowledge/mac-mini-nightly.md`** — the page described an arrangement the
mini does not have. It prescribed `lisa nightly install`, a launch agent named
`io.johnhkchen.lisa.nightly`, and runs at 04:30/05:30/06:30. The mini has none
of those. It kept the mover the `screen-design` desk already had: one launch
agent, `dev.b28.lisa-stay-current`, at 04:37, running a script whose one
meaningful line is `FORMULA=lisa-nightly`. A person following the page would
have gone looking for a job that is not there — which is the criterion *write it
down somewhere a person can find it later* failing quietly.

The page now opens with what the box actually runs, and the `One mover per box`
section no longer reads as a rule the certified machine breaks: one mover still
holds, but which mover to keep depends on what the box already has, and the cost
of the mini's choice is named where the choice is made.

**`docs/knowledge/mac-mini-nightly-record.json`** — the machine's own answer,
which was sitting untracked in the working tree. Committed, and corrected in two
places:

1. The unattended move ran with `FORMULA=lisa`. The script's formula line was
   changed to `lisa-nightly` at 18:42, *after* the 18:09 move. Same job, same
   script, same `brew upgrade` — the mechanism is proven — but the record read
   as though the move happened on the nightly formula, and it did not. On a
   ticket about freshness being install history rather than intent, that
   distinction is the whole point.
2. A `not_yet_true` list, so the record carries its own gaps instead of leaving
   them to a review nobody re-reads.

## What the machine says, read off it during this review

Read-only over ssh, 2026-08-14, with `PATH=/opt/homebrew/bin:...` — the same
PATH the launch agent sets:

```
lisa         channel nightly (from the Homebrew formula lisa-nightly),
             installed 0.5.0, nightly is not moving this cycle: v0.5.0 is the
             newest release and has 19h of its 24h soak window left   OK
lisa install one lisa, at /opt/homebrew/bin/lisa (Homebrew's lisa-nightly)
             — and `lisa` runs it                                     OK
zellij       mode system, version 0.44.3, supported >= 0.43.0         OK
```

`/opt/homebrew/bin/lisa` → `../Cellar/lisa-nightly/0.5.0/bin/lisa`. No
`~/.local/bin/lisa`. The log reads three lines and no more:

```
2026-08-14 16:28  current at 0.4.4
2026-08-14 18:09  moved 0.4.4 -> 0.5.0
2026-08-14 18:43  current at 0.5.0 on lisa-nightly
```

A live Zellij session (`overseer`, running `claude --continue`) was up while I
looked, so tonight's 04:37 run will log `held` and move nothing. The guard is
not theoretical on this box.

**One trap worth writing down, because it nearly cost this review its verdict.**
A plain non-login `ssh mini` has no Homebrew on `PATH`. Run `lisa doctor` that
way and it reports `lisa install unsupported`, `zellij unsupported`, and
`nothing named lisa is on this PATH` — about a machine that is fine. The first
read of this box said it had no Zellij and could not run Lisa at all; `ps`
showed a Zellij server running from `/opt/homebrew/bin` at the same moment. Ask
a remote box its health with the PATH its scheduler uses, or you will read a
healthy machine as broken. This is now in the record.

## Criteria, honestly

| criterion | where it stands |
| --- | --- |
| upgrades unattended on nightly, schedule written down | **met** — 04:37 launch agent, `brew upgrade lisa-nightly`; written down in the runbook and the record, as of this commit |
| never lands mid-run | **met** — the mover holds on any live Zellij session; held state observed live today |
| a failed upgrade leaves the working version | **met** — `brew upgrade` either lands a new keg or leaves the old one linked |
| a broken nightly reaches us the same morning | **not met** — see below |
| the mini is on nightly and level, next run recorded | **met** — `doctor` above; 18:43 cycle recorded |
| the soak window exercised at least once | **met** — rehearsed against a synthetic release list in attempt 1 (`waiting`, nothing taken); the mini now inherits the publisher-side soak, and `doctor` shows it holding v0.5.0's remaining 19h |
| rollback one command, tested for real on the mini | **not met** — see below |

## Why this is blocked

**Nothing leaves the machine when a night goes wrong.** The mover writes one
line to `~/ergo-fleet/lisa-stay-current.log` and that is all. Worse, that line
cannot tell a failure from a quiet night: the script compares the installed
version before and after, and a `brew upgrade` that *fails* leaves them equal,
so it logs `current at 0.5.0 on lisa-nightly` — the same words a healthy no-op
prints. `$out` is captured and discarded on that path, and there is no `set -e`.
So the one place the mini records its health reports a broken night as a fine
one. The criterion says in as many words: *do not leave this as "we'll notice."*

**Rollback has not been done on this machine.** Attempt 1 rehearsed it for real
on the dev desk — v0.5.0-rc.2 → v0.4.4 → back, with a board running under the
rolled-back binary — and that is worth something, but the criterion names the
mini, and the mini is the box where the way back is different: `brew switch` is
gone, so `lisa upgrade --tag` writes a pin into `~/.local/bin/lisa` that shadows
the Homebrew one. Two lisas on the box, PATH order deciding which runs, and the
launch agent's own PATH puts `/opt/homebrew/bin` first — meaning the pin would
be what your shell runs while the timer keeps upgrading the other one. That
interaction is documented and untested. It is exactly the kind of thing that is
cheap to find on a Tuesday and expensive to find during an incident.

Both need a person at the machine, which the `screen-design` desk owns. The
disposition names the steps and what to write back.

## What still concerns me

1. **The mini's mover is not Lisa's.** That is a defensible call — the channel
   is the formula name now, so a box that pulls its formula needs no client-side
   logic — but it means `lisa nightly status`, the pull-side check this ticket
   built and tested, answers for every kind of box except the one machine the
   fleet actually certifies on. The gap gets papered over by a hand-assembled
   JSON file that someone has to remember to refresh.
2. **The record is a snapshot, not a signal.** `mac-mini-nightly-record.json`
   says what was true when someone last looked. Nothing in this repo goes stale
   or turns red when the mini goes quiet for a week.
3. **The guard depends on `zellij` being findable.** The mover's check is
   `zellij list-sessions | grep -cv EXITED` with `PATH=/opt/homebrew/bin:...`.
   If Homebrew's zellij ever moves, the command vanishes, the count reads zero,
   and the box upgrades under a live run while logging nothing unusual. Failing
   open is the wrong direction for that check.
4. **`claude` is not on the launch agent's PATH.** It lives in `~/.local/bin`,
   which the plist does not include. Harmless today because the job only runs
   `brew` — but anything added to that job that wants `doctor`'s full answer will
   get `claude not found` and a machine that reads as unsupported.
5. **Nothing has caught a bad release yet.** One move, and it was a stable cut
   the box would have taken anyway. The ticket asks us to record how often
   nightly actually catches something; the honest count so far is zero out of
   one, which is too few to conclude anything about the soak window.
