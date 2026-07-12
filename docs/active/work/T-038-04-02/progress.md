# Release-readiness report

## Verdict at a glance

**Ready for release within the documented deterministic-local scope.**

The final `lisa 0.4.0-rc.6` tree is formatted, warning-clean under the
repository's canonical native and WASM Clippy boundaries, green under
`just check`, and successful in both maintained deterministic dogfood fixtures.

The final native CLI size is unchanged from the pre-pass baseline. The embedded
WASM is 1,526 bytes smaller. Warm CLI planning startup is 0.049 ms lower at the
primary median, within normal same-host noise and with an independent after
batch inside the declared repeatability tolerance.

**Zellij host-process RSS — not Lisa plugin-heap attribution** is 464 KiB lower
at both idle and active medians in this observation, while the paired active
minus idle host-state difference remains +152 KiB. These RSS observations do
not establish a Lisa heap reduction.

Critical issues: none.

This verdict does not claim live Codex or Claude provider validation, exact
plugin-heap attribution, cross-platform numeric equivalence, or focused
Zellij/provider launch latency.

## Before/after scorecard

| Measurement | Before | After | Delta | Interpretation |
| --- | ---: | ---: | ---: | --- |
| Native arm64 macOS release CLI logical length | 3,013,904 bytes | 3,013,904 bytes | 0 bytes (0.000%) | unchanged |
| Release WASM embedded by the CLI | 1,414,183 bytes | 1,412,657 bytes | −1,526 bytes (−0.108%) | modest size reduction |
| Warm release CLI planning startup median | 2.707 ms | 2.658 ms | −0.049 ms (−1.810%) | effectively stable at this scale |
| Idle Zellij host-process RSS — **not Lisa plugin-heap attribution** | 81,416 KiB | 80,952 KiB | −464 KiB (−0.570%) | variable host observation only |
| Active Zellij host-process RSS — **not Lisa plugin-heap attribution** | 81,568 KiB | 81,104 KiB | −464 KiB (−0.569%) | variable host observation only |
| Active minus idle Zellij host-state RSS — **not Lisa plugin-heap attribution** | +152 KiB | +152 KiB | 0 KiB | paired observed difference unchanged |

The size comparison is exact for the listed files and toolchain/source
snapshots. The startup and RSS comparisons are observations, not universal
constants or optimization thresholds.

## Release and environment identity

Final report source:

```text
git_head=4b9b26de023351a15122cf5d9b1957a2f7c6a9b0
git_description=4b9b26d Complete T-038-04-01
measurement_started_utc=2026-07-12T19:43:17Z
timezone=PDT
lisa=lisa 0.4.0-rc.6
rustc=rustc 1.99.0-nightly (c4af71034 2026-07-06)
cargo=cargo 1.99.0-nightly (2f0e7011e 2026-07-05)
just=just 1.56.0
zellij=zellij 0.44.3
host=Darwin 25.5.0 arm64
physical_memory_bytes=25769803776
```

The final report commit is the Lisa completion of T-038-04-01. Its difference
from T-038-04-01's dogfood source commit
`4fd5fe122b8bd798e1b71abbbb44b9bc730f2e93` is workflow documentation and
publication, not product source. The final artifact hashes match dogfood
exactly.

Before source identities vary by parallel baseline ticket:

- size baseline: `2f8230d1d36a264522c82112c41adeb63cadf9dd`;
- startup baseline: `51e45e09b4dabc35ff79adea041d50bbdfbe791c`;
- footprint baseline: measured before sibling documentation advanced the HEAD
  from `51e45e0` to `419ed22`.

All baselines and after measurements used Lisa `0.4.0-rc.6`, the same recorded
Rust/Cargo revisions, arm64 macOS, and `wasm32-wasip1` plugin target.

Native file length, process startup, and host RSS remain sensitive to host,
toolchain, linker, filesystem/process state, source, and input. Exact equality
on another platform is not claimed.

## Artifact size measurement

### Exact before reproduction command

Run from the repository root at the recorded before source revision:

