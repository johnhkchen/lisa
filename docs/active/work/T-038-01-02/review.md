# Review — T-038-01-02 startup-launch-timing-baseline

## Outcome

Acceptance is met. The post-E-037 release candidate's deterministically repeatable local startup
figure is:

> **Warm release CLI `loop --dry-run`: 2.707 ms median** over 30 measured invocations after
> three warmups, on macOS 26.5.2 at Git HEAD
> `51e45e09b4dabc35ff79adea041d50bbdfbe791c` using `lisa 0.4.0-rc.6`.

An independent second 30-sample batch measured **2.857 ms median**. The absolute difference is
0.150 ms, or **5.54%**, which passes the predeclared same-host **±20% median tolerance**.

The review does not generalize this number to Zellij, WASM instantiation, Codex, Claude,
assignment delivery, ownership, or ticket completion. Those boundaries are evaluated separately
and honestly below.

## Exact build command

From `/Users/johnchen/swe/repos/lisa`:

```bash
just build-cli
```

Observed: PASS. This ran the repository's canonical release sequence:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
touch target/wasm32-wasip1/release/lisa.wasm
cargo build -p lisa-cli --release
```

The release WASM and embedding CLI were rebuilt before timing.

## Exact reproduce command

Run the build above, then run this benchmark command twice from the same repository root:

```bash
ruby -e 'cmd=["target/release/lisa","loop","--dry-run","--path","."]; 3.times { abort "warmup failed" unless system(*cmd, out: File::NULL, err: File::NULL) }; xs=30.times.map { t=Process.clock_gettime(Process::CLOCK_MONOTONIC); abort "sample failed" unless system(*cmd, out: File::NULL, err: File::NULL); (Process.clock_gettime(Process::CLOCK_MONOTONIC)-t)*1000 }; s=xs.sort; median=(s[14]+s[15])/2.0; puts "raw_ms=#{xs.map { |x| format("%.3f",x) }.join(",")}"; puts format("min_ms=%.3f\nmedian_ms=%.3f\nmean_ms=%.3f\nmax_ms=%.3f",s.first,median,xs.sum/xs.length,s.last)'
```

Reproduction passes when the second median is within ±20% of the first on the same host,
checkout, path, and active-ticket input under comparable load.

## Recorded evidence

### Batch 1

```text
samples=30
min_ms=2.344
median_ms=2.707
mean_ms=2.764
max_ms=3.610
```

Raw execution-order values:

```text
3.610,2.745,3.163,2.735,2.510,3.144,3.107,2.673,3.063,3.465,
2.618,2.956,2.777,2.517,2.777,2.859,2.393,2.344,2.680,2.571,
2.405,2.456,2.929,2.518,2.547,2.761,3.108,2.503,2.428,2.559
```

### Batch 2

```text
samples=30
min_ms=2.393
median_ms=2.857
mean_ms=2.933
max_ms=3.937
```

Raw execution-order values:

```text
3.323,2.725,2.478,2.868,2.983,2.488,2.990,3.236,2.393,2.896,
3.403,2.623,2.471,3.822,2.886,2.778,3.383,2.428,2.526,3.403,
2.845,2.928,3.892,2.507,2.714,3.173,2.455,2.676,3.937,2.768
```

### Reproduction calculation

```text
abs(2.857 - 2.707) / 2.707 * 100 = 5.54%
5.54% <= 20% => PASS
```

## What the figure measures

The timer uses `Process::CLOCK_MONOTONIC` and brackets each complete child process. It includes
release executable spawn and loading, CLI argument parsing, config resolution, project
validation, ticket discovery/parsing, route/DAG computation, dry-run output formatting, clean
exit, and parent wait.

It is a warm measurement: three successful unrecorded invocations precede every batch. Child
stdout and stderr are redirected to the null device, preventing terminal rendering speed from
becoming part of the input while retaining Lisa's output-generation work.

It excludes compilation, Cargo overhead, provider dependency checks, trust/config mutation,
embedded WASM extraction, Zellij startup, plugin instantiation, pane creation, provider startup,
hooks, assignment delivery, acknowledgement, model execution, and completion.

## Path coverage

### Release CLI dry-run

Deterministically measurable within a tolerance: **2.707 ms median baseline**, independently
reproduced at **2.857 ms**, PASS at **5.54%** difference.

### Real-Zellij local stub launch

**Not deterministically measurable here as a focused numeric latency with the current committed
harness.**

The exact model-free behavioral harness is:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

It proves real launch and recovery behavior, but its `launch` event has no monotonic timestamp.
Total suite duration conflates four scenarios, deliberate bounded failures, fixture work,
polling, recovery, teardown, and fixed sleeps. Its wait limits are bounds, not observations.
Publishing either as launch latency would be false precision. Adding timestamps is a possible
future benchmark task, but was correctly excluded from this artifact-only baseline story.

### Native Codex startup

**Not deterministically measurable here.** The native path depends on an authenticated external
TUI, local configuration/cache state, service/network behavior, scheduler polling, hooks, and
model behavior. Codex's named startup grace is a pacing envelope, not readiness evidence. No
metered Codex run was authorized or performed.

Focused field-observation command, only with separate authorization:

```bash
LIVE_PROVIDER_CASES=codex \
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

