# Structure — T-038-01-02 startup-launch-timing-baseline

## File-level outcome

No repository source, test, harness, configuration, or dependency file is created, modified, or deleted by this ticket.

The ticket produces only these attempt-private phase artifacts:

- `.lisa/attempts/T-038-01-02/1/work/research.md`
- `.lisa/attempts/T-038-01-02/1/work/design.md`
- `.lisa/attempts/T-038-01-02/1/work/structure.md`
- `.lisa/attempts/T-038-01-02/1/work/plan.md`
- `.lisa/attempts/T-038-01-02/1/work/progress.md`
- `.lisa/attempts/T-038-01-02/1/work/review.md`

Lisa owns publication of admitted artifacts into `docs/active/work/T-038-01-02/`.

## Artifact responsibilities

### `research.md`

- Maps the CLI dry-run boundary.
- Maps the deterministic real-Zellij stub harness.
- Maps the installed-provider live harness.
- Describes timing tools and environmental constraints.
- Establishes the deterministic/non-deterministic boundary without choosing a solution.

### `design.md`

- Compares shallow CLI startup, dry-run startup, full stub-harness timing, live-provider timing, and new instrumentation.
- Selects warm release-CLI dry-run timing.
- Defines sample count, statistic, reproduction check, and tolerance.
- Rejects misleading proxies for real launch latency.

### `structure.md`

- Declares artifact-only ownership.
- Defines the measurement record's internal sections.
- Defines raw evidence and summary fields.
- Defines path classifications and review handoff shape.

### `plan.md`

- Orders fresh build, identity capture, two benchmark batches, computation, artifact writing, and integrity checks.
- Defines pass/fail criteria for the tolerance check.
- Defines verification that no source file changed.

### `progress.md`

This is the durable baseline record and implementation log. It will contain:

1. scope and release-candidate identity;
2. exact release build command;
3. exact benchmark command;
4. exact measured boundary;
5. batch 1 raw samples and statistics;
6. batch 2 raw samples and statistics;
7. tolerance formula and result;
8. explicit per-path measurement decisions;
9. deviations, if any;
10. repository-integrity verification.

### `review.md`

- Summarizes the accepted baseline and rerun result.
- Lists files created and confirms no repository source changes.
- Evaluates measurement coverage and limitations.
- Gives exact reproduce commands.
- Flags external-provider timing as deliberately non-deterministic.

## Measurement component boundary

The benchmark driver remains an inline command recorded in the artifacts. It is not committed as a new script because the parent story permits work artifacts only.

Inputs:

- executable: `target/release/lisa`;
- arguments: `loop --dry-run --path .`;
- working directory: repository root;
- repository state: HEAD and active tickets at measurement time;
- warmups: 3;
- recorded iterations: 30;
- output target: null device;
- clock: monotonic;
- units: milliseconds.

Outputs:

- ordered raw sample list;
- minimum;
- median;
- arithmetic mean;
- maximum.

Failure behavior:

- Any unsuccessful warmup aborts the driver.
- Any unsuccessful recorded invocation aborts the driver.
- No failed child timing is retained as a valid sample.
- A failed build prevents measurement.

## Measured boundary

Included:

- release executable process creation;
- dynamic-loader startup;
- CLI argument parsing;
- Lisa config resolution;
- loop dry-run validation;
- active ticket discovery and parsing;
- route/DAG computation;
- summary formatting and writes to redirected descriptors;
- process teardown and wait.

Excluded:

- release compilation;
- Cargo process startup;
- provider dependency checks;
- Codex trust mutation;
- embedded WASM extraction;
- Zellij cache/permission preparation;
- layout generation and write;
- Zellij execution;
- WASM plugin instantiation;
- pane creation;
- provider TUI process startup;
- hook initialization;
- assignment delivery and acknowledgement;
- model execution and ticket completion.

## Evidence schema

Identity fields:

```text
git_head=<40-hex commit>
lisa_version=<version output>
host_os=<product/version/build>
measurement_kind=warm release CLI dry-run wall time
```

Batch fields:

```text
batch=<1|2>
warmups=3
samples=30
raw_ms=<comma-separated values in execution order>
min_ms=<value>
median_ms=<value>
mean_ms=<value>
max_ms=<value>
```

Tolerance fields:

```text
baseline_median_ms=<batch 1 median>
rerun_median_ms=<batch 2 median>
relative_difference_pct=<absolute percentage>
tolerance_pct=20
reproduces_within_tolerance=<PASS|FAIL>
```

## Per-path classification

| Path | Numeric figure | Classification | Reason |
|---|---:|---|---|
| Release CLI `loop --dry-run` | yes | deterministic local baseline | read-only local path, repeat-sampled |
| Real-Zellij stub provider entry | no | not numerically exposed | harness has launch event but no focused monotonic timestamps |
| Native Codex startup | no | not deterministically measurable here | authenticated external client and remote/runtime variability |
| Native Claude startup | no | not deterministically measurable here | authenticated external client, hooks, and remote/runtime variability |

## Build ordering

1. Build release WASM through the `just build-cli` prerequisite.
2. Refresh the CLI embedding through the same recipe.
3. Capture executable version and repository HEAD.
4. Run batch 1.
5. Run batch 2 immediately without rebuilding or changing source.
6. Calculate the median delta.
7. Write results to `progress.md`.
8. Review repository integrity.
9. Write `review.md`.

## Commit boundary

- There are no ticket-owned repository source paths.
- Attempt-private work artifacts are not source units to pass to `lisa commit-ticket`.
- The implement phase therefore has zero isolated source transactions.
- Ordinary index state must remain untouched.
- Existing scheduler-owned ticket modifications remain outside this ticket's ownership.

## Verification boundary

The ticket passes implementation verification when:

- `just build-cli` succeeds;
- the release binary reports the expected RC version;
- both batches contain exactly 30 successful samples;
- batch summaries are calculated from those samples;
- the batch 2 median is within ±20% of batch 1;
- every relevant launch path has either a figure or an explicit not-deterministic note;
- exact commands are present;
- no source-owned path is modified, staged, or untracked by this ticket.

## Publication boundary

The attempt stops after `review.md`. It does not:

- edit the ticket's phase or status;
- copy artifacts into `docs/active/work/T-038-01-02/`;
- create a completion commit;
- release the seat;
- begin another ticket.

Lisa performs those operations only after lease and artifact verification.
