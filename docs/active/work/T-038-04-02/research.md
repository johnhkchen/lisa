# Research: release-readiness report

## Ticket boundary

`T-038-04-02` is the final aggregation ticket in `S-038-04` and the final
ticket in the E-038 dependency chain.

The acceptance criterion requires one release-readiness report containing:

- before/after native CLI size;
- before/after embedded-WASM size;
- before/after startup or launch timing where deterministically measurable;
- clearly caveated idle and active footprint observations;
- residual risks;
- the named repetition deliberately left alone; and
- the exact reproduction command for every measurement present.

The parent story constrains the evidence to deterministic local fixtures.
Live metered provider dogfood is explicitly outside scope.

The ticket begins in `phase: research`.
Lisa owns phase/status transitions, artifact admission, shared publication,
the final completion commit, and seat release.

## Repository and artifact boundary

The product is a Rust workspace with three crates:

- `lisa-core` supplies shared ticket, DAG, route, diagnostic, and provenance
  types;
- `lisa-plugin` is the Zellij plugin compiled for `wasm32-wasip1`;
- `lisa-cli` is the native command and embeds the plugin bytes at build time.

The root release profile uses size optimization and LTO.

The canonical release outputs are:

- `target/release/lisa` for the native CLI;
- `target/wasm32-wasip1/release/lisa.wasm` for the embedded plugin input.

`just build-cli` builds the WASM first, touches it to invalidate the CLI build
input, then builds the native CLI. `crates/lisa-cli/build.rs` copies the WASM
to its Cargo `OUT_DIR`, and `templates.rs` embeds that copy with
`include_bytes!`.

## Before-size evidence

`T-038-01-01` recorded the pre-cleanup release sizes at source commit
`2f8230d1d36a264522c82112c41adeb63cadf9dd`.

The environment was arm64 macOS with:

- Lisa `0.4.0-rc.6`;
- rustc `1.99.0-nightly (c4af71034 2026-07-06)`;
- Cargo `1.99.0-nightly (2f0e7011e 2026-07-05)`;
- native target `aarch64-apple-darwin`;
- WASM target `wasm32-wasip1`.

The exact locked build-and-size sequence was executed twice.
Both runs reported:

- CLI: 3,013,904 bytes;
- WASM: 1,414,183 bytes.

The build-script copy compared byte-for-byte equal to the measured WASM.
The native value is toolchain/host-specific, while the WASM is still
toolchain/source/profile-specific.

## After-size evidence

`T-038-04-01` freshly rebuilt the final source tree at commit
`4fd5fe122b8bd798e1b71abbbb44b9bc730f2e93` and recorded:

- CLI: 3,013,904 bytes;
- WASM: 1,412,657 bytes.

It also recorded SHA-256 identities before and after fixture execution:

- CLI: `5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485`;
- WASM: `5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79`.

The hashes and sizes were unchanged after dogfood.
The native CLI therefore changed by zero bytes.
The WASM decreased by 1,526 bytes, approximately 0.108% of the baseline.

## Before timing evidence

`T-038-01-02` selected one deterministically measurable numeric boundary:
warm release CLI planning startup via:

`target/release/lisa loop --dry-run --path .`

The timer spans process spawn through successful exit, including argument
parsing, config resolution, project validation, active-ticket parsing, route
and DAG computation, summary output, and teardown.

It excludes compilation, dependency/provider checks, embedded WASM loading,
Zellij execution, pane/provider startup, assignment delivery, and model work.

The before batch used three warmups and 30 monotonic-clock samples:

- minimum: 2.344 ms;
- median: 2.707 ms;
- mean: 2.764 ms;
- maximum: 3.610 ms.

An independent 30-sample rerun had a 2.857 ms median, 5.54% above the primary
batch and within the predeclared same-host ±20% tolerance.

The real-Zellij fixture does not expose a focused monotonic launch timestamp.
Its total duration includes multiple scenarios, deliberate timeouts, polling,
recovery, and fixed observation waits, so it is not a launch-time proxy.

Installed Codex and Claude startup are not deterministic local measurements.
They include external client, authentication, service, network, scheduler,
hook, and model variables.

## Footprint evidence and semantics

`T-038-01-03` established an observable host boundary on macOS: the RSS of the
uniquely named Zellij server process hosting the fixture session.

It explicitly did not claim Lisa plugin-heap attribution.

