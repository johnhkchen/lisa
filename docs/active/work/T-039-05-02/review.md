# Review: atomic publication boundary

## Outcome

Ticket `T-039-05-02` is implemented and verified. Five previously scattered
publication sites now use an explicit typed boundary. The predecessor behavior
catalog passes unchanged, including hostile filenames, exact serialization,
collision behavior, cleanup, and operator-facing errors.

## Source inventory

Created:

- `crates/lisa-plugin/src/publication.rs`.

Modified:

- `crates/lisa-plugin/src/lib.rs`.

Deleted:

- none.

No core, CLI, manifest, dependency, serialized schema, fixture, or public API
changed. No predecessor characterization test was edited.

## Source commit

Committed through Lisa's isolated ticket transaction:

```text
af788efab65d8619aa71cfcc96f633ca7ff24839
refactor(plugin): centralize atomic publication
```

The commit contains exactly the two source paths above. Both are clean after the
commit. The ordinary Git index is empty. Remaining status entries are
Lisa-managed provenance, ticket transition, and admitted workflow artifacts.

## Boundary shape

The new module defines three finite temporary-name policies:

- nonce-bearing sibling;
- attempt-plus-nonce sibling;
- exact deterministic sibling.

`PublicationPath` binds each destination to that typed sibling policy, keeping
temporary resolution in the same parent directory. Nonce generation is now
single-sourced with the same wall-clock nanosecond and zero-fallback behavior.

`PublicationErrors` gives write and publish labels named fields. Each call site
continues to supply its established operator vocabulary.

`RustPublication` owns exact-byte temporary write, same-directory rename,
best-effort cleanup after rename failure, and stable error formatting.

`ShellPublication` owns temporary resolution, shell quoting, and exact
`printf > temporary && mv temporary destination` rendering. It is deliberately
separate from Rust execution so shell collision behavior is not normalized.

## Preserved contracts by site

### Fresh launch

- destination remains `.lisa-launch-{pane}.sh`;
- temp remains `.lisa-launch-{pane}.sh.tmp.{nonce}`;
- bytes remain shebang, payload, and trailing newline;
- launch-specific errors display the same temporary/final paths;
- success still returns only a bounded quoted script reference.

### Assignment

- destination remains `assignment.md`;
- temp remains hidden `.assignment.md.tmp.{nonce}`;
- hostile assignment bytes remain exact;
- errors and returned canonical path remain unchanged.

### Pane lease marker

- destination remains `pane-{pane}.lease`;
- temp retains pane and attempt identity;
- compact `AttemptLease` JSON remains caller-owned;
- configuration, directory, serialization, write, and publication failures stay distinct.

### Admitted artifact

- exact current-lease validation remains before publication;
- source remains the attempt-private staged artifact;
- deterministic `.{artifact}.attempt-{id}.tmp` remains exact;
- regular temp/final collisions and directory-temp failure stay unchanged;
- staged source preservation and boolean results remain unchanged.

### Shell readiness

- compact lease JSON and `/host` translation remain caller-owned;
- temp retains pane, attempt, and nonce identity;
- all shell values remain independently single-quoted;
- regular destinations are replaced;
- destination directories still receive the temp as a child through `mv`.

## Honest boundary exclusions

The helper does not own directory creation, serialization, authority checks,
staged reads, host path translation, provenance append, or Git transactions.

Provenance intentionally remains append-only JSONL. Replacement rename would
destroy retry history. Git completion intentionally remains an isolated-index,
locked `commit-tree`/`update-ref` transaction with rollback and ordinary-index
reconciliation. Treating either as ordinary file replacement would weaken its
contract.

## Characterization verification

Before and after the refactor, and once more after commit:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

Additional focused call-path checks:

```text
shell_readiness_probe_publishes_exact_attempt_atomically: 1 passed
stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact: 1 passed
```

These tests cover all five sites, exact bytes, temp families, regular collisions,
hostile paths, quote safety, Rust cleanup, error prefixes, deterministic temp
collision, and the distinct shell destination-directory contract.

## Provenance and transaction verification

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
8 passed, 0 failed
```

This includes append history and failed-target integrity.

```text
cargo test -p lisa-cli commit_transaction --no-fail-fast
12 passed, 0 failed
```

This includes foreign staged preservation, staged-overlap rejection, failure
rollback, exact non-Done restoration, and verified completion publication. No
fixture left staged/index or done-not-committed residue.

## Broad verification

```text
cargo test -p lisa-plugin --no-fail-fast
328 passed, 0 failed
```

```text
cargo test --workspace --no-fail-fast
CLI unit: 274 passed
CLI integration: 4 passed
core: 156 passed
plugin: 328 passed
real-Zellij environment test: 1 ignored
```

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

```text
just check
WASM check passed; workspace tests passed
```

Formatting and whitespace checks also passed.

## Coverage assessment

Acceptance is satisfied by the unchanged predecessor characterization at the
five routed call sites, focused provenance and transaction seam tests, and full
repository gates. The new boundary has no direct module-only tests in this
ticket because successor `T-039-05-03` is explicitly responsible for direct
hostile-path regression locking.

## Open concerns and limitations

- Atomic rename still assumes the temporary and destination reside on the same
  filesystem; `PublicationPath` enforces one parent for current callers.
- Wall-clock nonce collision behavior intentionally remains overwrite rather
  than exclusive creation because characterization requires it.
- Shell readiness intentionally retains `mv`'s destination-directory behavior;
  this is surprising but now isolated and already characterized.
- Rust rename replacement semantics are platform-dependent; Lisa's supported
  runtime and current tests are Unix-like.
- Live done-not-committed and staged-residue field observation remains deferred
  to story `S-039-06`, as specified by this story's honest boundary.

No critical issue or unresolved behavior change was found.

