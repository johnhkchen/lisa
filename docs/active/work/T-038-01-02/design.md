# Design — T-038-01-02 startup-launch-timing-baseline

## Decision summary

Record one numeric, warm, end-to-end release-CLI startup baseline for `lisa loop --dry-run`, using two independent 30-sample monotonic-clock batches. Report the first batch median as the baseline and use the second batch to verify a same-host median tolerance of ±20%. Record explicit “not deterministically measurable here” decisions for the focused real-Zellij stub launch, native Codex startup, and native Claude startup.

## Option 1 — time `lisa --version`

This is the smallest executable-startup boundary.

Advantages:

- Uses the release executable directly.
- Requires no fixture and performs no writes.
- Is inexpensive enough for many samples.
- Is largely independent of active project state.

Disadvantages:

- Exercises little Lisa-specific work beyond loader startup and Clap dispatch.
- Does not scan tickets, parse the DAG, or enter the loop command.
- Is more sensitive to timer resolution because it is extremely short.
- A good number here would say little about operator-visible loop preparation.

Decision: rejected as the primary figure. It is too shallow to represent the startup path the ticket and release report care about.

## Option 2 — time `lisa loop --dry-run`

This launches the release executable and executes the read-only loop planning path.

Advantages:

- Exercises real process startup, CLI/config parsing, ticket scanning, ticket parsing, DAG construction, formatting, and clean exit.
- Avoids provider binaries, authentication, model calls, Zellij, and WASM runtime variability.
- Is safe, free, repeatable, and fast enough for distributions rather than anecdotes.
- Already has an explicit product contract: no writes and no launch in dry-run mode.
- Can run directly against the measured checkout without creating a synthetic source fixture.

Disadvantages:

- Does not include Zellij, plugin initialization, pane creation, or provider launch.
- The active ticket set is an input and may change across later release phases.
- Warm filesystem and loader caches make the result a warm-start baseline, not a cold-boot figure.
- Sub-second host scheduling still introduces noise.

Decision: selected. Its boundary is meaningful and reproducible when the report labels it accurately and pins the checkout, host, and input context.

## Option 3 — time the complete deterministic real-Zellij regression

The committed stub harness is free and uses the actual Zellij/WASM/PTY delivery boundaries.

Advantages:

- Includes real Zellij and the embedded plugin.
- Avoids provider tokens and remote services.
- Is already the authoritative deterministic behavioral regression.
- Covers successful launch as well as bounded fault and recovery paths.

Disadvantages:

- Complete wall time includes four scenarios, deliberate failure deadlines, polling, fixture creation, cleanup, and fixed post-condition sleeps.
- Its duration is a suite-runtime number, not a startup/launch-latency number.
- The event log records launch events without monotonic timestamps.
- Existing wait bounds indicate only upper bounds, not observations.
- Instrumenting the harness would be a source change outside the story's artifact-only scope.

Decision: do not publish its total runtime as launch latency. Record that it is deterministic as a behavioral harness but does not expose a focused deterministic timing observation.

## Option 4 — derive timing from the installed-provider live harness

The live harness already records first-observed scheduler state timestamps.

Advantages:

- Closest to actual operator experience.
- Separately observes provider-aware Codex and Claude state paths.
- Uses fresh isolated projects and real provider clients.
- Existing evidence receipts allow post-run timeline calculation.

Disadvantages:

- Requires authenticated, metered provider runs.
- Includes network, service, authentication, client-cache, hook, and model variability.
- Scheduler and sampler polling quantize observations.
- A single run would be a field observation, not a deterministic baseline.
- This ticket does not authorize the live run, and the preceding ticket explicitly deferred it.

Decision: reject for deterministic measurement. Name Codex and Claude separately as not deterministically measurable here.

## Option 5 — create a new focused launch benchmark harness

A temporary or committed harness could timestamp loop invocation and stub-provider entry.

Advantages:

- Would measure the desired real Zellij launch boundary directly.
- Could retain raw samples and isolate only the success scenario.
- Could become a future regression benchmark.

Disadvantages:

- A committed harness is a source/test change forbidden by the parent story.
- A temporary patched copy would not be the exact committed harness and would be harder to reproduce reliably.
- Zellij session startup and plugin scheduling still have material environmental variability.
- Designing a robust benchmark is larger than recording this release baseline.

Decision: rejected for this ticket. The missing timestamp is documented as a measurement gap, not silently filled with a proxy.

## Sampling design

The exact measured child command is:

```text
target/release/lisa loop --dry-run --path .
```

The benchmark driver is a Ruby one-liner run from the repository root. It:

1. stores the command as an argument vector rather than a shell string;
2. runs three unrecorded warmups and aborts if any fail;
3. runs 30 recorded child processes;
4. redirects child stdout and stderr to the null device;
5. brackets each child with `Process.clock_gettime(Process::CLOCK_MONOTONIC)`;
6. converts elapsed seconds to milliseconds;
7. sorts a copy for median calculation;
8. prints every sample and min, median, mean, and max.

The complete benchmark command will be recorded verbatim in `progress.md` and `review.md`.

## Statistic selection

- Primary baseline: median wall time of batch 1.
- Reproduction check: median wall time of independently invoked batch 2.
- Supporting data: raw 30 samples, min, mean, and max for both batches.
- Median is preferred because occasional host scheduling stalls remain visible in the range without dominating the primary number.
- Thirty samples make a short warm path's central value legible without turning the ticket into a performance study.

## Tolerance

The same-host immediate-rerun tolerance is ±20% of the recorded median.

Rationale:

- The boundary is short and proportionally sensitive to process scheduling.
- This is not a cycle-level microbenchmark.
- A tighter bound could fail from ordinary host noise rather than product change.
- A looser bound would have little regression value.
- The acceptance check compares medians, not individual extrema.
- Raw values remain visible so a passing median cannot hide pathological spread.

The verification formula is:

```text
absolute(batch2_median - batch1_median) / batch1_median <= 0.20
```

## Path-by-path reporting

### Release CLI dry-run startup

Numeric and reproducible. Includes executable spawn through successful dry-run exit. Excludes compilation, Zellij, WASM startup, provider startup, delivery, and completion.

### Real-Zellij local stub launch

Behaviorally deterministic and covered by a committed harness, but no focused numeric figure is available because the harness does not timestamp loop-start-to-provider-entry. Full suite duration is explicitly not substituted.

### Native Codex startup

Not deterministically measurable here. It depends on an external authenticated TUI and its local and remote runtime state; the truthful grace path also intentionally includes a scheduler pacing envelope rather than positive pre-prompt readiness.

### Native Claude startup

Not deterministically measurable here. It depends on an external authenticated TUI, hook delivery, service/network state, and sampling/polling boundaries.

## Freshness and reproducibility

- Run `just build-cli` immediately before measurement.
- Record Git HEAD, CLI version, OS version, and the command.
- Run both benchmark batches without source changes between them.
- Keep the checkout path and active ticket set unchanged during the two batches.
- A later “after” comparison must rebuild the then-current release binary and repeat the same command on comparable hardware.
- Exact numerical equality across unrelated hosts is not claimed.

## Scope protection

- No source, tests, harness, dependencies, or configuration files will change.
- No metered provider process will be launched.
- No ticket frontmatter will be edited.
- Only attempt-private RDSPI artifacts will be written.
- Because there is no ticket-owned source unit, no `lisa commit-ticket` call is expected.

## Chosen outcome

This design yields a useful number without pretending it is a native-provider launch number. It preserves the release report's ability to distinguish a deterministic local startup baseline from live, externally variable startup observations.
