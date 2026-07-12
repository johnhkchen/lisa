# Review: publication-site characterization

## Outcome

Ticket `T-039-05-01` is implemented and verified.

Four new regressions characterize the current atomic-publication surface before
the follow-up refactor:

- all five rename publication sites under success and regular-file collision;
- temp naming, serialization, cleanup, and operator-facing failures;
- hostile filesystem and shell paths;
- provenance append-target integrity and plugin failure logging.

No production behavior changed.

## Source inventory

Modified:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-core/src/provenance.rs`.

Created source files:

- none.

Deleted source files:

- none.

No dependency, Cargo manifest, configuration, public API, helper signature,
visibility, serialized schema, scheduler ordering, or teardown behavior changed.

## Source commit

Committed through Lisa's isolated ticket transaction:

```text
fcfd8c5760f752556cb48a477a30c80f7cf7ea4e
test: characterize atomic publication sites
```

The commit contains exactly:

- `crates/lisa-core/src/provenance.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Both ticket-owned source paths are clean. The ordinary Git index is empty.
Remaining worktree entries are Lisa-managed ticket, provenance, and admitted
workflow-artifact state.

## Plugin publication catalog

Added:

```text
publication_sites_preserve_serialization_and_collision_contracts
```

This single comparative fixture covers the five sites named by acceptance.

### Fresh provider launch

The test locks:

- destination `.lisa-launch-{pane}.sh`;
- replacement of an existing regular destination;
- shebang plus payload plus trailing-newline serialization;
- bounded `sh {quoted_path}` return command;
- absence of provider payload bytes from the PTY command;
- no success-path nonce temp residue.

### Assignment

The test locks:

- destination `assignment.md`;
- replacement of an existing regular destination;
- exact raw hostile payload bytes;
- canonical destination return value;
- no success-path nonce temp residue.

### Pane lease marker

The test locks:

- destination `pane-{pane}.lease`;
- replacement of an existing regular destination;
- compact `AttemptLease` JSON bytes;
- exact ticket/attempt round trip;
- no success-path nonce temp residue.

### Admitted phase artifact

The test locks:

- exact current-lease admission requirement in its fixture;
- deterministic `.{artifact}.attempt-{id}.tmp` naming;
- overwrite of a pre-existing regular temporary;
- replacement of a pre-existing canonical artifact;
- exact raw staged bytes;
- preservation of the attempt-attributed staged source;
- absence of temp residue after success.

### Shell readiness

The test locks:

- destination `pane-{pane}.shell-ready`;
- replacement of an existing regular destination;
- compact exact-lease JSON;
- successful execution of the actual quoted shell command;
- no command injection through hostile path or ticket text;
- no regular-success temp residue.

## Hostile path and diagnostic catalog

Added:

```text
publication_sites_preserve_temp_names_cleanup_and_operator_errors
```

The test discovers the host filesystem's deepest addressable directory rather
than hard-coding a macOS or Linux path limit. Publication filenames beyond that
point expose actual temp paths in deterministic write errors.

It locks these temp families:

- `.lisa-launch-7.sh.tmp.{numeric_nonce}`;
- `.assignment.md.tmp.{numeric_nonce}`;
- `pane-19.lease.tmp.1-{numeric_nonce}`;
- `.research.md.attempt-1.tmp`;
- `pane-23.shell-ready.tmp.1-{numeric_nonce}`.

It locks site-specific operator prefixes for temporary-write and rename errors,
while leaving platform-specific OS error tails unconstrained.

For Rust-side launch, assignment, and lease marker rename failures, the test
proves the final directory collision survives and the generated temp is removed.

For admitted artifacts, the test proves a directory collision at the exact
deterministic temp produces a temp-specific write error and leaves old canonical
bytes unchanged.

## Important shell-readiness distinction

The hostile collision fixture surfaced behavior that was not explicit in the
prior suite:

```text
mv TEMP DESTINATION_DIRECTORY
```

succeeds and moves the temporary inside the existing destination directory.
This differs from the Rust `rename(temp, destination_directory)` sites, which
return an error and clean their temporaries.

The regression now deliberately locks the observed shell behavior:

- command status is successful;
- the final destination remains a directory;
- exactly one temp-named child appears inside it;
- the child retains pane and attempt identity;
- the child contains exact lease JSON.

This is a key typed-option distinction for `T-039-05-02`; a shared mechanism
must not silently normalize it without an intentional behavioral decision.

## Provenance integrity

Added core test:

```text
append_failure_preserves_existing_target_contents
```

It proves an append attempt against a hostile target-directory collision:

- returns an I/O error;
- does not replace the directory;
- does not alter prior sentinel bytes;
- creates no extra entry.

Added plugin test:

```text
provenance_append_failure_is_logged_without_mutating_target
```

It proves `State::emit_provenance`:

- returns false on append failure;
- preserves target contents;
- retains current lease and thread attribution;
- emits a stable operator Error prefix containing the ticket ID;
- does not turn ledger I/O failure into scheduler teardown failure.

Existing provenance tests continue to cover successful append, append history,
schema, route, usage, lease attribution, and authoritative/fenced outcomes.

## Verification results

Focused publication catalog:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

Focused plugin provenance failure:

```text
cargo test -p lisa-plugin provenance_append_failure_is_logged_without_mutating_target --no-fail-fast
1 passed, 0 failed
```

Complete core provenance module after commit:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
8 passed, 0 failed
```

Complete workspace:

```text
cargo test --workspace --no-fail-fast
CLI unit: 274 passed
CLI integration: 4 passed
core: 156 passed
plugin: 328 passed
real-Zellij environment test: 1 ignored
```

Lint:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

Repository gate:

```text
just check
WASM check passed
workspace tests passed
```

Formatting and whitespace:

```text
cargo fmt --all -- --check
git diff --check
passed
```

Post-commit focused rerun passed for both publication tests and all eight core
provenance tests.

## Coverage assessment

Acceptance is covered as follows:

| Required surface | Characterized evidence |
|---|---|
| fresh launch | serialization, quoting, replacement, temp family, cleanup, errors |
| assignment | exact bytes, replacement, temp family, cleanup, errors |
| lease marker | exact JSON, replacement, attempt-bearing temp, cleanup, errors |
| admitted artifact | current attribution, raw copy, both collisions, deterministic temp, errors |
| shell readiness | exact JSON, quoting, replacement, temp identity, directory-target behavior |
| provenance | append history retained by old tests; failed-target integrity and operator error added |

The new coverage complements rather than replaces lifecycle tests. The latter
still prove when each publication is invoked and how leases gate admission.

## Open concerns and limitations

- Rename replacement and `mv` behavior are intentionally characterized on the
  project's Unix execution environment. Windows semantics are not claimed.
- The deep-path fixture performs a bounded directory-depth discovery. It passed
  on the current macOS host and allows Linux's larger path bound without a fixed
  constant.
- Shell readiness's directory-target behavior is surprising and may be judged
  undesirable later. This ticket only records it; changing it is outside the
  characterization scope.
- Atomic rename durability does not imply an `fsync` durability guarantee. The
  story explicitly targets the existing rename mechanism, not crash-consistent
  filesystem persistence.
- True simultaneous writer races are not introduced here. Collision behavior
  is pinned deterministically, and later boundary/regression tickets can add
  direct concurrency coverage if the abstraction changes race exposure.

No critical issue blocks the next ticket. The shell directory-target distinction
is the main item a human reviewer and `T-039-05-02` implementer should notice.
