# Progress: small demonstrated cleanups landed

## Implementation outcome

All four small cleanup candidates authorized by `T-038-03-01` are implemented,
individually proved, and committed through Lisa's isolated ticket transaction.

- C-01: complete.
- C-02: complete.
- C-03: complete.
- C-04: complete.
- C-05 through C-14: deliberately unchanged.

The canonical formatting, workspace test, native Clippy, WASM Clippy, and
`just check` gates pass. The environment-dependent real-Zellij integration test
was explicitly run and also passes.

## Starting state

- Starting source baseline: `bfe0e8d` (`Complete T-038-02-03`).
- Assignment: ticket `T-038-03-02`, generation 1.
- Starting ticket phase: Research.
- Ordinary Git index: empty.
- Pre-existing worktree changes: Lisa-managed provenance and ticket assignment.
- Shared branch already contained completed predecessor work.
- No pre-existing ticket-owned source modification was present.

## Phase artifacts

The required artifacts were written in sequence under the private attempt path:

- `research.md`: code map, candidate boundaries, existing proof seams, and
  deferred repetition.
- `design.md`: evaluated helper/default alternatives and chose three bounded
  source units.
- `structure.md`: exact file interfaces, call-site boundaries, tests, and
  commit ownership.
- `plan.md`: ordered implementation, focused proof, commit, integrated gate,
  and review steps.
- this `progress.md`: implementation and verification record.

No phase artifact was written directly to the shared publication path by this
attempt. Lisa detected the private artifacts and populated
`docs/active/work/T-038-03-02/` while work continued.

## C-01: pane signal filename parser

Source file:

`crates/lisa-plugin/src/lib.rs`

Implemented:

- Added private `pane_id_from_signal_filename` over `&std::ffi::OsStr`.
- Centralized UTF-8 conversion.
- Centralized exact `pane-` prefix matching.
- Centralized exact caller-provided suffix matching.
- Centralized `u32` parsing.
- Replaced repeated parsing in heartbeat scanning.
- Replaced repeated parsing in process-start scanning.
- Replaced repeated parsing in shell-readiness scanning.
- Replaced repeated parsing in Codex acknowledgement scanning.
- Replaced repeated parsing in awaiting-human scanning.
- Replaced pane-id parsing in stopped/cleared transition scanning.
- Replaced repeated parsing in error scanning.

Preserved:

- consumer-specific directory scans;
- payload reads and attempt admission;
- deletion timing;
- poll order and state effects;
- transition suffix branch order;
- removal of recognized transition filenames before numeric-id action;
- idle scanner legacy `<ticket-id>.idle` handling.

Tests added:

- `pane_signal_filename_parser_enforces_exact_grammar`;
- `pane_signal_filename_parser_rejects_non_utf8` on Unix.

The table covers zero, ordinary ids, `u32::MAX`, leading zeroes, wrong prefix,
wrong suffix, trailing extension, empty id, alphabetic id, negative id,
whitespace, overflow, and empty requested suffix behavior.

Focused commands:

```text
cargo fmt --all -- --check
cargo test -p lisa-plugin pane_signal_filename_parser -- --nocapture
cargo test -p lisa-plugin
```

Results:

- parser tests: 2 passed, 0 failed;
- complete plugin tests at this unit boundary: 292 passed, 0 failed;
- formatting and diff check: pass.

Ticket transaction:

```text
659995d9e3db749e58e75b9e63dde40921fcddbe
Centralize pane signal filename parsing
```

Exact include:

`crates/lisa-plugin/src/lib.rs`

## C-02 and C-03: shared native adapter defaults

Source file:

`crates/lisa-plugin/src/adapter.rs`

Implemented:

- `AgentAdapter::reset_strategy` now defaults to `ClearHandshake`.
- `AgentAdapter::follow_up` now defaults to the existing typed
  `finish_up_prompt` value.
- Comments name these as native interactive defaults and preserve the override
  expectation for other transports.