```bash
cargo build --locked -p lisa-plugin --target wasm32-wasip1 --release &&
touch target/wasm32-wasip1/release/lisa.wasm &&
cargo build --locked -p lisa-cli --release &&
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

The predecessor executed that command twice and received identical output:

```text
 3013904 target/release/lisa
 1414183 target/wasm32-wasip1/release/lisa.wasm
 4428087 total
```

### Exact after build command

Run from the repository root at the final source revision:

```bash
just build-cli
```

The observed recipe executed successfully in this order:

```text
cargo build -p lisa-plugin --target wasm32-wasip1 --release
touch target/wasm32-wasip1/release/lisa.wasm
cargo build -p lisa-cli --release
```

### Exact after size command

```bash
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

Observed output:

```text
 3013904 target/release/lisa
 1412657 target/wasm32-wasip1/release/lisa.wasm
 4426561 total
```

Supporting exact identity command:

```bash
shasum -a 256 \
  target/release/lisa \
  target/wasm32-wasip1/release/lisa.wasm
```

Observed identities:

```text
5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485  target/release/lisa
5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79  target/wasm32-wasip1/release/lisa.wasm
```

Both hashes match the pre-fixture and post-fixture identities in T-038-04-01.
The freshly dogfooded artifacts and the final reported artifacts are therefore
the same bytes.

Calculation:

```text
CLI delta = 3,013,904 - 3,013,904 = 0 bytes
CLI percent = 0 / 3,013,904 * 100 = 0.000000%

WASM delta = 1,412,657 - 1,414,183 = -1,526 bytes
WASM percent = -1,526 / 1,414,183 * 100 = -0.107907%
```

The WASM value is the uncompressed byte slice copied by the CLI build script
and embedded with `include_bytes!`. It is not archive size, allocated memory,
or runtime RSS.

## Startup and launch timing

### Deterministically measurable boundary

The numeric before/after boundary is warm release CLI **planning startup**:

```bash
target/release/lisa loop --dry-run --path .
```

Timing begins immediately before the parent spawns the child and ends after
successful child exit. It includes process/loader startup, Clap parsing, Lisa
configuration, project validation, active-ticket parsing, routing and DAG
computation, dry-run summary writes to redirected descriptors, teardown, and
parent wait.

It excludes compilation, provider dependency checks, embedded-WASM extraction
and instantiation, layout generation, Zellij execution, pane creation, provider
startup, lifecycle hooks, assignment acknowledgement, and model work.

### Exact before and after benchmark command

The same command produced every startup batch. Run it twice after the release
build:

```bash
ruby -e 'cmd=["target/release/lisa","loop","--dry-run","--path","."]; 3.times { abort "warmup failed" unless system(*cmd, out: File::NULL, err: File::NULL) }; xs=30.times.map { t=Process.clock_gettime(Process::CLOCK_MONOTONIC); abort "sample failed" unless system(*cmd, out: File::NULL, err: File::NULL); (Process.clock_gettime(Process::CLOCK_MONOTONIC)-t)*1000 }; s=xs.sort; median=(s[14]+s[15])/2.0; puts "raw_ms=#{xs.map { |x| format("%.3f",x) }.join(",")}"; puts format("min_ms=%.3f\nmedian_ms=%.3f\nmean_ms=%.3f\nmax_ms=%.3f",s.first,median,xs.sum/xs.length,s.last)'
```

Each batch has three successful warmups followed by 30 successful monotonic
samples. Any failed child aborts the batch; no sample is silently discarded.

### Before primary batch

```text
samples=30
min_ms=2.344
median_ms=2.707
mean_ms=2.764
max_ms=3.610
```

The independent before rerun measured a 2.857 ms median, 5.54% from the primary
batch and inside the predeclared same-host ±20% tolerance.

### After batch 1: primary after value

```text
raw_ms=3.763,2.604,3.384,3.604,2.612,3.335,2.929,2.567,3.122,2.658,2.491,2.712,3.167,2.659,2.894,3.157,2.419,2.605,3.184,2.433,2.418,2.903,2.736,2.369,2.589,2.991,2.413,2.427,2.243,2.475
min_ms=2.243
median_ms=2.658
mean_ms=2.795
max_ms=3.763
```

