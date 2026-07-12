# T-038-02-03 Structure: Verification Boundaries and Artifacts

## Structural outcome

This ticket is expected to produce verification artifacts but no product-source
change. The implementation structure is therefore a command/evidence pipeline,
not a new Rust module or configuration surface.

## File-level change set

### Private attempt files created

All agent-authored files for this ticket live under:

```text
.lisa/attempts/T-038-02-03/1/work/
```

The phase files are:

```text
research.md
design.md
structure.md
plan.md
progress.md
review.md
```

- `research.md` maps current commands, CI, workspace, predecessor state, and
  constraints.
- `design.md` chooses the verification ladder and evidence strategy.
- `structure.md` defines the file and command boundaries.
- `plan.md` sequences execution and validation.
- `progress.md` records actual implementation results.
- `review.md` is the final reviewer handoff.

### Product source files

- Created: none planned.
- Modified: none planned.
- Deleted: none planned.
- Rust module boundaries: unchanged.
- Cargo manifests and lockfile: unchanged.
- `Justfile`: unchanged.
- CI workflows: unchanged.
- Ticket frontmatter: agent does not modify it.
- Shared `docs/active/work/T-038-02-03/`: agent does not write to it.

### Generated files

- Cargo may update files under `target/` as a result of check, test, Clippy, and
  build execution.
- Those files are generated, ignored build state.
- They are not included in ticket transactions.
- No cleanup of shared Cargo cache state is required.

## Existing interfaces exercised

### Formatting interface

```text
cargo fmt --all -- --check
```

- Scope: every workspace package understood by rustfmt.
- Input: current Rust source files.
- Output contract: exit zero and no rewritten files.
- Failure boundary: formatting drift in a Rust source file.

### Native lint interface

```text
cargo clippy --workspace -- -D warnings
```

- Scope: all Cargo workspace members on the host target.
- Input: current Rust/Cargo graph and stable lint set.
- Output contract: exit zero with no warning diagnostic.
- Failure boundary: compiler error, Clippy warning, or build-script failure.

### WASM lint interface

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

- Scope: `lisa-plugin` on the deployed WASM target.
- Input: plugin source, core dependency, Cargo target configuration.
- Output contract: exit zero with no warning diagnostic.
- Failure boundary: target-specific compile or lint issue.

### Canonical combined verification interface

```text
just check
```

The recipe expands to:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

- First boundary: WASM plugin type/target compilation.
- Second boundary: host workspace test binaries and doc tests.
- Recipe contract: both commands must exit zero.
- Ordering: WASM check completes before native tests begin.

### Release WASM interface

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
```

- Scope: release-profile plugin artifact.
- Input: plugin and dependency graph for `wasm32-wasip1`.
- Output: generated WASM build state under the Cargo target directory.
- Output contract: exit zero after compilation and linking.
- Failure boundary: target code generation, linking, or release-profile issue.

## Command dependency graph

```text
current HEAD
  -> formatting check
  -> native warning-strict Clippy
  -> WASM warning-strict Clippy
  -> just check
       -> WASM cargo check
       -> native workspace tests
  -> release WASM build
  -> repository hygiene inspection
  -> progress record
  -> review handoff
```

The sequence is deliberately linear. Later acceptance evidence is only treated
as valid after the tightened baseline gates pass.

## Evidence boundaries

### Baseline evidence

The baseline section of `progress.md` will own:

- Starting commit.
- Formatting result.
- Native Clippy result.
- WASM Clippy result.
- Warning and error counts where observable.

### Primary acceptance evidence

The acceptance section will own:

- `just check` exact command and status.
- Expanded WASM check result.
- Expanded workspace test result.
- Test target summaries and aggregate counts.
- Release WASM build exact command and status.

### Hygiene evidence

The hygiene section will own:

- Ordinary index state.
- Ticket-owned tracked modifications.
- Ticket-owned untracked files outside the private attempt directory.
- Lisa-owned status entries, identified but not changed.
- Whether an isolated source commit was needed.

## Public and internal interfaces

- No public Rust API changes.
- No internal Rust function changes.
- No CLI option or output changes.
- No scheduler or plugin behavior changes.
- No test helper additions.
- No feature flag changes.
- No dependency changes.
- No developer command changes.

The existing `Justfile` and CI interfaces are consumed as-is.

## Failure structure

If formatting fails:

- Record the failing command and paths reported by rustfmt.
- Determine whether drift belongs to this ticket or a concurrent owner.
- Do not automatically rewrite unrelated source.

If native Clippy fails:

- Record package, lint, and path.
- Determine whether the issue invalidates the tightened-tree premise.
- Evaluate only a minimal behavior-preserving correction if ticket-owned.

If WASM Clippy/check/build fails:

- Separate target installation/environment failures from source failures.
- Record whether failure occurs during analysis, code generation, or linking.
- Do not substitute a native success for target-specific evidence.

If tests fail:

- Record the failing test target and test name.
- Re-run a focused test only for diagnosis.
- Keep the full workspace gate as the final acceptance command after any fix.

## Conditional source-unit structure

No source units are planned. If diagnostics prove a source correction is needed,
the unit boundary will be the smallest coherent set of exact repository-relative
paths. Examples of possible units are:

- One Rust implementation file plus its colocated unit tests.
- One integration test file when only regression coverage changes.
- One manifest plus lockfile only if a dependency/configuration correction is
  genuinely required.

Each meaningful unit would be committed with:

```text
lisa commit-ticket --ticket-id T-038-02-03 --message <message> --include <path>...
```

No broad include patterns or ordinary-index staging are part of the structure.

## Ownership boundaries

- Lisa owns ticket phase/status transitions.
- Lisa owns publication from private work to shared active work.
- Lisa owns final Done preparation and completion commit.
- This attempt owns its private phase artifacts.
- This ticket owns only source files that must be changed to satisfy its checks;
  currently that set is empty.
- Other working-tree and provenance changes remain outside the ticket boundary.

## Review structure

The final review will be organized around:

1. Overall outcome and critical issue statement.
2. Change summary, including an explicit empty source diff if applicable.
3. Acceptance criterion mapping.
4. Tightened-baseline results.
5. Workspace test coverage and counts.
6. WASM check/build coverage.
7. Transaction and repository hygiene.
8. Open concerns and limitations.
9. Final readiness assessment.

## Structural verification criteria

- Exactly six phase artifacts exist in the private work directory at completion,
  in addition to Lisa's assignment record.
- No phase artifact is directly written to the shared active-work path.
- The ticket frontmatter is not manually edited.
- Every verification command runs from the repository root.
- Primary tests and WASM compilation both exit zero.
- Formatting and warning-strict Clippy establish the premise.
- Any ticket-owned source change is isolated and committed; expected count is
  zero.
- The ordinary index contains no ticket-owned entry.
- Review records both coverage and limitations.

## Structure conclusion

The implementation is an evidence-producing verification pipeline over existing
interfaces. Its meaningful output is the attempt-local record of the current
tree's behavior. Product structure remains unchanged unless a gate reveals a real,
ticket-owned defect; the design avoids speculative changes and preserves Lisa's
transaction and publication boundaries.
