# Structure — T-036-01-02: plain, verb-forward command help

The blueprint. One file changes; the change is a set of `///` doc-comment
replacements on the `Commands` enum variants. No new files, no deletions, no
moved code.

## Files

| File | Change | Owner |
|------|--------|-------|
| `crates/lisa-cli/src/main.rs` | Rewrite `///` doc comments on 12 `Commands` variants. Nothing else. | this ticket |

Nothing created or deleted. No test file (T-036-01-03).

## Exact edits, top to bottom of `enum Commands`

Each edit replaces only the `///` line(s) directly above a variant. The
`#[command(...)]` attributes and the variant/field bodies are left byte-for-byte
identical. Order below follows source order in `main.rs`.

### Operator commands (the AC-gated five)

1. **Init** (`main.rs:31`)
   - from: `/// Initialize a project for lisa-loop completion`
   - to:   `/// Set up a project to run with Lisa.`

2. **Validate** (`main.rs:42`)
   - from: `/// Validate ticket DAG and project setup`
   - to:   `/// Check your tickets and project setup for problems before a run.`

3. **Status** (`main.rs:53`)
   - from: `/// Show DAG status: tickets, dependencies, execution waves, scheduling readiness`
   - to:   `/// Show which tickets are ready to run and which are waiting, and why.`

4. **Doctor** (`main.rs:71`)
   - from: `/// Check that all runtime dependencies are installed`
   - to:   `/// Check that the tools Lisa needs are installed.`

5. **Loop** (`main.rs:167`)
   - from: `/// Launch zellij with the Lisa plugin for DAG-driven task scheduling`
   - to:   `/// Start a run: work through the ready tickets, in parallel where they don't collide.`

### Hidden commands

6. **SetupGuide** (`main.rs:60`)
   - from: `/// Output LLM-friendly setup instructions for this project`
   - to:   `/// Print setup instructions for an agent to follow.`

7. **HooksGuide** (`main.rs:67`)
   - from: `/// Output the hooks setup guide for agents configuring Claude Code hooks`
   - to:   `/// Print the guide for wiring up Claude Code hooks.`

8. **Version** (`main.rs:78`)
   - from: `/// Print version information`
   - to:   `/// Print Lisa's version.`

### Hook/plumbing commands (reworded, precise machinery nouns retained)

9. **AgentExec** (`main.rs:80–86`) — replace only the first summary sentence;
   keep the multi-line body (LISA_PANE_ID/LISA_TICKET_ID, `codex exec --json`,
   signal files, "native Codex TUI" note) verbatim.
   - from first line: `/// Run Codex under Lisa's legacy JSON signal/rendering wrapper.`
   - to first line:   `/// Run Codex and turn its output into Lisa's pane signals.`

10. **CaptureUsage** (`main.rs:117–118`)
    - from: `/// Capture Claude session token usage from a Stop-hook payload on stdin,`
            `/// writing .lisa/claude/<ticket>.usage.json for the provenance ledger.`
    - to:   `/// Record a Claude session's token usage from its Stop-hook payload on stdin,`
            `/// writing .lisa/claude/<ticket>.usage.json for the provenance ledger.`

11. **CommitTicket** (`main.rs:125`)
    - from: `/// Commit ticket-owned paths without using the repository's ordinary index.`
    - to:   `/// Commit this ticket's own files without touching the repo's ordinary git index.`

12. **CompleteTicket** (`main.rs:144`)
    - from: `/// Mark a ticket done and commit its loop-owned files atomically.`
    - to:   `/// Mark a ticket done and commit its files in one step.`

## Interface / behavior surface

- **No public interface changes.** clap still derives the same subcommand names,
  flags, and dispatch. Only the human-readable help text differs.
- **No behavior changes.** `main()`'s match arms reference variant *names*, never
  doc comments, so the dispatch is provably untouched.
- **Rendering effect.** `lisa --help`'s command list shows the new first lines;
  each `lisa <cmd> --help` opens with the new sentence. AgentExec's long help
  keeps its detailed body.

## Ordering of changes

All twelve edits are independent (disjoint line ranges, no interdependency), so
ordering is cosmetic. They will be applied in one pass and committed as a single
meaningful unit — the ticket is one atomic copy change to one file. See plan.md.

## Risk / invariants to preserve

- Each `///` line must remain a valid Rust doc comment (leading `/// `).
- First line of each variant's comment must stand alone as a sentence (clap
  reuses it as the short help in the parent list).
- Do not alter indentation of the `#[command(...)]` attributes or fields; the
  edit targets only the comment text so the surrounding diff is minimal.
- Banned terms (`DAG`, `orchestrat*`, `scheduling`, `leverage`, `solutions`)
  must not appear in the five operator strings after the edit.