- Removed identical Claude reset and follow-up methods.
- Removed identical Codex reset and follow-up methods.

Preserved:

- `FreshExec` and `SpawnCommand` alternatives;
- provider-specific launch commands;
- provider-specific assignment and reuse logic;
- signal capability differences;
- readiness-mode differences;
- independent Claude and Codex behavior assertions;
- resolver and mixed-route trait-object coverage.

Focused command:

```text
cargo test -p lisa-plugin adapter::tests -- --nocapture
```

Result:

- adapter tests: 25 passed, 0 failed;
- formatting and diff check: pass.

Ticket transaction:

```text
c688f07dccf8cb5e74aa1c3ea7360544b0f4f7e7
Default shared native adapter policies
```

Exact include:

`crates/lisa-plugin/src/adapter.rs`

## C-04: deterministic harness event count

Source file:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Implemented:

- Added a script-local `event_count` value primitive.
- Centralized the missing-event-log zero fallback.
- Centralized the tab-delimited `awk` count.
- `event_count_is` retains exact comparison.
- `event_count_at_least` retains arithmetic lower-bound comparison.
- All harness call sites remain unchanged.
- No cross-harness library or live-harness edit was introduced.

Focused commands:

```text
bash -n crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
```

Results:

- Bash syntax: pass;
- real-Zellij integration: 1 passed, 0 failed;
- duration: 125.81 seconds;
- the Rust test's internal PASS-receipt assertion succeeded.

Ticket transaction:

```text
947711c57dada2b6061cc7487fcd23118407cefc
Centralize deterministic harness event counts
```

Exact include:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

## Integrated verification

Formatting:

```text
cargo fmt --all -- --check
```

Result: pass, no output and no rewrite.

Workspace tests:

```text
cargo test --workspace
```

Result: pass. Current aggregate is 725 passed, 0 failed, and 1 ignored in the
standard run. The ignored test is the real-Zellij integration, which separately
passed explicitly in this attempt.

Canonical native Clippy:

```text
cargo clippy --workspace -- -D warnings
```

Result: pass with warnings denied.

WASM Clippy:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: pass with warnings denied.

Canonical combined gate:

```text
just check
```

Result: pass. It ran the WASM plugin check and workspace tests on the final
committed source tree.

## Plan deviation and diagnostic observation

The Plan initially selected a stricter exploratory command:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

That non-canonical command failed on twelve pre-existing
`unnecessary_to_owned` lints in `crates/lisa-core/src/dag.rs` tests and one
pre-existing `needless_borrows_for_generic_args` lint in
`crates/lisa-cli/src/init.rs` tests. Neither file is modified by this ticket;
their latest commits predate this attempt.

The clean-gate predecessor `T-038-02-03` defines and passed the repository's
warning-strict contract as `cargo clippy --workspace -- -D warnings`, plus the
WASM-target equivalent. Both canonical commands pass on this ticket's final
tree. No unrelated lint cleanup was taken into this ticket, preserving the
predecessor-authorized source boundary.

## Deferred/larger repetition left in place

- C-05: complete signal-consumer loops.
- C-06: scheduler failure and reclaim paths.
- C-07: timeout and liveness loops.
- C-08: atomic publication paths.
- C-09: helpers repeated across independently executable harnesses.
- C-10: historical admitted harness evidence.
- C-11: lifecycle-hook JSON schemas and merge lists.
- C-12: broad scheduler test-fixture construction.
- C-13: provider-specific assignment and reuse construction.
- C-14: independent provider compatibility assertions.

## Ownership and repository hygiene

The three source commits after the predecessor completion modify exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.

Final checks show:

- no ticket-owned source file is staged;
- no ticket-owned source file is modified;
- no ticket-owned source file is untracked;
- the ordinary index is empty;
- workflow-managed provenance and ticket changes remain untouched;
- Lisa-published shared work artifacts remain outside the ticket source commits.

Implementation is complete. Continue immediately to Review and remain on this
ticket afterward.
