# Research: publication-site characterization

## Ticket boundary

- Ticket `T-039-05-01` is the first ticket in story `S-039-05`.
- The story will later introduce one typed atomic-publication boundary.
- This ticket precedes that refactor and is characterization-only.
- Production publication behavior must remain unchanged.
- The acceptance criterion names five rename publication sites.
- They are fresh launch, assignment, lease marker, admitted artifact, and shell readiness.
- It also names provenance emission.
- Tests must cover ordinary and hostile-path behavior on the current tree.
- The later boundary must preserve each site's distinct contract.

## Source locations

- Rename publications live in `crates/lisa-plugin/src/lib.rs`.
- `State::prepare_fresh_launch` publishes a shell script.
- `State::prepare_assignment` publishes complete attempt instructions.
- `State::write_pane_lease_marker` publishes scheduler authority JSON.
- `State::admit_artifact` copies current-attempt artifacts to canonical work.
- `State::shell_readiness_probe` constructs a shell-side publication command.
- Provenance record construction lives in `State::emit_provenance`.
- Provenance serialization and append I/O live in `crates/lisa-core/src/provenance.rs`.
- All five plugin helpers are private and directly accessible to inline tests.
- The core append helper is public and has an inline test module.

## Shared publication mechanics

- Every rename site uses a temporary in the destination directory.
- Same-directory rename is the intended atomicity property.
- Four sites execute filesystem writes and renames inside Rust.
- Shell readiness returns a quoted `printf` plus `mv` command instead.
- Rust-side rename failures attempt to remove the temporary.
- Rust-side success leaves only the destination.
- Existing destination files are replaced on the supported Unix test platform.
- Existing destination directories cause rename failure.
- No site uses `create_new` for its temporary.
- A pre-existing temporary file is truncated and replaced by `fs::write`.
- Shell redirection likewise truncates a pre-existing temporary file.

## Fresh-launch publication

- Destination is `.lisa-launch-{pane_id}.sh` under the attempt work directory.
- Temporary family is `.lisa-launch-{pane_id}.sh.tmp.{nanosecond_nonce}`.
- The nonce comes from wall-clock nanoseconds since Unix epoch.
- Clock errors fall back to nonce zero.
- Serialization wraps payload bytes as `#!/bin/sh\n{payload}\n`.
- The returned pane command is only `sh {quoted_destination}`.
- The large provider payload is absent from the returned PTY command.
- Parent directories are created before publication.
- Directory-creation errors begin with `cannot create launch directory`.
- Temporary-write errors begin with `cannot write launch payload`.
- Rename errors begin with `cannot publish launch payload`.
- Rename errors identify the final destination, not the temporary.
- A rename failure removes the generated temporary best-effort.
- Existing tests cover bounded command size, large hostile payload bytes, and no temp residue.
- Existing tests also cover failure before the launch command is queued.
- Existing tests do not explicitly pin replacement of an existing destination.
- They do not pin the temporary filename family or rename error text.

## Assignment publication

- Destination is the constant `assignment.md`.
- Temporary family is `.assignment.md.tmp.{nanosecond_nonce}`.
- Assignment bytes are written without a wrapper or JSON encoding.
- Parent directories are created before publication.
- Success returns the destination path.
- Error prefixes distinguish directory creation, temporary write, and publication.
- Rename error identifies the final assignment path.
- Rename failure removes the generated temporary best-effort.
- Existing tests cover a quote-, control-, and shell-metacharacter-heavy payload.
- Existing tests prove byte preservation and absence of temp residue.
- Existing tests do not explicitly cover destination replacement.
- They do not lock the temporary family or failure wording.

## Lease-marker publication

- Destination is `pane-{pane_id}.lease` in `State::signal_dir`.
- Temporary family is `pane-{pane_id}.lease.tmp.{attempt_id}-{nanosecond_nonce}`.
- Serialization is compact `serde_json::to_vec` of `AttemptLease`.
- JSON includes `ticket_id` followed by `attempt_id` under current struct order.
- The attempt ID appears both in JSON and in the temporary name.
- The ticket ID does not appear in the temporary name.
- Parent directories are created before publication.
- Empty signal-dir behavior differs under test and production configuration.
- Directory, write, serialization, and rename errors have distinct prefixes.
- Rename error identifies the final marker path.
- Rename failure removes the temporary best-effort.
- Dispatch callers wrap errors with ticket and pane context.
- Existing lifecycle tests read and deserialize successfully published markers.
- Existing tests do not isolate collision, temp naming, or failure cleanup.

## Admitted-artifact publication

