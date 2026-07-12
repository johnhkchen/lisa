# Research: T-034-03-02 live proof and Claude parity

## Ticket boundary

The ticket begins in `research` and requires a live validation rather than new
scheduler behavior.

The acceptance criterion has four connected requirements:

1. use an isolated temporary project;
2. use a freshly built Lisa CLI and its freshly embedded WASM;
3. execute the committed split-brain regression and prove the fenced boundary;
4. record unchanged Claude assignment and completion behavior across the same
   runtime harness.

The parent story explicitly says this slice changes no scheduler logic.

The ticket phase and status frontmatter are owned by Lisa and must not be edited
by this agent.

## Prerequisite regression

The prerequisite ticket T-034-03-01 is complete at commit
`0ffe40f67551774964cfaf3e229ba5052cee43ea`.

It added one test in `crates/lisa-plugin/src/lib.rs`:

`split_brain_timeline_fences_old_attempt_and_admits_one_winner`

The test uses production scheduler methods rather than a parallel model.

Its deterministic scenario is:

- attempt 1 owns a Codex ticket on pane 1;
- attempt 1 becomes over-budget and hard-silent;
- `check_session_timeouts` revokes its lease;
- pane 1 is fenced before its slot is released;
- real scheduling mints attempt 2 on pane 2;
- the replacement prompt is delivered but its acknowledgement is withheld;
- attempt-1 heartbeat, ack, idle, stopped, cleared, and error signals resume;
- attempt-1 artifact and completion requests are rejected;
- attempt 2 alone becomes owned and publishes canonical review bytes;
- attempt 2 alone produces authoritative Done provenance.

The regression asserts the lifecycle order
`LeaseRevoked -> PaneFenced -> SlotReleased`.

It also asserts the old pane remains terminally fenced and cannot be selected
for redispatch.

The append-only ledger retains the attempt-1 TimedOut row, but exactly one
authoritative Done row belongs to attempt 2.

That distinction is part of the current provenance contract.

## Native versus live boundary

The plugin test is compiled and run natively by `cargo test -p lisa-plugin`.

Native tests can inspect private scheduler state, inject timestamps, and avoid
sleeping.

They stub Zellij host operations such as terminal input and pane closure.

The prerequisite review therefore names actual Zellij pane closure as the
remaining live-proof boundary.

A live loop executes the plugin as `wasm32-wasip1` under Zellij.

That runtime supplies the host calls that native tests deliberately omit.

The two forms of evidence are complementary:

- the committed regression proves the complete adversarial state timeline;
- the fresh loop proves the built artifact loads and drives real panes and
  provider lifecycle hooks.

## Build and embedding path

`crates/lisa-cli/build.rs` copies the release WASM into the CLI build output.

`crates/lisa-cli/src/templates.rs` exposes it as `PLUGIN_WASM`.

`lisa loop` rejects an empty embedded plugin before launching Zellij.

For a real loop, `run_loop` writes the embedded bytes to a content-hashed path:

`$TMPDIR/lisa-plugin-<hash>.wasm`

The content hash changes when the WASM changes.

`run_loop` removes stale Lisa WASM files, clears Zellij's compiled plugin cache,
and pre-grants the new plugin permissions before launch.

It captures `current_exe()` and emits that exact Lisa binary into the plugin
configuration as `lisa_bin`.

Completion and usage hooks therefore call the fresh fixture binary rather than
the older Homebrew binary.

The generated `.lisa-layout.kdl` records both the exact WASM path and exact Lisa
binary path, providing inspectable build provenance.

## Installed tool state

The host currently has:

- Zellij `0.44.3` at `/opt/homebrew/bin/zellij`;
- Claude Code `2.1.207` at `/Users/johnchen/.local/bin/claude`;
- Codex CLI `0.144.1` at `/Users/johnchen/.local/bin/codex`;
- Homebrew Lisa `0.4.0-rc.5` at `/opt/homebrew/bin/lisa`.

The Homebrew Lisa predates current repository behavior and must not drive the
proof.

The repository-built binary is the valid transaction and loop entry point.

## Loop startup behavior

`lisa loop --path <fixture>` validates the scaffold before launching.

