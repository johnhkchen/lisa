# Review — T-045-02-02 zellij-injects-launcher-only

## Disposition

Pass.

The plugin now launches each fresh Codex ticket through the Lisa-owned native launcher
with the exact immutable assignment-file path.
The Zellij pane receives only the bounded atomic launch-script invocation.
The full assignment body remains in the private nonce-bearing attempt file.
A two-ticket native stub-pane fixture proves distinct fresh launch transport per ticket.
The prerequisite CLI fixture proves the path remains one uninterpolated Codex argv element.

All enabled tests and the WASM check pass.
Ticket-owned source is committed and clean.
No critical issue blocks completion.

## Ticket commit

The isolated ticket commit is:

```text
5f02b0f2682608fad4dc6779e3387b23f73ef6a8
feat(plugin): launch Codex with exact assignment reference
```

It contains exactly:

- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/lib.rs`.

It was created with:

```text
lisa commit-ticket \
  --ticket-id T-045-02-02 \
  --message "feat(plugin): launch Codex with exact assignment reference" \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/lib.rs
```

No ordinary `git add`, `git commit`, broad staging, or broad include was used.
`git show --check 5f02b0f` passes.
Both source paths are clean relative to HEAD.
The ordinary Git index is empty.

## What changed — adapter contract

`AgentAdapter::launch_command` now accepts two inputs:

- the existing `SpawnContext`;
- an already-published pane-addressable assignment `&Path`.

The assignment path is required rather than optional.
This moves the interface boundary from “construct some fresh provider command” to
“construct a fresh provider command for this exact durable assignment.”

The change prevents a caller from launching before assignment publication.
It also avoids weaker alternatives:

- scanning an attempt directory;
- reconstructing a nonce-bearing filename;
- selecting a newest assignment;
- passing an implicit stale environment value.

The adapter still performs no filesystem or process I/O.
It returns a command description to the WASM scheduler as before.

## What changed — Claude compatibility

`ClaudeCodeAdapter` accepts the assignment path as an intentionally unused argument.
It still returns the exact existing `build_claude_command` result.

No change was made to Claude's:

- binary or flags;
- `LISA_*` environment shape;
- routed model handling;
- `SessionStart` readiness;
- `/clear` reset handshake;
- reuse prompt;
- signal capabilities;
- follow-up delivery;
- ownership behavior.

The exact existing Claude command unit tests remain green after the interface change.
This meets the ticket's “Claude injection path untouched” constraint.

## What changed — Codex command construction

`CodexAdapter` no longer describes a direct `codex` child invocation.
It describes a Lisa native launcher invocation with this logical shape:

```text
LISA_BIN=<lisa> \
LISA_AGENT_CLIENT=codex \
LISA_PANE_ID=<pane> \
LISA_TICKET_ID=<ticket> \
LISA_ATTEMPT_ID=<attempt> \
<lisa> launch-codex [--model <model>] -- <assignment-path> || <error marker>
```

The same resolved Lisa binary is used as:

- the `LISA_BIN` value inherited by lifecycle hooks;
- the executable that handles `launch-codex`.

The binary, ticket, optional model, and assignment path are shell-quoted with the
existing single-quote helper.
The fixed `--` separator protects an assignment path beginning with `-`.
Only the path appears after the separator.

The full assignment body is not read during command construction.
It is not embedded in a sentence.
It is not pasted into the pane composer.
Direct Codex safety flags no longer appear in the WASM-generated shell description.

The prerequisite native command remains responsible for constructing:

- `--dangerously-bypass-approvals-and-sandbox`;
- `--dangerously-bypass-hook-trust`;
- optional model flag and exact value;
- `--`;
- the assignment path as one `OsString`.

This leaves shell parsing only on the bounded plugin-to-Lisa edge.
The Lisa-to-Codex edge remains native `std::process::Command` argv.

The existing `pane-<id>.error` fallback remains.
A missing launcher, rejected assignment path, or nonzero child result still becomes
operator-visible through the normalized error signal.

## What changed — scheduler path flow

All fresh-launch paths now retain the exact `AssignmentRef` returned by
`State::prepare_assignment`.
They translate its path through `strip_host_prefix` for the pane's host view.
They pass that translated path to `adapter.launch_command`.

The converted production paths are:

1. first launch into an empty pane;
2. prepared launch after cross-provider recycling;
3. prepared `ExitThenFresh` Codex launch;
4. the generic `FreshExec` adapter branch;
5. same-pane startup recovery relaunch;
6. launch after an old TUI exits and the shell grace elapses.

Every one of those paths already published assignment bytes before constructing a launch.
The change retains the returned reference instead of discarding it after success.
Existing error branches, lease revocation, activity logging, and retry control remain.

The stored `State::assignment_refs` map is unchanged.
It still retains the same lease/nonce/path reference for claim and later evidence checks.
No new assignment identity source was introduced.

## What changed — Zellij boundary

`State::prepare_fresh_launch` remains the atomic pane-transport boundary.
It writes the bounded Lisa launcher line to:

```text
<attempt-work-dir>/.lisa-launch-<pane>.sh
```

Zellij receives only:

```text
sh '<attempt-work-dir>/.lisa-launch-<pane>.sh'
```

This preserves the existing protection against PTY size limits and partial launch strings.
The launch script is atomically published before its path is injected.
The assignment body is independently atomically published under its nonce-bearing filename.

The two files have separate responsibilities:

- assignment Markdown: complete agent instructions;
- launch shell script: lifecycle environment plus exact path reference.

Neither the pane input nor the launch script contains the assignment body.

## Fresh TUI per ticket

`CodexAdapter::reset_strategy` remains `ExitThenFresh`.
No Codex `/clear` reuse path was introduced.

For an empty slot, scheduling submits one new launch script immediately.
For a resident Codex slot, scheduling submits `/exit`, waits for the bounded shell-return
transition, then submits the newly assigned ticket's own launch script.

Each `launch-codex` execution starts a new interactive Codex child through
`Command::status` with inherited terminal streams.
Each ticket has its own attempt lease, nonce-bearing assignment file, and launch script.

The ticket did not change ticket-boundary revocation or field-test process observation.
Those broader behaviors remain assigned to later E-045 stories.

## Acceptance fixture

Added the native plugin test:

```text
codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
```

Native plugin tests link no-op Zellij host functions.
The fixture therefore runs the production scheduler and pane-send logic against stub panes
without needing a real Zellij server or Codex process.

The fixture creates ten independent ready Codex tickets and two empty panes.
The scheduler cap admits two tickets in the first wave.
For both assigned ticket/pane pairs it observes:

- the exact current `AttemptLease`;
- the retained `AssignmentRef`;
- the complete assignment Markdown body;
- the per-pane atomic launch script;
- the recorded `SessionLaunch.command` pane line;
- the actual deferred Enter queued by `send_line_to_pane`;
- the slot client/session/transition state.

It requires exactly two active tickets and two queued pane submissions.
It requires distinct assignment paths and distinct launch-script paths.

For each assignment file it proves the body really contains:

- `Read the ticket`;
- `AGENTS.md`.

For each launch script it proves:

- the configured hostile-path Lisa executable is safely quoted;
- `launch-codex` is invoked;
- the ticket's own exact assignment path appears after `--`;
- `Read the ticket` is absent;
- `AGENTS.md` is absent;
- direct `codex --dangerously...` is absent.

For each pane line it proves exact equality with `sh <script-path>`.
It proves assignment text and even the launcher script body are absent from pane input.

For each slot it proves:

- `has_session` is true after dispatch;
- `last_client` is Codex;
- transition state is not `WaitingForClear`;
- assignment state is fresh `Starting` for the exact attempt generation.

This directly satisfies the fixture/stub-pane acceptance criterion.

## Prerequisite argv coverage

The completed dependency's black-box test remains green:

```text
cargo test -p lisa-cli --test codex_launcher
```

It invokes the actual built Lisa binary with a hostile assignment path and a capture stub.
It requires the child vector to contain exactly the fixed flags, optional model pair,
separator, and one unchanged final path element.

The two fixtures compose into the story contract:

```text
immutable assignment file
  → plugin launch script contains only exact path
  → pane receives only script invocation
  → native Lisa parses the path
  → Codex receives one exact argv element
