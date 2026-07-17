# Research — T-047-01-02 probe rematch on RC surface

## Ticket identity and boundary

- Ticket: `T-047-01-02`.
- Story: `S-047-01`.
- Title: `probe-rematch-on-rc-surface`.
- Type: task.
- Starting status: open.
- Starting phase: research.
- Dependency: `T-047-01-01`.
- The dependency is complete at commit
  `528ca498c720f869774685bf13fd64235941437b`.
- This ticket is the measurement leg of the story.
- It does not own another implementation of the purpose copy or run summary.
- The measured output is a landing page produced by a coding agent.
- The run is explicitly human-operated and token-metered.
- John operates the container and agent CLI.
- The ticket agent prepares, verifies, scores, and records the evidence.
- Missing manual evidence must produce a named blocked state.
- Missing evidence must never produce a fabricated series entry.

## Assignment and publication rules

- `AGENTS.md` delegates repository context to `CLAUDE.md`.
- `CLAUDE.md` identifies Lisa as a Rust workspace with a CLI, core crate, and
  Zellij WASM plugin.
- The RDSPI workflow requires Research, Design, Structure, Plan, Implement,
  and Review in order.
- The assignment redirects phase artifacts to this attempt-private directory:
  `.lisa/attempts/T-047-01-02/1/work/`.
- Lisa publishes admitted artifacts to `docs/active/work/T-047-01-02/`.
- The agent must not write phase artifacts directly to the shared work path.
- Lisa owns ticket phase and status transitions.
- The agent must not edit the ticket frontmatter phase or status.
- Ticket-owned shared source changes must use `lisa commit-ticket` with exact
  repository-relative include paths.
- Ordinary `git add` and `git commit` are outside this ticket workflow.
- Review requires both `review.md` and `review-disposition.json`.

## Landing-probe knowledge surface

- The authoritative benchmark description is
  `docs/knowledge/landing-probes/README.md`.
- The short preferred prompt is:
  “You just got lisa. Play with it, then make lisa-tour.html so the next
  person starts faster.”
- A second method lets an agent scaffold a Lisa project whose ticket chain
  builds the page, then executes that chain with `lisa loop`.
- The ticket prioritizes the loop-built method.
- That method is comparable to baseline entry b.
- The benchmark describes the page as both a tutorial experience and a
  comprehension readout.
- Its stated point is that Lisa runs coding agents through a ticket board so
  the operator does not babysit or approve every step.
- Its stated audit benefit is an evidence trail available afterward.

## Published rubric

- Column 1 is Actors.
- Actors passes when the headline or first paragraph names coding agents.
- Claude Code or Codex should be named.
- Column 2 is Benefit.
- Benefit passes when the page states no babysitting, no per-step approvals,
  or an equivalent walk-away-and-return operator outcome.
- Column 3 is Evidence trail.
- Evidence passes when the page mentions provenance, the completion journal,
  or per-ticket work documents as an audit story.
- Column 4 is Purpose before mechanism.
- It passes when DAG, scheduling, or Zellij vocabulary follows the purpose.
- Scores are yes/no in the published series.
- The prior b row uses `partial` for historical evidence-trail nuance.
- This ticket's acceptance explicitly requires yes for columns 1 through 3.
- A miss in columns 1 through 3 is not resolved by changing the rubric.
- A miss instead creates a concrete copy ticket and leaves this ticket open.

## Existing series entries

- The series currently has two rows, both dated 2026-07-16.
- Entry a is `2026-07-16-a-direct-codex-mini.html`.
- Entry a used gpt-5.4-mini through a direct tour.
- Its recorded surface is Lisa 0.3.0.
- It scored no in all four rubric columns.
- Its page described Lisa as a ticket-graph tool.
- “coding agent” was absent from the page.
- Entry b is `2026-07-16-b-loop-built-claude.html`.
- Entry b used Claude Code via a three-ticket `lisa loop` chain.
- Its recorded surface is Lisa 0.3.0 with Zellij 0.44.3.
- It scored yes for Actors.
- It scored no for Benefit.
- It scored partial for Evidence.
- It scored no for Order.
- Its headline names Claude Code agents and concurrency.
- Its footer cites work-document transcripts.
- It contains no no-babysitting framing.
- Its title begins with mechanism wording.
- The README warns that entries a and b changed both method and model class.
- Their delta is therefore directional rather than attributable.

## Required comparison axis

