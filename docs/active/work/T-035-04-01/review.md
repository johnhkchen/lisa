# T-035-04-01 Review — two-stage native chat assignment

## Review outcome

The ticket acceptance criteria are met. Fresh Claude and Codex assignments now separate
provider process readiness from ticket ownership. The complete attempt instructions are
atomically persisted, fresh provider launch scripts are bare, exact SessionStart reaches
only ReadyForAssignment, a later bounded chat reference surfaces Delivering, and only an
exact ticket/attempt `UserPromptSubmit` acknowledgement reaches Owned.

No critical defect was found in self-review. The implementation is committed through
Lisa's isolated transaction, the ticket-owned source paths are clean, the ordinary index
is empty, the full workspace suite passes, and the WASM target check passes.

## Source commit

```text
1bd6c353d0c3d0c11ed86771800f160da7904935
feat(plugin): split provider start from chat assignment
```

The isolated transaction contains exactly:

- `crates/lisa-plugin/src/lib.rs`;
- `crates/lisa-plugin/src/adapter.rs`;
- `crates/lisa-plugin/src/ui.rs`;
- `crates/lisa-cli/src/templates.rs`.

No ticket, story, epic, provenance, runtime hook instance, shared work artifact, or
unrelated dirty path was included.

## Change summary

### Atomic attempt instructions

Every scheduled attempt now constructs the full provider-specific RDSPI assignment
separately from the provider launch command.

`State::prepare_assignment` writes the complete bytes under:

```text
.lisa/attempts/<ticket>/<attempt>/work/assignment.md
```

Publication uses a unique same-directory temporary file followed by atomic rename. A
failed rename removes the temporary path. Dispatch submits no lifecycle input when
assignment preparation fails.

The body remains the existing `ticket_prompt` contract, including:

- exact discovered ticket path;
- `CLAUDE.md` for Claude or `AGENTS.md` for Codex;
- RDSPI workflow reference;
- exact private attempt artifact directory;
- no-frontmatter-edit instruction;
- isolated `lisa commit-ticket` instruction;
- Review handoff and wait instruction.

`assignment.md` is runtime state. It is not a phase artifact and is not admitted to the
shared work directory.

### Bare provider launch

Claude launch now contains only:

- optional `LISA_BIN`;
- pane, ticket, and attempt lifecycle identity;
- `claude --dangerously-skip-permissions`;
- optional routed model.

Codex launch now contains only:

- Lisa binary and `LISA_AGENT_CLIENT=codex`;
- pane, ticket, and attempt lifecycle identity;
- full-access, no-approval, and generated-hook trust flags;
- optional routed model;
- existing process-exit `.error` fallback.

Neither provider command contains:

- the ticket prompt;
- RDSPI instructions;
- a context filename;
- `assignment.md`;
- `LISA_ASSIGNMENT`;
- a positional chat prompt.

The `.lisa-launch-<pane>.sh` payload is therefore bounded by lifecycle identity,
configured binary/model values, provider flags, and error handling. Its length no longer
grows with the full assignment.

### Bounded chat reference

After exact process readiness, the provider receives a short message shaped as:

```text
Read and follow the complete assignment at <exact-attempt>/assignment.md.
LISA_ASSIGNMENT {"ticket_id":"...","generation":...}
```

The path is derived from the pane's exact current attempt lease. The marker is serialized
by the existing JSON-safe helper. Message size depends on path and identity, not ticket
body length.

Both providers use the same scheduler-visible message contract. Provider payloads may
carry additional fields; the classifier intentionally consumes only event name and
submitted prompt.

### Fresh state semantics

The positive path is now:

```text
Starting
  -- exact current SessionStart --> ReadyForAssignment
  -- bounded chat submission --> Delivering
  -- exact current UserPromptSubmit --> Owned
```

`acknowledge_process_start` still validates:

- current Starting generation;
- exact pane reservation;
- exact ticket and attempt lease;
- current authoritative lease.

Its only transition is now ReadyForAssignment. It does not inject chat, arm chat
acceptance, or claim ownership.

`deliver_ready_assignments` runs before new process-start consumption in the poll. A new
start therefore remains ReadyForAssignment for one complete scheduler boundary and can be
surfaced truthfully.

Ready delivery revalidates lease authority and the complete assignment file, submits the
bounded reference, and installs an Enter-delay-aware absolute deadline in Delivering.

`seat_is_owned` remains exact equality with Owned. Starting, ReadyForAssignment,
Delivering, both inherited pending/recovery states, and terminal failures are all
non-owned.

### Provider acknowledgement

Claude's generated settings now bind `UserPromptSubmit` to the same atomic `on-ack.sh`
transport already used by Codex. `merge_hooks` adds the binding idempotently and preserves
user-owned hook groups.

The raw payload scanner accepts an acknowledgement only when:

- the seat has an active gated generation;
- the slot ticket and attempt lease match;
- the lease is still current;
- event name is exactly `UserPromptSubmit`;
- the prompt carries a whole-line structured marker;
- marker ticket and generation exactly match.

Malformed payloads, stale tickets, stale attempts, other events, duplicate evidence, and
late terminal-state evidence fail closed.

The historical internal module/file remains named `codex_ack`, but its event envelope and
scheduler use are provider-neutral. This is a naming limitation only, not a behavioral
provider branch.

