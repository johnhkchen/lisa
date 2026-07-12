# Implementation progress: deterministic delivery boundary regression

## Baseline

- Ticket `T-035-02-01` entered this pass in `implement`.
- Research, Design, Structure, and Plan artifacts already exist in the attempt-private
  work directory and have been admitted to the shared work directory by Lisa.
- The ticket frontmatter is Lisa-owned and was not edited.
- Repository HEAD at implementation inspection was `a883d46` (`Complete T-035-04-02`).
- The shared worktree already contained unrelated modified and untracked Lisa runtime,
  epic, story, ticket, and hook paths. Those paths are outside this ticket's source
  ownership and will remain untouched.
- The two planned ticket-owned test paths existed as untracked files at inspection:
  `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs` and
  `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`.
- Neither ticket-owned path was present in the ordinary Git index.

## Implemented source unit

- Added an ignored Cargo integration-test wrapper that runs the shell harness with the
  checkout's freshly built `lisa` binary and requires a stable PASS receipt.
- Added a deterministic shell harness that creates a separate temporary Git repository
  and named real-Zellij session for each scenario.
- Added a PATH-injected `claude` stub that implements provider version preflight,
  process-start signaling, bounded assignment receipt, gated normalized acknowledgement,
  suppressed-start behavior, suppressed-ack behavior, and a real zsh `dquote>` fault.
- Added dashboard and terminal evidence gates for ReadyForAssignment, Delivering,
  Owned, startup failure, delivery failure, and same-pane recovery.
- Added bounded launch-count, chat-count, generation-order, assignment-reference, and
  launch-script contract assertions.

## Remaining work

1. Commit the meaningful source unit through `lisa commit-ticket` with the two exact
   repository-relative paths.
2. Run broad regression checks.
3. Confirm ticket-owned source paths are clean and write the Review artifact.

## Deviations

