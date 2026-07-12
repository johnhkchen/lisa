# T-038-02-03 Design: Verification and Evidence Strategy

## Design objective

The ticket needs a trustworthy, reproducible answer to one question: does the
formatting- and Clippy-clean workspace still pass its native test suite and its
WASM compilation gate? The design must produce durable evidence without changing
runtime behavior, weakening checks, or claiming ownership of Lisa-managed files.

## Decision drivers

- Match the acceptance wording directly.
- Match repository-local developer workflows and CI.
- Verify the tightened baseline rather than assume predecessor evidence remains
  sufficient.
- Record exact commands, exit statuses, and meaningful output summaries.
- Exercise the deployable WASM compilation boundary proportionately.
- Avoid source changes when verification succeeds.
- Preserve the ordinary index and unrelated working-tree changes.
- Keep all phase evidence in the private attempt directory.
- Make the final Review understandable without requiring raw terminal history.

## Option 1: Run only `just check`

### Shape

Run the root recipe once:

```text
just check
```

This sequentially executes the ordinary WASM `cargo check` and native workspace
tests.

### Advantages

- It is the repository's default developer gate.
- It directly covers both named acceptance outcomes.
- It matches the same package/target boundary as CI.
- It is concise and avoids redundant compilation.
- Sequential recipe execution gives a single failure status for the combined
  gate.

### Limitations

- It does not itself re-run formatting or Clippy.
- It uses `cargo check`, while the ticket also names `cargo build` as an example.
- A composite command's final status is clear, but separate per-command evidence
  is less explicit unless the emitted recipe lines and output are carefully
  recorded.
- It does not link the release-form WASM artifact.

### Assessment

This is sufficient for the literal test and “just check” acceptance language,
but it provides weaker independent evidence for the tightened-tree premise and
the release artifact boundary.

## Option 2: Run CI-equivalent gates only

### Shape

Execute formatting, package-specific warning-strict Clippy, workspace tests, and
ordinary WASM check exactly as `.github/workflows/ci.yml` does.

### Advantages

- It mirrors the authoritative automated merge contract.
- It independently verifies the formatting and lint premise.
- It produces distinct exit statuses for every gate.
- It avoids a release build that CI's check job does not require.

### Limitations

- Three separate Clippy invocations are more verbose than the predecessor's
  already successful workspace and WASM commands.
- It does not perform WASM linking/code generation.
- It duplicates checks completed by immediate predecessor tickets.
- Exact CI parity is not itself required by this ticket.

### Assessment

This is rigorous, but package-by-package lint replay adds little beyond a fresh
warning-strict native workspace invocation plus target-specific plugin Clippy.

## Option 3: Fresh baseline gates plus composite check and release build

### Shape

Run a sequential verification ladder:

```text
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
just check
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

### Advantages

- Formatting and warning-strict Clippy directly establish the tightened-tree
  premise within this attempt.
- `just check` uses the project's canonical combined WASM/test gate.
- The release build covers code generation and linking for the shipped target.
- Every acceptance interpretation is covered: tests, `just check`, explicit WASM
  check, and explicit WASM build.
- Sequential execution avoids lock contention and makes failures attributable.
- The commands are all already established by the repository.

### Limitations

- It performs redundant compilation work.
- `just check` repeats parts of dependency evaluation after Clippy.
- The release build can take longer and is stronger than strictly required.
- Cached build units may make later commands terse.

### Assessment

The redundancy is bounded and useful for a ticket whose entire deliverable is
confidence and recorded evidence. It verifies both the premise and all reasonable
readings of the acceptance criterion without changing source.

## Option 4: Run tests and only a release WASM build

### Shape

Run the two explicit Cargo commands:

```text
cargo test --workspace
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

### Advantages

- It is a direct reading of the acceptance wording.
- It exercises the strongest WASM build boundary.
- It minimizes command count.

### Limitations

- It does not establish formatting or Clippy cleanliness in this attempt.
- It bypasses the repository's default `just check` entry point.
- It omits the ordinary check mode used in CI.

### Assessment

This covers the outcomes but under-documents the “tightened tree” condition.

## Option 5: Trust predecessor evidence and write artifacts only

### Shape

Use `T-038-02-02` results without executing fresh commands.

