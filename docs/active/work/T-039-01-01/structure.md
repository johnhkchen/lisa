# Structure: T-039-01-01

## Modified source files

### `crates/lisa-core/src/dag.rs`

- Boundary: the existing `#[cfg(test)]` test module only.
- Production DAG implementation remains untouched.
- Twelve membership assertion probes change from temporary `String` values to `str` literals.
- Affected test areas:
  - `test_get_blocked_by`;
  - `test_get_dependencies`;
  - `test_dag_from_depends_on_only_no_blocks`;
  - `test_end_to_end_scan_to_dag`.
- No fixture definitions change.
- No assertion is added, deleted, or reordered.
- No public or internal production interface changes.

### `crates/lisa-cli/src/init.rs`

- Boundary: the existing `#[cfg(test)]` test module only.
- Production init planning and execution remain untouched.
- `test_plan_init_upserts_missing_config_keys` passes its formatted fixture content
  directly to `fs::write`.
- The generated TOML bytes remain identical.
- No assertion, fixture path, or config behavior changes.

## Created private phase artifacts

- `.lisa/attempts/T-039-01-01/1/work/research.md`
- `.lisa/attempts/T-039-01-01/1/work/design.md`
- `.lisa/attempts/T-039-01-01/1/work/structure.md`
- `.lisa/attempts/T-039-01-01/1/work/plan.md`
- `.lisa/attempts/T-039-01-01/1/work/progress.md`
- `.lisa/attempts/T-039-01-01/1/work/review.md`

Lisa owns publication of admitted artifacts to the shared active work directory.

## Deleted files

- None.

## Module boundaries

- `lisa-core::dag` production behavior remains the owner of DAG operations.
- The local DAG test module remains the owner of membership expectations.
- `lisa-cli::init` production behavior remains the owner of initialization planning.
- The local init test module remains the owner of temporary config fixtures.
- No responsibility moves between crates or modules.

## Interface impact

- Public interfaces: none.
- Crate-private production interfaces: none.
- Test helper interfaces: none.
- Cargo features: none.
- Dependencies: none.
- File formats: none.
- Command-line behavior: none.
- WASM exports: none.

## Ownership model

- The source unit is limited to the two Clippy-reported files.
- Both paths are passed explicitly to `lisa commit-ticket`.
- The Lisa-managed active ticket modification is excluded.
- Private attempt artifacts are not included in the source commit.
- The ordinary Git index is not used.

## Change ordering

1. Record the exact 13-warning baseline.
2. Write Research, Design, Structure, and Plan artifacts.
3. Edit the twelve DAG test expressions.
4. Edit the one CLI init test expression.
5. Run formatting and lint verification.
6. Run the complete native test suite.
7. Run target-specific WASM Clippy.
8. Commit both exact source paths as one coherent lint-cleanup unit.
9. Confirm no ticket-owned source changes remain.
10. Write Review and stop on the ticket.

## Atomicity

- The two source files together form one meaningful unit: the complete 13-warning baseline cleanup.
- Splitting by file would temporarily leave the acceptance command warning-producing.
- A single ticket transaction gives the commit an independently verifiable zero-warning state.
- Exact includes preserve isolation from Lisa's ticket-frontmatter update.

## Invariants

- The native all-target/all-feature command emits no warnings.
- The WASM target command emits no warnings.
- Tests observe the same values and bytes as before.
- No non-test product line changes.
- No existing user or Lisa-managed modification is absorbed into the ticket commit.
- No ticket-owned source path remains dirty after the transaction.
