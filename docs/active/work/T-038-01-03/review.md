# Review — T-038-01-03 caveated-memory-footprint-observations

## Outcome

Acceptance is met with a reproducible, same-server idle/active field observation
of the freshly built RC. The result is deliberately and repeatedly scoped as
**Zellij host-process RSS, NOT Lisa plugin-heap attribution**:

- Idle median: **81,416 KiB (79.51 MiB)**, observed range
  81,408–81,424 KiB.
- Active median: **81,568 KiB (79.66 MiB)**, observed range
  81,552–81,568 KiB.

These values and their +152 KiB paired difference are host-process observations.
They include Zellij, pane/screen state, WASM runtime machinery, allocator and OS
residency effects, and the Lisa plugin. They do not attribute bytes or change to
Lisa's plugin heap.

## What changed

Created in the private attempt work directory:

- `research.md` — codebase, process boundary, harness, and measurement constraints.
- `design.md` — evaluated alternatives and selected isolated same-PID sampling.
- `structure.md` — documentation/evidence boundaries and schema.
- `plan.md` — ordered build, fixture, sample, validation, and teardown steps.
- `measure-host-rss.sh` — exact disposable deterministic measurement helper.
- `measurement-raw.txt` — complete machine-readable run output and receipts.
- `evidence.md` — reviewer-facing baseline with command, identities, samples,
  statistics, and limitations.
- `progress.md` — execution record and workflow/commit status.
- `review.md` — this handoff.

Modified/deleted product files: none. No Rust, production shell harness, Cargo
configuration, scheduler/adapter/provider contract, shared runbook, or ticket
content was changed by this attempt.

## Method reviewed

`just build-cli` freshly built release WASM and then the release CLI. The helper
created an external temporary Git fixture, initialized it with that CLI, and
launched a unique Zellij session with a deterministic local Claude stub. It
resolved the session's one `zellij --server` PID from the full process command.

Idle sampling occurred after a settle with the plugin responsive, ticket blocked,
and stub absent. The helper changed the disposable ticket to open; Lisa reloaded
it and the stub recorded launch. Active sampling occurred while that stub was
held before start/ack. Both states used PID 83528. Ten `/bin/ps -o rss=` samples
were collected one second apart in each state. The unique session and fixture
were removed afterward.

This is a deterministic local scheduler/host observation; it did not launch an
authenticated provider, consume model quota, or include provider process RSS.

## Artifact and environment identity

- Source head captured at measurement: `51e45e09b4dabc35ff79adea041d50bbdfbe791c`.
- Lisa: `0.4.0-rc.6`.
- CLI SHA-256: `21364a09ca9f0b010475856c995069dd093f06c930682857c21abc40e4373449`.
- WASM SHA-256: `14db37eed0fbde7507bf6da45be5edaf9b17803c6e6ee300875b68b15761c57c`.
- Zellij: `0.44.3`.
- Host: Darwin 25.5.0 arm64, 25,769,803,776 physical-memory bytes.

Sibling T-038-01-02 completed concurrently after capture, advancing repository
HEAD to `419ed22`. That sibling completion is documentation/workflow state, not
a binary source mutation; the release identities are retained so the observed
artifact is unambiguous.

## Verification coverage

Passed:

- release WASM + CLI build;
- helper `bash -n` syntax check;
- unique-session server resolution;
- dashboard idle predicate;
- absence of stub work during idle;
- positive deterministic stub-launch receipt during active;
- unchanged server PID across both states;
- 10 numeric idle and 10 numeric active samples;
- independently recomputed min/median/max values;
- final server identity check;
- `measurement_complete=PASS` teardown receipt;
- follow-up process search found no measurement server/helper;
- inline caveats accompany all reviewer-facing numeric results.

No Cargo unit test was needed because product behavior did not change. The
fixture exercises real Lisa/Zellij scheduling with a model-free stub and serves
as the observation's behavioral validation.

## Acceptance mapping

The acceptance criterion asks for idle and active observations, explicitly
labeled as host-process RSS rather than plugin-heap attribution, with the method
noted inline.

- Idle observation: present in `evidence.md` with raw samples, range, median,
  state definition, and inline Zellij-host/non-attribution label.
- Active observation: present with raw samples, range, median, positive stub
  activity receipt, and the same inline label.
- Method: exact build/invocation command, sampling primitive, helper, artifact
  hashes, session/PID identity, and summary recomputation are retained.
- Caveat: repeated at the result table, both sample tables and summaries, paired
  difference, progress summary, and this review.

## Open concerns and limitations

RSS is a field metric, not deterministic allocation telemetry. Exact values can
vary with OS memory pressure, shared-page accounting, allocator retention,
terminal history, and tool/runtime versions. The ten-sample range describes
short-term variation only.

The active fixture is one held deterministic assignment, not peak production
load. The result excludes provider process memory and should not be used as a
whole-application process-tree footprint.

The paired +152 KiB difference cannot be assigned to Lisa. Zellij rendering,
pane state, the WASM host/runtime, and page residency all change at this boundary.
Any later report must preserve that caveat and should compare runs using the
same helper, host, versions, and active-state definition where practical.

No critical issue requires human intervention. The principal review risk is
semantic: downstream prose must not shorten “Zellij host-process RSS observation”
to “Lisa memory usage” or “plugin heap.”

## Repository and workflow integrity

No ticket-owned source change exists, so no `lisa commit-ticket` unit was needed.
No ordinary Git index or commit command was used. Pre-existing/concurrent
Lisa-managed ticket and provenance changes were preserved. Lisa also materialized
admitted artifacts under the shared work path during phase advancement; this
attempt did not write there directly and leaves final admission/commit to Lisa.

Ticket phase/status was not manually edited. After this `review.md`, the attempt
remains on T-038-01-03 and does not start another ticket, publish Done, or release
the seat.
