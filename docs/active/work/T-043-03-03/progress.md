# Progress: field repro regression guard

## Status

Implementation is complete and both meaningful source units are committed
through Lisa's isolated ticket transaction. Focused verification is green. Full
workspace and project gates are in progress.

## Completed: deterministic capture support

Added a non-default `test-support` feature to `lisa-cli`.

When enabled, the CLI library exports the existing `capture_usage` source
module. Normal library builds continue to expose only the existing transaction
boundary.

The plugin's existing `lisa-cli` dev-dependency enables this feature. Runtime
plugin dependencies are unchanged; the WASM plugin does not acquire the CLI as
a production dependency.

Refactored `capture_usage.rs` around one shared processor with explicit:

- Stop payload reader;
- Claude/Codex selection;
- pane input;
- epoch capture timestamp;
- diagnostic writer.

`run_capture_usage` remains the process-facing adapter. It still supplies:

- locked stdin;
- `LISA_AGENT_CLIENT`;
- `LISA_PANE_ID`;
- `SystemTime::now()` converted to epoch seconds;
- locked stderr.

Payload parsing, session validation, pane validation, transcript classification,
provider parsing, successful capture append, no-capture marker append, and error
propagation remain shared.

`append_no_capture_marker` now receives the already selected timestamp and a
generic diagnostic writer. It still emits the diagnostic only after the marker
is successfully appended.

A feature-gated, doc-hidden `run_capture_usage_for_test` wrapper delegates to
the same processor. It permits another package's unit tests to use explicit
timestamps and an in-memory diagnostic buffer without mutating process-global
environment or spawning a binary.

### Support-unit verification

The following passed before commit:

```text
cargo check -p lisa-cli
cargo check -p lisa-cli --features test-support
cargo test -p lisa-cli --test capture_usage_cli
cargo test -p lisa-plugin --no-run
```

The CLI integration result was:

```text
2 passed; 0 failed
```

Both the append-only successful capture and visible no-capture compiled-command
tests remained green.

### Support-unit commit

Commit:

```text
1347c3557455a9d64b33570907e9a5380c74ef5d
```

Message:

```text
test(cli): expose deterministic capture support
```

Exact committed paths:

```text
crates/lisa-cli/Cargo.toml
crates/lisa-cli/src/lib.rs
crates/lisa-cli/src/capture_usage.rs
crates/lisa-plugin/Cargo.toml
```

## Completed: field replay fixture

Added:

```text
provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures
```

The test lives beside the existing plugin provenance usage tests.

It uses one physical Claude pane and seven sequential ticket intervals:

```text
T-FIELD-01
T-FIELD-02
T-FIELD-03
T-FIELD-04
T-FIELD-05
T-FIELD-06
T-FIELD-07
```

The first is the process-birth ticket. Tickets 2 through 7 represent the six
later pane recycles that old inherited-environment logic would all have keyed to
ticket 1 and overwritten in place.

Every owned Stop is processed through the actual CLI outcome processor with:

- the same pane ID;
- a distinct provider session ID;
- a deterministic timestamp inside its ownership interval;
- a unique Claude transcript token pair.

The fixture also processes one successful observation before every ownership
interval. It has session `session-unattributable` and conspicuous token totals.
It enters the successful capture ledger but cannot acquire a pane-time owner.

The fixture processes one empty transcript with session
`session-no-capture`. It enters the no-capture ledger and emits a captured
diagnostic buffer; it never becomes a successful zero-token capture.

## Capture assertions

The field test proves:

- `captures.jsonl` contains exactly eight ordered rows;
- the unowned observation survives unchanged;
- all seven owned observations survive unchanged;
- no row uses the no-capture session;
- no `T-FIELD-01.usage.json` exists;
- no `last.usage.json` exists;
- successful Stops emit no no-capture diagnostics.

These assertions fail structurally against the predecessor writer, which
created one ticket-keyed JSON file and replaced it on every later Stop.

## No-capture assertions

The same test proves:

- `no-captures.jsonl` contains exactly one row;
- the row retains pane, session, injected time, and `empty-transcript`;
- the diagnostic contains the visible no-capture prefix;
- the diagnostic names `session-no-capture`;
- the diagnostic names `empty-transcript`;
- no successful capture is fabricated for this Stop.

