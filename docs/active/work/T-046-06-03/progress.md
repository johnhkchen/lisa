# Progress — T-046-06-03 closing acceptance run

## Current status

- Research: complete.
- Design: complete.
- Structure: complete.
- Plan: complete.
- Implement: complete for all agent-owned work.
- Review: pending at the time of this entry.
- Expected disposition: operator-owned block.

## Ownership baseline

At assignment start, the shared worktree already contained modified:

- `.lisa/completion-journal.jsonl`;
- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-046-06-03.md`.

An unrelated untracked T-047 work directory was also visible initially.

This attempt did not stage, reset, stash, or edit those paths.

The attempt-private directory initially contained only:

- `.lisa-launch-0.sh`;
- `assignment-1-1784257876315230000.md`.

During validation, `docs/active/work/T-046-06-03/` appeared as untracked shared
state through Lisa's artifact/lifecycle machinery.

This agent did not write phase artifacts directly to that path.

All authored phase/evidence files were written to the assignment-private work
directory.

## Phase artifacts completed

Created `research.md`.

It maps the evidence gate, runbook controls, prerequisite fixes, landing-probe
baseline, retained containers, protocol deviations, and finding ownership.

Created `design.md`.

It compares admission, reconstruction, duplicate tickets, rubric changes,
autonomous reruns, shallow blocking, and evidence-preserving blocking.

Created `structure.md`.

It defines private file ownership, the failed-attempt schema, privacy boundary,
Review shape, and future admission boundary.

Created `plan.md`.

It sequences read-only extraction, classification, preservation, validation,
Review, and lifecycle handoff.

No ticket phase or status field was manually edited.

## Repository evidence inventory

Searched shared docs and attempt directories for closing result-template
records.

No admitted primary closing matrix record was found.

No Zellij 0.40.1 seeded recovery record was found.

No new T-046-06-03 landing page or comparison record was found.

The standing landing benchmark remains under:

`docs/knowledge/landing-probes/`.

T-047-01-02 already owns the fuller loop-built rematch.

Its existing Review disposition is blocked on the missing human-operated page.

No duplicate tour ticket was created.

## Retained-container inspection

Located two stopped closing-named containers:

- `cbt-0716-182723-claude-a`;
- `cbt-0716-184858-codex-b`.

Inspected them without starting or mutating them.

Both retain 4 GiB/two-CPU caps.

Both have zero host mounts.

Both are arm64 fixture descendants.

Both exited with surrounding container exit zero.

Container exit zero was not treated as acceptance success.

## Evidence privacy

Used `docker inspect` and selected `docker diff` output.

Streamed selected session JSONL and shell-history content to stdout.

Did not copy `~/.claude`, `~/.codex`, auth JSON, config secrets, environment
dumps, provider account details, or token-bearing files into the repository.

Extracted only:

- model/CLI metadata;
- initial and subsequent user messages;
- ticket-relevant assistant text;
- install/verification commands;
- install/verification outputs;
- filtered shell-history commands.

## Protocol-file audit

Neither container contains the prescribed:

- `/tmp/instruction.txt`;
- `/tmp/disk.before`;
- `/tmp/disk.after`;
- `/tmp/t.before`;
- `/tmp/t.after`.

Neither contains a completed result-template record.

No time or disk measurement was reconstructed.

Final Docker layer sizes and container lifetimes were recorded only as caveated
identity facts.

## Claude attempt classification

The relevant Claude session used `claude-sonnet-5`.

The acceptance leg requires a Haiku-class model.

The initial instruction referenced the entire `/tmp/lisa-README.md` file.

It did not embed the exact install-section bytes.

Shell history shows the README was fetched from fixed pre-fix commit
`b5af5fa9d2ac304edfad2e9992ae11bd04834e98`.

The old installer resolved v0.3.0 GNU Linux and failed on missing xz.

The agent installed `xz-utils` with sudo apt.

It installed Lisa and Zellij into `~/.cargo/bin`.

It obtained a green v0.3.0 doctor and declared success.

The later operator shell history does not contain validate or dry-run.

No independent exit matrix or negative result was preserved.

The attempt is not admitted.

## Codex attempt classification

The relevant Codex session used CLI 0.144.5 and `gpt-5.4-mini`.

The initial prompt referenced a nonexistent `.md` path.

The retained file ended in `.m` and contained the pre-fix README.

The old installer again resolved v0.3.0 GNU Linux and failed on missing xz.

The agent installed `xz-utils` and Lisa into `~/.cargo/bin`.

It stopped before doctor.

The operator then sent a second message asking for help installing Zellij.

That violates the one-prompt hands-off boundary.

The agent attempted apt Zellij installation, then downloaded Zellij 0.44.3.

Later shell history does not contain validate or dry-run.

No independent exit matrix or negative result was preserved.

The attempt is not admitted.

## Failed-attempt evidence artifact

Created `closing-attempts-2026-07-16.md`.

The artifact begins with `NOT ADMITTED`.

It contains separate Claude and Codex records.

It records unavailable values as `NOT RECORDED`.

It includes a control-by-control primary matrix.

It includes only short relevant exact strings.

It maps the observed pre-fix behavior to existing ticket owners.

It states that no new current-surface defect was established.

## Finding routing

The observed v0.3.0 channel path maps to T-046-03-02.

The observed xz dependency maps to T-046-03-03.

The observed manual Zellij work maps to T-046-02-*.

The observed old source-oriented documentation maps to T-046-04-*.

The missing purpose rematch maps to T-047-01-02.

These attempts deliberately or effectively used the pre-fix surface.

They did not exercise the current fixes.

No new product ticket was filed from non-current evidence.

If a conforming current-surface rerun reproduces any of these failures, that
future attempt must file a new bug and keep this ticket open.

## Source transaction status

No ticket-owned shared source file was created or modified.

No `lisa commit-ticket` transaction was required.

No ordinary `git add`, `git add -A`, or `git commit` command was used.

The ordinary index contains no current-ticket source entry.

## Implement verification

Confirmed all Implement-stage private artifacts are present and nonempty.

Checked the private Markdown files for trailing whitespace; none was found.

Confirmed the evidence artifact includes:

- both retained container names;
- `NOT ADMITTED` markers;
- `NOT RECORDED` markers.

Confirmed both retained containers remained stopped after inspection.

Rechecked Git status without altering unrelated state.

No Rust tests were run because no executable source changed.

## Plan deviations

No substantive deviation occurred.

The repository evidence inventory initially suggested the closing runs might be
entirely absent.

Two stopped closing-named containers were then discovered.

The plan accommodated them through its read-only evidence-inspection steps.

Their contents changed the handoff from “no runs exist” to “two runs exist but
are nonconforming and not admitted.”

They did not change the expected blocked disposition.

## Acceptance status

Criterion 1 is unmet.

Neither primary leg used the complete required protocol or current fixed
surface.

Criterion 2 is unmet.

No 0.40.1 seeded recovery record exists.

Criterion 3 is unmet.

No new short-prompt tour page or comparison exists.

## Remaining operator work

John must run two fresh current-surface instruction-A legs:

- Claude with an exact Haiku-class model;
- Codex with an exact mini-class model.

Each record must preserve snapshots, prompt bytes, all positive exits, all
negative results, strings, interventions, apt/sudo actions, and changed paths.

John must run a separate fresh ancient-Zellij 0.40.1 variant.

That record must show recovery through Lisa's own current error strings without
authorship-context help.

John must run the standing short landing probe and preserve the unedited HTML,
metadata, scores, and comparison.

## Implementation conclusion

All safe agent-side inspection, preservation, classification, and routing is
complete.

The remaining acceptance work is intentionally human-operated and metered.

Review must block with an operator-owned structured remedy.
