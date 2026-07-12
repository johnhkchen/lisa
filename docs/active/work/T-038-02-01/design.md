# Design: T-038-02-01 cargo-fmt-clean

## Decision context

- The ticket asks for a canonical-format workspace.
- The required observable is a successful formatter check.
- Research found that the exact acceptance command already succeeds.
- Research also found no repository-local rustfmt configuration.
- The installed rustfmt defaults are therefore the canonical rules in force.
- No Rust source file is currently reported as modified.
- The two existing working-tree modifications belong to Lisa workflow state.
- The design must not claim or disturb those workflow-managed files.

## Goals

- Leave every workspace Rust target in canonical rustfmt form.
- Demonstrate that `cargo fmt --all -- --check` exits zero.
- Ensure any source diff, if one becomes necessary, is formatting-only.
- Keep unrelated and workflow-managed changes out of ticket commits.
- Finish with no ticket-owned staged, modified, or untracked source files.

## Non-goals

- Do not change runtime behavior.
- Do not refactor code while touching formatting.
- Do not change public or internal interfaces.
- Do not add or update dependencies.
- Do not introduce a repository rustfmt configuration.
- Do not alter Cargo workspace membership.
- Do not change tests merely to create a ticket diff.
- Do not update ticket phase or status manually.
- Do not publish artifacts to the shared work directory directly.

## Option 1: Preserve the clean tree and verify

- This option treats the acceptance command as the source of truth.
- It records the passing baseline as the implementation outcome.
- It makes no source edit because rustfmt reports none is required.
- It reruns the exact check during implementation and review.
- It inspects Git state to ensure no ticket-owned residue exists.
- It produces no `lisa commit-ticket` commit without a meaningful source unit.

### Advantages

- It directly satisfies the stated acceptance criterion.
- It produces the smallest possible change surface.
- It cannot accidentally mix behavioral edits into a formatting ticket.
- It respects shared-tree ownership boundaries.
- It avoids meaningless commit churn.
- It remains correct if prerequisite work already normalized formatting.

### Disadvantages

- The implementation phase has no source commit.
- Reviewers must rely on verification evidence rather than a code diff.
- A different toolchain could theoretically format defaults differently.

## Option 2: Run rustfmt in write mode, then commit any result

- This option runs `cargo fmt --all` without `--check`.
- Git diff would then reveal any formatter rewrites.
- Exact changed source paths would be passed to `lisa commit-ticket`.
- A second check would validate the rewritten tree.

### Advantages

- It automatically corrects any formatting drift that appears between phases.
- It naturally creates a formatting-only diff when drift exists.
- It follows the usual remediation path for a formatter ticket.

### Disadvantages

- On the observed clean tree, it is operationally redundant.
- In a shared workspace, write mode touches the ticket surface unnecessarily.
- It could interact with concurrent source modifications not owned by this ticket.
- A no-op invocation still provides no stronger evidence than check mode.

## Option 3: Add a rustfmt configuration

- This option would add `rustfmt.toml` or `.rustfmt.toml` at the root.
- It could pin explicit style choices or edition behavior.
- The workspace would then have repository-defined formatting policy.

### Advantages

- It makes formatting choices visible in version control.
- It can reduce dependence on implicit defaults.
- It can standardize non-default rules if the project wants them.

### Disadvantages

- The ticket does not request a policy change.
- Any new options could reformat the whole workspace unexpectedly.
- Stable and nightly rustfmt support different option sets.
- Configuration design is broader than making the existing tree fmt-clean.
- It would introduce a non-formatting policy artifact into the ticket diff.
- There is no research evidence that configuration is missing by mistake.

## Option 4: Make a harmless formatting perturbation and normalize it

- This option would edit a Rust file away from canonical style.
- rustfmt would then restore it to its original content.
- The final tree would remain clean.

### Advantages

- It exercises the formatter's rewrite path locally.

### Disadvantages

- It yields no durable change.
- It creates avoidable risk in a shared workspace.
- It does not improve acceptance evidence.
- It is artificial work unrelated to repository needs.
- It could obscure ownership if another ticket edits the same file.

## Evaluation criteria

- Acceptance fidelity: does the option prove the exact required command passes?
- Scope control: does it avoid behavioral or policy changes?
- Ownership safety: does it avoid unrelated shared-tree content?
- Reviewability: is the outcome easy to understand and reproduce?
- Durability: does it leave the repository in the required state?
- Transaction compliance: does it use Lisa commits only when source changes exist?

## Comparison

- Option 1 has the strongest acceptance fidelity because it uses the exact command.
- Option 2 has equal remediation ability but adds no value on a clean baseline.
- Option 3 expands scope from formatting state to formatting policy.
- Option 4 is artificial and has no durable outcome.
- Option 1 has the smallest ownership footprint.
- Option 1 is the clearest response to an already-satisfied state ticket.
- Option 2 remains the contingency if drift appears before final verification.

## Chosen design

- Choose Option 1: preserve the clean tree and verify.
- Do not create or alter Rust source while the exact formatter check passes.
- Do not invoke `lisa commit-ticket` without a ticket-owned source diff.
- Record the no-op result explicitly in `progress.md`.
- Run the exact acceptance command again during implementation.
- Run it once more as final review verification if the tree may have changed.
- Inspect `git status --short` after verification.
- Classify existing Lisa metadata changes as out of ticket scope.
- Confirm that no Rust path is staged, modified, or untracked.

## Contingency design

- If the exact check begins failing, run `cargo fmt --all` once.
- Inspect the resulting diff before committing anything.
- Reject any diff that changes non-Rust behavior or unrelated files.
- Identify every changed Rust path exactly.
- Commit only those paths with `lisa commit-ticket`.
- Use a message identifying the unit as canonical workspace formatting.
- Rerun `cargo fmt --all -- --check` after the transaction.
- Confirm the committed diff contains whitespace/layout changes only.

## Verification design

- Primary verification is `cargo fmt --all -- --check` from the root.
- Success requires process exit status 0.
- Success also requires no formatter-generated diff.
- Git verification checks ticket-owned source cleanliness.
- No runtime test is required for a zero-source-diff outcome.
- No compile test adds useful coverage to rustfmt's formatting predicate.
- The review will state the exact commands and observed results.

## Rationale

- Repository state, not the presence of a new commit, is the acceptance target.
- The current state already has the required property.
- Preserving it is safer than producing ceremonial edits.
- The workflow supports meaningful implementation units, not mandatory empty commits.
- A no-op implementation is therefore the most precise ticket execution.
