# Design — T-038-01-03 caveated-memory-footprint-observations

## Decision

Record a same-session, host-level RSS observation from a freshly built,
isolated deterministic Lisa/Zellij fixture. Capture a short fixed-interval
sample series in two named states: plugin loaded with no assignment in flight
(idle), and a stub-backed assignment in flight (active). Report raw samples,
median, and range, with every numeric result explicitly labeled **Zellij host
process RSS — not Lisa plugin-heap attribution**.

The measurement will be recorded in an attempt-private `evidence.md`. Product
source remains unchanged. `progress.md` records execution and `review.md`
hands off the evidence and limitations.

## Option 1 — measure the current parent Lisa loop

The easiest available PID is the Zellij server hosting this attempt. Sampling it
would require no fixture setup and could provide a number immediately.

This option is rejected. The parent loop contains the dashboard, three worker
panes, terminal history, and multiple active Codex processes. Although provider
process RSS is separate, the server still owns pane state and output for all of
them. There is no clean idle point, the embedded WASM may predate the ticket's
fresh release build, and sibling activity can change the server throughout the
sample. It is not a reproducible RC baseline.

## Option 2 — report an RSS delta around plugin load

A bare Zellij server could be sampled before and after loading Lisa. Subtracting
the values might appear to isolate the plugin.

This option is rejected because the subtraction does not establish allocation
ownership. Loading a plugin also makes Zellij create panes, instantiate its WASM
runtime, allocate host integration structures, and render output. Shared-page
and allocator effects make the delta especially easy to mislabel. The ticket
asks for idle and active observations, not a pseudo-precise plugin load delta.

## Option 3 — instrument the plugin heap

The plugin or runtime could be changed to expose allocator statistics, or a
profiler could trace WASM allocations.

This option is out of scope. The story explicitly says the precise plugin heap
is out of reach and must not be claimed. Adding instrumentation would mutate the
release candidate before its “before” baseline and violate the measurement-only
scope. It could also change the very memory behavior being measured.

## Option 4 — sum a process tree

The Zellij server, CLI, shell, and provider processes could be summed to describe
the whole operational footprint.

This is a valid different metric, but it is rejected for this ticket. Provider
processes dominate and vary by client version, model interaction, and terminal
state. The ticket language specifically emphasizes host-process RSS caveating;
a tree total would obscure the stable host boundary and make active results
mostly a provider-client observation.

## Option 5 — isolated Zellij host RSS, selected

Use the release CLI/WASM to start one unique external fixture. Identify its
Zellij server PID from the unique session socket/command. Hold the fixture with
the plugin loaded but no schedulable ticket, then sample idle RSS. Introduce or
enable one deterministic stub ticket and sample while delivery is deliberately
held in flight. Use the same server for both states and tear it down afterward.

This approach is selected because it matches the observable boundary, avoids
paid providers and network variability, and lets idle/active share layout,
binary, PID, and startup history. It remains a host observation, which is stated
prominently rather than “corrected” through subtraction.

## State definitions

Idle is defined operationally as:

- a unique isolated Zellij server is running;
- the freshly built embedded Lisa WASM is loaded;
- the dashboard is responsive;
- no ticket is assigned and no stub provider process is running;
- samples begin only after a short settling interval.

Active is defined operationally as:

- the same unique Zellij server and PID remain running;
- one deterministic fixture ticket is assigned;
- the stub provider has launched and assignment delivery is held in flight at a
  known gate long enough to collect the complete sample series;
- provider RSS is not included in the values.

The active state is not intended to simulate peak production load. It is a
repeatable single-assignment field observation suitable for later comparison.

## Sampling design

Collect ten samples per state at one-second intervals using:

```text
ps -o rss= -p "$server_pid"
```

`ps` reports KiB on this Darwin host. Preserve all raw samples. Compute median,
minimum, and maximum without converting through floating-point arithmetic. Ten
samples give a central pair; the reported median may be the arithmetic mean of
those two values and can therefore end in `.5 KiB`.

Do not describe the active-minus-idle difference as plugin memory. If shown for
reviewer convenience, label it only as a difference between two Zellij host RSS
medians under the named fixture states. Raw state values are the primary result.

## Reproducibility record

The evidence must contain:

- UTC timestamp and source commit;
- `uname`, physical-memory, Rust/Cargo, Lisa, and Zellij versions;
- release CLI and target WASM absolute paths, byte sizes, and SHA-256 hashes;
- fixture root, unique session name, and Zellij server PID/command;
- the exact build and observation command or fully copyable command block;
- idle and active state checks;
- ten raw samples for each state;
- median and range summaries;
- teardown result and worktree-integrity check.

The later S-038-04 comparison should rerun the same procedure on the same host
when practical. Exact equality is not promised: RSS is an environmental field
metric. The command makes the method reproducible, while the range communicates
observed short-term variation.

## Fixture choice

Reuse the behavioral shape of `real_zellij_delivery_boundary.sh`: an external
temporary Git repository, a stub provider, unique named Zellij session, and
explicit lifecycle gates. The observation may use an attempt-local disposable
script or a documented shell block; neither becomes product source.

The fixture should start empty for the idle phase, then make one ticket eligible
without restarting Zellij. If runtime ticket discovery cannot reliably add a
ticket after launch, an acceptable equivalent is a pre-existing ticket whose
dependency is initially unresolved, followed by a dependency-state change that
makes it ready. The key invariant is that the same host PID is measured in both
states.

## Failure policy

Fail rather than report numbers if any of these occur:

- release build fails;
- the server PID is ambiguous or changes between states;
- the idle dashboard is not responsive;
- a real provider launches instead of the stub;
- the active gate is not observed;
- fewer than ten valid numeric samples are collected in either state;
- teardown cannot identify the unique session safely.

If the environment prevents a trustworthy observation, record that limitation
instead of substituting the parent loop or manufacturing a value.

## Scope and commit policy

No Rust, shell harness, runbook, ticket, or shared work file is modified. The
phase and evidence artifacts live only under
`.lisa/attempts/T-038-01-03/1/work/`. Since there is no ticket-owned source unit,
there is no `lisa commit-ticket` invocation during Implement. Lisa will publish
admitted work artifacts and handle final completion.

## Rationale summary

The selected design measures what the operating system can actually expose and
names it accurately. Same-PID paired states improve comparability; deterministic
stub activity removes provider cost and network noise; raw series prevent false
precision; and the repeated host-RSS caveat prevents the later release report
from quietly converting this field observation into plugin-heap attribution.
