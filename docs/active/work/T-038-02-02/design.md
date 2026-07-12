# T-038-02-02 Design: Clippy Zero Warnings

## Decision context

Research established that both required lint surfaces already return success
with warnings denied. There is no Clippy diagnostic to remediate, and the source
tree is clean of ticket-owned changes. The design question is therefore how to
satisfy and preserve the acceptance criterion without introducing an unrelated
change.

The design is constrained by four facts:

1. The ticket requires zero warnings on native and WASM targets.
2. The ticket requires the commands and output to be recorded.
3. It permits no behavior change beyond what a lint requires.
4. Lisa owns phase transitions and completion publication, while source changes
   must be committed through an isolated ticket transaction.

## Option 1: Validation-only completion

Run the native workspace command and the repository-standard WASM plugin command
with `-D warnings`, record their exact output and exit status, run supporting
format/test/check gates, and make no source change.

### Benefits

- Directly tests the two acceptance surfaces.
- Treats successful warning-strict commands as proof rather than relying on a
  subjective reading of ordinary warning output.
- Preserves behavior exactly because no source is edited.
- Preserves existing CI and `justfile` conventions.
- Avoids speculative cleanup that is outside ticket scope.
- Leaves no ticket-owned source file requiring an isolated commit.
- Produces a concise, auditable record in `progress.md` and `review.md`.

### Costs

- The implementation phase consists primarily of verification rather than code.
- The zero-warning invariant still depends on the current toolchain and may need
  renewed maintenance after future Clippy releases.
- A cached build produces shorter output than a clean build, though Cargo still
  evaluates the requested packages and the exit code remains authoritative.

## Option 2: Change local lint orchestration

Modify the `justfile` so its native commands collapse into
`cargo clippy --workspace -- -D warnings`, retaining the separate WASM command.

### Benefits

- The local recipe would spell the native acceptance command verbatim.
- It could reduce duplication between the `lisa-core` and `lisa-cli` lines.

### Costs

- The existing recipe is already warning-strict and clean.
- The change is not required by any diagnostic.
- It changes developer workflow behavior despite the ticket's behavior-change
  constraint.
- Package-specific commands currently mirror CI steps and make failures easier
  to attribute.
- It creates a source/configuration commit with no acceptance benefit.

## Option 3: Change CI to add a workspace lint job

Replace or supplement package-specific native CI checks with
`cargo clippy --workspace -- -D warnings`.

### Benefits

- CI would visibly contain the exact native command named by the ticket.
- One job step would cover every current workspace package.

### Costs

- Existing CI already checks all three relevant crate/target combinations.
- Native workspace Clippy is not a substitute for target-specific plugin Clippy,
  so the WASM step would still remain.
- Changing CI is not necessary to eliminate a warning.
- A duplicated workspace step would increase CI time without expanding current
  lint coverage materially.
- Replacing per-package steps would reduce failure localization.
- This would violate the narrowest reading of “no behavior change beyond what a
  lint requires.”

## Option 4: Force source churn with stylistic refactors

Search for code that could be manually modernized even though Clippy does not
currently warn, then submit those changes as lint cleanup.

### Benefits

- Produces a conventional code diff for an implementation ticket.
- Might preempt some future lint depending on toolchain evolution.

### Costs

- No observed diagnostic supplies evidence that a refactor is required.
- Hand-selected cleanup can change behavior or readability.
- It makes the validation result harder to audit because unrelated changes enter
  the diff.
- It risks introducing new warnings or regressions.
- It conflicts directly with the ticket's narrow behavior constraint.
- It would manufacture work instead of demonstrating the requested invariant.

## Option 5: Add lint allow/suppression configuration

Add crate attributes or Clippy configuration to pin or suppress categories of
warnings preemptively.

### Benefits

- Could make future toolchain changes less likely to break lint commands.

### Costs

- There are no warnings to suppress.
- Suppression weakens the meaning of zero warnings.
- Broad allows could hide future actionable diagnostics.
- Pinning lint levels without an observed need creates maintenance overhead.
- This option works against the story principle that state should mean what it
  says.

## Chosen approach

Use Option 1, validation-only completion.

The decisive reason is that warning-strict Clippy already proves the desired
state on both required targets. No lint identifies a source unit that should be
changed. Under the ticket constraint, absence of a code diff is the correct
implementation outcome rather than an omission.

The final verification will use these primary commands:

```text
cargo clippy --workspace -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

The first command exactly matches the ticket's native scope and strengthens it
by denying warnings. The second follows the package/target boundary established
by both CI and the local `lint` recipe, also with warnings denied.

Supporting gates will be:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo check -p lisa-plugin --target wasm32-wasip1
```

Formatting confirms that the checkout is not carrying an unrelated formatting
issue. Tests guard behavior at the native test boundary. The WASM check confirms
ordinary target compilation in addition to Clippy's lint compilation.

## Output recording design

- `progress.md` will name every command exactly.
- It will record exit status and the complete meaningful command output.
- Cargo cache notices and completion lines will be retained where emitted.
- The warning count will be stated explicitly as zero for each primary command.
- Supporting gate results will be summarized with test counts or relevant Cargo
  completion output.
- `review.md` will repeat the final acceptance evidence at handoff level.
- No separate generated log file is necessary because the phase artifacts are
  the durable, human-readable evidence required by the assignment.

## Source and commit design

- No Rust, Cargo, CI, or `justfile` change is planned.
- Therefore there is no meaningful ticket-owned source unit to pass to
  `lisa commit-ticket`.
- The command must not be invoked with an empty or artificial include set.
- Existing Lisa-controlled modifications will remain untouched.
- Phase artifacts will remain in the private attempt work directory.
- Lisa will publish admitted artifacts and prepare the completion commit after
  Review.

## Failure handling

- If either warning-strict command reports a diagnostic, the diagnostic's source
  file becomes a ticket-owned unit.
- Only the smallest semantics-preserving lint remediation would then be made.
- The focused crate command would be rerun after each remediation.
- The changed source unit would be committed with `lisa commit-ticket`, an exact
  repository-relative include path, and a message describing the lint fix.
- Full native and WASM gates would then be rerun and recorded.
- If a failure comes from missing tooling rather than source, the progress record
  would distinguish environment failure from a lint failure.

## Rejected approaches

- Local recipe changes are rejected because the current recipe is already clean
  and target-aware.
- CI restructuring is rejected because current CI already denies warnings for
  every relevant crate/target combination.
- Speculative refactors are rejected because no lint requires them.
- Suppressions are rejected because they would weaken rather than demonstrate
  the invariant.
- A clean rebuild is not required as the primary proof; Cargo dependency caching
  does not bypass Clippy's fingerprinting or warning-denial semantics.

## Expected result

The expected implementation produces no ticket-owned source diff, records two
successful warning-strict Clippy invocations with zero diagnostics, confirms the
supporting gates, and hands Lisa a complete six-phase artifact set. This is the
smallest approach consistent with both the acceptance criterion and the explicit
behavior-preservation boundary.
