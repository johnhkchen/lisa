# T-038-02-02 Structure: Clippy Zero Warnings

## Structural outcome

This ticket is structured as a validation-only change set unless final Clippy
execution reveals a diagnostic. The baseline has no diagnostic, so the expected
repository source-file set is empty. The deliverables are the required RDSPI
artifacts in the private attempt directory, including an explicit command/output
record.

## Files created in the attempt workspace

### `research.md`

- Maps the ticket scope and dependency state.
- Describes the three-crate workspace.
- Identifies native and WASM lint boundaries.
- Records the baseline warning-strict commands and their output.
- Describes CI and `justfile` lint conventions.
- Captures repository-state and ownership constraints.
- Contains observations only, with no source remediation proposal.

### `design.md`

- Enumerates validation-only, orchestration-change, CI-change, speculative
  refactor, and suppression options.
- Evaluates each against the observed zero-warning baseline.
- Selects validation-only completion.
- Defines primary and supporting commands.
- Defines how output and status evidence will be recorded.
- Defines conditional handling if a final diagnostic appears.

### `structure.md`

- Defines the file-level shape of the ticket.
- Separates attempt artifacts from shared published artifacts.
- Establishes the expected empty source diff.
- Defines evidence sections for implementation and review.
- Establishes conditional source ownership if a lint appears later.

### `plan.md`

- Sequences repository-state checks, primary lint commands, supporting gates,
  evidence capture, ownership verification, and final review.
- Associates verification criteria with each step.
- Defines the atomic commit boundary for the conditional remediation path.

### `progress.md`

- Tracks implementation status.
- Records exact final commands.
- Records meaningful stdout/stderr and exit status for each command.
- States the native and WASM warning counts.
- Records whether any source remediation or ticket commit was necessary.
- Records final repository ownership checks.

### `review.md`

- Summarizes the no-source-change result.
- Lists every artifact created.
- Repeats the acceptance evidence.
- Evaluates test and target coverage.
- Calls out toolchain sensitivity and any remaining concerns.
- Serves as the final handoff to Lisa and a human reviewer.

## Files not modified

### Rust source

- `crates/lisa-core/src/*.rs` remains unchanged.
- `crates/lisa-cli/src/*.rs` remains unchanged.
- `crates/lisa-cli/tests/*.rs` remains unchanged.
- `crates/lisa-plugin/src/*.rs` remains unchanged.
- `crates/lisa-plugin/tests/*.rs` remains unchanged.
- No lint diagnostic currently identifies a required source edit.

### Cargo metadata

- Root `Cargo.toml` remains unchanged.
- Crate `Cargo.toml` files remain unchanged.
- `Cargo.lock` remains unchanged.
- No dependency or feature change is part of the ticket.
- No lint-level configuration is introduced.

### Developer and CI orchestration

- `justfile` remains unchanged.
- `Justfile` remains unchanged.
- `.github/workflows/ci.yml` remains unchanged.
- Existing commands already deny warnings for each crate/target boundary.
- No workflow behavior needs adjustment to satisfy the observed state.

### Shared workflow state

- `docs/active/tickets/T-038-02-02.md` is not manually edited.
- `.lisa/provenance.jsonl` is not manually edited.
- `docs/active/work/T-038-02-02/` is not written by this attempt.
- Lisa owns ticket frontmatter transitions, provenance updates, publication, and
  the final completion transaction.

## Component boundaries

### Native lint boundary

```text
cargo clippy --workspace -- -D warnings
  -> lisa-core (host library)
  -> lisa-cli (host executable and selected Cargo targets)
  -> lisa-plugin (host compilation surface)
```

- Cargo workspace membership defines inclusion.
- `-D warnings` converts any compiler or Clippy warning into a failure.
- Exit status 0 plus no diagnostics is the verification interface.

### WASM lint boundary

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
  -> lisa-plugin compiled for wasm32-wasip1
```

- Package selection avoids attempting to treat the native CLI as the plugin
  deliverable.
- The explicit target exposes target-conditioned code and platform constraints.
- Exit status 0 plus no diagnostics is the verification interface.

### Behavior regression boundary

```text
cargo test --workspace
  -> lisa-core unit tests
  -> lisa-cli unit and integration tests
  -> lisa-plugin native unit tests
```

- Native tests provide the repository's comprehensive automated behavior gate.
- The plugin does not run its test suite under the WASM target.
- The WASM compilation boundary is checked separately.

### Formatting boundary

```text
cargo fmt --all -- --check
```

- This is read-only verification.
- It must not rewrite unrelated source.

## Evidence organization

`progress.md` will have the following implementation-level sections:

1. Initial state.
2. Primary native Clippy result.
3. Primary WASM Clippy result.
4. Formatting result.
5. Workspace test result.
6. WASM check result.
7. Source/commit result.
8. Final state.

Each primary Clippy section will contain:

- an exact shell command;
- a fenced text block with emitted output;
- numeric exit status;
- explicit warning count;
- a pass/fail conclusion.

`review.md` will organize the same evidence around reviewer concerns:

- scope and change summary;
- acceptance-criterion mapping;
- verification coverage;
- repository and transaction hygiene;
- open concerns and limitations;
- final assessment.

## Conditional diagnostic structure

If final linting differs from the baseline, this blueprint expands only at the
diagnostic's ownership boundary:

1. Identify the exact reported source path and lint name.
2. Inspect local semantics and tests for that code.
3. Apply the smallest lint-mandated source edit.
4. Run formatting in check mode first; format only the owned file if required.
5. Run focused Clippy for the affected package/target.
6. Run focused tests associated with the edited module.
7. Commit that exact source path with:

   ```text
   lisa commit-ticket --ticket-id T-038-02-02 \
     --message "fix: resolve <lint-name> warning" \
     --include <exact-repository-relative-path>
   ```

8. Repeat for a second meaningful source unit only if a distinct file/lint
   boundary exists.
9. Run the full primary and supporting gates again.
10. Record the deviation and transaction in `progress.md` and `review.md`.

No empty commit, broad include, ordinary staging, or ordinary commit belongs in
this structure.

## Ordering constraints

- Research precedes design because the approach depends on actual diagnostics.
- Design precedes structure because it decides whether source files exist in the
  intended change set.
- Structure precedes plan because the plan must reflect the empty expected source
  boundary and conditional remediation path.
- Final lint execution occurs during implementation, after the plan is recorded.
- Review occurs only after all final gates and ownership checks complete.
- The agent stops on this ticket after `review.md` is written.

## Public interfaces

- No Rust public API changes.
- No CLI surface changes.
- No configuration schema changes.
- No serialized data changes.
- No plugin protocol changes.
- No workflow command changes.
- The only new interface is documentary: reviewers can inspect the attempt
  artifacts to see exact commands, outputs, and conclusions.

## Deletion policy

- No file is deleted.
- No obsolete code is identified by Clippy in the baseline.
- Build outputs under `target/` remain untracked implementation byproducts and
  are not ticket-owned source artifacts.

## Structure completion criteria

- Six phase artifacts exist in the private attempt directory by the end.
- No shared work-artifact directory is written directly.
- No source file changes unless a final warning specifically requires one.
- Both primary commands are warning-strict and successful.
- Supporting gates are successful.
- Exact output and exit status are available in `progress.md` and summarized in
  `review.md`.
- No ticket-owned source remains modified, untracked, or staged.
