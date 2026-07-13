# Progress: pane-time ownership lookup

## Status

Implementation is complete, verified, and committed through the required
isolated `lisa commit-ticket` transaction.

## Completed: baseline inspection

The ordinary working tree was inspected before source edits.

Pre-existing or scheduler-managed paths were:

- `docs/active/tickets/T-043-01-01.md`;
- `docs/active/tickets/T-043-01-02.md`.

During the phase workflow Lisa also published work artifacts beneath:

- `docs/active/work/T-043-01-01/`;
- `docs/active/work/T-043-01-02/`.

Those paths are outside this ticket's source transaction. Neither planned
plugin source path had an existing modification at baseline, and
`crates/lisa-plugin/src/ownership.rs` did not exist.

## Completed: focused ownership module

Created `crates/lisa-plugin/src/ownership.rs`.

The module defines:

```rust
pub(crate) fn owner_at<'a>(
    records: impl IntoIterator<Item = &'a ProvenanceRecord>,
    pane_id: u32,
    captured_at: u64,
) -> Option<&'a str>
```

The input iterator permits future capture attribution to combine already
persisted ledger records with the terminal record currently being constructed,
without cloning or changing provenance append order.

The function scans terminal `ProvenanceRecord` values and matches both:

- physical `pane_id`;
- inclusive epoch-second time window.

The interval predicate is:

```text
record.started_at <= captured_at <= record.ended_at
```

The result borrows the unique matching ticket ID.

No filesystem access, allocation, sorting, scheduler mutation, capture parsing,
usage aggregation, or UI behavior was introduced.

## Completed: ambiguity policy

The lookup reduces all matching records to a unique ticket identity.

- No matches return `None`.
- One ticket identity returns that ticket.
- Repeated or overlapping records for the same ticket still return the ticket.
- Overlapping records for different tickets return `None`.

The different-ticket case fails closed rather than selecting the first or last
row. Lookup results therefore do not depend on ledger order.

This behavior aligns with the later quarantine story: conflicting evidence is
not sufficient to attribute usage confidently.

## Completed: crate registration

Modified `crates/lisa-plugin/src/lib.rs` only to add:

```rust
mod ownership;
```

The module is available to future code inside the plugin crate but is not
exposed as an external API.

No existing `State` field, scheduler path, provenance writer, or usage reader was
changed.

## Completed: recycled-pane regression

Added `owner_at_resolves_each_ticket_window_on_a_recycled_pane`.

The deterministic fixture contains:

- ticket A on pane 7 for epoch seconds 100 through 199;
- ticket B on the same pane 7 for epoch seconds 300 through 399.

The test proves:

- time 150 resolves to A;
- time 350 resolves to B;
- both start and end timestamps resolve inclusively;
- time 50 before both returns `None`;
- time 250 between attempts returns `None`;
- time 450 after both returns `None`;
- pane 8 at A's time returns `None`.

This directly satisfies the ticket acceptance criterion for a pane recycled from
A to B and for timestamps outside both owned windows.

## Completed: overlap regression

Added
`owner_at_accepts_duplicate_identity_but_rejects_conflicting_overlap`.

It proves that two matching attempts carrying the same ticket retain a confident
answer and two matching different-ticket intervals return `None`.

The conflict assertion is run with both forward and reverse iterators, locking
the ordering-independent policy.

## Completed: formatting

Commands:

```text
cargo fmt --all
cargo fmt --all -- --check
```

Result: pass.

Formatting touched only the planned plugin source unit. The check returned
success with no output.

## Completed: focused tests

Command:

```text
cargo test -p lisa-plugin owner_at
```

Result:

```text
2 passed; 0 failed; 0 ignored; 375 filtered out
```

Both new lookup tests passed.

## Completed: plugin package tests

Command:

```text
cargo test -p lisa-plugin
```

Result:

```text
377 passed; 0 failed; 0 ignored
```

This includes scheduler, provenance, completion, signal, deadline, publication,
adapter, pane-name, and UI coverage in addition to the new ownership tests.

## Completed: workspace tests

Command:

```text
cargo test --workspace
```

Result: pass with exit status 0.

The run covered the CLI library and binary, core, plugin, integration surfaces,
and doc tests. The plugin suite again reported 377 passing tests, and all other
workspace test binaries and doc tests completed without failure.

## Completed: diff and index audit

The owned diff contains:

- one line in `crates/lisa-plugin/src/lib.rs`;
- the new `crates/lisa-plugin/src/ownership.rs` module.

The ordinary Git index is empty.

No changes were made to:

- `lisa-core` schema or types;
- CLI capture code;
- Stop hooks or templates;
- legacy `State::read_usage`;
- `State::emit_provenance`;
- Cargo manifests or lockfile;
- ticket frontmatter by this implementation;
- shared work artifacts directly.

## Plan deviations

There were no implementation or test-strategy deviations.

Lisa published phase artifacts into `docs/active/work` while this attempt was
running. That is expected scheduler behavior described by the assignment, not a
ticket-owned source change. Those paths remain excluded from the isolated source
commit.

## Source transaction ownership

The one meaningful source unit consists of exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/ownership.rs`.

They will be committed together because the module declaration and module file
are build-dependent.

The intended command is:

```text
lisa commit-ticket \
  --ticket-id T-043-01-02 \
  --message "feat(plugin): add pane-time ownership lookup" \
  --include crates/lisa-plugin/src/lib.rs \
  --include crates/lisa-plugin/src/ownership.rs
```

No ordinary `git add`, broad staging, or ordinary `git commit` has been used.

## Completed: isolated source commit

The exact planned command succeeded and reported commit:

```text
ace7af7d0d4030b62cdd9806fcd22a9ca4516818
```

Commit subject:

```text
feat(plugin): add pane-time ownership lookup
```

`git show` confirms the commit contains exactly:

- modified `crates/lisa-plugin/src/lib.rs`;
- added `crates/lisa-plugin/src/ownership.rs`.

The commit contains 121 insertions across two files. No ticket, work artifact,
core, CLI, manifest, hook, or unrelated plugin path is present.

Post-commit `git diff` and `git diff --cached` are empty for both owned source
paths. Neither path is modified, staged, nor untracked.

## Remaining

1. Write `review.md` and `review-disposition.json`.
2. Perform final hygiene checks and remain on this ticket.