### After batch 2: independent reproduction

```text
raw_ms=2.426,2.336,2.206,2.289,2.473,2.455,3.048,2.650,2.353,2.245,2.537,2.450,2.331,2.255,2.581,2.280,2.312,2.905,2.545,2.477,2.589,2.782,2.351,2.414,2.766,2.448,2.239,2.239,2.358,2.491
min_ms=2.206
median_ms=2.437
mean_ms=2.461
max_ms=3.048
```

Repeatability calculation:

```text
absolute after-batch median difference = abs(2.437 - 2.658) = 0.221 ms
relative difference = 0.221 / 2.658 * 100 = 8.314522%
declared tolerance = ±20%
result = PASS
```

Before/after primary calculation:

```text
delta = 2.658 - 2.707 = -0.049 ms
percent = -0.049 / 2.707 * 100 = -1.810122%
```

The after primary median is 1.81% lower, but this scale is dominated by normal
same-host process scheduling and the active ticket set is part of the dry-run
input. The result supports “no material planning-startup regression,” not a
claimed user-visible speedup.

### Real-Zellij and installed-provider launch timing

Focused real-Zellij local-stub launch latency remains **not deterministically
measurable with the maintained harness**. Its event log records launch without
a monotonic timestamp. Whole-fixture time contains four scenarios, deliberate
failure deadlines, polling, recovery, teardown, and fixed waits, so it is not a
launch-latency proxy.

The exact behavioral harness is:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

Installed Codex and Claude startup remain **not deterministically measurable
here**. The existing authorized field harness commands, which this epic did not
run, are:

```bash
LIVE_PROVIDER_CASES=codex \
EVIDENCE_DIR=/absolute/private/evidence/path \
  bash crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

```bash
LIVE_PROVIDER_CASES=claude \
EVIDENCE_DIR=/absolute/private/evidence/path \
  bash crates/lisa-cli/tests/fixtures/live_provider_startup.sh