```

No assignment-body paste or shell-composed Codex argv remains in that chain.

## Test coverage

Focused adapter suite:

```text
cargo test -p lisa-plugin adapter::tests
```

Result:

- 25 passed;
- 0 failed.

Focused acceptance fixture:

```text
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
```

Result:

- 1 passed;
- 0 failed.

Prerequisite native launcher fixture:

```text
cargo test -p lisa-cli --test codex_launcher
```

Result:

- 1 passed;
- 0 failed.

Complete plugin package:

```text
cargo test -p lisa-plugin
```

Result:

- 391 passed;
- 0 failed;
- 0 ignored.

Complete workspace:

```text
cargo test --workspace
```

Passed all enabled unit, integration, and doc tests.
Principal observed totals included:

- CLI library: 19 passed;
- CLI binary: 269 passed;
- plugin: 391 passed;
- core and all remaining integration suites passed;
- doc tests passed.

Repository quick check:

```text
just check
```

Passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- a second full workspace test run.

Formatting and whitespace:

```text
cargo fmt --all -- --check
git diff --check
```

Both passed for ticket-owned source.

## Shared-worktree commit interaction

Concurrent ticket `T-045-03-01` edited `crates/lisa-plugin/src/lib.rs` for claim admission.
Its changes were initially identifiable as foreign:

- `AssignmentClaim` import;
- claim signal cleanup;
- exact claim admission;
- claim signal consumption;
- poll ordering and claim tests.

This ticket preserved those changes.
After this ticket made non-overlapping assignment-path launch edits, the neighboring
isolated transaction created:

```text
67a7f0e feat(plugin): own assignments from exact claims
```

That shared-file transaction serialized the already-present production launch-path hunks
alongside the neighbor's claim work.
The current ticket commit then added the adapter side, comments, and acceptance fixture,
restoring a coherent committed interface.

This is recorded because commit-level attribution across the shared file is imperfect.
It does not create a product defect:

- final HEAD contains all required launch-path flow;
- no source hunk was lost;
- no foreign pending diff was captured by this ticket;
- plugin, workspace, and WASM verification pass on final HEAD;
- the ticket's isolated commit itself contains only the two declared shared source paths.

The overlap reveals a missing serialization edge between `T-045-02-02` and
`T-045-03-01`: both edit `lib.rs`, while the latter depends only on the claim command.
Future decomposition should add a dependency when adjacent tickets share the same central
scheduler file, even when their logical concerns differ.

## Scope assessment

The implementation remains at the assignment launch boundary.
It does not change:

- assignment filename or atomic writer;
- CLI launcher argv implementation;
- claim command validation;
- claim wire schema;
- evidence-tier ordering beyond concurrently completed work;
- dashboard labels;
- completion transaction;
- ticket phase/status frontmatter;
- Claude delivery;
- real provider field-test harness;
- ticket-boundary nonce revocation.

The ticket did not remove the legacy supplemental assignment-reference state-machine path.
That path contains only a bounded file reference, never the assignment body.
Evidence ranking and retry behavior belong to subsequent `S-045-03` tickets.
The current acceptance contract is the initial per-ticket launcher transport and no body paste.

## Open concerns and limitations

### Real Codex/Zellij behavior is not claimed

The fixture stops at the no-op Zellij host boundary.
It proves exactly what the plugin asks Zellij to inject and what the native launcher asks
the Codex child to receive.
It does not prove terminal timing, installed Codex parsing, or actual screen behavior.
The epic explicitly assigns live validation to the field-test story.
This is a declared limitation, not a ticket gap.

### Assignment path is the initial Codex prompt

The native launcher passes the exact path, not file contents or a sentence.
The prior launcher ticket deliberately selected this smallest contract.
Whether a particular installed Codex version reliably treats the bare path as a file-reading
instruction remains part of real field validation.

### Shell edge exists before native argv construction

The WASM plugin must reach the native Lisa process through a pane shell.
The binary, model, and path are shell-quoted using the repository's tested helper.
After Clap parses them, Lisa uses native `OsString` argv for Codex.

Non-UTF-8 paths cannot be represented losslessly in this shell command `String` boundary.
Lisa's project and attempt paths are already transported through string-based Zellij commands,
and the ticket fixture covers hostile UTF-8 shell characters.
If non-UTF-8 project paths become a supported requirement, the broader pane command transport
will need a separate binary-safe design.

### Current Codex CLI surface can drift

`launch-codex` uses current interactive Codex flags and positional prompt behavior.
The prerequisite review already records this version-sensitive surface.
The later real field test should rerun after Codex upgrades.

None of these limitations blocks the requested fixture-proven transport change.

## Human review focus

A reviewer should verify:

1. `AgentAdapter::launch_command` requires the published path;
2. Claude ignores it and preserves exact command output;
3. Codex invokes the resolved Lisa binary, not bare direct Codex;
4. optional model appears before `--`;
5. only the exact path appears after `--`;
6. every production fresh-launch site uses the returned `AssignmentRef`;
7. `strip_host_prefix` is applied before pane command construction;
8. launch scripts contain no assignment body;
9. pane commands contain only `sh <script>`;
10. the two-ticket fixture proves distinct fresh sessions;
11. the shared commit interaction is understood and not mistaken for missing code;
12. no Review artifact or ticket frontmatter was manually published by this agent.

## Final assessment

The current branch satisfies the ticket acceptance criterion.
Zellij's stub-pane inputs are bounded per-ticket launcher invocations.
The assignment body never appears in pane input or launch script.
Each admitted ticket has its own fresh Codex launcher, immutable assignment reference,
and session state.
The native boundary preserves the exact path as one Codex argv element.
Claude behavior is unchanged.

The work is ready for Lisa's completion publication.
