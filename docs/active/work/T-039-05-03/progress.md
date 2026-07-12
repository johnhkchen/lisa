# Progress: hostile publication boundary regression

## Status

Implementation is complete. All planned source is committed through Lisa's
isolated transaction, all focused and broad gates pass, and the three
ticket-owned source paths are clean.

## Baseline

Before editing, the predecessor behavioral bracket was green:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

The provenance seam was also green:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
8 passed, 0 failed
```

Initial status contained only Lisa-managed provenance and ticket changes. No
ticket-owned source path was modified or staged.

## Publication boundary implementation

Completed in `crates/lisa-plugin/src/publication.rs`:

- imported `std::path::Component`;
- changed every typed temporary resolution to return `Result`;
- formats nonce, attempt-nonce, and exact names exactly as before;
- accepts only one `Component::Normal` after formatting;
- rejects empty, special, rooted, absolute, and multi-component names;
- returns an invalid-policy diagnostic before any path join or I/O;
- keeps ordinary spaces, quotes, metacharacters, and Unicode valid;
- propagates rejection through Rust publication;
- propagates rejection through shell command rendering;
- keeps payload bytes absent from validation errors.

Completed in `crates/lisa-plugin/src/lib.rs`:

- adapted `shell_readiness_probe` to return `ShellPublication::command()`;
- valid shell command formatting and serialization remain unchanged.

## Direct publication regressions

Added five direct module tests:

1. repeated publication replaces complete bytes without append, duplicate, or residue;
2. rename failure preserves the destination sentinel and removes the complete temp;
3. parent traversal is rejected before adjacent ticket files can mix or disappear;
4. exact, nonce, and attempt-nonce policies all reject non-sibling components;
5. shell rendering rejects escape paths while accepting literal hostile filename bytes.

The cross-ticket regression constructs adjacent `T-A` and `T-B` directories.
It proves that the previously dangerous `../T-B/research.md` temporary policy
cannot overwrite or move ticket B's canonical file.

## Focused plugin verification

```text
cargo test -p lisa-plugin publication::tests --no-fail-fast
5 passed, 0 failed
```

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

```text
cargo test -p lisa-plugin \
  shell_readiness_probe_publishes_exact_attempt_atomically --no-fail-fast
1 passed, 0 failed
```

This confirms both direct invariants and unchanged valid call-site behavior.

## Plugin implementation deviations

The first direct run had one new-test assertion failure. The test looked for a
quoted leaf name, while production correctly quotes the entire temporary path.
The expected value was corrected to the full path; no production change was
made in response.

No plan-level design deviation occurred.

## Plugin source commit

Committed through the required isolated transaction:

```text
a4fdeb77ca4ff4cbb253de1465bcc69e816e5264
fix(plugin): reject non-sibling publication temporaries
```

Exact include paths:

- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-plugin/src/lib.rs`.

No ordinary `git add` or `git commit` was used.

## Provenance regression

Completed in `crates/lisa-core/src/provenance.rs` tests:

- created distinct records for `T-027-01` and `T-027-02`;
- assigned distinct leases, attempt IDs, outcomes, authority, fence, and pane fields;
- appended both to a hostile-but-valid ledger path;
- asserted exactly two complete newline-terminated JSON records;
- parsed both and compared them to their original complete records;
- asserted every outer ticket ID matches its nested attempt lease ticket ID;
- asserted the ledger directory contains only the append-only ledger file.

Production provenance append code and schema were not changed.

## Provenance implementation deviation

The first compile used `AttemptLease::mint` as though it accepted a numeric
high-water mark. Its actual API accepts an optional previous lease reference.
The fixture was corrected to construct an explicit valid attempt-42 lease. No
production code or test objective changed.

## Provenance verification

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
9 passed, 0 failed
```

```text
cargo test -p lisa-core --no-fail-fast
157 passed, 0 failed
```

## Provenance source commit

Committed through the required isolated transaction:

```text
e7d8cc0081406e90c6c3945f52750e8b61a025ba
test(core): lock provenance ticket attribution
```

Exact include path:

- `crates/lisa-core/src/provenance.rs`.

No ordinary index operation was used.

## Broad verification

```text
cargo test --workspace --no-fail-fast
```

Passed:

- CLI unit: 274;
- CLI atomic provider integration: 1;
- CLI help integration: 3;
- core: 157;
- plugin: 333;
- real-Zellij environment test: 1 ignored by declared prerequisites;
- failures: 0.

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed
```

```text
cargo fmt --all -- --check
passed
```

```text
just check
```

Passed the WASM target check and repeated workspace tests.

`git diff --check` passed for each source unit before commit.

## Final repository audit

Ticket-owned maintained source:

- `crates/lisa-plugin/src/publication.rs`: clean;
- `crates/lisa-plugin/src/lib.rs`: clean;
- `crates/lisa-core/src/provenance.rs`: clean.

Ordinary index: empty.

Remaining worktree entries are Lisa-managed:

- `.lisa/provenance.jsonl` modified;
- `docs/active/tickets/T-039-05-03.md` modified;
- `docs/active/work/T-039-05-03/` untracked during phase publication.

These were not included in ticket source commits and were not manually edited
as source. Lisa owns their final publication.

## Remaining work

- Write `review.md`.
- Remain on this ticket and wait for Lisa's completion transaction.
