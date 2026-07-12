# Structure: T-038-02-01 cargo-fmt-clean

## Structural outcome

- The implementation is expected to preserve the current source tree unchanged.
- No Rust module needs creation, modification, relocation, or deletion.
- No Cargo manifest needs modification.
- No formatting configuration needs creation.
- No test source needs modification.
- The only new files are attempt-private RDSPI artifacts.
- Lisa will later publish admitted artifacts through its own transaction.

## Workspace boundary

- The root `Cargo.toml` defines the formatting workspace.
- Its `crates/*` member glob is the discovery boundary.
- `crates/lisa-core` is inside that boundary.
- `crates/lisa-cli` is inside that boundary.
- `crates/lisa-plugin` is inside that boundary.
- The formatter check applies to all Rust targets in those packages.
- No root-level standalone Rust target exists outside the member crates.

## Source-file changes

- Created Rust files: none.
- Modified Rust files: none expected.
- Deleted Rust files: none.
- Renamed Rust files: none.
- Created manifest files: none.
- Modified manifest files: none.
- Deleted manifest files: none.
- Created formatter configuration files: none.
- Modified formatter configuration files: none.
- Deleted formatter configuration files: none.

## Module boundaries

- `lisa-core` public interfaces remain unchanged.
- `lisa-core` internal organization remains unchanged.
- `lisa-cli` command interfaces remain unchanged.
- `lisa-cli` internal organization remains unchanged.
- `lisa-cli` build-script behavior remains unchanged.
- `lisa-cli` integration test organization remains unchanged.
- `lisa-plugin` plugin interfaces remain unchanged.
- `lisa-plugin` internal organization remains unchanged.
- No dependency edge between crates changes.

## Public interface impact

- No function signature changes.
- No type definition changes.
- No trait implementation changes.
- No command-line flag changes.
- No serialized format changes.
- No environment-variable changes.
- No plugin event-contract changes.
- No test API changes.
- No build output changes are intended.

## Attempt-private artifact files

- `.lisa/attempts/T-038-02-01/1/work/research.md` maps the relevant state.
- `.lisa/attempts/T-038-02-01/1/work/design.md` records the no-op decision.
- `.lisa/attempts/T-038-02-01/1/work/structure.md` defines this file blueprint.
- `.lisa/attempts/T-038-02-01/1/work/plan.md` will sequence verification.
- `.lisa/attempts/T-038-02-01/1/work/progress.md` will record execution.
- `.lisa/attempts/T-038-02-01/1/work/review.md` will provide handoff evidence.
- These files are not ticket-owned source units for manual commit.
- Lisa owns admission and publication of these files.

## Existing workflow-managed files

- `.lisa/provenance.jsonl` is already modified by Lisa.
- `docs/active/tickets/T-038-02-01.md` is already modified by Lisa.
- The ticket modification advances workflow phase state.
- These files remain outside implementation ownership.
- They must not be passed to `lisa commit-ticket`.
- They must not be restored or rewritten manually.
- Their presence does not indicate source-tree formatting drift.

## Verification component

- The verification component is the Cargo fmt subcommand.
- Its root is the repository working directory.
- Its package selection is `--all`.
- Its rustfmt operation is `--check`.
- Its success interface is process exit status 0.
- Its diagnostic interface is standard output and standard error.
- A clean run may produce no textual output.

## Git inspection component

- `git status --short` provides the worktree inventory.
- Status output is classified by ownership and file type.
- Rust source paths would indicate possible ticket work or interference.
- Lisa metadata and ticket phase paths are expected concurrent state.
- `git diff --check` may supplement whitespace-error inspection.
- No ordinary index mutation is part of this structure.
- No broad pathspec is permitted for a ticket commit.

## Transaction component

- The normal transaction entry point is `lisa commit-ticket`.
- It is conditional on a meaningful ticket-owned source diff.
- The clean-baseline structure contains no such diff.
- Therefore the expected transaction count is zero.
- An empty transaction would not represent a meaningful source unit.
- Lisa's later completion publication is separate and workflow-owned.

## Contingency file structure

- If formatter drift appears, only rustfmt-rewritten Rust files may change.
- Each changed file remains in its existing crate and module.
- No new abstraction or interface is introduced.
- Exact repository-relative paths are collected from Git status.
- Those exact paths become the `--include` arguments.
- All changed paths are reviewed before the transaction.
- A single formatting unit is appropriate if rustfmt rewrites are cohesive.
- Multiple units are unnecessary because formatting has no functional layering.

## Test structure

- The acceptance check is a static formatting verification.
- It covers library targets through Cargo package discovery.
- It covers binary targets through Cargo package discovery.
- It covers the build script through Cargo package discovery.
- It covers integration tests through Cargo package discovery.
- There is no new logic requiring unit-test structure.
- There is no new interaction requiring integration-test structure.
- There is no behavioral delta requiring regression tests.

## Ordering constraints

- Complete the Plan artifact before implementation verification.
- Recheck Git state immediately before running the formatter check.
- Run the exact formatter check from the workspace root.
- If it succeeds, do not invoke write mode.
- If it fails, inspect diagnostics before rewriting.
- If rewriting becomes necessary, inspect the full diff before committing.
- Record the outcome in `progress.md` after verification.
- Confirm source cleanliness before producing `review.md`.

## Ownership invariant

- Ticket-owned files are only files this attempt intentionally changes.
- Existing Lisa state changes are not ticket-owned.
- Concurrent changes from other attempts are not ticket-owned.
- No path becomes owned merely because rustfmt can discover it.
- A formatting rewrite would become owned only after confirming no overlap.
- The expected clean result avoids all source ownership claims.

## Final tree shape

- The Cargo workspace layout remains identical to the starting layout.
- All Rust content remains byte-for-byte identical to the starting content.
- The exact formatter check remains successful.
- Attempt-private artifacts document every RDSPI phase.
- No ticket source file remains dirty or staged.
- Lisa retains control of ticket state and final artifact publication.