### Bounded missed-chat recovery

Fresh Delivering allows one same-attempt retry.

On the first missed deadline Lisa:

- revalidates the current lease;
- revalidates `assignment.md`;
- resubmits only the bounded tagged reference;
- increments retry count;
- replaces the absolute deadline.

On the second missed deadline Lisa enters red terminal `DeliveryFailed`.

DeliveryFailed:

- never grants ownership;
- fails the logical thread;
- retains pane, ticket, attempt lease, assignment, and current authority;
- emits one deduplicated error alert;
- logs explicit reset-to-retry guidance;
- does not release the seat;
- does not mint another attempt;
- does not send `/exit` or another launch;
- does not arm another delivery;
- is inert on later polls;
- rejects a late exact acknowledgement.

This failure is classified as assignment delivery, not an owned hard-silent agent
timeout.

### Inherited E-033 fallback

The existing one-fresh-Codex fallback after a missed reused prompt now also honors the
stronger bare-launch contract.

The successor lease receives an atomic assignment file, launches bare, proves exact
process readiness, receives the bounded chat reference, and requires exact successor
acceptance. It cannot become owned from synthetic prompt evidence before chat delivery.

The inherited policy remains bounded to one fresh provider session. Missing successor
chat acceptance uses the same one-redelivery then terminal-failure path. The predecessor
generation stays fenced throughout.

### Dashboard

Added common assignment statuses:

- yellow `ready-for-assignment`;
- yellow `delivering`;
- red `delivery-failed`.

Existing `starting`, `assigned-pending-ack`, `recovering`, `startup-failed`,
`recovery-failed`, and green `owned` remain available for their existing paths.

The main native regression checks rendered rows for Starting, ReadyForAssignment,
Delivering, and Owned.

## Acceptance criteria assessment

### Bare bounded launch and atomic instructions

Met. Adapter and scheduler tests inspect both providers' prepared scripts and assert they
contain no ticket instructions or marker, while the exact attempt `assignment.md`
contains the complete instructions. A long hostile payload round-trips byte-for-byte
through atomic publication with no temporary file left behind.

### Starting to Ready to Delivering to Owned

Met for both Claude and Codex in one table-driven native test. Exact process start reaches
ReadyForAssignment only. A separate ready dispatcher reaches Delivering. Exact prompt
acceptance reaches Owned.

### Provider-specific evidence and stale rejection

Met. Generated Claude and Codex configurations both produce raw `UserPromptSubmit`
evidence through the shared atomic hook. Exact ticket/attempt classification is common;
stale start and prompt evidence fails closed. Dashboard states are common.

### Missing acknowledgement is bounded and never Owned

Met. The deterministic injected-time regression observes one original chat delivery, one
retry, then DeliveryFailed. It proves zero ownership, zero relaunches, retained lease,
late-ack rejection, and inert repeated timeout evaluation.

### Predecessor regressions

Met. Existing acknowledgement, lease fencing, startup, pane naming, mixed-provider,
consecutive reuse, and workspace tests pass. Fresh fallback tests were strengthened to
exercise the new startup/chat boundary explicitly.

## Test coverage

Focused recovery filters passed:

```text
cargo test -p lisa-plugin bounded_ack_wait
cargo test -p lisa-plugin dropped_post_prompt_ack
cargo test -p lisa-plugin recovery_ack_promotes
cargo test -p lisa-plugin consecutive_reused
```

Plugin suite:

```text
cargo test -p lisa-plugin
```

Result: 280 passed, 0 failed.

CLI/template suite:

```text
cargo test -p lisa-cli
```

Result: 274 unit tests plus the atomic provider integration test passed.

Full verification:

```text
cargo test --workspace
just check
```

Result: all workspace tests passed. `just check` also passed
`cargo check -p lisa-plugin --target wasm32-wasip1` and repeated the full suite.

Formatting and diff hygiene passed:

```text
cargo fmt --all
git diff --check -- <four ticket-owned source paths>
```

## Open concerns and limitations

- The real Zellij/provider rerun remains intentionally outside this native ticket. The
  follow-on standing regression ticket proves installed-provider behavior end to end.
- Recovery from a shell trapped at `dquote>` is T-035-04-02. This ticket does not add
  Ctrl-C/shell-readiness probing or rotate a never-started attempt.
- Same-process Claude reuse retains its predecessor immediate-Owned behavior. This ticket
  changes fresh native assignment; extending the new gate to established Claude reuse
  needs a separately specified recovery policy.
- The `codex_ack` internal name is historical and now underspecifies its shared use. A
  future mechanical rename can improve naming without changing the wire marker.
- Assignment chat retry uses the same live process and immutable attempt, by design. It
  does not attempt to distinguish “hook evidence lost” from “message not submitted”;
  either case is resolved by the same finite redelivery boundary.

No open concern blocks this ticket's acceptance criteria.

## Repository hygiene

The ticket-owned source paths are clean after commit. The ordinary Git index is empty.
Pre-existing unrelated dirty and untracked paths remain untouched.

The ticket frontmatter was not manually edited. Phase artifacts were written only to
`.lisa/attempts/T-035-04-01/1/work/`; Lisa alone handled admitted publication and phase
transitions.

Review is complete. Remain on T-035-04-01 and wait for Lisa's completion commit; do not
start another ticket or publish Done manually.
