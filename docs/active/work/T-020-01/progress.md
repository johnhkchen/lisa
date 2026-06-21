# T-020-01 Progress — spike execution log

This is a **spike**: the deliverable is findings + a design, not production code (ticket
"Notes"; AC: "No production code merged from this ticket beyond the design"). So the Implement
phase here = run the investigation, capture evidence, and confirm no source files changed.

## Completed

- [x] **Research** — mapped the hook/signal/injection/notification machinery (`research.md`).
- [x] **Design** — answered Q1–Q6 with evidence; GO on the gate (`design.md`).
- [x] **Empirical probe (the "implementation" of a spike).** Throwaway project with a
      `PreToolUse[AskUserQuestion]` + `PostToolUse[AskUserQuestion]` hook; ran
      `claude --dangerously-skip-permissions -p "<forces a question>"` (claude 2.1.185).
      Result: PreToolUse hook **fired**, payload captured with
      `"permission_mode":"bypassPermissions"`. Throwaway deleted; the captured payload is kept
      as `pretooluse-payload-sample.json` (evidence, not code).
- [x] **Structure / Plan** — blueprint + sequenced steps for T-020-02..04 (`structure.md`,
      `plan.md`).

## Evidence captured

- `pretooluse-payload-sample.json` — real `PreToolUse[AskUserQuestion]` stdin payload under
  bypassPermissions. Doubles as a unit-test fixture for the future `sed`-extraction test
  (plan step 2).

## Findings summary (full detail in `design.md`)

| Q | Question | Answer | How determined |
|---|----------|--------|----------------|
| 1 | PreToolUse fires? matcher? | YES, `"AskUserQuestion"` | probe + docs |
| 2 | GATE: skip-perms agents use it? | **YES → GO** | **empirical probe** (payload shows bypassPermissions) |
| 3 | Question text in POSIX `sh`? | YES, best-effort `sed` | payload is single-line JSON |
| 4 | PostToolUse heartbeat clears flag? | YES — via matcher-less heartbeat on next tool call | design (not single-hook dependent) |
| 5 | Suppression design | `.awaiting` signal → `awaiting_human: HashSet<u32>`; guard `send_line_to_pane` + 5 callers | design |
| 6 | Timeout exemption | exempt awaiting panes from reclamation, keep visible | design |

## Deviations from plan

- None. The plan's Phase 0 (the gate) was the spike's own scope and is the only phase executed
  as real work; T-020-02..04 are deliberately left unimplemented (spike boundary).

## Explicitly NOT done (by design — spike boundary)

- No edits to `templates.rs`, `init.rs`, `lib.rs`, `ui.rs`, or `hooks-guide.md`. The plugin and
  CLI source are untouched by this ticket.

## Source-tree verification

`git status` for this ticket's scope shows only **new files under
`docs/active/work/T-020-01/`** (the artifacts + payload sample). No `crates/**` files were
modified by T-020-01. (Pre-existing uncommitted S-019 / T-019-* changes in the working tree are
unrelated to this ticket and were present before it started.)

## Handoff

Ready for `review.md`. Next actionable work is **T-020-02** (with its step-1 interactive
validation as the first gate before further build-out).
