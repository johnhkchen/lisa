# Plan: hostile publication regression

## 1. Establish baseline

Run focused predecessor tests before editing:

- plugin publication-site characterization;
- core provenance tests.

Record failures as baseline rather than attributing them to this ticket.
Confirm intended source files are clean and absent from the ordinary index.

## 2. Enforce sibling temporary naming

In `crates/lisa-plugin/src/publication.rs`:

- format each typed temporary policy exactly as today;
- inspect the resolved name with `Path::components`;
- accept exactly one `Component::Normal`;
- reject every empty, special, rooted, or multi-component name;
- return a stable error before path join or I/O;
- propagate validation through `PublicationPath::resolve`;
- propagate through `RustPublication::publish`;
- change shell command rendering to return a result.

Verification:

- the plugin compiles;
- valid current policies render and publish unchanged;
- invalid policies cannot touch disk or produce an executable shell command.

## 3. Adapt the shell-readiness call site

If required by the interface change, update
`crates/lisa-plugin/src/lib.rs` so `shell_readiness_probe` returns the shell
publication result directly.

Verification:

- existing exact shell readiness test passes;
- predecessor serialization/collision test passes;
- command bytes for valid inputs remain unchanged.

## 4. Add direct atomicity regressions

Add inline tests in `publication.rs`.

Repeated-success test:

- seed an old destination;
- publish complete first and second bodies with one exact temp policy;
- verify only the second body is visible;
- verify no append, duplicate, or temp residue;
- verify the containing directory has one entry.

Failure test:

- use a directory as destination;
- preserve a sentinel inside it;
- force rename failure after the complete temp write;
- assert the old destination is unchanged;
- assert the temporary is removed;
- assert no partial body is published.

## 5. Add hostile path and isolation regressions

Construct adjacent ticket directories and seed distinct canonical bytes.

- request publication with `../T-B/research.md` as ticket A's exact temp;
- require an invalid-name error;
- verify both A and B bytes and directory inventories remain unchanged;
- cover slash-bearing nonce prefix;
- cover rooted attempt-nonce prefix;
- cover shell command rejection;
- verify valid quotes/metacharacters still publish literally.

Verification:

- every invalid request fails before I/O;
- no neighbor path is overwritten, moved, created, or deleted;
- no error contains body bytes.

## 6. Focused plugin verification

Run:

- direct `publication::tests`;
- predecessor `publication_sites_` tests;
- shell readiness exact-attempt test;
- full `lisa-plugin` tests as practical before commit.

Run formatting on the modified Rust paths and inspect the exact diff. Run
`git diff --check` for those paths.

## 7. Commit plugin unit

Commit only exact plugin paths through Lisa:

```text
lisa commit-ticket --ticket-id T-039-05-03 \
  --message "fix(plugin): reject non-sibling publication temporaries" \
  --include crates/lisa-plugin/src/publication.rs \
  [--include crates/lisa-plugin/src/lib.rs if changed]
```

Do not use the ordinary Git index. Verify committed paths are clean afterward.

## 8. Add provenance attribution regression

In `crates/lisa-core/src/provenance.rs` tests:

- create ticket A record and ticket B record with separate minted leases;
- use distinct attempts/outcomes and a hostile-but-valid ledger path;
- append both records;
- assert exactly two newline-terminated JSON records;
- parse and compare each complete record;
- assert outer ticket and nested lease ticket stay paired;
- assert no temporary residue exists.

Production append behavior and schema remain unchanged.

## 9. Verify and commit provenance unit

Run focused provenance tests and the full core package. Format and inspect the
exact diff, then commit only:

```text
lisa commit-ticket --ticket-id T-039-05-03 \
  --message "test(core): lock provenance ticket attribution" \
  --include crates/lisa-core/src/provenance.rs
```

Verify the path is clean after the isolated transaction.

## 10. Broad gates

Run:

- `cargo test --workspace --no-fail-fast`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo fmt --all -- --check`;
- `just check`.

If a failure originates in ticket-owned code, fix the smallest source unit,
document the deviation in `progress.md`, and recommit that exact path through
Lisa. Do not absorb unrelated concurrent changes.

## 11. Final repository audit

Inspect:

- `git status --short`;
- ordinary index path list;
- exact source diffs for all ticket-owned files;
- recent commit identities.

Required outcome:

- no ticket-owned source is staged;
- no ticket-owned source is modified;
- no ticket-owned source is untracked;
- workflow-owned ticket/provenance changes remain excluded;
- all implementation commits used exact `lisa commit-ticket` includes.

## 12. Review artifact

Write `review.md` in the attempt-private work directory. Include source
inventory, behavior, test evidence, commit evidence, limitations, and any open
concern. Then remain on this ticket and stop; Lisa handles completion.
