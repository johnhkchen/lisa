# Plan — T-046-06-03 closing acceptance run

## Goal

Assess all available closing-run evidence without fabricating missing field
results, preserve useful failed-attempt facts, and produce an actionable Review
disposition.

## Step 1 — freeze ownership and lifecycle boundaries

Record the repository status present at assignment start.

Identify modified/untracked paths unrelated to this ticket.

Confirm the attempt-private work directory's initial files.

Do not modify shared ticket phase/status fields.

Do not touch unrelated T-047 work artifacts.

Verification:

- ownership baseline is documented in Progress;
- only attempt-private files are created by this ticket;
- no ordinary index command is used.

## Step 2 — map the governing protocol

Read `AGENTS.md`, `CLAUDE.md`, the ticket, and the full RDSPI workflow.

Read S-046-06 and the fixture/runbook.

Extract every positive, negative, timing, disk, prompt, model, and operator
control.

Read prerequisite ticket reviews to distinguish deterministic fixes from live
field proof.

Verification:

- Research names all three acceptance clauses;
- Research names the exact primary matrix and post-checks;
- deterministic evidence is not substituted for live evidence.

## Step 3 — inventory repository evidence

Search shared and attempt-private artifacts for completed closing run records.

Search for seeded 0.40.1 recovery records.

Search for a new landing-probe HTML and series entry.

Read the standing landing-probe prompt and rubric.

Inspect T-047-01-02 to avoid duplicate ownership.

Verification:

- every located artifact is classified;
- absent evidence is named absent;
- baseline artifacts are not relabeled as closing artifacts.

## Step 4 — inspect retained containers read-only

List stopped containers matching the closing naming pattern.

Use `docker container inspect --size` for identity, state, caps, mounts, and
final layer size.

Use `docker diff` for selected install and system paths.

List only relevant `/tmp` evidence filenames.

Do not start or mutate a retained container.

Verification:

- both container identities and isolation facts are recorded;
- final layer size is caveated as non-measurement;
- no container state changes.

## Step 5 — audit protocol snapshots

Check for:

- `/tmp/instruction.txt`;
- `/tmp/install-section.md`;
- `/tmp/disk.before`;
- `/tmp/disk.after`;
- `/tmp/t.before`;
- `/tmp/t.after`.

Check for completed result-template records.

Do not infer absent values.

Verification:

- each required file is recorded present or absent;
- unavailable measurements use `NOT RECORDED`.

## Step 6 — extract the Claude attempt safely

Identify the relevant Claude JSONL by timestamp and size.

Stream it from the container without copying auth state.

Extract initial prompt, model, assistant text, command calls, and command
outputs relevant to Lisa installation.

Filter shell history to installation and verification commands.

Exclude account/auth data and unrelated plugin caches.

Build a chronological chain from preflight to declaration.

Verification:

- actual model and prompt are explicit;
- old README commit is explicit;
- xz and Zellij actions are explicit;
- no secret material appears in artifacts.

## Step 7 — extract the Codex attempt safely

Identify the relevant Codex rollout JSONL.

Stream only session metadata, user messages, agent messages, command calls, and
command outputs.

Record CLI/model/effort.

Record the mistyped README path and recovered old contents.

Record the second human intervention.

Record the xz install and later Zellij install.

Filter shell history to post-run verification commands.

Verification:

- one-shot prompt violation is explicit;
- initial doctor omission is explicit;
- no auth JSON or token data is copied.

## Step 8 — classify each primary leg

Compare each attempt against:

- provider/model;
- current surface;
- exact instruction;
- hands-off boundary;
- snapshots;
- independent positives;
- independent negatives;
- time threshold;
- disk threshold.

Mark both `NOT ADMITTED` if any mandatory control is missing.

Do not average the two legs or allow one to cover the other.

Verification:

- matrix contains no inferred pass;
- every rejection cites observable evidence.

## Step 9 — route findings

Map the old v0.3.0/xz/cargo-path/Zellij chain to existing E-046 tickets.

Determine whether any behavior was observed against the fixed current surface.

Create a new bug only if a genuinely new current-surface product failure is
established.

Link the tour gap to T-047-01-02.

Verification:

- no duplicate ticket is created for baseline behavior;
- the rerun requirement stays on T-046-06-03;
- tour ownership stays on T-047-01-02.

## Step 10 — write the failed-attempt evidence artifact

Create `closing-attempts-2026-07-16.md`.

Lead with `NOT ADMITTED`.

Record shared controls and separate Claude/Codex sections.

Include short exact strings that establish the observed path.

Include the acceptance matrix and finding map.

Do not include secrets, complete transcripts, or environment dumps.

Verification:

- artifact is nonempty and self-contained;
- both attempts are explicitly non-admitted;
- every unavailable field says `NOT RECORDED`.

## Step 11 — write Progress

Record all completed phase artifacts.

Record the Docker inspection and privacy boundary.

Record evidence classification and finding routing.

Record source transaction status.

Record any plan deviations.

Record acceptance status and remaining operator work.

Verification:

- Progress distinguishes implementation completion from ticket acceptance;
- no source commit is claimed when no source changed.

## Step 12 — validate Implement artifacts

Run `git diff --check` for the attempt-private Markdown files.

Check required phase artifacts are nonempty.

Check the failed-attempt artifact contains both container names.

Check it contains `NOT ADMITTED` and `NOT RECORDED` markers.

Inspect `git status --short` for ownership drift.

Inspect the ordinary index for current-ticket shared source entries.

Verification:

- whitespace check passes;
- all expected files exist;
- no shared source path is ticket-modified;
- no source commit is required.

## Step 13 — assess acceptance criteria

Criterion 1 requires both primary legs to pass all controls.

Criterion 2 requires a 0.40.1 recovery using Lisa strings alone.

Criterion 3 requires the tour rematch page and comparison.

Assess each independently.

Do not convert partial evidence into a checklist pass.

Verification:

- Review names met/unmet status for all three;
- missing operator evidence remains blocking.

## Step 14 — write Review artifacts

Create `review.md` with:

- disposition summary;
- evidence inspected;
- files created;
- privacy treatment;
- per-attempt findings;
- acceptance mapping;
- verification coverage;
- open concerns;
- exact handoff.

Create one-line `review-disposition.json` with block schema.

Use remedy owner `operator`.

Include a one-sentence ask.

Include exact steps for both primary legs, seeded variant, and tour rematch.

Include a read-only artifact-presence check.

Verification:

- JSON parses;
- disposition is block;
- reason is nonempty;
- ask is one sentence;
- check does not perform remediation.

## Step 15 — final verification

Run Markdown whitespace checks including Review.

Parse and inspect the final JSON.

Verify all required files are present and nonempty.

Recheck repository status and index ownership.

Do not run Rust tests because no executable code changed.

Do not use `lisa commit-ticket` because no ticket-owned shared source unit
exists.

Verification:

- all artifact checks pass;
- current ticket source ownership is clean;
- unrelated dirty state remains untouched.

## Step 16 — stop on this ticket

After both Review artifacts exist, remain on T-046-06-03.

Do not edit ticket lifecycle fields.

Do not publish shared work artifacts manually.

Do not start another ticket.

Allow Lisa to park or retain the ticket according to the structured block.

## Plan conclusion

The plan completes every agent-side phase while preserving the human-operated
acceptance boundary.

Its deliverable is a precise evidence record and operator remedy, not an
unearned closing pass.
