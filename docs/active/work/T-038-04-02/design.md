# Design: release-readiness report

## Decision statement

Create one self-contained release-readiness report in the required
`progress.md` artifact.

The report will combine predecessor evidence with fresh like-for-like after
measurements for:

- release artifact byte sizes;
- warm release CLI planning startup;
- idle and active Zellij host-process RSS.

It will also contain clean-gate and dogfood status, the complete named
repetition-left-alone list, residual risks, and exact reproduction commands.

`review.md` will assess and summarize that report without duplicating every raw
measurement, preserving `progress.md` as the single authoritative report.

No product or maintained test source is expected to change.

## Goals

The selected design must:

- make release readiness visible in one readable artifact;
- preserve the exact meaning and provenance of each before value;
- produce fresh after observations from the final source tree;
- compare only equivalent measurement boundaries;
- include exact copyable commands for every numeric measurement;
- distinguish measurement commands from behavioral verification commands;
- state environment and source identities;
- avoid host-RSS-as-plugin-heap overclaiming;
- explicitly name all deferred repetition C-05 through C-14;
- retain the deterministic-local/no-live-provider boundary;
- surface critical and non-critical residual risks;
- avoid changing behavior to improve a result.

## Non-goals

The report will not:

- claim cross-platform reproducibility for native binary size or RSS;
- claim exact reproducible builds from matching byte lengths alone;
- attribute Zellij server RSS or its delta to Lisa's heap;
- treat whole-fixture wall time as focused startup latency;
- add timestamps to a maintained harness;
- launch authenticated Codex or Claude clients;
- consume provider/model traffic;
- broaden Clippy to unrelated `--all-targets --all-features` remediation;
- pull deferred repetition into the current ticket;
- alter source, dependencies, profiles, or public CLI behavior to hit a number;
- create a second shared report outside the RDSPI artifact flow.

## Option 1: summarize only existing predecessor records

This option would combine the before baselines with T-038-04-01's artifact
sizes and dogfood results.

### Advantages

- Fast and entirely documentation-only.
- Avoids new process and RSS variance.
- Uses already reviewed records.

### Limitations

- T-038-04-01 did not rerun the 30-sample planning-startup benchmark.
- T-038-04-01 did not rerun the idle/active RSS fixture.
- Fixture wall durations are not equivalent to planning-startup timing.
- A before-only footprint table would not satisfy the natural before/after
  reading of the ticket.

Rejected because it would leave two comparison rows unsupported or substitute
non-equivalent figures.

## Option 2: rerun every predecessor phase from scratch

This option would rebuild twice, rerun all gates, all size comparisons, timing,
RSS, and dogfood fixtures.

### Advantages

- Maximizes same-ticket evidence collection.
- Reduces reliance on admitted predecessor documents.
- Gives one temporal window for all after checks.

### Limitations

- Repeats already fresh dogfood and gate work without a source change.
- The real-Zellij fixture takes about two minutes and is not needed to compute
  the missing comparison values.
- Repetition adds host noise and operational cost, not stronger equivalence.
- It obscures the dependency design, where predecessors intentionally provide
  reviewed inputs to this aggregator.

Rejected as unnecessary duplication.

## Option 3: use predecessor records plus targeted fresh after measurements

This option treats reviewed predecessors as the durable before/gate/dogfood
inputs and reruns only the missing like-for-like after boundaries:

1. canonical final release build;
2. exact logical byte lengths;
3. two independent warm startup batches;
4. the exact predecessor host-RSS helper;
5. integrity and source-state checks.

### Advantages

- Produces every required comparison using equivalent methods.
- Keeps the report tied to freshly rebuilt final artifacts.
- Preserves the predecessor's measurement semantics and caveats.
- Avoids rerunning expensive evidence that is already fresh and reviewed.
- Makes command provenance explicit and compact.
- Requires no source change if all observations complete normally.

### Limitations

- The dry-run DAG input has changed since the before measurement because ticket
  state is part of the checkout.
- RSS is inherently variable and the fixture runs at a different time.
- The private predecessor helper is attempt-local rather than maintained
  product source.
- The report necessarily aggregates evidence captured at several commits.

Selected because it is the smallest method-complete path.

## Option 4: add a maintained benchmark/report script

This option would create a repository script that builds, benchmarks, samples,
and renders the report.

### Advantages

- Future release passes could invoke one maintained command.
- Machine-readable output could reduce transcription risk.
- The process could become a recurring regression tool.

### Limitations

- The ticket asks for a report, not new product tooling.
- RSS fixture maintenance has nontrivial macOS/Zellij assumptions.
- A generalized runner would require interface and portability decisions.
- It creates a source unit without demonstrated repeat use.
- It risks turning a closing evidence slice into a new harness project.

