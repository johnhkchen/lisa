# Structure: field repro regression guard

## Change shape

The ticket makes one production-neutral testability refactor in the CLI package
and adds one cross-crate regression in the plugin package. No domain schema,
runtime policy, or persisted path changes.

Five tracked files are modified:

```text
crates/lisa-cli/Cargo.toml
crates/lisa-cli/src/lib.rs
crates/lisa-cli/src/capture_usage.rs
crates/lisa-cli/src/main.rs
crates/lisa-plugin/Cargo.toml
crates/lisa-plugin/src/lib.rs
```

The list contains six paths: four CLI paths and two plugin paths. No new Rust
source file is needed because the existing capture module remains the single
implementation.

## `crates/lisa-cli/Cargo.toml`

### Modification

Add an empty opt-in feature:

```toml
[features]
test-support = []
```

### Boundary

- The feature controls only library visibility of deterministic capture test
  support.
- It does not add dependencies.
- It is not part of default features.
- The binary continues to compile the existing capture module normally.
- Release CLI behavior does not require the feature.

## `crates/lisa-cli/src/lib.rs`

### Modification

Conditionally export the existing source module:

```rust
#[cfg(feature = "test-support")]
pub mod capture_usage;
```

### Boundary

- `commit_transaction` remains unconditionally public.
- Capture test support is unavailable to normal library consumers.
- The module path is `lisa_cli::capture_usage` only when explicitly enabled.
- No capture types are moved into the library root.

### Compilation model

- The binary target continues to include `src/capture_usage.rs` as its own
  private module.
- Feature-enabled library tests compile the same source as a library module.
- This avoids source duplication while preserving the current binary layout.

## `crates/lisa-cli/src/capture_usage.rs`

### Existing public adapter

Keep:

```rust
pub fn run_capture_usage(cwd: &Path) -> std::io::Result<()>
```

This remains the command-facing function used by `main.rs`.

### New internal processor

Introduce one generic function, named to describe a single supplied Stop:

```rust
fn capture_usage_from<R: Read, W: Write>(
    cwd: &Path,
    input: &mut R,
    is_codex: bool,
    pane_id: u32,
    captured_at: u64,
    diagnostics: &mut W,
) -> std::io::Result<()>
```

Exact generic ownership may use values rather than mutable references if the
implementation reads more clearly; the behavioral inputs remain the same.

### Responsibilities

The internal processor owns:

1. reading the supplied Stop payload;
2. JSON deserialization and missing-session validation;
3. transcript path classification;
4. provider-specific transcript parsing;
5. successful `CaptureRecord` append;
6. no-capture marker append;
7. visible no-capture diagnostic write;
8. error propagation.

It does not read environment, wall clock, global stdin, or global stderr.

### Command adapter responsibilities

`run_capture_usage` owns:

1. provider selection from `LISA_AGENT_CLIENT`;
2. pane parsing from `LISA_PANE_ID`;
3. the current epoch timestamp;
4. stdin locking;
5. stderr locking;
6. calling the internal processor.

Pane validation stays in the adapter because pane is environment-provided
process context. Session validation stays in the processor because session is
payload data used by both production and tests.

### No-capture helper change

Change `append_no_capture_marker` so it accepts:

- an explicit `captured_at: u64`;
- a generic diagnostic writer.

It continues to:

- build `NoCaptureMarker`;
- serialize compact newline-delimited JSON;
- create the provider directory;
- append `no-captures.jsonl`;
- emit the diagnostic only after persistence succeeds.

Replace `eprintln!` with `writeln!(diagnostics, ...)` so the real adapter and
deterministic fixture share exactly one output path.

### Test-support wrapper

Add a feature-gated, doc-hidden wrapper:

```rust
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn run_capture_usage_for_test<R: Read, W: Write>(...) -> io::Result<()>
```

The wrapper delegates directly to the internal processor.

It accepts no ticket ID because ticket identity is intentionally absent from
the honest capture contract. The field fixture proves stale ticket artifacts
are absent through filesystem assertions.

### Internal tests

Existing parser unit tests remain in place and unchanged unless formatting
requires minor import adjustment.

## `crates/lisa-cli/src/main.rs`

### Expected modification

No behavior change is needed if the binary retains `mod capture_usage;` and its
existing call. This path is listed only if compilation organization requires
moving the binary to `lisa_cli::capture_usage`.

### Preferred result

Keep `main.rs` unchanged. The same source file can be compiled privately by the
binary and feature-gated by the library without changing command dispatch.

If unchanged after implementation, omit this path from the ticket commit.

## `crates/lisa-plugin/Cargo.toml`