### Native Claude startup

**Not deterministically measurable here.** The native path depends on an authenticated external
TUI, local configuration/cache state, `SessionStart` hook delivery, service/network behavior,
scheduler/sampler polling, and model behavior. No metered Claude run was authorized or performed.

Focused field-observation command, only with separate authorization:

```bash
LIVE_PROVIDER_CASES=claude \
EVIDENCE_DIR=/absolute/private/evidence/path \
  crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

## Files created, modified, or deleted

Attempt-private artifacts created:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`

Repository source/test/harness files created: none.

Repository source/test/harness files modified: none.

Repository source/test/harness files deleted: none.

Lisa began materializing phase artifacts under the shared work path while this attempt advanced;
that scheduler-owned publication was not written or edited directly by this attempt.

## Test and measurement coverage

- Fresh release WASM build: PASS.
- Fresh release CLI build and embedding refresh: PASS.
- CLI version check: PASS (`lisa 0.4.0-rc.6`).
- Benchmark warmups: 6 successful total.
- Recorded benchmark invocations: 60 successful total.
- Batch 1 raw count: 30.
- Batch 2 raw count: 30.
- Independent median tolerance: PASS.
- Deterministic/non-deterministic classification: recorded for all four relevant paths.
- Metered provider usage: none.
- Product unit/integration tests: not rerun; no product behavior or source changed.

The fresh build is the proportionate product verification for this artifact-only measurement
ticket. The committed real-Zellij regression remains the behavioral proof, but rerunning its full
fault suite would not improve the focused timing evidence because the needed timestamp is absent.

## Open concerns and limitations

- The numeric result is host-specific and workload-specific. Do not compare it as an exact value
  across different hardware, OS versions, filesystem state, or active ticket DAGs.
- It is deliberately warm. It is not a machine-cold or filesystem-cold startup figure.
- Redirected output removes terminal rendering but retains formatting and system writes.
- A 2–4 ms boundary is sensitive to host scheduling; median distributions and the ±20% tolerance
  are part of the result, not optional context.
- The current real-Zellij harness cannot yield focused loop-to-provider-entry time without added
  instrumentation.
- Installed-provider timestamps can be useful field evidence, but cannot close a deterministic
  regression claim without a controlled local substitute.
- The active ticket set may evolve before the “after” measurement. The closing report should
  either reproduce against comparable input or explicitly note the DAG/input difference.

No critical product issue was found. The only measurement gap is focused real-Zellij launch
instrumentation, which is explicitly surfaced rather than hidden.

## Repository integrity

The final ordinary worktree check showed only scheduler/concurrent-ticket state:

```text
 M docs/active/tickets/T-038-01-01.md
 M docs/active/tickets/T-038-01-02.md
 M docs/active/tickets/T-038-01-03.md
?? docs/active/work/T-038-01-02/
?? docs/active/work/T-038-01-03/
```

The ordinary index is empty (`git diff --cached --name-only` produced no paths). This attempt did
not touch those shared ticket/work paths directly. It made no ticket-owned source change, used no
ordinary `git add` or `git commit`, and required no `lisa commit-ticket` transaction.

## Acceptance assessment

The ticket asks for a recorded startup/launch timing figure or an explicit
not-deterministically-measurable note per path, exact reproduction commands, and a deterministic
rerun within a stated tolerance.

- Numeric deterministic figure: present.
- Exact build and benchmark command: present.
- Independent rerun: present.
- Stated tolerance: ±20% median.
- Observed difference: 5.54%, PASS.
- Real-Zellij focused path note: present.
- Native Codex path note: present.
- Native Claude path note: present.

Final assessment: **acceptance criteria satisfied** with an honest warm CLI startup baseline and
explicit non-deterministic boundaries for the paths the current environment/harness cannot
measure reproducibly.
