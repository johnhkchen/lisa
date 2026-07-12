# Structure — T-038-01-03 caveated-memory-footprint-observations

## Change boundary

This ticket creates documentation artifacts only inside the attempt-private
work directory. No repository source, test harness, shared runbook, ticket
frontmatter, or published work path is created, modified, or deleted.

## Artifact set

### `research.md`

Maps the story and ticket scope, runtime/process boundary, existing deterministic
and live harnesses, macOS RSS semantics, current-host constraints, workflow
rules, and the reason plugin-heap attribution is unavailable.

### `design.md`

Evaluates parent-session sampling, load deltas, heap instrumentation, process-tree
totals, and isolated host sampling. Selects a paired same-PID idle/active Zellij
server observation with deterministic stub activity and raw fixed-interval
samples.

### `structure.md`

Defines this documentation-only file layout, the evidence schema, execution
boundaries, and phase ordering.

### `plan.md`

Sequences build identification, fixture preparation, idle sampling, active
sampling, evidence validation, cleanup, and final repository-integrity checks.

### `evidence.md`

The ticket's substantive baseline deliverable. It contains the exact rerunnable
command/method, environment and artifact identity, fixture identity, state
definitions, raw RSS samples, summary statistics, and limitations.

Every table heading or result paragraph containing memory numbers includes the
label “Zellij host process RSS — not Lisa plugin-heap attribution.” This keeps
the caveat inline for a report reviewer rather than relying on a distant note.

### `progress.md`

Tracks completion of the plan, commands run, observation status, validation,
deviations, and the no-source-change/no-ticket-commit result.

### `review.md`

Summarizes the recorded baseline, artifact changes, verification coverage,
acceptance mapping, limitations, and repository integrity. It is the handoff to
S-038-04 and the human reviewer.

## Evidence document organization

`evidence.md` has the following sections in order:

1. Result and mandatory attribution caveat.
2. Operational definitions of idle and active.
3. Exact reproduction command.
4. Environment and build identity.
5. Fixture/session/process identity.
6. Raw idle samples and summary.
7. Raw active samples and summary.
8. Paired-state interpretation.
9. Validation and teardown receipts.
10. Limitations and permitted use.

The result begins with the caveat so a reviewer cannot encounter the values
before their meaning. The command precedes the sample tables so reproduction is
part of the baseline, not an appendix.

## Measurement helper boundary

If a helper script is needed to execute the fixture, it remains in the
attempt-private directory and is treated as disposable measurement machinery,
not product source. Its complete invocation is recorded in `evidence.md`.
Prefer adapting the existing deterministic harness interfaces and environment
variables instead of duplicating scheduler behavior.

The helper has these internal responsibilities:

- build or select the fresh release CLI and WASM;
- create an external temporary fixture and unique session;
- install a non-metered stub provider;
- launch Lisa/Zellij without inherited `ZELLIJ*` variables;
- identify exactly one server PID for the session;
- prove idle and active state predicates;
- sample only that server PID;
- compute summaries from raw integer KiB samples;
- kill only its unique session and remove disposable state.

Any helper output used as evidence is transcribed or embedded into `evidence.md`.
The helper itself need not be published if the exact command block in the
evidence is self-contained.

## Interfaces reused

The observation relies on existing public operational interfaces rather than
new code interfaces:

- `just build-cli` for release WASM followed by release CLI;
- `target/release/lisa` as the freshly built native executable;
- `lisa init` and `lisa loop` for fixture/session preparation;
- Zellij CLI session actions for readiness and screen inspection;
- the repository's stub lifecycle signal contract for deterministic activity;
- `/bin/ps -o pid=,ppid=,rss=,command=` for process identity and RSS;
- `shasum -a 256` for artifact identity;
- standard integer sorting/awk for summary statistics.

No Rust module gains a public API. No Cargo manifest or configuration schema
changes.

## State and data flow

The release build produces the CLI and target WASM. The CLI embeds/uses the
release plugin and generates an isolated Zellij layout. Zellij creates a unique
server that hosts the WASM plugin. The measurement resolves that server to one
PID.

In the idle phase, the plugin dashboard is responsive but no assignment is
runnable. Ten `ps` readings for the server PID flow into the idle raw-sample
list and summary.

The fixture then makes one deterministic ticket runnable. Lisa schedules it,
launches a stub provider, and reaches an explicitly held delivery boundary. Ten
readings from the unchanged server PID flow into the active raw-sample list and
summary. Provider process memory is outside both lists.

Finally the gate is released or the session is terminated, the unique session
is killed, and evidence records cleanup and repository state.

## Naming and units

Use `rss_kib` for raw and summary fields. Do not label `ps` output as bytes or
MB. Optional MiB conversions are derived as `KiB / 1024` and displayed only for
readability alongside the authoritative KiB value.

Use `idle` and `active` only with their operational definitions. Do not generalize
active to “peak,” “maximum,” or “production workload.” Do not call the paired
difference “plugin overhead,” “plugin memory,” or “heap growth.”

## Ownership

Ticket-owned files are the seven attempt artifacts listed above. Product source
ownership is empty. Existing modified ticket frontmatters belong to Lisa's
workflow and remain untouched.

RDSPI artifacts are not committed with an ordinary Git index. The assignment
states that Lisa admits and publishes them after lease verification. Because
there are no source files, `lisa commit-ticket --include ...` has no meaningful
unit to commit.

## Ordering

Research, Design, Structure, and Plan are written before executing the
measurement. Implement then runs the build and observation, writes evidence and
progress, validates the numbers and caveats, and performs cleanup. Review is
written only after the evidence is complete and repository integrity is checked.

## Verification surface

There is no product behavior change requiring Cargo unit tests. Verification is
instead evidence-oriented:

- build command succeeds and identities exist;
- one unique server PID is selected;
- PID remains identical across idle and active samples;
- each state has exactly ten numeric RSS values;
- summaries recompute from raw values;
- active gate/state receipt exists;
- every numeric result is inline-caveated;
- no real provider is used;
- unique session is absent after teardown;
- Git status contains no ticket-created source changes.

## Publication boundary

All paths in this blueprint are under
`.lisa/attempts/T-038-01-03/1/work/`. Nothing is written directly to
`docs/active/work/T-038-01-03/`. Lisa alone performs admission, phase changes,
Done publication, and final seat release after `review.md` exists.
