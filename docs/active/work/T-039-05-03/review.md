# Review: hostile publication boundary regression

## Outcome

Ticket `T-039-05-03` is implemented and verified. The extracted publication
boundary now enforces its sibling-temporary invariant instead of relying only on
safe current callers. Direct regressions prove complete replacement, no duplicate
publication, failure cleanup, hostile traversal rejection, cross-ticket
isolation, shell-side rejection, secret-body non-disclosure, and provenance
record attribution.

All workspace tests, Clippy with warnings denied, formatting, the WASM check,
and `just check` pass.

## Source inventory

Modified:

- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-core/src/provenance.rs`.

Created in maintained source: none.

Deleted: none.

No manifest, dependency, public schema, fixture, CLI contract, ticket format, or
provenance production implementation changed.

## Security and preservation fix

Before this ticket, `PublicationPath` joined a resolved temporary string to the
destination parent without constraining the string to one component. A typed
`Exact` policy containing `../T-B/research.md` could therefore address a
neighboring ticket file. The Rust sequence would overwrite that path and then
rename it into the requesting ticket, mixing bytes across tickets and removing
the neighbor's canonical file.

The boundary now validates the fully resolved name for all three policies:

- `Nonce`;
- `AttemptNonce`;
- `Exact`.

Only exactly one `std::path::Component::Normal` is accepted. Empty, `.`, `..`,
rooted, absolute, and separator-bearing multi-component names return an error
before filesystem I/O or shell command generation.

## Compatibility behavior

Validation occurs after existing policy formatting, so valid current names are
unchanged. Ordinary filename bytes remain accepted, including:

- spaces;
- single quotes;
- semicolons;
- dollar signs and command-looking text;
- backticks;
- Unicode.

The boundary does not sanitize or rewrite names. Invalid policy is rejected
explicitly, avoiding surprising collisions. Validation errors include the bad
name for diagnosis but never include the publication body.

## Rust publication coverage

The direct repeated-publication test begins with an old destination, publishes
two different complete bodies through one deterministic temporary, and proves:

- final bytes are exactly the second body;
- bytes are not appended or duplicated;
- the temporary is absent;
- the directory contains exactly one destination.

The direct failure test uses a destination directory with a sentinel to force
rename failure after the temporary write. It proves:

- the publish error is returned;
- the original destination and sentinel remain intact;
- the complete temporary is removed;
- no partial new destination is exposed.

## Hostile and cross-ticket coverage

The direct isolation fixture creates adjacent ticket A and ticket B canonical
directories with distinct bytes. Ticket A then requests a temporary path that
lexically traverses to ticket B's artifact.

The regression proves:

- the request is rejected before I/O;
- ticket A's prior bytes remain intact;
- ticket B's prior bytes remain intact;
- neither ticket directory gains residue;
- no path is overwritten, moved, created, or deleted;
- unpublished body text does not appear in the error.

Separate policy coverage rejects traversal through exact names, slash-bearing
nonce prefixes, and absolute attempt-nonce prefixes.

## Shell coverage

`ShellPublication::command` now returns `Result<String, String>` so invalid
temporary policy cannot become an executable command. `shell_readiness_probe`
propagates that result through its existing result interface.

The direct shell regression proves traversal is rejected without filesystem
effects or body disclosure. It also proves harmless hostile characters remain
quoted as literal full-path and body arguments.

The predecessor shell readiness test still executes and publishes the exact
lease successfully, confirming no valid command-shape change.

## Provenance integrity coverage

The new core regression appends two complete, distinct ticket records and proves:

- exactly two newline-terminated JSONL records exist;
- each record round-trips to its complete original value;
- outer ticket ID and nested attempt lease ticket ID stay paired;
- distinct attempt IDs, outcomes, authority, fence state, and pane IDs do not mix;
- the ledger directory contains no replacement-publication temporary residue.

This complements the existing failed-target preservation test and the plugin's
authoritative-current-lease tests. Provenance remains append-only and is not
routed through sibling replacement.

## Existing behavioral bracket

The predecessor call-site catalog remains green:

```text
cargo test -p lisa-plugin publication_sites_ --no-fail-fast
2 passed, 0 failed
```

It continues to cover all five sites, exact serialization, collision behavior,
temp naming families, cleanup, hostile operator paths, and site-specific errors.

Direct module regressions:

```text
cargo test -p lisa-plugin publication::tests --no-fail-fast
5 passed, 0 failed
```

Shell compatibility:

```text
shell_readiness_probe_publishes_exact_attempt_atomically
1 passed, 0 failed
```

Provenance:

```text
cargo test -p lisa-core provenance::tests --no-fail-fast
9 passed, 0 failed
```

## Broad verification

`cargo test --workspace --no-fail-fast` passed:

- CLI unit: 274;
- atomic provider integration: 1;
- help integration: 3;
- core: 157;
- plugin: 333;
- real-Zellij environment test: 1 ignored by prerequisites;
- failures: 0.

Additional gates:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
passed

cargo fmt --all -- --check
passed

just check
passed (WASM check plus workspace tests)
```

## Commit evidence

Plugin boundary unit:

```text
a4fdeb77ca4ff4cbb253de1465bcc69e816e5264
fix(plugin): reject non-sibling publication temporaries
```

Exact paths:

- `crates/lisa-plugin/src/publication.rs`;
- `crates/lisa-plugin/src/lib.rs`.

Provenance regression unit:

```text
e7d8cc0081406e90c6c3945f52750e8b61a025ba
test(core): lock provenance ticket attribution
```

Exact path:

- `crates/lisa-core/src/provenance.rs`.

Both commits used `lisa commit-ticket`. No ordinary `git add` or `git commit`
was used. All three source paths are clean and the ordinary index is empty.

## Coverage assessment

The acceptance criterion is satisfied:

- atomicity: direct complete replacement and failed-rename preservation tests;
- no partial publish: old destination survives failure and temp is cleaned;
- no duplicate publish: repeated exact publication leaves only the newest bytes;
- hostile rejection: every temp policy rejects non-sibling components;
- no cross-ticket mixing: adjacent ticket sentinel fixture remains unchanged;
- provenance integrity: two-ticket complete-record attribution regression;
- suite and Clippy: green.

The tests are deterministic and require no permission manipulation, sleeps,
nonce prediction, syscall injection, or network access.

## Open concerns and limitations

- Atomic rename semantics remain targeted at Lisa's supported Unix-like runtime.
- The boundary lexically constrains temporary names; it does not attempt
  filesystem canonicalization or symlink-policy enforcement for destination
  parents. Current scheduler-owned directories define that separate trust seam.
- Shell readiness intentionally retains existing shell `mv` collision semantics
  after a valid policy is rendered.
- Provenance uses append semantics, not replace-by-rename atomicity; this is
  intentional so retry history is preserved.
- The real-Zellij environment integration remains ignored unless its declared
  external prerequisites are available; deterministic native coverage is green.

No critical issue, failing gate, TODO, uncommitted ticket-owned source, staged
ticket-owned path, or unexplained behavior change remains.

## Handoff

Review is complete. Lisa should now publish the attempt artifacts and perform
the completion transaction. The agent remains on `T-039-05-03` and does not
start another ticket.