### Advantages

- It is fastest.
- The predecessor ran both supporting and primary gates successfully.

### Limitations

- It fails the ticket's request to confirm results on this ticket's tree.
- It turns historical evidence into an assumption.
- It would not detect toolchain, environment, or concurrent-tree changes.
- The acceptance record would not belong to the assigned attempt.

### Assessment

Rejected. Prior evidence informs expectations but cannot replace fresh evidence.

## Option 6: Add a script, CI job, or test to preserve the commands

### Shape

Create a new repository file that invokes the checks or asserts their presence.

### Advantages

- It could make the verification command discoverable in a new location.
- A source commit would create an obvious implementation diff.

### Limitations

- The root `Justfile` and CI workflow already preserve the command contract.
- A test that shells out to Cargo would be slow and environment-coupled.
- A duplicate script introduces maintenance drift.
- The ticket asks to confirm and record, not redesign developer tooling.
- Manufacturing a change increases risk without improving acceptance coverage.

### Assessment

Rejected as unnecessary scope expansion.

## Chosen approach

Choose Option 3: fresh baseline gates, the canonical composite check, and the
release WASM build.

The formatting and two warning-strict Clippy commands establish that the tree is
still tightened at the moment the primary gates run. `just check` then executes
the exact repository-local pair required by the ticket. The release WASM build
adds confidence that the plugin not only type-checks for `wasm32-wasip1` but also
completes target code generation and linking in the configuration used for
delivery.

## Execution semantics

- Commands will run sequentially from the repository root.
- Shell exit status will be captured for each logical command.
- No command will rewrite source:
  - Formatting uses `--check`.
  - Clippy performs analysis only.
  - Tests write only ignored build/test state.
  - Cargo check/build write only under ignored target state.
- On any failure, execution will stop at that gate for diagnosis.
- A fix will not be assumed necessary until diagnostics identify a ticket-owned
  cause.
- If a fix is necessary and in scope, it will be committed as an exact source unit
  through `lisa commit-ticket`.

## Evidence design

`progress.md` will record:

- Starting `HEAD` and relevant repository state.
- Every exact command.
- Exit status for each command.
- Complete short outputs or concise summaries of long test output.
- Test counts split by target where available.
- Total passed, failed, and ignored counts.
- WASM target and profile used.
- Whether warnings or errors appeared.
- Whether any source changes were necessary.
- Final index and ticket-owned working-tree hygiene.

`review.md` will convert that execution record into a reviewer-facing mapping:

- Acceptance criterion to observed evidence.
- Source and artifact change summary.
- Coverage strengths and known gaps.
- Transaction hygiene.
- Open concerns and critical issues.

## Test-count handling

- Cargo prints one summary per test binary, not a built-in workspace aggregate.
- The result record may sum the `passed`, `failed`, and `ignored` fields across
  those summaries.
- Doc-test summaries will be included if Cargo emits them.
- The ignored real-Zellij integration boundary will be named explicitly.
- A pre-existing ignored test is not a failure, but it is a coverage limitation.
- Counts will be taken from the fresh execution, not copied from the predecessor.

## Source-change decision rule

- If every gate passes, the correct implementation has no source diff.
- No empty `lisa commit-ticket` transaction will be created.
- No documentation outside the private phase artifacts will be added merely to
  force a commit.
- If a gate exposes a genuine defect, the smallest ticket-owned correction will
  be evaluated against the ticket boundary.
- Any meaningful correction will be isolated by exact repository-relative paths.

## Rejected behavior

- Do not edit ticket phase or status.
- Do not write directly to the shared active-work directory.
- Do not use ordinary staging or commits.
- Do not clean the Cargo target directory solely to make output verbose.
- Do not use `cargo fmt` without `--check`.
- Do not add lint allowances or skip tests to obtain green output.
- Do not run the ignored real-Zellij test as part of this standard gate; its
  external prerequisites make it a separate delivery exercise.

## Design conclusion

The selected verification ladder is intentionally stronger than the minimum but
remains entirely read-only with respect to source. It ties predecessor cleanliness,
the repository's default check recipe, CI's WASM boundary, and the release build
into a single attempt-local evidence chain. If all commands pass, recording that
fact without a source commit is the most accurate implementation of the ticket.
