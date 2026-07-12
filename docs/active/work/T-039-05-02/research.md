# Research: atomic publication boundary

## Scope and repository state

- `T-039-05-02` is the implementation center of story `S-039-05`.
- Predecessor `T-039-05-01` added behavior characterization without changing production.
- Successor `T-039-05-03` will add direct hostile boundary regressions.
- The ticket names fresh launch, assignment, lease marker, admitted artifact, and shell readiness.
- The story's honest boundary unifies rename mechanics, not site-specific policy.
- Serialization, attribution, filesystem side, collisions, and operator errors must remain distinct.
- Attempt artifacts belong only in `.lisa/attempts/T-039-05-02/1/work/`.
- Lisa owns ticket phase/status changes and final completion publication.
- Ticket source must be committed through `lisa commit-ticket` with exact include paths.
- At research start only Lisa-managed provenance and ticket files are modified.
- Predecessor source commit is `fcfd8c5`; its completion commit is `1464aed`.

## Source layout

- All five scheduler publication sites live in `crates/lisa-plugin/src/lib.rs`.
- The plugin has focused adapter, deadline, pane-name, signal, and UI modules.
- It has no publication module today.
- `shell_quote` is a crate-visible free function in `lib.rs`.
- `lisa-core::provenance` owns JSONL schema and append I/O.
- `lisa-cli::commit_transaction` owns isolated Git transactions.
- Provenance and Git publication use different durability mechanisms from sibling rename.

## Common Rust-side mechanism

- Launch, assignment, lease marker, and admitted artifact write a sibling temporary.
- Each then calls `std::fs::rename` in the destination directory.
- Same-directory rename is the intended atomic visibility guarantee.
- `std::fs::write` truncates an existing regular temporary.
- Unix rename replaces an existing regular destination.
- Rename failure triggers best-effort temporary removal.
- Directory creation, input reads, serialization, and authority checks occur outside this sequence.
- Each site has distinct temporary naming and error prefixes.

## Fresh launch

- `State::prepare_fresh_launch` creates the attempt artifact directory.
- Destination is `.lisa-launch-{pane_id}.sh`.
- Temporary is `.lisa-launch-{pane_id}.sh.tmp.{nanosecond_nonce}`.
- Clock failure defaults the nonce to zero.
- Bytes are `#!/bin/sh\n{payload}\n`.
- Write errors name the temporary after `cannot write launch payload`.
- Rename errors name the destination after `cannot publish launch payload`.
- Success returns only a quoted `sh {destination}` command.
- Host-prefix stripping happens after publication.

## Assignment

- `State::prepare_assignment` creates the attempt artifact directory.
- Destination is `assignment.md`.
- Temporary is `.assignment.md.tmp.{nanosecond_nonce}`.
- The leading dot means naming is not just a suffix on the destination path.
- Instructions are written as exact bytes without a wrapper.
- Write and publish diagnostics retain assignment-specific labels.
- Success returns the canonical assignment path.

## Lease marker

- `State::write_pane_lease_marker` creates `signal_dir`.
- Destination is `pane-{pane_id}.lease`.
- Temporary is `pane-{pane_id}.lease.tmp.{attempt_id}-{nonce}`.
- Attempt identity is deliberately embedded in the temporary name.
- Body is compact `serde_json::to_vec(AttemptLease)` output.
- Serialization, write, and publication have distinct errors.
- The empty-signal-directory test compatibility path is separate from publication.

## Admitted artifact

- `State::admit_artifact` first validates the exact current lease.
- It reads only the attempt-private staged artifact.
- Missing staged files return `Ok(false)` without publication.
- Destination is the canonical work artifact.
- Temporary is deterministic: `.{artifact}.attempt-{attempt_id}.tmp`.
- A regular temp collision is overwritten; a directory collision fails at write.
- Staged bytes are copied exactly and the staged source remains.
- Success returns `Ok(true)`.

## Shell readiness

- `State::shell_readiness_probe` does not write through Rust filesystem APIs.
- It returns a command executed later by the target shell.
- Destination is `pane-{pane_id}.shell-ready`.
- Temporary is `pane-{pane_id}.shell-ready.tmp.{attempt_id}-{nonce}`.
- Body is compact `serde_json::to_string(AttemptLease)` output.
- Host-prefix stripping occurs before command construction.
- Body, temporary, and destination are independently shell quoted.
- The command is `printf > temporary && mv temporary destination`.
- Regular temp and destination collisions are overwritten.
- A destination directory causes `mv` to move the temporary inside it.
- This behavior differs from Rust `rename` and is characterized intentionally.

## Characterization bracket

- `publication_sites_preserve_serialization_and_collision_contracts` covers all five sites.
- It locks exact per-site bytes, hostile quoting, replacement, and success residue.
- `publication_sites_preserve_temp_names_cleanup_and_operator_errors` covers hostile paths.
- It locks nonce families, deterministic naming, cleanup, and diagnostic prefixes.
- It also locks the shell destination-directory behavior.
- Both tests invoke the `State` helpers and can remain unchanged through extraction.

## Provenance seam

- `State::emit_provenance` constructs a record after an attempt ends.
- `provenance::append_record` serializes compact JSON plus one newline.
- It opens with `create(true).append(true)` and accumulates retry history.
- This is append durability, not replace-by-rename publication.
- Rename replacement would violate provenance history.
- Failure is logged and does not abort teardown.
- Predecessor tests lock failed-target integrity and operator errors.

## Commit transaction seam

- `commit_ticket` uses a repository lock and alternate Git index.
- It stages only normalized exact include paths into that index.
- It creates a commit with `commit-tree` and advances `HEAD` with `update-ref`.
- The ordinary index is snapshotted and verified unchanged.
- Post-ref failures attempt rollback and index reconciliation.
- Git ref compare-and-swap is not ordinary filesystem publication.
- This ticket should verify this seam unchanged, not route it through a file helper.

## Extraction constraints

- Rust filesystem paths must remain lossless.
- Temporary and destination must share a parent.
- Naming must support nonce-only, attempt-plus-nonce, and deterministic forms.
- Per-site write and publish error labels must remain intact.
- Rust rename failures must retain best-effort cleanup.
- Shell publication must retain shell-side collision semantics.
- Callers should retain serialization, directory errors, and authority checks.
- The helper should remain plugin-private because no cross-crate rename caller exists.
- No dependency, schema, ticket, provenance, or CLI behavior change is required.

## Verification surface

- Predecessor publication tests are the primary behavioral bracket.
- Core provenance tests guard append integrity.
- CLI commit-transaction tests guard staged/index and completion residue.
- Plugin, workspace, Clippy, formatting, and `just check` are final gates.

