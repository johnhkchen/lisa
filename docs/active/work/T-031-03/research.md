# Research: T-031-03 provider contract and live regression

## Ticket scope

- T-031-03 is the convergence ticket for story S-031, atomic ticket completion.
- The ticket is stored at
  `docs/active/tickets/T-031-03-provider-contract-live-regression.md`.
- Its frontmatter begins this pass in `phase: research` and `status: open`.
- The requested short ticket filename does not exist; Lisa discovers the real
  suffixed Markdown path by ticket ID.
- T-031-01 and T-031-02 are complete and committed on the current branch.
- T-031-01 supplies a native isolated-index Git transaction.
- T-031-02 gates every scheduler Done publication on a successful native
  completion transaction.
- This ticket owns the remaining provider contract, upgrade compatibility,
  end-to-end regression, and user documentation work.
- The repository already contains unrelated modified and untracked files.
- Ticket edits and verification must remain path-scoped and must preserve the
  ordinary index.

## Existing atomic transaction

- `crates/lisa-cli/src/commit_transaction.rs` owns the Git implementation.
- `CommitTransactionRequest` accepts an explicit repository root, ticket ID,
  message, and repository-relative include paths.
- Include normalization rejects empty, absolute, escaping, and repository-wide
  pathspecs.
- The transaction discovers the repository and acquires
  `.lisa-commit.lock` for the entire critical section.
- It snapshots the ordinary index before creating any ticket commit.
- It reserves a temporary alternate index under the Git directory.
- `GIT_INDEX_FILE` points preparation commands at that alternate index.
- The alternate index starts from the current `HEAD` tree.
- Git stages only explicit include paths with pathspec-scoped `git add -A --`.
- Ticket changes never become staged entries in the ordinary index.
- Concrete changed paths are compared with the foreign staged snapshot.
- Any overlap with an ordinarily staged path fails before ref movement.
- `git write-tree` and `git commit-tree` create the exact ticket commit.
- `git update-ref HEAD <new> <old>` provides guarded ref movement.
- A targeted reset reconciles only committed paths in the ordinary index.
- Exact ordinary stage tuples are verified after reconciliation.
- Cleanup removes alternate-index files and releases the advisory lock.
- Post-ref failure attempts a guarded rollback before reporting failure.
- The public `lisa commit-ticket` command exposes repeated `--include` paths.
- The command prints the commit ID only after the transaction succeeds.

## Existing completion boundary

- `CompleteTicketRequest` accepts the real ticket Markdown path and work path.
- `complete_ticket` saves the ticket's exact original bytes.
- It updates phase and status to Done in one filesystem write.
- It calls the isolated transaction with the ticket file and full work directory.
- A failed transaction restores the original ticket bytes.
- An already committed, clean Done ticket is handled idempotently.
- The public `lisa complete-ticket` command exposes this wrapper.
- The plugin builds the native command with explicit ticket and work paths.
- `pending_completions` records prior phase/status and the completion source.
- Pending Done is masked during DAG rebuilds.
- Artifact, idle, stopped, manual, and observed-Done triggers share one request.
- A thread, pane, dependent, and provenance record remain blocked while pending.
- Only an attributed zero exit with a plausible commit hash can publish Done.
- Failure keeps the Review thread and its provider seat recoverable.
- Success publishes once, releases the seat, and permits dependents to schedule.

## Source ownership boundary

- The completion wrapper includes the ticket file and work directory only.
- It intentionally does not scan the shared working tree for arbitrary changes.
- A shared tree cannot reveal which agent or human owns a modified source file.
- Broad inference could steal another ticket's or a human's work.
- T-031-01 therefore makes explicit include paths the ownership authority.
- T-031-02 requires implementation changes to be committed before Review ends.
- The existing RDSPI text says only “commit incrementally.”
- It does not say to use `lisa commit-ticket`.
- It does not forbid ordinary-index `git add`, `git add -A`, or staged handoff.
- It does not explain that the agent must leave no ticket-owned residue.
- The final completion commit records Done plus all work artifacts.
- Earlier isolated implementation commits make the final source tree durable.
- The completion commit's tree therefore contains the final ticket code even
  when the code changes themselves appear in earlier ticket commits.

## Prompt surfaces

- `ticket_prompt` in `crates/lisa-plugin/src/lib.rs` is the common prompt body.
- It is used for both fresh provider launches and same-provider reuse.
- Claude receives `CLAUDE.md` as its context filename.
- Codex receives `AGENTS.md` as its context filename.
- Both adapter implementations delegate `reuse_prompt` to `ticket_prompt`.
- Both adapter implementations delegate Review follow-up to
  `finish_up_prompt`.
