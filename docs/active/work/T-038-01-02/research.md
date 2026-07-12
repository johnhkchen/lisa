# Research — T-038-01-02 startup-launch-timing-baseline

## Ticket boundary

- The ticket asks for a reproducible startup/launch timing baseline.
- A numeric figure is required only where timing is deterministic in this environment.
- Every recorded figure must carry the exact command or harness that produced it.
- A deterministic figure must reproduce within an explicitly stated tolerance.
- A path that cannot be measured deterministically must be named as such rather than assigned a misleading number.
- The parent story is a measurement-only slice.
- The story explicitly limits writes to this ticket's work artifacts.
- Production code, scheduler behavior, provider contracts, dependencies, and public CLI behavior are out of scope.
- The measurement is a pre-tightening “before” snapshot of the post-E-037 release candidate.

## Release-candidate identity

- The repository HEAD observed during research is `51e45e09b4dabc35ff79adea041d50bbdfbe791c`.
- The release CLI reports `lisa 0.4.0-rc.6`.
- The host is macOS 26.5.2, build 25F84.
- Measurements on another host, OS, filesystem, CPU power state, or toolchain are not expected to match exactly.
- Reproduction tolerance therefore applies to reruns on the same host and checkout under comparable load.

## Build and embedding path

- `just build-cli` is the repository's canonical release CLI build recipe.
- It first builds `lisa-plugin` for `wasm32-wasip1` in release mode.
- It touches the release WASM so the CLI build script refreshes its embedded input.
- It then builds `lisa-cli` in release mode.
- The resulting executable is `target/release/lisa`.
- The resulting WASM is `target/wasm32-wasip1/release/lisa.wasm`.
- `crates/lisa-cli/build.rs` supplies the WASM to the CLI build.
- Startup measurements must use the release binary, not `cargo run`, because Cargo startup and compilation noise are not product startup.

## CLI startup boundary

- `lisa loop --dry-run` exercises real release CLI process startup.
- Clap argument parsing and Lisa configuration resolution occur before `loop_cmd::run_loop`.
- `run_loop` validates the project has `CLAUDE.md` and the configured ticket directory.
- In dry-run mode it calls `run_dry` before provider dependency checks, trust updates, WASM extraction, layout writes, or Zellij execution.
- `run_dry` scans ticket markdown, parses ticket metadata, builds the DAG, and prints the schedule summary.
- The repository itself is a valid, readily available input fixture for this path.
- Redirecting stdout and stderr removes terminal rendering from the measured boundary while retaining output generation.
- The repository's active ticket set is part of the input and must be held fixed for comparison.

## Real local launch boundary

- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` is the committed deterministic, model-free launch harness.
- It requires a freshly built Lisa CLI through `LISA_BIN`.
- It creates isolated temporary Git projects.
- It substitutes a local shell stub for the provider.
- It launches real Zellij and a real Lisa WASM plugin.
- It verifies the bounded bare-provider launch reaches the stub.
- It separately gates process-start evidence and assignment acknowledgement.
- It covers success, suppressed startup, suppressed acknowledgement, and real-zsh `dquote>` recovery.
- The harness uses polling at 250 ms and scheduler/plugin polling boundaries.
- Several scenarios intentionally wait through bounded recovery deadlines and fixed seven-second non-retry observation windows.
- Its full wall time is therefore primarily a regression-suite duration, not a focused startup latency.
- Its event log records launch identity but not a monotonic timestamp for the loop-start-to-launch interval.
- The success scenario waits for launch with a 15-second bound, but a bound is not an observed latency.
- Treating total harness duration as launch latency would conflate startup, test orchestration, deliberate fault deadlines, and sleeps.

## Installed-provider paths

- `crates/lisa-cli/tests/fixtures/live_provider_startup.sh` is the installed-provider harness.
- It covers Codex-first and Claude-first in separate isolated projects and Zellij sessions.
- It samples dashboard and lifecycle state at 250 ms intervals.
- It records UTC timestamps for first-observed states.
- Codex and Claude follow intentionally different truthful bootstrap contracts.
- Grace-mode Codex waits through the named scheduler startup grace and moves from `starting` directly to `delivering`.
- Claude reaches `ready-for-assignment` through positive `SessionStart[startup]` evidence before delivery.
- Both become owned only after matching prompt acknowledgement.
- The harness launches authenticated external provider clients.
- Provider startup includes executable cold/warm state, local client configuration, authentication state, hook initialization, service/network behavior, and provider response behavior.
- Completion includes model inference and consumes quota or may incur charges.
- The previous ticket's review states that the metered live run was explicitly deferred and did not manufacture a PASS.
- No live-provider run is authorized by this measurement ticket.
- Even with authorization, those elapsed values would be field observations, not deterministic local baselines.

## Timing facilities available

- `hyperfine` is not installed on this host.
- `/usr/bin/time` is installed but its portable `-p` output is too coarse for a short CLI path.
- Ruby is available as a host scripting runtime and exposes `Process.clock_gettime(Process::CLOCK_MONOTONIC)`.
- A single Ruby process can warm the executable, execute repeated samples, and report raw milliseconds plus summary statistics.
- `system(*cmd, out: File::NULL, err: File::NULL)` avoids shell quoting differences and measures complete child-process wall time.
- A monotonic clock avoids wall-clock adjustments during sampling.

## Sources of variability

- Process creation and dynamic loader state vary slightly between runs.
- Filesystem metadata and file-content caches affect ticket scanning.
- Background CPU activity and power management affect sub-second timings.
- The active-ticket directory can change while concurrent Lisa tickets transition.
- Output size changes if the DAG input changes.
- A warmup phase reduces cold-loader and cold-filesystem effects.
- Multiple samples expose a median and range rather than overclaiming one invocation.
- Two independently invoked batches establish whether the chosen tolerance holds on immediate rerun.

## Measurement semantics

- The deterministic numeric candidate is warm release-CLI dry-run startup against this checkout.
- The measured interval starts immediately before spawning `target/release/lisa`.
- It ends after the child exits successfully.
- It includes process spawn, loader startup, CLI parsing, config resolution, ticket scan/parse, DAG computation, output formatting, and process teardown.
- It excludes compilation.
- It excludes Zellij startup, WASM loading, provider launch, hooks, assignment delivery, and ownership.
- “Warm” means three unrecorded successful invocations precede each recorded batch.
- The primary statistic can be the median of 30 samples.
- Raw samples, minimum, maximum, mean, and median should be retained in the work artifact.

## Determinism boundary

- Dry-run startup is locally reproducible, although not bit-exact in time.
- A percentage tolerance is more meaningful than exact equality.
- The full deterministic real-Zellij harness is reproducible as a pass/fail behavioral fixture, but its total duration is not a startup metric.
- Stub loop-to-launch latency is not emitted by the existing harness and cannot be recovered exactly from its current artifacts.
- Adding instrumentation would modify source/harness behavior and violate this story's artifact-only scope.
- Codex native startup latency is not deterministic here.
- Claude native startup latency is not deterministic here.
- Provider assignment-to-owned and completion latency are also not deterministic here.

## Repository-state constraints

- `docs/active/tickets/T-038-01-01.md` and `docs/active/tickets/T-038-01-02.md` appear modified in the ordinary worktree.
- Those changes are scheduler-owned ticket transitions and must not be touched.
- This ticket owns no source files.
- Phase artifacts belong only under `.lisa/attempts/T-038-01-02/1/work/` during the attempt.
- Lisa will publish admitted artifacts after validating the attempt lease.
- No ordinary Git staging or commit operation is appropriate.
- No `lisa commit-ticket` source transaction is necessary unless an implementation unexpectedly creates a repository-owned measurement file, which the story forbids.

## Research conclusion

- The release CLI dry-run path provides an honest, deterministic-enough numeric startup baseline.
- It should be measured in two warm 30-sample batches with a monotonic clock.
- The exact build and benchmark commands must be captured.
- The accepted tolerance must be checked against the independently invoked batch medians.
- Real-Zellij stub launch should be named as behaviorally deterministic but not numerically instrumented at the required boundary.
- Codex and Claude installed-provider startup should each receive an explicit not-deterministically-measurable note.
- No product or harness source change is justified by this baseline ticket.