```

Those paths include authenticated client, service/network, local configuration,
hook, scheduler, and model variables and would be metered field observations,
not deterministic local baselines.

## Idle and active footprint observation

### Mandatory interpretation boundary

Every value in this section is **Zellij host-process RSS — not Lisa
plugin-heap attribution**.

The observed PID hosts the Zellij session, plugin runtime, terminal/pane state,
and related server allocations. RSS also reflects OS residency behavior. It is
not an exact measurement of Lisa's WASM linear memory, allocator, retained
objects, or heap.

### Exact before and after method

The after observation reused the exact predecessor helper without modification:

```bash
bash -n .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
bash .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
```

The helper:

- builds no product source;
- creates an external disposable repository;
- initializes it with `target/release/lisa`;
- starts one uniquely named real-Zellij session;
- resolves exactly one session server PID;
- settles for five seconds;
- takes ten one-second idle samples while the sole ticket is blocked;
- makes that ticket open and waits for a deterministic local stub launch;
- takes ten one-second active samples while the stub remains in flight;
- rechecks the same server PID;
- prints `measurement_complete=PASS`;
- kills the unique session and removes the fixture.

No authenticated provider, network request, or model work is part of this
method.

### Before observation

**Zellij host-process RSS — not Lisa plugin-heap attribution:**

```text
idle:   count=10 min=81,408 KiB median=81,416 KiB max=81,424 KiB
active: count=10 min=81,552 KiB median=81,568 KiB max=81,568 KiB
paired active-minus-idle median difference=+152 KiB
```

### After observation identity

```text
timestamp_utc=2026-07-12T19:45:39Z
source_head=4b9b26de023351a15122cf5d9b1957a2f7c6a9b0
session=lisa-rss-97135
server_pid=97244
lisa_bytes=3013904
lisa_sha256=5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485
wasm_bytes=1412657
wasm_sha256=5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79
active_launch_receipt=2026-07-12T19:45:55Z
measurement_complete=PASS
```

The final PID check still identified PID 97244 and the same unique session.

### After raw samples

Idle **Zellij host-process RSS — not Lisa plugin-heap attribution**, KiB:

```text
80960,80960,80960,80960,80960,80944,80944,80944,80944,80944
```

Active **Zellij host-process RSS — not Lisa plugin-heap attribution**, KiB:

```text
81072,81088,81088,81104,81104,81104,81104,81104,81104,81104
```

Independent recomputation:

```text
idle count=10 min=80,944 KiB median=80,952 KiB max=80,960 KiB
active count=10 min=81,072 KiB median=81,104 KiB max=81,104 KiB
paired active-minus-idle median difference=+152 KiB
```

### Before/after interpretation

**Zellij host-process RSS — not Lisa plugin-heap attribution:**

```text
idle median delta = 80,952 - 81,416 = -464 KiB (-0.569913%)
active median delta = 81,104 - 81,568 = -464 KiB (-0.568851%)
paired host-state difference delta = 152 - 152 = 0 KiB
```

The matching 464 KiB shift in both absolute medians and unchanged paired
difference are consistent with different base residency between sessions. They
do not demonstrate that the cleanup removed 464 KiB from Lisa. The useful
regression evidence is the repeatable method, stable same-run PID, raw samples,
and honestly scoped state comparison.

### Measurement-run capture deviation

Two earlier helper invocations completed beyond the command runner's 30-second
yield boundary, but their session handles were not retained, leaving partial
terminal captures through active sample 7. They are not treated as failed
fixtures and none of their values appear in the scorecard.

The final invocation retained and polled the command session through all 20
samples, final PID identity, and `measurement_complete=PASS`.

After the helper completed successfully, an outer diagnostic wrapper attempted
to assign to zsh's reserved read-only variable `status`, producing a wrapper
exit code of 1. That assignment occurred after the helper's PASS marker and
cleanup and did not alter the measurement. The exact reproduction command above
does not include that diagnostic wrapper.

## Quality gates on the tightened tree

The final post-cleanup verification recorded these commands and results:

### Formatting

```bash
cargo fmt --all -- --check
```

Result: PASS, no formatting diff.

### Native warning-strict Clippy

```bash
cargo clippy --workspace -- -D warnings
```

Result: PASS, zero warnings.

### WASM warning-strict Clippy

```bash
cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings
```

Result: PASS, zero warnings.

### Canonical check

```bash
just check
```

Result: PASS. The command covers ordinary WASM check and workspace tests.

Final post-cleanup standard workspace test count:

```text
725 passed, 0 failed, 1 ignored
```

The ignored entry is the environment-equipped real-Zellij integration test. It
was explicitly executed after the cleanup with:

```bash
cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture
```

Result: PASS, 1 passed, 0 failed in 125.81 seconds.

The optimized plugin release path also passed via `just build-cli` during final
measurement and dogfood builds.

## Deterministic local dogfood

### Atomic provider contract

Exact fixture command:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash docs/active/work/T-031-03/harness/run.sh
```

Observed result against the fresh final CLI:

```text
PASS: six-ticket atomic provider contract
```

This covers real Git repositories, init/validate, Codex- and Claude-routed
logical tickets through deterministic drivers, exact-path isolated source
transactions, completion publication, dependency ancestry, provenance, Done
first appearing in the completion commit, and foreign ordinary-index tuple
preservation.

It does not load the embedded WASM.

The recorded 1.31-second whole-fixture wall observation can be reproduced with:

```bash
/usr/bin/time -p env LISA_BIN="$PWD/target/release/lisa" \
  bash docs/active/work/T-031-03/harness/run.sh
```

That is fixture execution duration, not startup latency.

### Real-Zellij delivery boundary

Exact fixture command:

```bash
LISA_BIN="$PWD/target/release/lisa" \
  bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

Observed receipts:

```text
scenario success
scenario suppress-start
scenario suppress-ack
scenario dquote
real-zellij-delivery-boundary: PASS
```

This invokes `lisa loop`, writes and loads the CLI's embedded WASM in real
Zellij, and covers normal assignment delivery, bounded missing-start recovery,
bounded missing-ack retry/failure, and same-pane dquote recovery.

The recorded 125.50-second whole-fixture wall observation can be reproduced
with:

```bash
/usr/bin/time -p env LISA_BIN="$PWD/target/release/lisa" \
  bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh
```

That duration includes four scenarios and deliberate waits and is not a
focused launch metric.

Both fixtures use deterministic local inputs. The real-Zellij fixture uses a
shell stub named `claude`. No installed Codex or Claude client, provider
authentication, network request, or model token was used.

## Small demonstrated cleanups landed

Only the four candidates authorized by the repetition inventory landed:

- C-01: one pure `pane-<u32>.<suffix>` filename parser used by seven scheduler
  signal-consumer families;
- C-02: shared native adapter `ClearHandshake` reset default;
- C-03: shared native adapter type-into-pane follow-up default;
- C-04: one script-local deterministic harness event-count primitive.

Changed maintained paths:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.

Each path entered through its own exact-path `lisa commit-ticket` predecessor
transaction. Focused parser, adapter, and real-Zellij tests passed, followed by
the complete canonical gates.

No dependency, public CLI, provider contract, scheduling authority, hook
schema, or release profile changed. No behavior was changed to hit a size,
timing, or RSS number.

## Named repetition deliberately left alone

- **C-05 — whole signal scanner loops.** Payloads, deletion timing, legacy
  naming, lease admission, polling order, and state effects differ. A typed
  ingestion boundary is future-epic work, not a local loop extraction.
- **C-06 — scheduler failure/reclaim paths.** Sites have different authority
  over seats, threads, panes, leases, provenance, and retry state; superficial
  unification risks changing recovery semantics.
- **C-07 — timeout/liveness loops.** Clocks, exemptions, nudges, deadlines, and
  reclaim actions encode distinct policy and need a designed subsystem.
- **C-08 — atomic publication paths.** Temporary names, serialization,
  collision handling, execution side, and attribution differ; centralization
  requires a broader publication architecture.
- **C-09 — maintained-harness helper families.** The scripts are independently
  executable evidence contracts with different globals, cleanup, retention,
  and metering boundaries; sharing would couple them.
- **C-10 — historical admitted harness evidence.** It is immutable completed
  ticket evidence. Consolidation would rewrite history; no future epic is
  recommended unless evidence-retention policy changes.
- **C-11 — lifecycle-hook JSON and merge enumerations.** A declarative schema
  must preserve generation, provider matchers, user ownership, legacy upgrade,
  and idempotence semantics; this is larger than a safe local cleanup.
- **C-12 — scheduler test fixture construction.** A broad builder migration
  would churn historical authority regressions and should follow production and
  test module decomposition.
- **C-13 — provider assignment and reuse construction.** Claude context
  selection and Codex acknowledgement tagging are intentional differences; a
  third provider should demonstrate any genuine shared policy first.
- **C-14 — adapter compatibility assertions.** Independent repeated assertions
  are provider-parity and no-op regression evidence and should remain explicit.

This list is the deliberate scope boundary, not an accidental backlog omission.

## Residual risks and limitations

### Non-blocking: no live installed-provider dogfood

The release passed deterministic provider-contract and real-Zellij fixtures,
but no authenticated Codex or Claude process ran. Client upgrades, account
state, provider service/network behavior, and live hook behavior remain field
risks. This is explicitly outside E-038's authorized story boundary.

### Non-blocking: focused launch latency is unavailable

The numeric startup result is CLI planning startup only. The maintained
real-Zellij fixture lacks a monotonic loop-to-stub-entry timestamp, and installed
provider startup is externally variable. The report refuses to substitute
whole-fixture duration for launch latency.

### Non-blocking: RSS is not attributable

Zellij server RSS cannot isolate plugin heap. The −464 KiB absolute shift is
best interpreted as different base residency; only the method and same-session
paired observation are durable evidence.

### Non-blocking: native numbers are environment-specific

The CLI size and timing were measured on arm64 macOS under a nightly Rust
toolchain. Other targets, linkers, compiler versions, DAG inputs, and host load
can produce different values.

### Non-blocking: one standard integration remains ignored

`cargo test --workspace` retains one ignored real-Zellij integration because it
has external tool prerequisites. The ticket explicitly ran it and it passed,
so the seam is covered in this environment even though the standard suite does
not execute it automatically.

### Non-blocking: broader non-canonical Clippy scope

The repository's canonical warning-strict native and WASM commands pass. The
broader exploratory command
`cargo clippy --workspace --all-targets --all-features -- -D warnings` reports
thirteen pre-existing test-only lints in untouched paths. Expanding cleanup
would violate the demonstrated-value inventory boundary.

### Non-blocking: trait defaults require future-provider care

Future adapters with different transport semantics must override the native
reset/follow-up defaults rather than inheriting them accidentally. The trait
comments preserve that extension warning and alternative enum variants remain.

### Non-blocking: deferred repetition remains structurally complex

C-05 through C-13 include genuine future architectural work. Their presence is
not a release regression, but future changes in those areas should retain the
inventory's authority and evidence boundaries rather than pursue line-count
reduction alone.

No residual risk is critical within the stated deterministic-local release
scope.

## Reproduction command index

Run from the repository root unless an explicit source revision is required.

| Evidence | Exact command |
| --- | --- |
| Final CLI + embedded WASM build | `just build-cli` |
| CLI + WASM logical byte lengths | `wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm` |
| CLI + WASM SHA-256 identities | `shasum -a 256 target/release/lisa target/wasm32-wasip1/release/lisa.wasm` |
| Warm planning startup | the exact Ruby command in “Startup and launch timing,” invoked twice |
| Idle/active Zellij host-process RSS | `bash .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh` |
| Formatting gate | `cargo fmt --all -- --check` |
| Native warning-strict Clippy | `cargo clippy --workspace -- -D warnings` |
| WASM warning-strict Clippy | `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings` |
| Canonical WASM/test gate | `just check` |
| Ignored real-Zellij integration | `cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture` |
| Atomic deterministic dogfood | `LISA_BIN="$PWD/target/release/lisa" bash docs/active/work/T-031-03/harness/run.sh` |
| Real-Zellij deterministic dogfood | `LISA_BIN="$PWD/target/release/lisa" bash crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` |

For the exact before size result, check out the recorded before revision and use
the explicit locked build sequence in the size section. For exact native byte
equality, retain the recorded compiler/host inputs.

## Implementation, transactions, and repository integrity

Ticket-owned product/test source changes: zero.

Ticket source commits: zero.

`lisa commit-ticket` calls: zero, because no meaningful ticket-owned source unit
exists. Phase artifacts and generated `target/` outputs are not source commit
inputs.

Ordinary `git add`, `git add -A`, and ordinary `git commit` were not used.

Final pre-report audit found the ordinary index empty. The only modified tracked
paths are Lisa-managed:

- `.lisa/provenance.jsonl`;
- `docs/active/tickets/T-038-04-02.md`.

Lisa also exposed admitted phase artifacts as untracked files under
`docs/active/work/T-038-04-02/` while automatically advancing phases. This
attempt wrote only to the private attempt directory and did not edit those
shared publication inputs.

No ticket-owned maintained source path is staged, modified, or untracked.

## Acceptance status

The ticket acceptance criterion is satisfied:

- one authoritative release-readiness report exists in this required
  `progress.md` artifact;
- CLI and embedded-WASM before/after sizes are present;
- deterministic warm planning-startup before/after timing is present;
- non-deterministic launch paths are explicitly classified instead of assigned
  false precision;
- idle and active footprint observations are present with repeated host-process
  RSS/non-attribution caveats;
- residual risks are explicit;
- C-05 through C-14 are named as repetition left alone;
- every reported measurement has an exact reproduction command;
- final artifacts are bound to the freshly dogfooded hashes;
- no behavior was changed to improve a number.