- The common prompt tells agents to work through all remaining phases.
- It tells agents to write artifacts and not edit phase/status frontmatter.
- It currently ends with “simply stop — Lisa handles the rest.”
- It does not state how implementation changes must be committed.
- It does not tell agents to keep the ordinary index untouched.
- It does not explicitly wait for Lisa's durable completion confirmation.
- `finish_up_prompt` repeats the frontmatter prohibition.
- It does not mention isolated commits, staged residue, or seat reuse gating.
- Provider initial and reuse paths are single-sourced, reducing drift risk.
- Follow-up delivery differs only mechanically; the text is also single-sourced.

## Bundled and installed workflow copies

- `crates/lisa-cli/data/rdspi-workflow.md` is the bundled source.
- `templates::RDSPI_WORKFLOW` embeds it at compile time.
- `docs/knowledge/rdspi-workflow.md` is this repository's installed copy.
- The two copies currently carry the same pre-ticket contract.
- `lisa init` plans workflow ownership through `plan_owned_template`.
- Exact current content becomes a no-op.
- Exact known legacy content is upgraded to the current bundled content.
- Unknown/customized content produces a safety skip and remains byte-identical.
- `LEGACY_RDSPI_WORKFLOWS` currently contains the v0.2 five-phase template.
- Changing the bundle without preserving the outgoing six-phase bytes as a
  legacy template would strand clean existing Lisa installations.
- Adding the outgoing template to `data/legacy/` makes clean upgrades provable.
- Existing arbitrary project customization tests already assert preservation.
- Upgrade tests currently cover only the first legacy workflow entry.
- A new regression must distinguish outgoing-clean content from customization.

## User-facing documentation

- README's workflow section says there are five phases despite Review existing.
- README's Implement step says “commit incrementally” without isolation details.
- README describes commit serialization generally as file locking.
- README does not explain the alternate index or foreign staged preservation.
- README does not describe completion-commit failure recovery.
- `docs/knowledge/lisa-loop-setup-guide.md` has older lifecycle instructions.
- It says the agent updates frontmatter after every phase.
- It says the agent sets Done and Lisa reacts afterward.
- Those statements conflict with the current artifact-driven scheduler.
- Its artifact tree omits `review.md`.
- Its recovery guidance does not cover a failed completion transaction.
- Both documents are user-visible surfaces named by the acceptance criteria.

## Existing tests

- CLI process tests use real temporary Git repositories.
- They cover foreign staged preservation, explicit paths, lock contention,
  rollback, completion bytes, and already-Done idempotence.
- Plugin state tests cover pending completion, failure/retry, dependents,
  reused Codex seat retention, and exact-once provenance.
- Prompt tests currently assert only paths, context files, and phase wording.
- Adapter tests prove Claude and Codex reuse/follow-up delegate to common text.
- Init tests cover current workflow no-op, known legacy upgrade, and unknown
  customization preservation.
- No checked-in test composes five Codex-routed tickets through repeated seat
  reuse while a foreign ordinary-index entry remains staged.
- No regression captures commit-tree, index, activity, and provenance evidence
  together for later diagnosis.
- Existing live-loop scripts are manual runbooks and describe older Codex paths.
- A deterministic harness can exercise the durable Git contract without a
  network account or nested interactive Zellij session.

## Harness constraints

- The harness repository must be outside the Lisa source tree.
- A temporary directory is the established repository-test pattern.
- At least five tickets must route to Codex.
- One ticket must depend on an earlier ticket.
- A single logical seat must be reused rather than allocating five seats.
- A mixed Claude/Codex case must exercise the same durable invariant.
- A foreign file must stay staged before, during, and after all completions.
- Each agent implementation unit must use `lisa commit-ticket` with exact paths.
- Each Done transition must use `lisa complete-ticket`, not direct frontmatter.
- The dependent may start only after its prerequisite completion hash exists.
- Evidence must include commit ancestry/tree data, ordinary-index tuples,
  activity order, route/seat attribution, and provenance-like completion rows.
- Runtime-only evidence should be written under the temporary repository.
- The checked-in harness and assertions are reusable; generated evidence is not.

## Validation surface

- The ticket requires `lisa validate` against the fixture project.
- It requires focused tests and the workspace suite.
- It requires a release WASM build.
- It requires plugin Clippy with warnings denied.
- `just check` covers WASM checking and workspace tests.
- Shell harness assertions should fail fast and print the retained fixture path.
- Rust prompt/init tests should protect exact behavioral phrases rather than a
  brittle full prompt snapshot.
- `git diff --check` should be scoped to ticket-owned files.

## Constraints carried into Design

- Do not update this ticket's phase or status manually.
- Preserve the shared ordinary index and unrelated worktree changes.
- Keep provider wording single-sourced where provider behavior is identical.
- Make every agent-side commit use Lisa's alternate-index transaction.
- Keep code ownership explicit; never introduce repository-wide staging.
- Preserve customized installed workflows under S-030 ownership rules.
- Make the regression deterministic and runnable without paid provider access.
- Still model Codex routing, reuse, dependency release, and mixed-provider flow.
- Leave the seat blocked until the completion commit is confirmed.
- Record sufficient durable evidence to distinguish scheduling, Git, and
  provider-attribution failures.
