# Research: T-035-03-02 fresh-loop live startup harness

## Ticket boundary

The ticket begins in Research and owns the final live validation slice of S-035-03.

Its deliverables are a committed reusable harness/runbook and a recorded execution.

The harness must use a freshly built Lisa CLI whose embedded WASM comes from the same
checkout, not the already-running parent loop.

It must exercise two independently first-ready arrangements: Codex first and Claude first.

Each arrangement must proceed without terminal command repair and without interactive
project-trust intervention.

The proof obligation ends at accepted first assignment and Owned publication; provider
completion of the synthetic ticket is useful cleanup evidence but is not the ticket's
primary boundary.

## Failure lineage

T-034-03-02 used an isolated fresh loop and exposed the original defect.

The first provider launch embedded the complete RDSPI assignment in a long shell command.

Real zsh received only a prefix, entered `dquote>`, and never started either provider.

Lisa nevertheless published the seat as assigned/owned.

Manual command completion and a Codex trust confirmation allowed later work to continue,
but disqualified that run as unattended startup evidence.

The same run proved a later Claude assignment could work after the loop was already live,
which localized the failure to initial pane delivery rather than either model parser.

## Implemented production contract

The E-035 prerequisite tickets have now landed on the current branch.

Fresh native dispatch atomically writes an attempt-private `assignment.md` containing the
complete ticket instructions.

It also writes an attempt-private launch script containing lifecycle environment and a
bare provider command.

The launch script intentionally contains no ticket prompt and no `LISA_ASSIGNMENT` tag.

The provider's `SessionStart[startup]` hook copies the scheduler-owned lease to
`.lisa/signals/pane-<id>.started` only when pane, ticket, and attempt identity match.

The plugin consumes that exact signal and changes `Starting` to `ReadyForAssignment`.

Ready assignments are collected before start signals are consumed in each poll, leaving
readiness observable for one scheduler boundary.

On the following boundary the plugin sends a bounded two-line chat reference to the
attempt-private assignment file and changes the state to `Delivering`.

Both provider hook configurations route `UserPromptSubmit` payloads through
`.lisa/hooks/on-ack.sh` to `pane-<id>.ack`.

The plugin parses the exact `LISA_ASSIGNMENT` ticket/generation marker and changes only a
matching current Delivering seat to Owned.

## Existing deterministic regression

T-035-02-01 committed a real-Zellij, real-zsh, model-free integration harness at:

- `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs`;
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.

It executes the freshly built Cargo test CLI and embedded production WASM.

Its stub provider gates process start and acknowledgement independently, allowing strong
negative assertions before each boundary.

The success scenario proves bare launch, start-signal consumption, Delivering, matching
acknowledgement, and Owned ordering.

Other scenarios prove bounded missing-start recovery, missing-ack retry/failure, and real
`dquote>` same-pane recovery.

That harness deliberately routes only a local executable named `claude`; it cannot prove
installed Codex and Claude accept the actual bounded chat in their native TUIs.

## Available live-provider environment

The host currently provides:

- Claude Code 2.1.207 at `/Users/johnchen/.local/bin/claude`;
- Codex CLI 0.144.1 at `/Users/johnchen/.local/bin/codex`;
- Zellij 0.44.3 at `/opt/homebrew/bin/zellij`;
- BSD `script` at `/usr/bin/script`;
- installed Rust target `wasm32-wasip1`.

The current shell also has Lisa 0.4.0-rc.6, but its installed binary is not valid proof.

The repository release build path is WASM first, then CLI, because `lisa-cli/build.rs`
embeds the already-built release WASM.

`just build-cli` encodes that ordering.

## Codex trust prerequisite

T-035-03-01 changed `pregrant_codex_trust_in` to canonicalize existing project roots.

This covers macOS `/var/...` to `/private/var/...` identity before writing Codex's user
configuration table.

Its symlink regression proves the emitted trust key equals a canonicalized cwd.

For live isolation, replacing HOME or CODEX_HOME would also replace existing provider
authentication and is therefore inappropriate.

The harness can use the real provider home while relying on Lisa's idempotent trust
pregrant for the disposable canonical project path.

The run must inspect the resulting exact project header and record that it equals
`pwd -P` for the fixture.

## Harness topology

Codex-first and Claude-first must each be the first assignment in a new Lisa plugin and
new Zellij session.

A single fixture with sequential tickets would prove only the first provider at the
fresh-loop boundary; the dependent provider would be a warm-loop assignment.

Therefore the reusable harness needs two isolated project/session cases.

Each case can contain one artifact-only ticket explicitly routed to its provider.

The project must have a committed Git baseline because Lisa's completion path and ticket
transactions require a repository.

`lisa init` supplies the current hooks, workflow, ignore rules, and provider configs.

The fixture ticket should instruct the provider to write the six short RDSPI artifacts,
make no product changes, and stop after Review.

## Evidence surfaces

The launch script is durable under `.lisa/attempts/<ticket>/<attempt>/work/` and can be
checked for the bare provider command and absence of assignment prose.

The assignment file is durable at the same attempt boundary and can be checked separately.

Zellij dashboard snapshots expose `ready-for-assignment`, `delivering`, and `owned`.

The readiness interval is intentionally at least one plugin poll, but external screen
polling can still miss a short rendered frame; the harness must poll frequently and retain
every distinct dashboard state.

The plugin activity viewport records delivery and acknowledgement messages, providing a
second order witness if a transient row snapshot is difficult to capture.

Signal-directory snapshots can record the lease/start/ack lifecycle, although consumed
files are removed and polling them alone is not authoritative.

Provider terminal snapshots show the installed native TUI, accepted prompt, and absence
of an interactive trust screen.

The completed ticket, six admitted artifacts, Git log, and provenance prove the bounded
assignment was actually acted upon rather than merely typed.

The generated `.lisa-layout.kdl` records the exact Lisa executable and extracted WASM.

SHA-256 comparison between target and extracted WASM proves the fresh plugin bytes.

## Reuse and source ownership

The deterministic harness already contains reliable helpers for PTY launch, unique named
sessions, pane discovery, dashboard dumps, time-bounded waits, cleanup, and fresh CLI use.

The live harness has different provider/home/evidence requirements and should not overload
the deterministic failure scenarios.

A sibling shell harness can reuse the same operational patterns while remaining directly
invocable as an authorized metered validation.

A committed Markdown runbook should document prerequisites, invocation, evidence layout,
and the fact that the command starts real provider turns.

The likely ticket-owned source paths are limited to that shell harness and runbook.

The recorded live output belongs in the attempt-private work artifact tree for Lisa to
publish, rather than in an environment-specific checked-in fixture output directory.

## Repository constraints

The parent worktree contains unrelated modified and untracked Lisa runtime/project files.

They must remain untouched and excluded from ticket commits.

Ticket source must be committed only with `lisa commit-ticket` and exact paths.

Ordinary index operations are prohibited for parent ticket work.

Temporary fixture repositories may use normal Git commands because they are independent
of the parent repository and exist solely for live proof.

Ticket frontmatter phase/status and shared `docs/active/work` publication are Lisa-owned.

## Research conclusion

The production behavior and deterministic real-Zellij boundary regression already exist.

The remaining gap is a reusable installed-provider control that creates two truly fresh
loops, validates build identity and trust identity, records launch/readiness/delivery/ack
ordering, and retains enough durable evidence to distinguish accepted assignment from a
green dashboard label.

No scheduler change is indicated by current code or prerequisite results.
