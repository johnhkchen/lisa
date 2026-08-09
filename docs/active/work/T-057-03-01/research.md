# Research — T-057-03-01, release-0-5-0-rc-1

## What this attempt inherited

Attempt 1 of this ticket ran 5449 seconds and was **fenced on timeout**
(`.lisa/provenance.jsonl`: `"outcome":"timed-out","fenced":true`). Its work
directory holds a complete `review.md` and `review-disposition.json`, but Lisa
never published them, because a fenced attempt has no lease. Nothing under
`docs/active/work/T-057-03-01/` exists.

What did survive is what attempt 1 committed. That is the workflow document's
own answer to a session dying mid-ticket, and here it is the whole difference
between a restart and a resume:

| Commit | What |
|---|---|
| `09570c9` | Workspace version `0.4.4` → `0.5.0-rc.1` |
| `18aa699` | The version-compare tests the 0.5.0 upgrade path hangs off |
| `a242036` | `release-checklist.md` re-parameterized for this cut |
| `d0b827f` | `release-0.5.0-rc.1-cut-record.md` |

So this attempt's research question is not "how do I cut an RC" but **"is the
prepared cut actually true, right now, on this machine and against the live
channels?"** Every claim in it is re-proven below rather than inherited.

## The two documents that govern

`docs/knowledge/release-checklist.md` — 574 lines, maintainer runbook,
version-parameterized from a block at the top. Its opening lines say only John
authorizes publication and that preparing or reviewing the checklist is not
authorization. That sentence is the ticket's boundary, stated by the source of
truth rather than by the ticket.

`docs/knowledge/lisa-workflow.md` — the current workflow definition. Note the
assignment prompt points at `docs/knowledge/rdspi-workflow.md`, which does not
exist. That is not an error to route around: it is change 3 of this very
release, observed from inside. The installed `lisa` writing the assignment is
0.4.4; the repository it is driving is 0.5.0-rc.1. The rename is real, and the
stale prompt is first-hand evidence of the upgrade path S-057-02 built.

## Live state, read not assumed

Channel baseline, captured with the checklist's own "Channel baseline" commands
on 2026-08-08:

```text
releases/latest:       v0.4.4   prerelease=false  published 2026-07-19T18:15:42Z
newest release:        v0.4.4   (target 9f21d0aa) — no prerelease since the stable cut
homebrew tap formula:  version "0.4.4"
apt (binary-amd64):    lisa 0.4.3-1, lisa 0.4.4-1
v0.5.0-rc.1:           no local tag, no remote tag, GitHub release API 404
```

`v0.5.0-rc.1` exists nowhere. The checklist's stop condition ("stop if `$TAG`
already exists") does not fire.

## The machine this ticket has to change

```text
which -a lisa      → /Users/johnchen/.local/bin/lisa   (0.4.4, shell installer)
/opt/homebrew/bin/lisa → does not exist
brew info lisa     → johnhkchen/lisa/lisa 0.4.4 installed, and Homebrew itself says:
                     "shadowed by /Users/johnchen/.local/bin/lisa"
```

This is the trap the ticket names, confirmed rather than predicted. `brew upgrade
lisa` would move the Cellar copy and leave `lisa --version` reporting `0.4.4`,
because PATH never reaches Homebrew's `lisa` — there is not even a symlink in
`/opt/homebrew/bin` for it to reach. The step that changes what `lisa --version`
reports is re-running the shell installer, and it must use the **tagged** asset
URL: `releases/latest` does not resolve to a prerelease, so the README's
one-command install would fetch 0.4.4 and look like a failed upgrade.

## What cannot be settled here

The tap serving `0.5.0-rc.1` is a published fact that does not exist yet and
cannot be made to exist from this seat. That is the Review `check`'s whole
subject, and the reason this ticket ends in a block with
`remedy_owner: "operator"` rather than a pass.
