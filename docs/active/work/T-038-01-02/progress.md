# Progress — T-038-01-02 startup-launch-timing-baseline

## Outcome

Implementation is complete. The freshly rebuilt `lisa 0.4.0-rc.6` release CLI has a warm
`loop --dry-run` end-to-end startup baseline of **2.707 ms median** on this host and checkout.
An independently invoked second 30-sample batch measured **2.857 ms median**, a **5.54%**
difference, passing the declared same-host **±20% median tolerance**.

No source or harness code changed. No native Codex or Claude provider was launched. The real
Zellij stub harness and both installed-provider paths are classified explicitly below rather
than assigned misleading proxy figures.

## Completed steps

- Read the ticket, parent story, epic, project instructions, and RDSPI workflow.
- Mapped CLI, deterministic stub, and installed-provider startup boundaries.
- Selected the warm release CLI dry-run boundary for a deterministic numeric baseline.
- Defined a monotonic-clock, 3-warmup, 30-sample procedure.
- Declared ±20% same-host median tolerance before running the measurement.
- Rebuilt release WASM and release CLI with the canonical repository recipe.
- Captured release candidate and host identity.
- Ran benchmark batch 1 successfully.
- Ran an independent benchmark batch 2 successfully.
- Verified the batch medians reproduce within tolerance.
- Recorded an explicit figure-or-note decision for every relevant path.
- Preserved artifact-only story scope.

## Release candidate identity

```text
git_head=51e45e09b4dabc35ff79adea041d50bbdfbe791c
lisa_version=lisa 0.4.0-rc.6
host_product=macOS
host_version=26.5.2
host_build=25F84
working_directory=/Users/johnchen/swe/repos/lisa
measurement_started_utc=2026-07-12T15:52:03Z
timezone=America/Los_Angeles
```

The active ticket set in this checkout is part of the dry-run input. Later comparisons should
pin or record their input rather than assume different DAGs are identical workloads.

## Exact fresh-build command

From the repository root:

```bash
just build-cli
```