- Canonical directory is `{work_dir}/{ticket_id}`.
- Canonical destination is `{canonical_dir}/{artifact_name}`.
- Leased publication first validates exact current attempt authority.
- Source is `{attempt_dir}/{ticket_id}/{attempt_id}/work/{artifact_name}`.
- Missing or non-file staged artifacts return `Ok(false)`.
- Stale and mismatched leases return an authority error.
- Source bytes are read completely before canonical publication starts.
- Temporary name is deterministic: `.{artifact_name}.attempt-{attempt_id}.tmp`.
- The temporary is overwritten if it already exists as a file.
- Destination collision replaces the existing canonical artifact on Unix.
- Parent directory creation, source read, temp write, and rename errors are distinguished.
- Rename error identifies the canonical destination.
- Rename failure removes the deterministic temporary best-effort.
- Existing tests prove stale attempts cannot publish and current attempts can.
- Existing tests prove raw source byte preservation for ordinary names.
- Existing tests do not isolate temp collision or destination collision.
- They do not assert hostile path rendering in operator errors.

## Shell-readiness publication

- Destination is `pane-{pane_id}.shell-ready`.
- Temporary family is `pane-{pane_id}.shell-ready.tmp.{attempt_id}-{nanosecond_nonce}`.
- The helper strips a leading `/host` before constructing shell paths.
- Serialization is compact `serde_json::to_string` of the exact successor lease.
- The command uses `command printf '%s' BODY > TEMP && command mv TEMP DEST`.
- Body, temporary, and destination are independently single-quoted.
- Single quotes are escaped by the shared `shell_quote` helper.
- Redirection collision truncates an existing temporary.
- `mv` collision replaces an existing regular destination on Unix.
- A failed `printf` prevents `mv` through `&&`.
- A failed `mv` leaves the temporary because the shell command has no cleanup clause.
- The helper itself reports only JSON serialization errors.
- Shell execution errors surface as process status rather than a Rust error string.
- Existing tests execute a hostile quoted path and hostile ticket ID.
- Existing tests prove exact lease JSON, no injection, and success-path temp cleanup.
- Existing tests do not explicitly pin replacement or failed-`mv` residue.

## Provenance emission

- `State::emit_provenance` rejects absent threads and absent attempt leases.
- Authoritative Done records require the exact current lease.
- Failed and timed-out records can describe revoked attempts.
- The state layer fills route, timing, outcome, authority, fence, concurrency, and pane fields.
- Provider usage is merged before append.
- `provenance::append_record` creates missing parent directories.
- Serialization uses compact `serde_json::to_string` plus exactly one newline.
- The ledger opens with `create(true).append(true)`.
- Existing bytes are never intentionally rewritten.
- Each call performs one `write_all` of the completed line.
- Append failures are returned by core.
- The plugin logs `provenance write failed for {ticket}: {error}` and returns false.
- Provenance failure does not abort teardown.
- Existing core tests prove two valid records append and parse.
- Existing plugin tests prove attempt attribution, outcomes, routes, and usage.
- Existing tests do not prove a hostile-path append failure preserves prior bytes.
- Existing tests do not lock the plugin's operator-facing failure event.

## Hostile-path observations

- Filesystem paths may contain spaces, quotes, and shell metacharacters.
- Rust filesystem APIs treat those as ordinary path bytes.
- Shell readiness must quote them because it crosses a shell boundary.
- Fresh-launch return commands also quote their destination.
- Error strings render paths with `Path::display`.
- Destination-as-directory fixtures reliably force rename failure on Unix.
- Parent-as-file fixtures reliably force directory-creation failure.
- Read-only permission fixtures are unreliable when tests run as privileged users.
- Non-UTF-8 paths are possible on Unix but `to_string_lossy` is used at shell boundaries.
- Ticket and artifact names are scheduler-controlled in production call sites.
- Direct private helpers accept general strings and paths in native tests.

## Baseline test results

- `cargo test -p lisa-plugin prepare_ --no-fail-fast`: 3 passed.
- Shell-readiness focused test: 1 passed.
- Current/stale admitted-artifact focused test: 1 passed.
- `cargo test -p lisa-core provenance::tests --no-fail-fast`: 7 passed.
- The baseline tree therefore passes all currently relevant coverage.

## Repository constraints

- The worktree already has Lisa-managed ticket and provenance modifications.
- Those paths are not ticket-owned source changes.
- Phase artifacts belong only in this attempt-private directory.
- Ticket phase and status frontmatter must not be edited manually.
- Test source changes must be committed through `lisa commit-ticket`.
- Exact repository-relative include paths are required.
- Ordinary `git add` and `git commit` are prohibited.
- Ticket-owned source must be clean and unstaged before Review ends.

## Observed scope conclusion

- The ticket can be satisfied entirely with tests.
- Production helpers and serialized schemas need no change.
- Plugin inline tests are the narrow surface for all five rename sites.
- Core inline tests are the narrow surface for append integrity.
- Characterization should assert behavior, not introduce the future abstraction.
- Hostile failure fixtures should pin cleanup and operator-visible diagnostics.
- Existing broad lifecycle and provenance tests remain integration safety nets.
