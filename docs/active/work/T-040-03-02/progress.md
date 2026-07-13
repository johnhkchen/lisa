# Progress: pre-ownership miss hostile regression

## Status

Implementation is complete and verified.

The ticket-owned source unit is ready for the required isolated Lisa
transaction.

## Completed work

Added
`tests::rc6_preownership_delivery_miss_is_durable_and_cli_retrievable` to the
plugin native test module.

The regression constructs a complete Codex seat assignment for `T-NAME` with:

- pane 10;
- a current attempt lease shared by thread and slot;
- a temporary physical provenance ledger;
- a `Delivering` assignment state;
- the configured maximum delivery retry count already consumed;
- an injected acknowledgement deadline.

The test advances the clock past that deadline through the production
`check_assignment_ack_timeouts_at` evaluator.

It does not call the terminal persistence helper directly.

The resulting transition is the real bounded delivery-miss path used by the
scheduler when no matching provider acknowledgement arrives.

## Scheduler assertions

The timeout evaluator returns exactly one
`AssignmentDeliveryFailed` outcome for pane 10 and ticket `T-NAME`.

The retained seat state is `DeliveryFailed`.

The assertion explicitly records that the unacknowledged seat must never be
classified as owned.

## Durable ledger assertions

The test opens the physical ledger created by the production scheduler writer.

It requires exactly one physical JSONL row.

Raw JSON assertions require the absence of execution-only `authoritative` and
`outcome` fields.

Typed decoding requires:

- the current schema version;
- `assignment-transition` record type;
- ticket `T-NAME`;
- the exact minted attempt lease;
- pane 10;
- provider `openai` derived from Codex;
- state `DeliveryFailed`;
- the exact production reason: `provider did not acknowledge the bounded chat
  assignment`.

This assertion would fail against the pre-S-040-02 scheduler because no
pre-ownership row or physical ledger would exist.

## CLI retrieval seam

Extracted the existing ticket-evidence reader/renderer from
`crates/lisa-cli/src/status.rs` into
`crates/lisa-cli/src/preownership_status.rs`.

The extracted module retains the same:

- JSONL open/read/parse diagnostics;
- mixed schema-v2/v3 decoding;
- ticket filter;
- append-order report;
- stable state spellings;
- empty-result behavior;
- no-partial-output behavior;
- stdout adapter.

`write_preownership_status` is now public so deterministic tests can provide a
byte buffer.

`main.rs` declares the new module and routes `lisa status --ticket` to it.

Normal DAG status still routes to `status::run_status`.

The plugin's native test module includes this exact CLI source file under
`preownership_status_surface`.

No report logic is copied or reimplemented.

The historical regression queries the scheduler-written ledger through that
module and asserts the visible report contains:

- a one-row ticket heading;
- attempt and pane;
- `delivery-failed`;
- exact scheduler reason;
- `openai` provider;
- start, end, and duration fields.

## Relocated coverage

Moved the three existing focused report tests with their implementation:

- mixed ledger filtering;
- valid ledger with no matches;
- malformed later row before output.

The CLI black-box integration fixture and test remain unchanged.

Because the extracted source is compiled inside the plugin test module, these
three fast module tests also run in the plugin suite. This verifies the exact
included implementation remains self-contained.

## Focused verification

Historical producer-to-consumer regression:

```text
cargo test -p lisa-plugin rc6_preownership_delivery_miss_is_durable_and_cli_retrievable
```

Result: 1 passed, 0 failed.

CLI report unit tests:

```text
cargo test -p lisa-cli preownership_status
```

Result: 3 passed, 0 failed.

Existing binary-level CLI fixture:

```text
cargo test -p lisa-cli --test preownership_status
```

Result: 1 passed, 0 failed.

The binary test continues to assert exact stdout and empty stderr from
`lisa status --ticket --ledger` without project tickets or a live pane.

## Broad verification

Formatting:

```text
cargo fmt --all -- --check
```

Result: passed.

Native workspace:

```text
cargo test --workspace
```

Result: passed.

Observed unit totals:

- `lisa-cli`: 279 passed;
- `lisa-core`: 169 passed;
- `lisa-plugin`: 341 passed;
- all enabled integration and doc-test targets passed;
- the existing real-Zellij environment-gated test remained ignored.

Deployed plugin target:

```text
cargo check -p lisa-plugin --target wasm32-wasip1
```

Result: passed.

Native Clippy:

```text
cargo clippy --workspace --all-targets -- -D warnings
```

Result: passed with warnings denied.

WASM Clippy:

```text
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: passed with warnings denied.

## Warning handling

The embedded source exposes the CLI stdout wrapper, which the plugin regression
does not call because it uses an in-memory writer.

Initial focused compilation reported that wrapper as dead code only in the
embedded plugin test copy.

Added `#[allow(dead_code)]` to the test-only included module.

The production CLI module has no allowance and uses the wrapper normally.

Both warning-denied Clippy gates then passed.

## Diff verification

`git diff --check` passed for all four source paths.

The meaningful diff contains:

- one new self-contained CLI report module;
- removal of the same code/tests from general DAG status;
- two-line main dispatch/module wiring;
- one test-only include seam;
- one historical regression.

No production scheduler or provenance behavior changed.

No persisted schema, command argument, help text, or rendered report changed.

The ordinary index contains no staged ticket-owned path.

Unrelated Lisa-managed provenance, ticket frontmatter, generated docs, and
canonical work artifacts remain outside the source diff.

## Plan deviations

No functional deviation.

The plan anticipated that included module tests might duplicate in the plugin
suite. They did, adding three fast tests. This was retained because it keeps the
source literal and avoids a custom cfg contract.

## Source transaction

Completed exact transaction:

```text
lisa commit-ticket \
  --ticket-id T-040-03-02 \
  --message "Pin pre-ownership CLI evidence regression" \
  --include crates/lisa-cli/src/main.rs \
  --include crates/lisa-cli/src/status.rs \
  --include crates/lisa-cli/src/preownership_status.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Result:

```text
1d8d0ad20813ceed6dcb22bb13cb2929afbc0d7f
```

Post-commit verification confirmed:

- the commit contains exactly the four listed source paths;
- `git show --check` passes;
- all four ticket-owned paths are clean;
- the ordinary index has no staged paths;
- unrelated dirty and untracked paths remain outside the commit.

## Remaining work

- Write Review artifacts and disposition.
- Stop on this ticket for Lisa publication.
