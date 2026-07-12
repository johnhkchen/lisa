# Review: small demonstrated cleanups

## Review outcome

`T-038-03-02` satisfies its acceptance criterion.

- Only C-01 through C-04 from the predecessor inventory landed.
- Each cleanup has a focused passing proof.
- `cargo test --workspace` passes.
- Canonical native Clippy passes with warnings denied.
- WASM-target Clippy passes with warnings denied.
- `just check` passes.
- The normally ignored real-Zellij test was explicitly run and passes.
- C-05 through C-14 remain in place and are named below for the report.
- Every ticket-owned source change is committed through exact-path
  `lisa commit-ticket` transactions.
- No ticket-owned source file remains staged, modified, or untracked.

## Change summary

Three maintained source files changed. No file was created or deleted in product
or test source.

### `crates/lisa-plugin/src/lib.rs`

Added one private pure parser for scheduler signal filenames:

`pane_id_from_signal_filename(&OsStr, suffix) -> Option<u32>`

The helper owns only the repeated `pane-<u32>.<suffix>` grammar:

- reject non-UTF-8 OS filenames;
- require literal `pane-`;
- require the exact requested terminal suffix;
- parse the middle component as `u32`.

Seven signal-consumer families now call it:

- heartbeat;
- process start;
- shell readiness;
- Codex assignment acknowledgement;
- awaiting-human;
- stopped/cleared transition;
- provider error.

The change deliberately does not centralize directory scans, payloads, lease
admission, deletion, activity updates, logging, or transitions. The transition
consumer still removes a recognized `.stopped` or `.cleared` file before acting
on numeric parse success, preserving its prior malformed-file cleanup behavior.
Idle legacy naming is unchanged.

Two focused tests were added:

- a table-driven grammar/boundary test;
- a Unix-only invalid-UTF-8 rejection test.

### `crates/lisa-plugin/src/adapter.rs`

Moved two identical native policies into existing crate-private trait defaults:

- `reset_strategy` defaults to `ClearHandshake`;
- `follow_up` defaults to typing the existing `finish_up_prompt` into the pane.

Removed the four duplicate concrete methods from Claude and Codex adapter
implementations. All provider-specific behavior remains concrete and visible:

- launch construction;
- assignment context selection;
- reuse behavior and Codex acknowledgement tagging;
- readiness mode;
- optional signal capabilities.

The trait still permits future adapters to override both defaults, and the
`FreshExec` / `SpawnCommand` enum alternatives remain intact.

Independent per-provider tests were retained because the predecessor classified
those expectations as regression evidence rather than cleanup targets.

### Deterministic real-Zellij fixture

Added one script-local `event_count` primitive in:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

It owns the shared missing-log fallback and `awk` counting expression.
`event_count_is` and `event_count_at_least` retain their named comparison
semantics and every call site remains unchanged.

No helper was shared with the live-provider fixture. The deterministic harness
remains independently executable and carries no new dependency.

## Commit review

The source history contains three meaningful ticket units after the predecessor
completion commit:

1. `659995d9e3db749e58e75b9e63dde40921fcddbe`
   `Centralize pane signal filename parsing`
2. `c688f07dccf8cb5e74aa1c3ea7360544b0f4f7e7`
   `Default shared native adapter policies`
3. `947711c57dada2b6061cc7487fcd23118407cefc`
   `Centralize deterministic harness event counts`

Each transaction includes exactly one repository-relative ticket-owned path.
No ordinary `git add` or ordinary `git commit` was used.

Diff scope from `bfe0e8d` to the final source head is exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.

No dependency, configuration, public CLI, documentation source, or historical
artifact change is part of the source commits.

## Test coverage

### C-01 focused coverage

Command:

```text
cargo test -p lisa-plugin pane_signal_filename_parser -- --nocapture
```

Result: 2 passed, 0 failed.

Covered valid inputs:

- pane id zero;
- ordinary decimal id;
- `u32::MAX`;
- leading zeroes;
- multiple authorized suffix shapes.

Covered rejection inputs:

- wrong prefix;
- wrong suffix;
- suffix followed by trailing text;
- empty id;
- non-numeric id;
- negative id;
- whitespace-bearing id;
- `u32` overflow;
- non-UTF-8 filename.

The full plugin test target also passed with 292 tests, retaining state-effect
coverage for acknowledgement, awaiting, heartbeat, transitions, errors,
readiness, and scheduler ownership behavior.

### C-02/C-03 focused coverage

Command:

