# Progress: atomic publication boundary

## Status

Implementation and post-commit verification are complete. The five publication
sites now route through one typed plugin boundary while their distinct policy
remains at each caller. All focused, seam, workspace, lint, formatting, and WASM
gates pass. The source unit was committed through Lisa as `af788ef`.

## Baseline

The ordinary index was empty. Lisa-managed worktree state existed in:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-039-05-02.md`.

`crates/lisa-plugin/src/lib.rs` was clean and the new module did not exist.

Baseline characterization:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

Baseline provenance:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
8 passed, 0 failed
```

## Implemented source

Created `crates/lisa-plugin/src/publication.rs` with:

- `TemporaryName` variants for nonce, attempt-plus-nonce, and exact siblings;
- `PublicationPath` binding destination to the typed sibling policy;
- centralized wall-clock nonce generation with the prior zero fallback;
- `PublicationErrors` with named write and publish labels;
- `RustPublication` for exact-byte write, rename, and failure cleanup;
- `ShellPublication` for exact delayed shell command rendering;
- the existing POSIX single-quote implementation.

Modified `crates/lisa-plugin/src/lib.rs` to route all five sites through those
types. No predecessor test source changed.

## Per-site mapping

### Fresh launch

- retains artifact-directory creation and its error;
- retains `.lisa-launch-{pane}.sh` destination;
- retains shebang/payload/trailing-newline serialization;
- uses typed nonce prefix `.lisa-launch-{pane}.sh.tmp.`;
- retains launch-specific write and publication diagnostics;
- retains host stripping and bounded quoted return command.

### Assignment

- retains directory creation and raw instruction bytes;
- uses typed hidden nonce prefix `.assignment.md.tmp.`;
- retains assignment diagnostics and canonical returned path.

### Pane lease marker

- retains configuration and directory checks;
- retains compact `AttemptLease` JSON and serialization error;
- uses typed `pane-{pane}.lease.tmp.{attempt}-{nonce}` naming;
- retains marker-specific diagnostics.

### Admitted artifact

- retains exact current-lease validation;
- retains staged existence/read and canonical-directory behavior;
- uses the exact deterministic `.{artifact}.attempt-{id}.tmp` variant;
- retains canonical artifact diagnostics and boolean results.

### Shell readiness

- retains compact lease JSON and host-prefix translation;
- uses typed attempt-plus-nonce naming;
- renders the same independently quoted `printf && mv` command;
- intentionally retains shell destination-directory collision behavior.

## Honest exclusions

Provenance was not routed through replacement publication because it is an
append-only JSONL history. Git completion was not routed through it because the
CLI transaction publishes refs with locking, compare-and-swap, rollback, and
ordinary-index reconciliation. Both seams were verified unchanged.

## Focused verification

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

```text
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically --no-fail-fast
1 passed, 0 failed
```

```text
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact --no-fail-fast
1 passed, 0 failed
```

The characterization tests passed without edits. They continue to lock
serialization, naming, regular collisions, hostile paths, errors, cleanup, and
shell directory-target behavior.

## Seam verification

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
8 passed, 0 failed
```

```text
cargo test -p lisa-cli commit_transaction --no-fail-fast
12 passed, 0 failed
```

The transaction group includes foreign-staged-entry preservation, staged
overlap rejection, compensating rollback, failed completion restoration, and
verified complete-ticket commit fixtures. It left no fixture residue.

## Broad verification

```text
cargo test -p lisa-plugin --no-fail-fast
328 passed, 0 failed
```

```text
cargo test --workspace --no-fail-fast
CLI unit: 274 passed
CLI integration: 1 + 3 passed
core: 156 passed
plugin: 328 passed
real-Zellij environment test: 1 ignored
all executed tests passed
```

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

```text
just check
WASM check passed
workspace tests passed
```

```text
cargo fmt --all -- --check
git diff --check
passed
```

## Deviations

- No functional deviation from Design or Plan was required.
- The new module contains no direct unit tests because the predecessor tests
  already execute all five routed call sites; direct boundary hostile-path
  tests are explicitly assigned to successor `T-039-05-03`.
- The source diff leaves directory creation and serialization visibly repeated
  because those are intentionally different caller contracts.

## Ownership and residue

- Ticket-owned source is limited to `lib.rs` and `publication.rs`.
- No core, CLI, manifest, fixture, schema, or ticket source was edited manually.
- The ordinary index remained empty throughout implementation.
- Test filesystem state was confined to temporary directories.
- Lisa committed exactly the two ticket-owned source paths as `af788ef`.
- Both source paths are clean after commit.
- A post-commit rerun of both publication characterization tests passed.

