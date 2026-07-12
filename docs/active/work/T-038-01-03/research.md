# Research — T-038-01-03 caveated-memory-footprint-observations

## Ticket boundary

This ticket records a pre-tightening memory-footprint observation for E-038.
Its only acceptance criterion asks for idle and active observations, the method
used to obtain them, and an explicit warning that the values are host-process
RSS rather than attribution to Lisa's plugin heap.

The parent story, `S-038-01`, defines this as a measurement-only slice. It says
the three sibling tickets write only their own work artifacts, touch no source,
and do not change scheduler, adapter, or provider contracts. This ticket is the
memory member of that fan-out. Size belongs to T-038-01-01 and timing belongs to
T-038-01-02.

The story also distinguishes the current “before” snapshot from the later
“after” measurement and aggregated release report. Those later deliverables
belong to S-038-04. This attempt therefore must preserve the current code and
record evidence that the later report can compare honestly.

## Relevant runtime boundary

Lisa has two built release artifacts: the native `lisa` CLI and the
`lisa-plugin` WASM module embedded by the CLI. `lisa loop` generates a Zellij
layout and launches Zellij. Zellij loads and executes the WASM plugin inside a
Zellij host process; the plugin is not exposed as an independently measurable
native process with its own RSS row.

On this macOS host, `ps -o rss=` reports resident set size in KiB for a process.
The independently visible process for a Lisa session is the Zellij server,
whose command has the form `zellij --server <session-socket-path>`. Any RSS
value read from that PID covers the whole Zellij server process: Zellij itself,
terminal panes and screen state held by the server, WASM runtime machinery,
Lisa plugin state, allocator effects, shared pages as accounted by the OS, and
other host-owned allocations.

The native `lisa` launcher is not a suitable proxy for plugin memory. It prepares
the layout and then starts Zellij; the durable in-session state is hosted by
Zellij. Provider processes such as `codex` or `claude` are separate processes
and must not be summed into the Zellij host observation. Doing so would turn the
result into a process-tree measurement with a different meaning.

## Existing reproducible harness

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` is the
repository's deterministic, model-free real-Zellij harness. The Rust integration
test `real_zellij_delivery_boundary` invokes it with a freshly built CLI. The
shell harness creates external temporary repositories, installs a stub provider,
starts uniquely named Zellij sessions, drives Lisa through startup and assignment
delivery states, and tears each session down.

The harness's success case crosses the same scheduler and host boundaries needed
for an active observation without launching an authenticated paid model. Its
stub emits deterministic started and acknowledgement signals. That makes it a
better active workload than the live-provider harness for a baseline: it avoids
quota, network, model latency, and provider-process allocation as uncontrolled
variables.

`crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the canonical
fresh-build wrapper. In safe preparation mode it builds release WASM first and
release CLI second, records hashes and tool versions, and runs the deterministic
real-Zellij regression. Its full mode launches authenticated providers and is
explicitly metered. This memory ticket does not need a metered provider run.

The live runbook also states that observations from an already-running parent
loop are not substitutes for a fresh isolated control. The current repository is
itself running inside a parent Lisa/Zellij loop with three Codex workers. That
server is contaminated by unrelated panes, sessions, terminal histories, and
provider activity, so its RSS cannot serve as the ticket baseline.

## Observable states

For this ticket, “idle” needs to mean a freshly started isolated Zellij server
with the Lisa plugin loaded and no ticket-owned provider work in flight. It does
not mean an empty operating-system process: Zellij, its pane graph, the WASM
runtime, the loaded plugin, and normal polling are all resident.

“Active” needs to mean the same isolated server while a deterministic Lisa
assignment is in flight. The useful active boundary is after the ticket has
been scheduled and the stub/provider delivery path is running. Comparing two
states of the same server avoids conflating different session layouts or server
startup histories.

A single instantaneous sample is vulnerable to scheduler timing. A short series
at a fixed interval is more descriptive. Reporting the sample series plus a
simple summary (for example median and observed range) exposes variability
instead of presenting one lucky sample as precision.

## Host and tool facts observed before measurement

- Operating system: macOS/Darwin 25.5.0 on arm64.
- Physical memory reported by `sysctl -n hw.memsize`: 25,769,803,776 bytes.
- Zellij: 0.44.3 at `/opt/homebrew/bin/zellij`.
- Lisa CLI selected by the repository build: `target/release/lisa`.
- Native RSS source: `/bin/ps` with `rss=` output in KiB.
- Existing parent server RSS was deliberately rejected as evidence because the
  parent has multiple unrelated agent panes.

These environment facts matter because RSS accounting and runtime versions can
change values. The result is a field observation on this host, not a portable
memory requirement or a deterministic byte-for-byte test.

## Measurement constraints

RSS is page-residency accounting, not allocation ownership. It can vary with
OS memory pressure, shared-library accounting, allocator retention, terminal
history, JIT/runtime behavior, and sampling time. Even the difference between
active and idle host RSS cannot be assigned wholly to Lisa: Zellij and its WASM
runtime also react to pane output and plugin callbacks.

The measurement must identify the server PID from the unique isolated session,
not by selecting the first `zellij --server` process. This host can run several
sessions concurrently. The command and evidence must retain the session name,
PID, exact `ps` fields, source revision, binary identity, and versions.

The release artifacts need to be freshly built before the observation. Build
activity itself should finish before sampling because compiler and linker
processes do not belong in the Zellij server RSS value. A post-build isolated
session is the appropriate boundary.

The worktree currently contains unrelated Lisa-managed modifications to the
three S-038-01 ticket frontmatters. They are workflow state owned by Lisa and
must not be staged, reverted, or included by this ticket. The attempt-private
artifact directory is the only write boundary for this measurement.

## Artifact conventions

RDSPI requires six phase artifacts under this attempt-private work directory.
Lisa later publishes admitted artifacts to `docs/active/work/T-038-01-03/`.
The ticket and workflow prohibit manually editing phase/status fields or writing
directly to that shared publication path.

Because this is an artifacts-only ticket, the Implement phase can consist of
running and recording the observation and writing `progress.md`. There is no
ticket-owned product source unit to commit with `lisa commit-ticket`. The final
review must say this explicitly and verify that no source file was left staged,
modified, or untracked by this attempt.

## Research conclusions

The codebase already provides the right deterministic active workload and the
right fresh-build path, but not a plugin-specific heap meter. The honest unit of
observation available from the host is the isolated Zellij server's RSS in KiB.
Idle and active must be named operational states of that server, and every table
or summary containing the numbers must repeat the non-attribution caveat inline.

No source modification is supported by the ticket or story. The deliverable is
a reproducible evidence artifact whose command, identities, raw samples, and
limitations allow S-038-04 to compare the later “after” run without turning a
host-level field observation into a false plugin-memory claim.