- The ticket asks future runs to vary one axis at a time.
- A loop-built run is the priority because it holds entry b's method constant.
- A comparable model/CLI would further isolate the surface-copy change.
- The intended changed axis is the Lisa surface.
- The old side is Lisa 0.3.0 plus the 2026-07-16 loop-built experience.
- The new side is the RC surface carrying T-046-07-* and T-047-01-01.
- The series row must state this comparison explicitly.
- Method, model, CLI, surface version, and fixture must be factual run data.

## Purpose-copy changes already present

- T-046-07-01 landed CLI-facing purpose-first wording.
- Its source commit is `e43f4d5`.
- T-046-07-02 landed README and generated-context wording.
- Its source commits are `e90ae07` and `6633cf4`.
- T-047-01-01 landed managed-session purpose context and run reporting.
- Its source commits are `4d1384e`, `e150f40`, `931aa02`, `cd9c439`,
  `c506744`, and `c4675cb`.
- The dependency completion commit is `528ca49`.
- The canonical purpose paragraph is in
  `crates/lisa-core/src/context.rs`.
- It states that Lisa runs coding agents like Claude Code and Codex through a
  ticket board so the operator does not approve every step by hand.
- `crates/lisa-cli/src/templates.rs` imports that constant.
- The rendered RDSPI workflow begins with the constant.
- Managed ticket assignments also begin with the constant.
- The raw workflow body remains mechanics-only.

## Run-summary surface already present

- `crates/lisa-cli/src/run_summary.rs` owns the factual narrative.
- It records pre-run byte offsets in `.lisa/run-baseline.json`.
- It reads only provenance and interaction rows appended after that baseline.
- It counts completion from the current ticket board.
- It counts failures and timeouts from latest-run provenance.
- It counts question and permission gates from latest-run events.
- A fully completed clean run with zero gates prints the unattended-win line.
- The summary prints the completed-ticket count.
- It prints `Manual approvals requested: 0.` only with supporting tracking.
- It names evidence paths only when they exist.
- Candidate evidence paths include `.lisa/provenance.jsonl`.
- They include `.lisa/completion-journal.jsonl`.
- They include the configured per-ticket work directory when real documents
  exist for current tickets.
- `lisa status` and post-loop reporting share this renderer.
- T-047-01-01's published review records 972 passing workspace tests, zero
  failures, and one intentionally ignored external-boundary test.

## Version and surface observations

- The workspace version in root `Cargo.toml` is `0.4.0-rc.8`.
- The installed `lisa` executable is `/Users/johnchen/.local/bin/lisa`.
- `lisa --version` reports `lisa 0.4.0-rc.8`.
- The ticket prose calls the intended measurement surface `0.4.1-rc`.
- No `0.4.1` version string is present in the inspected Cargo manifests.
- The current branch nevertheless contains the required T-046-07 and
  T-047-01-01 source commits.
- A recorded probe must distinguish the semantic release-train label from the
  exact executable version and revision actually run.

## Evidence inventory at attempt start

- `docs/knowledge/landing-probes/` contains only the README and the two
  2026-07-16 HTML files.
- No new dated HTML artifact exists there.
- This attempt-private directory initially contains only the assignment and
  Lisa launch helper.
- No generated `lisa-tour.html` is present in the attempt directory.
- No manual-run metadata record is present in the attempt directory.
- No container or fixture identity for a rematch is present.
- No new model or CLI identity is present.
- No prompt transcript or execution chronology is present.
- No post-run summary capture is present.
- No human attestation of a hands-off run is present.
- Repository-wide search found no other rematch artifact tied to this ticket.
- The current provenance row for T-047-01-02 describes this assignment agent,
  not an independently operated landing probe.
- It cannot substitute for the required manual probe evidence.

## Worktree ownership observations

- The branch was already ahead of `origin/main` when this attempt began.
- Lisa-managed journal files were modified.
- Active ticket frontmatter files were modified.
- `crates/lisa-plugin/src/lib.rs` had a concurrent modification.
- Several shared work directories were untracked.
- None of those paths are owned by this measurement attempt.
- The attempt must preserve them without reset, stash, staging, or inclusion.
- A new landing-probe HTML and README row would be ticket-owned only after
  admissible manual evidence exists.

## Acceptance-state facts

- Acceptance criterion 1 requires both a new dated artifact and a new series
  row.
- Neither exists.
- Criterion 1 also requires method, model, surface version, and scores.
- Those run facts are unavailable.
- Acceptance criterion 2 requires yes for Actors, Benefit, and Evidence.
- There is no page to inspect for those statements.
- Therefore none of the required scores can be honestly assigned.
- The ticket context defines this exact absence as a blocked boundary.
- No repository-only unit test can replace the missing field measurement.
- No agent-generated surrogate page in this assignment session would be the
  specified human-operated, metered experiment.