Rejected for this release. Exact commands in the report provide reproducibility
without committing speculative tooling.

## Report location options

### Auxiliary `release-readiness.md`

The title would be explicit, but auxiliary attempt artifacts are not the
workflow's guaranteed publication contract. Predecessor reviews note that
auxiliary evidence may not be published with required artifacts.

### `review.md`

Review is guaranteed to publish, but using it as the primary report postpones
the implementation result until the final phase and makes progress tracking
awkward.

### `progress.md`

Progress is the required Implement artifact and naturally records measurements,
commands, results, deviations, and acceptance status. It is guaranteed to enter
Lisa's completion publication.

Selected: `progress.md`, titled “Release-readiness report.”

## Size comparison design

The after command will run the canonical `just build-cli` first, then measure:

```bash
wc -c target/release/lisa target/wasm32-wasip1/release/lisa.wasm
```

The report will calculate:

- absolute byte difference: after minus before;
- percentage difference: difference divided by before times 100.

It will not infer performance or quality from size alone.

The after build's hashes will identify the measured files and will be compared
with the T-038-04-01 dogfood hashes.

## Startup comparison design

The exact predecessor Ruby command will be run twice after the final build.

Each batch includes:

- three successful warmups;
- 30 successful timed child invocations;
- monotonic clock timing;
- redirected child output;
- raw values and min/median/mean/max.

The primary after value is batch 1's median.
Batch 2 must be within the predecessor's declared same-host ±20% tolerance of
batch 1.

The before/after comparison will use the primary medians:

- before: 2.707 ms;
- after: newly observed batch 1 median.

The report must state that the active-ticket input changed and that millisecond
differences at this scale include host scheduler/process-spawn noise.

Real-Zellij and installed-provider launch timing remain “not deterministically
measurable here” for the same reasons recorded in the baseline.

## Footprint comparison design

Run the exact retained predecessor helper:

```bash
bash .lisa/attempts/T-038-01-03/1/work/measure-host-rss.sh
```

The helper already binds to the current release artifacts and prints their
identities. No copy or modification is necessary.

The report will show before and after for:

- idle median and range;
- active median and range;
- paired host-state median difference.

Every row and interpretation will label the unit as Zellij host-process RSS in
KiB and repeat that it is not Lisa plugin-heap attribution.

No pass/fail threshold will be imposed on RSS equality. The verification
criteria are stable method, unique same-session server PID, correct sample
counts, state receipts, and successful teardown.

## Gate and dogfood aggregation design

The report will distinguish current release gates from measurements.

It will cite the final post-cleanup outcomes:

- fmt pass;
- native Clippy pass with warnings denied;
- WASM Clippy pass with warnings denied;
- `just check` pass;
- 725 tests passed, zero failed, one ignored in the standard suite;
- ignored real-Zellij integration explicitly passed;
- deterministic atomic provider fixture passed;
- deterministic real-Zellij four-scenario fixture passed.

Exact commands will be included for these verification boundaries as well,
even though the acceptance clause specifically requires them for measurements.

## Retained repetition design

The report will name C-05 through C-14 individually, with a compact reason each
remains. It will not collapse them into “larger cleanup deferred,” because the
ticket explicitly asks for named repetition left alone.

C-10 and C-14 are intentionally retained evidence, not merely backlog.
The others are future-epic or demonstration-gated boundaries.

## Residual-risk design

Risks will be grouped by operational meaning:

- no live installed-provider dogfood;
- timing boundary excludes Zellij/WASM/provider launch;
- RSS is host-process observation, not attribution;
- native size and timing are host/toolchain/input specific;
- one standard integration remains ignored but was manually executed;
- broader non-canonical Clippy reports pre-existing test-only lints;
- future adapters could inherit native defaults incorrectly without override;
- deferred scheduler/hook/harness repetition remains structurally complex.

Each risk will state whether it blocks release readiness.
The expected conclusion is “ready within deterministic local scope,” not an
unqualified production guarantee.

## Failure handling

If the build fails, no stale size will be reported as after evidence.

If startup samples fail, the batch aborts rather than dropping samples.

If batch medians differ by more than ±20%, the report records the failure and
does not characterize timing as reproduced.

If the RSS helper fails, retain its output, inspect unique-session cleanup, and
do not claim an after footprint.

If a source fix appears necessary, document a plan deviation first and commit
only its exact ticket-owned path through `lisa commit-ticket`.

## Decision rationale

The selected approach respects the DAG: predecessor tickets provide reviewed
before, gate, cleanup, and dogfood evidence; this final ticket fills only the
missing after comparisons and renders one authoritative report. It maximizes
semantic equivalence while keeping the release candidate unchanged.
