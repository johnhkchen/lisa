# Progress: publication-site characterization

## Status

Implementation is complete. All focused, workspace, lint, formatting, and WASM
verification gates pass. The ticket-owned source unit was committed through
Lisa's isolated transaction as `fcfd8c5760f752556cb48a477a30c80f7cf7ea4e`.

## Baseline

Before source edits, the worktree contained Lisa-managed changes:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-039-05-01.md`.

The planned ticket-owned source files were clean and unstaged:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-core/src/provenance.rs`.

Baseline focused results:

```text
cargo test -p lisa-plugin prepare_ --no-fail-fast
3 passed
```

```text
cargo test -p lisa-plugin shell_readiness_probe_publishes_exact_attempt_atomically --no-fail-fast
1 passed
```

```text
cargo test -p lisa-plugin stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact --no-fail-fast
1 passed
```

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
7 passed
```

These results established that the added characterization runs against unchanged
production behavior.

## Implemented source changes

Modified test modules only:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-core/src/provenance.rs`.

No production helper, signature, visibility, serialized schema, dependency,
configuration, or scheduler call site changed.

## Publication success and collision catalog

Added:

```text
publication_sites_preserve_serialization_and_collision_contracts
```

The test covers all five rename publication sites under paths containing spaces,
quotes, dollar syntax, command substitution syntax, semicolons, and backticks.

### Fresh launch

- seeds the final launch script with old bytes;
- proves publication replaces the existing regular file;
- proves serialization remains `#!/bin/sh`, payload, trailing newline;
- proves the returned command is only the quoted destination reference;
- proves provider payload bytes do not enter the returned PTY command;
- proves no nonce temporary remains after success.

### Assignment

- seeds `assignment.md` with old bytes;
- proves publication replaces the existing regular file;
- proves hostile assignment bytes are preserved exactly;
- proves the return value is the canonical assignment path;
- proves no assignment temporary remains after success.

### Lease marker

- seeds `pane-19.lease` with old bytes;
- proves publication replaces the existing regular file;
- proves output equals compact `AttemptLease` JSON;
- proves the bytes deserialize to the exact ticket and attempt;
- proves no lease temporary remains after success.

### Admitted artifact

- installs an exact current lease;
- seeds staged, deterministic-temp, and canonical files with distinct bytes;
- proves the deterministic temp collision is overwritten;
- proves the canonical destination collision is replaced;
- proves raw staged bytes are preserved exactly;
- proves the attempt-attributed staged source remains intact;
- proves the deterministic temporary is removed after rename.

### Shell readiness

- seeds the final readiness file with old bytes;
- executes the real quoted `printf && mv` command;
- proves destination replacement;
- proves compact exact-lease JSON serialization;
- proves hostile path text does not create an injection sentinel;
- proves no shell temporary remains after regular-file success.

## Hostile temp, cleanup, and error catalog

Added:

```text
publication_sites_preserve_temp_names_cleanup_and_operator_errors
```

The test dynamically constructs the deepest addressable directory for the host
filesystem. Adding a publication filename then reliably exceeds the full-path
limit without assuming macOS's or Linux's numeric `PATH_MAX`.

It pins nonce-bearing temp families through actual write-error paths:

- `.lisa-launch-7.sh.tmp.{numeric_nonce}`;
- `.assignment.md.tmp.{numeric_nonce}`;
- `pane-19.lease.tmp.1-{numeric_nonce}`.

It pins stable write-error prefixes while deliberately not asserting
platform-specific OS error tails.

It then occupies Rust-side final destinations with directories and proves:

- launch returns `cannot publish launch payload {destination}: ...`;
- assignment returns `cannot publish assignment payload {destination}: ...`;
- lease marker returns `cannot publish pane lease marker {destination}: ...`;
- destination directories remain intact;
- generated nonce temporaries are cleaned after rename failure.

For admitted artifacts it occupies the exact deterministic temporary with a
directory and proves:

- error begins `cannot write canonical artifact temporary`;
- the error identifies `.research.md.attempt-1.tmp` exactly;
- old canonical bytes remain unchanged;
- the colliding temp directory remains intact.

For shell readiness it discovered and now pins a distinct collision contract:

- an existing destination directory does not make `mv` fail;
- `mv` succeeds and treats that destination as its target directory;
- the nonce temporary is moved inside the directory;
- the child name retains pane `23`, attempt `1`, and numeric nonce;
- the child bytes are the exact serialized lease.

This behavior differs materially from Rust's direct `rename` sites and is
valuable evidence for the typed-boundary follow-up.

## Provenance integrity coverage

Added core test:

```text
append_failure_preserves_existing_target_contents
```

It occupies a hostile ledger path with a directory containing prior provenance
sentinel bytes, invokes the real append writer, and proves:

- append returns a directory-open failure;
- the target remains a directory;
- prior bytes remain exact;
- no additional child is created.

Added plugin test:

```text
provenance_append_failure_is_logged_without_mutating_target
```

It constructs a current leased Done attempt and proves:

- `emit_provenance` returns false rather than aborting;
- the hostile target directory and sentinel remain unchanged;
- current lease and thread attribution remain intact;
- operator activity contains an Error beginning
  `provenance write failed for T-PROV-FAIL:`.

## Focused verification

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

```text
cargo test -p lisa-plugin provenance_append_failure_is_logged_without_mutating_target --no-fail-fast
1 passed, 0 failed
```

```text
cargo test -p lisa-core append_failure_preserves_existing_target_contents --no-fail-fast
1 passed, 0 failed
```

## Broad verification

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
passed via formatted source and subsequent checks
```

```text
git diff --check
passed
```

## Deviations from plan

- The initial hostile temp fixture incorrectly targeted the per-component name
  limit by using a long parent leaf. Publication filenames themselves remained
  legal, so launch succeeded. The fixture was corrected to discover the deepest
  addressable directory dynamically and now exercises the full-path limit.
- The initial expectation that an existing shell-ready destination directory
  would make `mv` fail was disproved by the current implementation. POSIX `mv`
  treats it as a target directory and succeeds. Design, structure, plan, and
  tests were updated to characterize the observed behavior.
- Core provenance failure uses a target-directory collision with sentinel bytes,
  rather than non-finite JSON serialization. This avoids assumptions about
  `serde_json` float handling and directly exercises the append open boundary.

## Ownership and residue assessment

- Source diff is test-only.
- The isolated commit contains exactly the two ticket-owned source files.
- Both ticket-owned source paths are clean after commit.
- The ordinary Git index is empty.
- Lisa-managed ticket, provenance, and automatically admitted workflow paths
  remain outside the source unit.
- No production temp or test residue remains; all hostile fixtures live under
  `TempDir` and are removed with it.
