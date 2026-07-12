# Research: hostile publication boundary regression

## Ticket and workflow boundary

- `T-039-05-03` is the final ticket in story `S-039-05`.
- The ticket begins in Research and requires all remaining RDSPI phases.
- Artifacts belong in this attempt-private work directory.
- Lisa owns ticket frontmatter transitions and final artifact publication.
- Ticket-owned source commits must use `lisa commit-ticket` with exact paths.
- The initial worktree has only Lisa-managed provenance and ticket changes.
- The acceptance criterion is regression-test focused.
- It names atomicity, hostile-path rejection, cross-ticket isolation, provenance integrity, suite, and Clippy.

## Predecessor sequence

- `T-039-05-01` characterized five existing publication sites.
- Its tests cover launch, assignment, lease, artifact, and shell readiness.
- Those tests pin serialization, collision behavior, temp families, cleanup, and errors.
- It also added failed-target provenance integrity coverage.
- `T-039-05-02` extracted the common sibling-temp mechanism.
- Its production commit created `crates/lisa-plugin/src/publication.rs`.
- It routed all five sites through typed publication requests.
- It deliberately deferred direct module-only hostile tests to this ticket.
- Existing characterization passed before and after extraction.

## Publication module shape

- `TemporaryName` is a private enum with three finite variants.
- `Nonce` carries a string prefix and appends a wall-clock nanosecond nonce.
- `AttemptNonce` carries a prefix and attempt ID, then appends a nonce.
- `Exact` carries a deterministic filename string.
- `PublicationPath` combines a destination with one temporary-name policy.
- `PublicationPath::resolve` joins the resolved string to the destination parent.
- The resolved pair is private to the module.
- `RustPublication` receives bytes and site-specific write/publish labels.
- `ShellPublication` receives text and renders a quoted shell command.
- `shell_quote` encodes arbitrary UTF-8 as one POSIX shell argument.

## Rust publication behavior

- Resolution occurs before filesystem I/O.
- `std::fs::write` writes the complete body to the temporary.
- A regular existing temporary is truncated.
- `std::fs::rename` replaces the destination on the supported Unix platform.
- Rename failure triggers best-effort temporary removal.
- Write failures identify the temporary and site label.
- Rename failures identify the destination and site label.
- Success returns the destination path.
- Payload bytes do not appear in generated error text.
- No direct tests currently live in `publication.rs`.

## Shell publication behavior

- Resolution occurs while rendering a command, not during execution.
- Body, temporary, and destination are quoted independently.
- The rendered sequence is `printf > temporary && mv temporary destination`.
- A failed write prevents the move.
- Shell-side failed moves retain their temporary under existing behavior.
- Existing call-site tests execute hostile quoted paths successfully.
- `ShellPublication::command` currently returns an infallible `String`.

## Same-directory invariant

- The module documentation claims sibling-temporary publication.
- The destination parent is selected structurally.
- The temporary name is still accepted as an unconstrained `String`.
- `Path::join` does not enforce a single filename component.
- A string containing `..` may resolve outside the destination parent.
- An absolute string may replace the destination parent entirely during join.
- A slash-bearing prefix can also escape after the generated suffix is appended.
- Ordinary quotes, spaces, dollar signs, and semicolons are valid filename bytes.
- Those ordinary hostile characters are not traversal and must remain accepted.

## Cross-ticket consequence

- Canonical ticket artifacts live in adjacent ticket directories.
- A crafted exact temporary such as `../T-B/research.md` can name ticket B's file.
- Rust publication would write ticket A bytes into that foreign path first.
- Rename would then move that foreign path into ticket A's destination.
- The result would both mix A bytes through B's namespace and remove B's file.
- This violates the module's sibling claim and P1 repository preservation.
- Current production call sites synthesize safe values from controlled constants.
- The typed boundary itself does not yet enforce that assumption.
- Direct boundary tests are the correct place to lock the invariant.

## Atomicity observations

- Successful rename exposes one complete old or new destination image.
- Repeated publication with an exact temp should replace, not append.
- Success should leave one destination and no temporary residue.
- A destination-directory collision reliably forces Rust rename failure.
- On failure the old destination directory remains and the complete temp is removed.
- A traversal rejection should occur before write, rename, or cleanup is necessary.
- Direct tests can inspect all sibling and neighboring paths deterministically.

## Provenance seam

- Provenance lives in `crates/lisa-core/src/provenance.rs`.
- It is intentionally excluded from replacement publication.
- `append_record` serializes a complete compact JSON line first.
- It creates parents, opens with create-plus-append, and calls `write_all` once.
- Retry history is cumulative rather than destination replacement.
- Records include both `ticket_id` and an exact `AttemptLease`.
- Existing tests prove normal two-line append and failed-target preservation.
- Plugin tests prove stale attempts cannot emit authoritative Done provenance.
- Direct regression should make cross-ticket record attribution explicit.

## Relevant existing tests

- `publication_sites_preserve_serialization_and_collision_contracts` covers all callers.
- `publication_sites_preserve_temp_names_cleanup_and_operator_errors` covers failure shapes.
- `shell_readiness_probe_publishes_exact_attempt_atomically` covers shell quoting.
- `stale_attempt_cannot_keep_replacement_alive_or_publish_same_artifact` covers lease isolation.
- `split_brain_timeline_fences_old_attempt_and_admits_one_winner` covers one winner.
- `append_creates_then_appends` covers append history.
- `append_failure_preserves_existing_target_contents` covers failed-target integrity.
- These tests are broad behavioral brackets, not direct boundary invariants.

## Source ownership and verification

- The likely plugin source unit is `crates/lisa-plugin/src/publication.rs`.
- A provenance regression may also modify `crates/lisa-core/src/provenance.rs`.
- `lib.rs` should not require behavioral changes beyond adapting a return type if needed.
- Existing call-site behavior must remain byte-for-byte compatible for valid policies.
- Focused plugin and core tests should run before broad gates.
- Final gates are workspace tests, Clippy with warnings denied, formatting, and `just check`.

## Research conclusion

- The extracted mechanism is small enough for exhaustive direct tests.
- Atomic replacement and cleanup can be proven without fault injection.
- Traversal through typed temporary strings is the uncovered hostile boundary case.
- Rejecting non-single-component temp names would make the documented invariant true.
- Validation must happen before I/O and must not reject harmless metacharacters.
- Provenance remains append-only and should be tested as a separate integrity seam.