### Modification

Enable the test-support feature on the existing dev-dependency:

```toml
lisa-cli = { path = "../lisa-cli", features = ["test-support"] }
```

### Boundary

- This is a dev-dependency only.
- The WASM/runtime dependency graph does not gain `lisa-cli`.
- Existing transaction regression imports remain available.
- Feature unification during tests exposes only the test wrapper.

## `crates/lisa-plugin/src/lib.rs`

### New test

Add one test beside the existing provenance usage regressions:

```text
provenance_field_repro_keeps_six_recycles_distinct_and_surfaces_failures
```

The exact name may be shortened while retaining `field_repro` and the ticket
contract.

### Fixture constants

Define local constants for:

- recycled pane ID;
- seven ticket IDs;
- seven session IDs;
- unowned session ID;
- no-capture session ID;
- deterministic base timestamp;
- interval width and spacing.

### Transcript fixtures

For each owned ticket:

- create one transcript file in the temp directory;
- write one Claude assistant usage line;
- use unique input/cache/output fields;
- build a Stop payload pointing to that file;
- call `lisa_cli::capture_usage::run_capture_usage_for_test` with the pane,
  interval-contained timestamp, and a diagnostic buffer.

For the unowned successful observation:

- create a valid transcript with conspicuous totals;
- use a timestamp before the first ownership interval;
- call the same writer support.

For the no-capture observation:

- create an empty transcript;
- use a distinct session and timestamp;
- call the same support with a dedicated diagnostic buffer.

### Capture assertions

Deserialize `captures.jsonl` as `Vec<CaptureRecord>`.

Assert:

- row count is eight;
- row ordering matches calls;
- every row retains its pane/session/time/totals;
- the no-capture session has no successful row;
- stale ticket and `last` usage artifacts do not exist.

### No-capture assertions

Define a test-local deserializable marker matching the private CLI JSON schema.
Read `no-captures.jsonl` and assert one row with:

- recycled pane;
- no-capture session;
- injected timestamp;
- `empty-transcript`.

Assert diagnostic bytes contain the visible prefix, session, and reason.

### Attribution replay helper

Within the test, construct one `ProvenanceRecord` for each ticket interval.
Use unique attempt leases, the Claude route, null usage, and the recycled pane.

For each record:

1. call `state.read_usage(AgentClient::Claude, &record)`;
2. assert returned tokens equal that ticket's expected totals;
3. create the filled record;
4. append it to the real ledger;
5. retain expected ticket/token tuples.

After all seven, deserialize the ledger and compare every row against the full
expected vector.

### Quarantine assertions

The first attribution scan sees the earlier unowned row within the closed
boundary and quarantines it.

Assert:

- session-specific path exists;
- it contains one `QuarantinedCaptureRecord`;
- source line matches the unowned row's physical position;
- the original capture is unchanged;
- no shared quarantine path exists;
- one `ActivityEvent::Warning` names the session;
- UI projection is `ActivityType::Warning` and names the session;
- later scans do not duplicate the row or warning.

## Files not changed

- `crates/lisa-core/src/capture.rs`: schema and append behavior are already
  correct.
- `crates/lisa-plugin/src/ownership.rs`: ownership semantics are already
  correct.
- `crates/lisa-plugin/src/quarantine.rs`: persistence and encoding are already
  correct.
- Hook templates and guide: visible no-capture behavior is already covered by
  the prerequisite.
- Ticket/story/epic frontmatter: Lisa owns workflow transitions.
- Shared work artifact paths: this attempt writes only private artifacts.

## Commit units

### Unit 1: deterministic capture test support

Owned paths:

```text
crates/lisa-cli/Cargo.toml
crates/lisa-cli/src/lib.rs
crates/lisa-cli/src/capture_usage.rs
crates/lisa-plugin/Cargo.toml
```

This unit compiles the feature-gated seam and preserves existing CLI tests.

### Unit 2: field regression

Owned path:

```text
crates/lisa-plugin/src/lib.rs
```

This unit adds the seven-ticket replay and all combined assertions.

Every unit is committed through `lisa commit-ticket` with only its exact paths.

## Verification topology

Focused verification:

```text
cargo test -p lisa-cli capture_usage
cargo test -p lisa-plugin provenance_field_repro
```

Compatibility verification:

```text
cargo test -p lisa-plugin provenance_recycled_pane
cargo test -p lisa-plugin provenance_unattributable
cargo test --workspace
just check
```

Formatting is checked with `cargo fmt --all -- --check`; implementation may run
`cargo fmt --all` as a mechanical rewrite before committing exact owned paths.