- The first real-Zellij run reached CLI bootstrap but timed out discovering the requested
  session. Zellij 0.44 treats `--session <name> --layout <path>` as adding a layout to an
  already-existing named session rather than creating it. The harness wrapper now
  translates Lisa's production `--layout <path>` invocation to `--session <name>
  --new-session-with-layout <path>`. It also removes inherited `ZELLIJ`,
  `ZELLIJ_PANE_ID`, and `ZELLIJ_SESSION_NAME` values so the isolated client cannot attach
  to the surrounding developer session. Other invocations fail explicitly. This changes
  only test bootstrap and retains the production CLI path.
- The second real-Zellij run created the named session and scheduled the fixture ticket,
  but pane discovery expected a `pane_type` JSON field. Zellij 0.44 exposes `is_plugin`,
  `plugin_url`, and `title` instead, and terminal/plugin numeric IDs may overlap. Pane
  discovery now uses the observed schema, excludes the compact-bar plugin, selects the
  assigned terminal by ticket title, and retains explicit `plugin_<id>`/`terminal_<id>`
  prefixes for screen dumps.
- The first run with the current embedded WASM proved the bounded launcher but revealed
  that the pane's interactive zsh sourced the developer's `.zshrc`, which prepended an
  installed provider ahead of the injected stub. The failed named session was killed and
  no fixture process remained. Fixture startup now assigns disposable `HOME` and
  `ZDOTDIR` directories while retaining the explicit fixture-first PATH, preventing user
  shell configuration from selecting a real provider. This isolation is required for the
  test's no-model-token contract.
- With shell isolation active, the stub published process-start evidence and accepted the
  bounded assignment, proving the no-token route. Zellij inserted a transient plugin while
  the session initialized and renumbered plugin IDs after initial discovery, so dashboard
  polling continued to dump the compact bar. Dashboard, terminal, and negative ownership
  assertions now refresh pane identities before every observation and follow the current
  Lisa file-plugin ID.
- Zellij 0.44 still returned an empty screen when `dump-screen --pane-id plugin_<id>`
  targeted the correctly refreshed Lisa plugin, despite the dashboard being rendered in
  the session stream. Plugin observation now focuses the explicit plugin ID and dumps the
  focused viewport. Terminal dumps remain explicitly pane-scoped, and all delivery writes
  remain production plugin actions against the assigned terminal ID.
- Focused plugin dumps exposed a timing issue: the stub published start while Zellij and
  its transient initialization plugin were still settling, allowing the scheduler to pass
  through ReadyForAssignment and exhaust chat retries before observation began.
  Positive-start scenarios now hold start publication behind a fixture gate. The harness
  opens that gate only after confirming the expected launch (or same-pane replacement),
  so ReadyForAssignment remains observable at the real scheduler boundary.
- ReadyForAssignment lasts exactly one scheduler tick, so one-second harness polling can
  still alias past it. Observation polling is now 100 ms. The acknowledgement deadline is
  five seconds rather than the minimum one second so Delivering is stably observable while
  both negative scenarios remain bounded by their existing wall-clock limits.
- Real rendering still did not reliably expose the single-tick Ready label. The harness
  now uses stronger boundary evidence: Lisa removes `pane-0.started` only while scanning
  the exact current lease, and production cannot emit the later bounded chat unless that
  scan moved the seat through ReadyForAssignment. Each positive scenario requires this
  consumption, asserts non-ownership, then requires the chat event and visible Delivering
  state. This avoids treating a transient render frame as the authoritative transition.
- The focused plugin dump showed only the thread-table header because the inherited PTY
  was 80x24, leaving the 30% dashboard just five content rows and clipping the actual
  state row. The isolated client runner now establishes a 140x50 PTY before launching
  Zellij, preserving production layout ratios while making status rows observable.
- With the larger PTY, the success scenario passed through start consumption, Delivering,
  gated acknowledgement, and Owned. Suppressed-start then produced exactly two same-pane
  launches and a bounded failure. Once failure releases the seat, the transient
  `startup-failed`/`delivery-failed` row is replaced by the durable `FAILED T-STUB-01`
  alert, so negative completion waits now match that durable alert while retaining
  scenario-specific event counts and non-ownership checks.
- Repeated `--full` plugin dumps accumulated thousands of scrollback lines and delayed the
  suppressed-start observation until just after its original bound. All required evidence
  is present in the current viewport, so dumps no longer request scrollback, polling is
  250 ms, and negative alert waits allow 75/60 seconds. Separate post-failure stability
  checks still enforce exactly two startup launches and exactly two chat deliveries.
- The durable suppressed-start alert was present, but pane refresh rejected it after the
  failed assigned terminal had been removed: discovery required a focused terminal while
  the dashboard plugin was focused. Discovery still prefers the ticket-titled terminal,
  but now falls back to any remaining terminal solely to keep dashboard observation valid
  after seat release. Active and `dquote>` observations continue to select by ticket title.

## Verification completed

- `bash -n crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh` passed.
- `cargo fmt --all -- --check` passed before the real-Zellij debugging cycle; no Rust
  source changed afterward.
- `cargo test -p lisa-cli --test real_zellij_delivery_boundary --no-run` passed.
- Rebuilt the current production plugin with
  `cargo build -p lisa-plugin --target wasm32-wasip1 --release`, refreshed the existing
  CLI embed input timestamp, and rebuilt the integration target.
- `cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture`
  passed all four scenarios in 125.19 seconds with no residual named session.
- The passing run required no model provider and used only the isolated local `claude`
  stub under a disposable HOME and fixture-first PATH.
- Success proved exact start-signal consumption, bounded assignment receipt, visible
  Delivering without ownership, gated matching acknowledgement, and visible Owned.
- Suppressed start proved exactly two increasing generations in the same physical pane,
  no chat, no acknowledgement, bounded failure, and no third launch.
- Suppressed acknowledgement proved one launch, exactly two chat deliveries, no ack,
  no ownership, bounded failure, and no third delivery.
- `dquote>` proved a real zsh continuation prompt, exactly one same-pane replacement,
  replacement-only process start and chat, gated ownership, and no third launch.
- Committed the two ticket-owned source paths through `lisa commit-ticket` as
  `ad8d5915a8cc10260ce690e171fd444c044d4cd1` with message
  `test(cli): cover real Zellij delivery boundary`.
- `cargo test --workspace` passed after the ticket commit.
- `cargo check -p lisa-plugin --target wasm32-wasip1` passed after the ticket commit.
- Final `git diff --check` passed.
- `git show` confirms the ticket commit contains exactly the two intended test paths.
- Both ticket-owned source paths are clean, tracked, and absent from the ordinary index.

## Implementation status

Implement is complete. No ticket-owned source work remains. Review follows immediately.