The baseline used the same server PID for both states, ten one-second samples
per state, a deterministic local Claude-named shell stub, and no authenticated
provider or model work.

Recorded baseline observations were:

- idle Zellij host-process RSS median: 81,416 KiB, range 81,408–81,424 KiB;
- active Zellij host-process RSS median: 81,568 KiB, range 81,552–81,568 KiB;
- paired host-state median difference: +152 KiB.

Every number is a host-process RSS observation, not an exact measurement of
Lisa, WASM linear memory, allocator state, or plugin heap.

The exact reproduction helper remains at:

`.lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh`

It creates an external disposable repository, initializes it with the release
CLI, launches a unique Zellij session, resolves exactly one server PID, samples
the blocked/idle state, makes one ticket open, waits for a held stub launch,
samples active state, prints evidence, and tears down.

The predecessor plan explicitly says the later after run should reuse this
same method; equality is not expected because OS residency varies.

## Clean-gate evidence

`T-038-02-01` recorded `cargo fmt --all -- --check` passing without edits.

`T-038-02-02` recorded both warning-strict Clippy commands passing:

- `cargo clippy --workspace -- -D warnings`;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`.

`T-038-02-03` recorded:

- `just check` passing;
- 723 workspace tests passing, zero failing, one ignored;
- ordinary WASM check passing;
- optimized release WASM build passing.

After the bounded cleanups, `T-038-03-02` repeated the gates and recorded:

- 725 workspace tests passing, zero failing, one ignored;
- both warning-strict Clippy boundaries passing;
- `just check` passing;
- the normally ignored real-Zellij test explicitly passing.

## Cleanup and retained-repetition evidence

`T-038-03-01` classified fourteen semantic repetition families.
Only four were authorized for the bounded cleanup ticket:

- C-01 pane signal filename parser;
- C-02 native adapter reset default;
- C-03 native adapter follow-up default;
- C-04 deterministic harness event counter.

`T-038-03-02` landed those as three exact-path source commits and verified
them with focused plus integrated coverage.

The named repetition left alone is C-05 through C-14:

- whole signal scanner loops;
- scheduler failure/reclaim paths;
- timeout/liveness loops;
- atomic publication paths;
- maintained-harness helper families;
- historical admitted harness evidence;
- lifecycle-hook JSON and merge enumerations;
- scheduler test fixture construction;
- provider assignment/reuse construction;
- adapter compatibility assertions.

These remain because their differences encode policy, authority, independent
evidence, historical immutability, or a future architectural boundary.

## Dogfood evidence

`T-038-04-01` used the freshly rebuilt release CLI for two deterministic local
fixtures:

- six-ticket atomic provider contract: PASS in 1.31 seconds wall time;
- four-scenario real-Zellij delivery boundary: PASS in 125.50 seconds wall
  time.

The real-Zellij fixture passed success, suppressed-start, suppressed-ack, and
dquote-recovery scenarios and loaded the CLI's embedded WASM.

These durations are fixture execution observations, not startup latency.
No live Codex or Claude process, provider authentication, network request, or
model token was used.

## Current workspace state

Current HEAD is `4b9b26d` (`Complete T-038-04-01`).

The ordinary worktree contains only Lisa-managed tracked modifications:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-02.md`.

The ticket path contains Lisa's phase/assignment transition and must not be
edited manually.

The attempt-private output directory is:

`.lisa/attempts/T-038-04-02/1/work/`

No phase artifact may be written directly to the shared work path.

## Constraints and open questions surfaced

- The single report should use a required RDSPI artifact so Lisa reliably
  publishes it; `progress.md` is the implementation/report evidence surface.
- After timing must use the same child command and sample method as before.
- After footprint must reuse the predecessor script without modifying the
  measured product or falsely implying attribution.
- The report must distinguish artifact size, planning startup, fixture wall
  duration, and host RSS as different units and boundaries.
- Every numeric row needs its exact producing command adjacent or referenced
  unambiguously.
- The active ticket set is part of dry-run timing input and differs from the
  baseline; this is a comparison caveat.
- No behavior change is authorized to improve a number.
- A documentation/evidence-only implementation may legitimately produce no
  ticket-owned source commit.

## Research conclusion

All required before evidence and final-tree dogfood evidence exist, but the
report still needs like-for-like after timing and footprint observations.
Those can be produced without source changes by rerunning the exact predecessor
methods against the freshly built final tree, then aggregating the results,
gate state, retained repetition, and residual risks in `progress.md`.