It discovers every provider named by the loop default or ticket frontmatter.

For a mixed fixture it checks both Claude and Codex dependencies.

If any ticket routes to Codex, it requires `.codex/hooks.json` and pre-grants
directory trust.

The loop generates a stacked layout with twice `max_threads` terminal panes and
one dashboard plugin pane.

The plugin discovers those terminal panes as physical agent slots.

Per-ticket `agent` frontmatter selects the provider even when the loop has a
different default.

## Provider assignment contract

Claude and Codex share ticket, slot, lease, artifact, completion, and provenance
authority checks.

They intentionally do not share identical transport ownership evidence.

Claude remains immediately owned when assigned.

Claude reuse uses the established `/clear` handshake and Claude hook signals.

Codex reused-session ownership is generation-acknowledged.

Codex may be `AssignedPendingAck` or `RecoveringFresh` before becoming Owned.

The split-brain regression exercises that Codex-specific acknowledgement risk
while enforcing provider-neutral lease fencing around it.

“Claude parity” therefore means unchanged Claude assignment/completion behavior,
not adding Codex acknowledgements to Claude.

## Temporary-project requirements

The fixture needs its own Git repository because implementation and final
completion use isolated Git transactions.

It needs the files created by `lisa init`, including:

- `CLAUDE.md` and `AGENTS.md`;
- `.lisa.toml`;
- `.lisa/hooks/` scripts;
- `.claude/settings.local.json`;
- `.codex/hooks.json`;
- ticket, story, and work directories.

The baseline must be committed before the loop starts.

Tickets should be minimal, isolated, and explicit about producing all RDSPI
artifacts without modifying unrelated files.

The same fixture shape should be used for both providers so differences arise
from the adapter contract rather than ticket content.

## Evidence surfaces

The temporary repository exposes durable evidence without relying only on the
visual dashboard:

- ticket frontmatter after Lisa's completion transaction;
- six canonical work artifacts per ticket;
- Git commit history and changed paths;
- `.lisa/provenance.jsonl` attempt and outcome records;
- `.lisa/signals/` lifecycle files while active;
- `.lisa/attempts/<ticket>/<attempt>/work/` staging boundaries;
- `.lisa-layout.kdl` binary/WASM paths;
- Zellij pane metadata and session state;
- captured build, test, loop, and inspection output.

The proof should preserve an evidence directory under this ticket's work tree,
not the temporary project itself as source code.

## Repository state constraints

The parent repository contains many unrelated modified and untracked paths.

Those paths belong to other work and must remain untouched.

This ticket is expected to add documentation evidence rather than production
source.

If a reusable harness script becomes necessary, it is a ticket-owned source
change and must be committed through `lisa commit-ticket` with an exact path.

If no source change is needed, Lisa will later commit the ticket work artifacts
through its completion transaction.

Ordinary `git add` and ordinary `git commit` are prohibited for parent ticket
work.

The temporary fixture is independent and may use normal Git commands for its
own baseline and for verification.

## Constraints and risks

A live provider run is nondeterministic in duration and consumes external model
capacity.

Zellij is terminal-oriented, so launch and inspection must preserve a PTY.

The parent Lisa loop cannot hot-reload the just-built scheduler and is not valid
evidence for this ticket.

The fresh binary must be invoked by absolute path so shell PATH order cannot
select Homebrew Lisa.

The embedded WASM hash and generated layout must be recorded before cleanup.

Provider completion must be distinguished from merely seeing a process launch.

Done frontmatter, canonical review, a completion commit, and authoritative
provenance together form the durable completion proof.

## Research conclusion

No scheduler implementation gap is visible at the start of this ticket.

The required work is an evidence-producing validation pass:

1. build release WASM and CLI from current `HEAD`;
2. rerun the committed deterministic split-brain regression;
3. scaffold and commit an isolated mixed-provider project;
4. launch that project with the exact fresh binary and embedded WASM;
5. observe equivalent minimal tickets through real Codex and Claude assignment
   and commit-gated completion;
6. retain hashes, layout, pane, Git, artifact, and provenance evidence;
7. report any runtime limitation honestly rather than substituting parent-loop
   observation.