The old best-effort implementation would have returned success with no marker
and no diagnostic.

## Attribution assertions

The test replays seven non-overlapping ownership intervals chronologically.

For each interval it:

1. builds the current null-usage `ProvenanceRecord`;
2. calls the real `State::read_usage`;
3. verifies only that interval's capture contributes;
4. fills the returned token values;
5. appends the execution row to the real provenance ledger.

After all six later recycles, the ledger contains exactly seven ordered records.
The test compares the complete ordered vector of `(ticket_id, tokens_in,
tokens_out)` values to expected unique values. Earlier rows therefore cannot be
silently replaced by a later recycle.

Every cost remains null because captures contain no dollar-cost observation.
The conspicuous unowned totals occur in no provenance record.

## Quarantine assertions

The test proves the unowned successful observation:

- is written to the path returned by `quarantine::session_path`;
- uses session `session-unattributable` as its encoded partition key;
- retains source line 1;
- retains the complete original `CaptureRecord`;
- does not create provider-wide `quarantine.jsonl`;
- raises exactly one matching `ActivityEvent::Warning`;
- projects to a dashboard `ActivityType::Warning`;
- remains one row and one warning across all later consumer rescans.

## Focused regression results

New test:

```text
cargo test -p lisa-plugin provenance_field_repro -- --nocapture
1 passed; 0 failed; 381 filtered out
```

Neighboring prerequisite regressions:

```text
cargo test -p lisa-plugin provenance_recycled_pane
1 passed; 0 failed

cargo test -p lisa-plugin provenance_unattributable
1 passed; 0 failed

cargo test -p lisa-plugin provenance_claude_usage
1 passed; 0 failed
```

## Field-test commit

Commit:

```text
4fee31cf4574962f426dde9d9f1c338d2837377a
```

Message:

```text
test(plugin): replay six usage overwrites
```

Exact committed path:

```text
crates/lisa-plugin/src/lib.rs
```

## Deviations and adjustments

The planned file list mentioned `crates/lisa-cli/src/main.rs` only as a possible
compilation adjustment. It was not changed; the binary continues to compile its
private module directly.

Cargo feature unification caused the feature-gated test wrapper to compile in
the binary target during plugin test builds. The wrapper is unused in that
target, so it has a narrow `#[allow(dead_code)]`. This avoids a warning while
keeping the support unavailable without the feature.

The first test compile exposed an unmatched brace in a hand-built JSON format
string. The transcript construction was replaced with `serde_json::json!` before
the test ever passed. No behavioral plan changed.

## Final gate results

All final gates passed.

```text
cargo fmt --all -- --check
PASS

cargo test -p lisa-cli --test capture_usage_cli
2 passed; 0 failed

cargo test -p lisa-plugin provenance_field_repro
1 passed; 0 failed; 381 filtered out

cargo test --workspace
873 passed; 0 failed; 1 ignored (real-Zellij opt-in harness)

just check
PASS
```

`just check` completed both of its configured commands:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
cargo test --workspace
```

The WASM target check passed. The repeated workspace suite again reported all
ordinary tests green and only the documented real-Zellij harness ignored.

## Final ownership audit

`git diff --cached --name-only` is empty, so the ordinary index contains no
ticket-owned or foreign staged entry.

`git diff --name-only` restricted to all six ticket-owned source paths is empty.

`git ls-files --others --exclude-standard` restricted to both changed crates is
empty.

The two Lisa commits contain exactly the planned source paths:

```text
1347c35
  crates/lisa-cli/Cargo.toml
  crates/lisa-cli/src/capture_usage.rs
  crates/lisa-cli/src/lib.rs
  crates/lisa-plugin/Cargo.toml

4fee31c
  crates/lisa-plugin/src/lib.rs
```

The remaining worktree entries are Lisa-managed workflow state:

```text
M  .lisa/provenance.jsonl
M  docs/active/tickets/T-043-03-03.md
?? .lisa/completion-journal.jsonl
?? docs/active/work/T-043-03-03/
```

They were not included in either ticket source commit. The shared work directory
was populated by Lisa's artifact admission while private phase artifacts were
written; this attempt did not write phase artifacts directly there.

Implementation is complete. No ticket-owned source file is staged, modified, or
untracked.