```text
cargo test -p lisa-plugin adapter::tests -- --nocapture
```

Result: 25 passed, 0 failed.

The suite proves both concrete native adapters receive the same reset and
follow-up values through inherited methods. Resolver tests exercise those
methods through trait objects for default, missing-ticket, invalid-route,
per-ticket-route, and mixed-provider cases.

The full expected `FollowUp` values are compared independently for Claude and
Codex. The tests do not derive expected values from the new defaults.

### C-04 focused coverage

Commands:

```text
bash -n crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
```

Results:

- shell syntax: pass;
- real-Zellij integration: 1 passed, 0 failed in 125.81 seconds.

The harness uses both exact-count and at-least-count predicates across normal
delivery, suppressed start, suppressed acknowledgement, and same-pane recovery.
The Rust integration test also requires the script's completion receipt, so its
passing result proves the internal `real-zellij-delivery-boundary: PASS` marker
was observed.

### Integrated coverage

Commands and results:

```text
cargo fmt --all -- --check
# pass

cargo test --workspace
# pass: 725 passed, 0 failed, 1 ignored

cargo clippy --workspace -- -D warnings
# pass

cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
# pass

just check
# pass: WASM check plus workspace tests
```

The one ignored entry in the standard workspace run is the real-Zellij test,
which was explicitly executed and passed for this ticket. There is therefore no
unexercised ticket-specific integration seam.

## Deferred/larger repetition for the release-readiness report

The following predecessor-classified repetition remains unchanged:

- C-05 — whole signal scanner loops: payloads, deletion, lease admission,
  ordering, and effects differ; a typed ingestion redesign is future-epic work.
- C-06 — scheduler failure/reclaim paths: authority over seats, panes, leases,
  provenance, and retries differs.
- C-07 — timeout/liveness loops: clocks, exemptions, nudges, and reclamation
  actions are distinct policy.
- C-08 — atomic publication paths: temporary names, serialization, execution
  sides, collision behavior, and attribution differ.
- C-09 — repeated maintained-harness helpers: sharing them would couple
  independently executable fixtures and evidence contracts.
- C-10 — historical admitted harness evidence: consolidation would rewrite
  completed-ticket evidence; no epic is currently recommended.
- C-11 — lifecycle-hook JSON and merge enumerations: a declarative schema must
  preserve generation, user ownership, legacy upgrades, and idempotence.
- C-12 — scheduler test fixture construction: broad builder migration would
  churn historical authority regressions and belongs after module boundaries.
- C-13 — provider assignment and reuse construction: Claude context selection
  and Codex acknowledgement tagging are intentional distinctions.
- C-14 — adapter compatibility assertions: independent repetition is the
  provider-parity/no-op proof and should remain.

## Behavior and risk assessment

The parser helper is a local equivalence extraction. The greatest risk was
accidentally changing transition deletion timing; diff review confirms the
remove operation remains before parse-result action for recognized transition
suffixes. Existing transition tests pass.

Trait defaults can become an accidental policy for a future adapter. The method
comments identify the native assumption and explicitly direct different
transports to override it. Both alternative enum values remain, so no extension
mechanism was removed.

The shell helper uses command substitution where the prior predicates assigned
`awk` output directly. `event_count` prints exactly one integer and preserves
zero for an absent log, so comparison semantics are unchanged. The full harness
proof passed against real Zellij.

## Open concerns and known limitations

One non-blocking diagnostic observation remains. An exploratory command broader
than the repository's canonical gate:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

reports thirteen pre-existing test-only lints in `lisa-core/src/dag.rs` and
`lisa-cli/src/init.rs`. Those paths were not changed by this ticket. The
clean-gate predecessor defines native Clippy as
`cargo clippy --workspace -- -D warnings`, which passes on the final tree, as
does the WASM-target warning-strict command. Expanding this ticket to unrelated
test cleanup would have violated the authorized inventory boundary.

No ticket-owned TODO was introduced. No public API changed. No critical issue
requires human attention.

## Repository hygiene

Final audit confirms:

- ordinary index: empty;
- ticket-owned source modifications: none;
- ticket-owned staged paths: none;
- ticket-owned untracked paths: none;
- workflow-managed `.lisa/provenance.jsonl` change: left untouched;
- workflow-managed ticket phase/status updates: left untouched;
- Lisa-published shared work directory: left to Lisa's admission flow.

Review is complete. Remain on `T-038-03-02`; Lisa owns Review admission, Done
publication, the completion commit, and seat release.
