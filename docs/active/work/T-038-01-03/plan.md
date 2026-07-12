# Plan — T-038-01-03 caveated-memory-footprint-observations

## Step 1 — establish source and environment identity

Record the current Git commit, dirty status, UTC timestamp, OS/architecture,
physical memory, Rust/Cargo versions, and Zellij version. Preserve the initial
dirty-path list so unrelated Lisa-managed ticket transitions are not attributed
to this attempt.

Verify that required local tools (`cargo`, `just`, `zellij`, `jq`, `ps`,
`shasum`, and shell utilities) exist.

Success criteria:

- environment identity is complete;
- unrelated pre-existing modifications are named;
- no source file is edited.

## Step 2 — build and identify the release artifacts

Run the repository's canonical release build path (`just build-cli`), which
builds the release WASM before the release CLI embedding step. Record the exact
command, exit status, artifact paths, Lisa version, byte sizes, and SHA-256
hashes.

Do not sample while compilation or linking is active. Finish the build first so
build processes cannot contaminate the host observation.

Success criteria:

- `target/release/lisa` is executable;
- `target/wasm32-wasip1/release/lisa.wasm` exists;
- both artifacts have retained hashes;
- source commit is retained with the identities.

## Step 3 — prepare an isolated deterministic fixture

Create an external temporary repository rather than nesting in the parent Lisa
checkout. Initialize Lisa configuration using the freshly built CLI. Install a
stub provider compatible with the existing lifecycle signal contract. Use one
unique Zellij session name and clear inherited `ZELLIJ`, `ZELLIJ_PANE_ID`, and
`ZELLIJ_SESSION_NAME` for launch.

Arrange the initial DAG so no ticket can be scheduled while retaining the same
session for the later active phase. Start the loop and wait until Zellij actions
and the plugin dashboard respond.

Success criteria:

- fixture is external and disposable;
- session name is unique;
- freshly built CLI/layout identity is confirmed;
- no authenticated provider is launched;
- dashboard is responsive.

## Step 4 — resolve the Zellij host PID

Use the unique session identity to select the corresponding
`zellij --server` process. Record PID, PPID, full command, and the resolution
command. Fail on zero or multiple matches.

Re-check PID identity before each state and after sampling. Do not fall back to
the parent server or the first system-wide Zellij process.

Success criteria:

- exactly one server PID is resolved;
- command contains the fixture session identity/socket;
- PID is distinct from the parent Lisa loop server.

## Step 5 — collect idle samples

Define idle inline as plugin loaded and responsive, with no assignment or stub
provider work in flight. Allow a short settling interval. Collect ten server-PID
RSS samples one second apart with `ps -o rss=`.

Retain the raw integer KiB values and state receipts. Compute minimum, median,
and maximum directly from the raw values.

Success criteria:

- exactly ten numeric samples;
- all refer to the same live server PID;
- no provider process/activity receipt exists;
- summaries independently recompute.

## Step 6 — activate one deterministic assignment

Make one stub-backed ticket schedulable without restarting the Zellij server.
Wait for a positive active-state receipt: assignment scheduled, stub launched,
and delivery held at the configured gate. Confirm the server PID is unchanged.

Fail if a real `codex` or `claude` process belongs to the fixture or if the
active boundary cannot be held long enough for sampling.

Success criteria:

- deterministic ticket identity is visible;
- stub lifecycle event proves active work;
- same server PID remains selected;
- active sampling window is stable.

## Step 7 — collect active samples

Collect ten one-second RSS readings from the unchanged Zellij server PID while
the deterministic assignment remains held in flight. Retain raw integer KiB
values and compute minimum, median, and maximum with the same method as idle.

Label the table and summary **Zellij host process RSS — not Lisa plugin-heap
attribution**. If a median difference is included, label it only as a paired
host-state difference.

Success criteria:

- exactly ten numeric samples;
- active receipt spans the sample window;
- summaries recompute;
- provider process RSS is excluded.

## Step 8 — write and validate `evidence.md`

Write the result, definitions, exact rerunnable method, environment/build
identity, session/PID identity, raw samples, summaries, teardown receipt, and
limitations. Repeat the mandatory caveat next to every numeric result.

Mechanically check sample counts and numeric syntax. Recompute summaries from
the recorded series. Search the artifact for ambiguous phrases such as “plugin
memory,” “heap usage,” or uncaveated “RSS” claims and correct them.

Success criteria:

- both idle and active results are present;
- method is copyable and exact;
- all results are host-process scoped;
- no claim attributes values or differences to the plugin heap.

## Step 9 — teardown and integrity checks

Release the gate if necessary, kill only the uniquely named fixture session,
stop helper processes, and remove the disposable fixture unless retained paths
are explicitly recorded. Confirm the session/server is absent.

Compare Git status with the initial status. Verify no ticket-owned product file
is staged, modified, or untracked. Do not use ordinary `git add`, `git commit`,
or broad index operations. Since this ticket owns no source change, do not call
`lisa commit-ticket` with artificial artifact paths.

Success criteria:

- fixture session is gone;
- no measurement process remains;
- repository changes are limited to workflow-owned state and private artifacts;
- no source commit is needed.

## Step 10 — complete progress and review

Write `progress.md` with executed steps, commands/results, deviations, and the
artifacts-only implementation boundary. Write `review.md` with the baseline
handoff, acceptance mapping, validation coverage, and open limitations.

After `review.md` exists, remain on T-038-01-03. Do not edit ticket phase/status,
publish to the shared work path, start another ticket, or release the seat.

## Testing strategy

No Rust unit or integration suite is required solely for a documentation-only
observation. The deterministic stub fixture is the behavioral verification
surface. The measurement validation is data-oriented: state receipts, fixed
sample counts, stable PID, recomputed summaries, artifact hashes, unique-session
cleanup, and Git integrity.

The later “after” run should reuse the exact method. RSS equality is not an
acceptance criterion because OS residency is variable; method identity and
honest interpretation are the regression value.
