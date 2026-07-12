# Review: release-readiness report

## Review outcome

`T-038-04-02` satisfies its acceptance criterion.

The required `progress.md` is the single authoritative release-readiness
report. It contains:

- before/after native CLI byte size;
- before/after embedded-WASM byte size;
- before/after deterministically measurable planning-startup timing;
- explicit classifications for launch paths that are not deterministically
  measurable here;
- before/after idle and active Zellij host-process RSS observations with
  repeated non-attribution caveats;
- final clean-gate and deterministic dogfood status;
- all named repetition C-05 through C-14 left alone;
- residual risks and release-scope interpretation;
- exact reproduction commands for every reported measurement.

No critical issue was found.

The evidence supports “ready within the documented deterministic-local scope,”
not an unqualified live-provider or cross-platform guarantee.

## Principal before/after result

| Measurement | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Native arm64 macOS release CLI | 3,013,904 bytes | 3,013,904 bytes | 0 bytes |
| Embedded release WASM | 1,414,183 bytes | 1,412,657 bytes | −1,526 bytes |
| Warm CLI planning-startup median | 2.707 ms | 2.658 ms | −0.049 ms |
| Idle Zellij host RSS, not Lisa plugin heap | 81,416 KiB | 80,952 KiB | −464 KiB |
| Active Zellij host RSS, not Lisa plugin heap | 81,568 KiB | 81,104 KiB | −464 KiB |
| Active-minus-idle Zellij host RSS | +152 KiB | +152 KiB | 0 KiB |

The CLI is unchanged in size. The WASM decreased by 0.108%.

The planning-startup median is 1.81% lower, but this is correctly interpreted
as stable rather than a demonstrated user-visible speedup.

The 464 KiB absolute RSS shift appears equally in both observed states, while
the paired difference remains identical. The report correctly treats that as
base host-residency variation and makes no Lisa heap-reduction claim.

## Acceptance mapping

### Single report

The release-readiness report is the required Implement artifact:

`.lisa/attempts/T-038-04-02/1/work/progress.md`

Using a required artifact makes it part of Lisa's standard admission and
completion publication rather than depending on auxiliary-file behavior.

This Review summarizes the result but does not create a second competing
report.

### CLI and WASM size

The report records the exact locked before command and canonical final build
command, raw `wc -c` output, hashes, arithmetic, source identities, and
host/toolchain limitations.

Final artifact hashes exactly match the freshly rebuilt, pre/post-dogfood
hashes from T-038-04-01:

```text
CLI  5f079b3f96f482d84e6ca6adb0a398bd483e16375c3500d89df7904abcc80485
WASM 5f2743441e5a16024b5bd6019ddc917f347869c6d4c0d9b0d2a435e4c299ed79
```

This binds the report to the tested artifact bytes.

### Startup and launch timing

The report reuses the exact baseline Ruby benchmark:

- three warmups;
- 30 successful monotonic-clock samples;
- child argv without shell evaluation;
- redirected child output;
- abort on any child failure;
- raw samples and min/median/mean/max.

After batch 1 measured a 2.658 ms median.

After batch 2 measured a 2.437 ms median.

The after batches differ by 8.31%, passing the predeclared same-host ±20%
tolerance without discarding or selecting samples.

The report explicitly states that this is planning startup, not Zellij, WASM,
provider, or ticket-completion latency. Real-Zellij focused launch and installed
Codex/Claude startup are classified as not deterministically measurable with
the current local harnesses rather than assigned misleading proxies.

### Idle and active footprint

The exact predecessor helper was syntax-checked and executed against the final
release artifacts.

The complete accepted run recorded:

- one unique Zellij session;
- one server PID for both states;
- ten numeric idle samples;
- deterministic local stub launch receipt;
- ten numeric active samples;
- unchanged final server PID;
- `measurement_complete=PASS`;
- final artifact sizes and hashes;
- unique-session teardown.

Raw samples were retained in the report and the count/min/median/max summaries
were independently recomputed.

Every footprint table and conclusion labels the numbers as Zellij host-process
RSS and not Lisa plugin-heap attribution.

### Residual risks

The report explicitly covers:

- no live installed-provider dogfood;
- no focused deterministic Zellij/provider launch latency;
- RSS non-attribution and residency variability;
- host/toolchain/input specificity;
- one standard ignored integration that was separately run;
- pre-existing broader non-canonical test-only Clippy lints;
- future-provider trait-default care;
- structurally deferred scheduler/hook/harness cleanup.

Each risk is non-blocking inside the ticket's stated deterministic-local scope.

### Named repetition left alone

The report lists C-05 through C-14 individually with the approved rationale:

- signal scanners;
- scheduler failure/reclaim paths;
- timeout/liveness loops;
- atomic publication paths;
- maintained-harness helper families;
- historical admitted harness evidence;
- lifecycle-hook generation and merge enumerations;
- scheduler test fixture construction;
- provider assignment/reuse construction;
- independent adapter compatibility assertions.

C-10 and C-14 are described as intentionally retained evidence rather than
undifferentiated technical debt.

### Exact reproduction commands

The report contains commands adjacent to each measurement and a consolidated
command index for:

- final release build;
- logical byte counts;
- artifact hashes;
- two planning-startup batches;
- idle/active RSS observation;
- fmt, native Clippy, WASM Clippy, and canonical checks;
- manually executed ignored integration;
- both deterministic dogfood fixtures;
- optional installed-provider field harness paths explicitly marked unrun.