Observed result: PASS. The recipe completed both stages:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
touch target/wasm32-wasip1/release/lisa.wasm
cargo build -p lisa-cli --release
```

The WASM build completed successfully, and the CLI rebuilt successfully in release mode.

## Deterministic numeric path — release CLI dry-run startup

### Measured child command

```bash
target/release/lisa loop --dry-run --path .
```

### Boundary definition

The timer begins immediately before Ruby spawns the release executable and ends after the child
exits successfully. It includes:

- process spawn and loader startup;
- Clap argument parsing;
- Lisa configuration resolution;
- project-structure validation;
- active ticket scan and parse;
- route and DAG computation;
- dry-run summary formatting and writes to redirected descriptors;
- process teardown and parent wait.

It excludes:

- compilation and Cargo startup;
- provider dependency checks;
- Codex trust changes;
- embedded WASM extraction and instantiation;
- layout generation and Zellij execution;
- pane creation and provider startup;
- hook delivery, assignment acknowledgement, and model work.

The result is therefore a **warm release CLI planning-startup** baseline, not a Zellij, WASM,
Codex, Claude, or end-to-end ticket-completion latency.

### Exact benchmark command

Run this command twice from the repository root after `just build-cli`:

```bash
ruby -e 'cmd=["target/release/lisa","loop","--dry-run","--path","."]; 3.times { abort "warmup failed" unless system(*cmd, out: File::NULL, err: File::NULL) }; xs=30.times.map { t=Process.clock_gettime(Process::CLOCK_MONOTONIC); abort "sample failed" unless system(*cmd, out: File::NULL, err: File::NULL); (Process.clock_gettime(Process::CLOCK_MONOTONIC)-t)*1000 }; s=xs.sort; median=(s[14]+s[15])/2.0; puts "raw_ms=#{xs.map { |x| format("%.3f",x) }.join(",")}"; puts format("min_ms=%.3f\nmedian_ms=%.3f\nmean_ms=%.3f\nmax_ms=%.3f",s.first,median,xs.sum/xs.length,s.last)'
```

The child command is passed as an argument vector, not evaluated by a shell. Three successful
warmups precede each batch. The clock is monotonic. Child stdout and stderr are generated but
redirected to the null device. Any unsuccessful child aborts the batch.

## Batch 1 — recorded baseline

```text
batch=1
warmups=3
samples=30
raw_ms=3.610,2.745,3.163,2.735,2.510,3.144,3.107,2.673,3.063,3.465,2.618,2.956,2.777,2.517,2.777,2.859,2.393,2.344,2.680,2.571,2.405,2.456,2.929,2.518,2.547,2.761,3.108,2.503,2.428,2.559
min_ms=2.344
median_ms=2.707
mean_ms=2.764
max_ms=3.610
```

Primary before-baseline: **2.707 ms median**.

## Batch 2 — independent reproduction

```text
batch=2
warmups=3
samples=30
raw_ms=3.323,2.725,2.478,2.868,2.983,2.488,2.990,3.236,2.393,2.896,3.403,2.623,2.471,3.822,2.886,2.778,3.383,2.428,2.526,3.403,2.845,2.928,3.892,2.507,2.714,3.173,2.455,2.676,3.937,2.768
min_ms=2.393
median_ms=2.857
mean_ms=2.933
max_ms=3.937
```

Reproduction median: **2.857 ms**.

## Tolerance verification

Declared tolerance: batch 2 median must be within ±20% of batch 1 median on the same host,
checkout, working directory, and warm procedure.

```text
baseline_median_ms=2.707
rerun_median_ms=2.857
absolute_difference_ms=0.150
relative_difference_pct=abs(2.857 - 2.707) / 2.707 * 100
relative_difference_pct=5.54
tolerance_pct=20
reproduces_within_tolerance=PASS
```

The rerun is 14.46 percentage points inside the allowed bound.

## Real-Zellij local stub launch path

**Not deterministically measurable here as a focused numeric launch latency with the committed
harness.**

Exact existing behavioral harness:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

The harness is deterministic and model-free for pass/fail behavior. It proves the real Zellij,
WASM, shell, bounded bare-provider launch, start evidence, separate assignment, acknowledgement,
and recovery contracts. However, its event log records `launch` without a monotonic timestamp,
so it does not expose loop-invocation-to-stub-entry latency.

The full harness duration is not used as a proxy because it contains four scenarios, deliberate
startup/delivery failure deadlines, fixture setup, Zellij teardown, polling, recovery, and fixed
seven-second retry-observation sleeps. Its 15-second success wait is an upper bound, not an
observed launch time. Adding focused timestamps would modify the harness and violate this
story's artifact-only scope.

## Native Codex startup path

**Not deterministically measurable here.**

The exact installed-provider harness, if a separately authorized metered field observation is
needed, is:

```bash
LIVE_PROVIDER_CASES=codex \
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

It was not run by this ticket. Native Codex timing includes authenticated external-client state,
local cache/configuration, hooks, provider/service/network behavior, scheduler polling, and model
work. Grace-mode Codex also intentionally waits through a named pacing envelope before first
assignment; elapsed grace is not readiness evidence. A resulting timestamp delta would be a
metered field observation, not a deterministic local baseline.

## Native Claude startup path

**Not deterministically measurable here.**

The exact installed-provider harness, if a separately authorized metered field observation is
needed, is:

```bash
LIVE_PROVIDER_CASES=claude \
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

It was not run by this ticket. Native Claude timing includes authenticated external-client state,
local cache/configuration, hook delivery, provider/service/network behavior, scheduler/sampler
polling, and model work. Its `SessionStart[startup]` signal is truthful positive evidence, but the
elapsed interval to that evidence is still environmentally variable.

## Deviations from plan

None. The selected build and benchmark commands ran successfully, the first measurement pair
passed the predeclared tolerance, and no fallback or repeated-until-green sample pair was needed.

## Test and commit status

- Product tests added: none; no product behavior changed.
- Measurement verification: 60 successful recorded CLI invocations plus 6 successful warmups.
- Fresh release build: PASS.
- Median reproduction tolerance: PASS.
- Metered provider calls: none.
- Ticket-owned source units: none.
- `lisa commit-ticket` transactions: none required.
- Ordinary `git add` / `git commit`: not used.

## Remaining work

- Run repository integrity checks.
- Write `review.md` with the self-contained handoff.
- Stop on this ticket for Lisa's completion transaction.