Fixture wall observations are accompanied by `/usr/bin/time -p` command forms
and explicitly distinguished from startup latency.

## Change summary

No maintained product source changed.

No maintained test or fixture source changed.

No Cargo manifest, lockfile, build script, release profile, CI workflow, ticket
frontmatter, or shared work file was manually changed by this attempt.

No file was deleted.

The attempt created six private required artifacts:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `progress.md`;
- `review.md`.

Generated release files under `target/` were refreshed and remain ignored.

The RSS helper created and removed external disposable fixtures and unique
Zellij sessions. No measurement process or session from the accepted run
remained after cleanup.

Lisa automatically admitted completed phase artifacts to the shared work path
and advanced ticket phase. This attempt did not write those shared files or
edit phase/status directly.

## Measurement validation

### Release build and identity

`just build-cli` passed. The plugin was built before the CLI embedding step.

`file` identified the native CLI as an arm64 Mach-O executable and the plugin
as WebAssembly.

`wc -c` and `shasum -a 256` matched T-038-04-01's fresh dogfood receipts.

### Size arithmetic

Independent calculations confirmed:

```text
CLI:  0 bytes, 0.000000%
WASM: -1,526 bytes, -0.107907%
```

### Timing arithmetic

Both 30-sample batches completed successfully. Independent calculations
confirmed:

```text
after rerun difference=8.314522% (PASS against ±20%)
before-to-after primary delta=-0.049 ms (-1.810122%)
```

### Footprint arithmetic

Independent sorting confirmed:

```text
idle count=10 min=80,944 median=80,952 max=80,960 KiB
active count=10 min=81,072 median=81,104 max=81,104 KiB
active-minus-idle median=+152 KiB
```

The report preserves raw values so a reviewer can recompute the summaries.

## Test and gate coverage

This ticket introduced no source behavior, so it did not rerun the entire gate
suite after collecting documentation-only observations.

The final source predecessor recorded:

- `cargo fmt --all -- --check`: pass;
- `cargo clippy --workspace -- -D warnings`: pass;
- `cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`: pass;
- `just check`: pass;
- workspace tests: 725 passed, 0 failed, 1 ignored;
- ignored real-Zellij integration explicitly run: 1 passed, 0 failed;
- optimized release WASM build: pass.

The immediately following dogfood predecessor changed no source and recorded:

- atomic provider contract: PASS;
- real-Zellij delivery boundary: PASS;
- success, suppressed-start, suppressed-ack, and dquote scenarios: PASS;
- pre/post artifact hashes: identical.

The current ticket's fresh `just build-cli` additionally reconfirmed both
release compilation paths and artifact identity.

This is proportionate coverage for an evidence-only aggregator on an unchanged
source tree.

## Implementation deviations

The product/report scope did not deviate from Plan.

Two RSS helper invocations ran beyond the command runner's initial 30-second
capture window without their session handles being retained. Their partial
output was excluded from the report.

The final invocation retained the session handle and was polled through all
twenty samples and the PASS marker.

After that helper completed, an outer diagnostic wrapper used zsh's reserved
read-only variable `status`, causing only the wrapper to exit nonzero. The
helper had already printed its successful final identity and PASS receipt and
had cleaned up. This is transparently documented and does not invalidate the
measurement.

No source fix or measurement-method modification followed either capture issue.

## Commit and ownership review

Meaningful ticket-owned source units: zero.

`lisa commit-ticket` transactions: zero, intentionally.

An empty, artifact-only, or generated-output commit would violate the source
transaction boundary rather than improve durability; Lisa owns artifact
publication in its final completion transaction.

No ordinary `git add`, broad add, or ordinary `git commit` was used.

Final audit:

- ordinary index: empty;
- ticket-owned staged source: none;
- ticket-owned modified source: none;
- ticket-owned untracked source: none;
- `.lisa/provenance.jsonl`: Lisa-managed modification, preserved;
- active ticket frontmatter: Lisa-managed phase change, preserved;
- shared work files: Lisa-admitted publication inputs, not directly written.

## Open concerns and limitations

The exact RSS helper command currently points to the retained predecessor
attempt path. It is exact and executable in this checkout, and the report also
documents the method and raw evidence. If Lisa later adopts attempt-directory
garbage collection, a future release-measurement epic should promote a
portable, platform-aware footprint harness only after its maintenance value is
demonstrated. This does not block the present recorded observation.

The report is intentionally longer than a typical phase artifact because it is
the acceptance-facing aggregate of a multi-ticket release pass. Its top
scorecard and command index keep the operational information accessible.

No live provider conclusion can be inferred from deterministic shell stubs.

No exact plugin memory conclusion can be inferred from Zellij server RSS.

No native numeric value can be assumed for another OS, architecture, toolchain,
linker, checkout, DAG input, or host-load state.

The broader non-canonical Clippy observation and deferred repetition list remain
future scope, not hidden failures.

## Critical issues

None.

## Final assessment

The release candidate has an honest, reproducible readiness record:

- artifact sizes are exact and bound to dogfooded hashes;
- deterministic planning startup shows no material regression;
- footprint observations are paired, raw, repeatable, and correctly caveated;
- canonical formatting, lint, test, WASM, and fixture gates are green;
- deferred repetition and residual risks are visible rather than erased;
- the product tree remained unchanged during reporting;
- all ticket-owned source state is clean.

Review is complete. This attempt remains on `T-038-04-02`.

Lisa must admit this Review, prepare the final completion commit, publish Done,
and release the seat. This attempt must not start another ticket or perform
completion itself.
