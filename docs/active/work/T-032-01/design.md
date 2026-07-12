# Design: T-032-01 Zellij pane lifecycle names

## Goals

- Give every coding pane a deterministic scheduler-owned title.
- Show the actual resolved provider, ticket ID, and parsed human title while assigned.
- Show resident-provider idle state after release, or Lisa idle state for an empty shell.
- Ensure a previous ticket name is replaced before new input is submitted.
- Keep commit failures, awaiting-human waits, and transition recovery visibly assigned.
- Bound and sanitize untrusted title text.
- Avoid repeated Zellij host calls when the desired name has not changed.
- Make lifecycle behavior testable without a live Zellij process.

## Option 1: Provider adapters name their panes

Each `AgentAdapter` could expose a pane-name method alongside launch and reuse behavior.
The scheduler would ask the selected adapter for a name during each provider-specific path.

Advantages:

- Provider labels sit near provider-specific command construction.
- A future provider can customize its displayed spelling.

Disadvantages:

- The ticket requires one deterministic formatter used by every provider.
- Idle naming depends on scheduler slot state, not adapter command behavior.
- Completion gating and release happen outside adapters.
- Cross-provider recycling temporarily involves both resident and incoming adapters, which
  creates ambiguity over which adapter owns the title.
- Duplicate calls across launch, clear, timeout, and release paths are likely.

Decision: reject. Adapters describe client interaction, while pane naming is lifecycle state.

## Option 2: Derive names from `Thread` records during polling

The plugin could reconcile every slot title on each poll: find its thread, derive an assigned
name from the thread and DAG, otherwise derive an idle name from the slot.

Advantages:

- A single reconciliation pass covers most state changes.
- It naturally repairs titles after unexpected internal mutations.
- Deduplication can suppress unchanged host calls.

Disadvantages:

- Scheduling currently sends the launch/reset input before it inserts the `Thread`.
- A reconciliation-only design leaves a stale title visible between prompt submission and
  the next poll, violating the explicit ordering criterion.
- A poll reads already-mutated state and obscures the exact commit-gated release boundary.
- Unit tests would validate eventual state rather than the important operation ordering.

Decision: reject as the primary mechanism. Reconciliation is unnecessary if mutation points
use a common gate, and it cannot alone satisfy before-submit ordering.

## Option 3: Scheduler mutation points call one formatter and rename gate

Add a pure formatter for assigned and idle names. Add one `State` helper that compares the
desired name with a per-slot last-applied cache, updates the cache, and invokes Zellij only
when changed. Call it at slot discovery, assignment before pane input, release, and the one
clean-shell recovery path.

Advantages:

- The scheduler owns every fact the formatter needs.
- One formatter establishes identical behavior across providers.
- Exact placement before input makes stale-title ordering explicit.
- The common release helper aligns idle naming with actual slot release.
- A per-slot cache makes redundant-call suppression local and deterministic.
- Tests can inspect the cache as an observable record of rename intent.

Disadvantages:

- Lifecycle code must carefully order borrows and host calls.
- Any future direct `AgentSlot` mutation that changes assignment/session truth must remember
  to use the helper.
- Discovery and clean-shell recovery need explicit idle calls in addition to release.

Decision: choose this option. It matches current architecture and acceptance boundaries.

## Formatter contract

Use one function with a small semantic input enum:

- Assigned: actual `AgentClient`, full ticket ID, parsed title.
- Idle resident: resident `AgentClient`.
- Idle shell: no resident client.

Outputs:

- Assigned: `<agent> · <ticket-id> · <sanitized-title>`.
- Resident idle: `<agent> · idle`.
- Empty shell: `lisa · idle`.

The provider is always `ResolvedRoute.agent` on assignment. Requested agent text is never
used in a pane name. On later reuse and completion paths, the slot's `last_client` remains
the actual resident-provider authority.

## Sanitization contract

Only the human title is untrusted display prose. The formatter will:

- Replace every Unicode control character with a normal ASCII space.
- Normalize all whitespace runs to one ASCII space.
- Trim leading and trailing whitespace through that normalization.
- Use `untitled` if sanitization leaves no visible content.

This prevents embedded newline, carriage return, tab, escape, bidi-control, or other control
characters from entering a Zellij pane title. Ticket ID and agent are stable scan keys from
Lisa's own parsed scheduling vocabulary and are preserved verbatim.

## Length contract

Define `MAX_PANE_NAME_CHARS = 80`, measured in Unicode scalar values (`char` count). Scalar
count makes the implementation deterministic and prevents invalid UTF-8 slicing. Eighty
characters is narrow enough for tab-bar utility while retaining meaningful titles.

For assigned names:

1. Construct the immutable prefix `<agent> · <ticket-id> · `.
2. Reserve one scalar for the ellipsis when truncation is needed.
3. Keep the complete prefix.
4. Take only as many sanitized title scalars as fit and append `…`.

Canonical Lisa ticket IDs are much shorter than the limit, so the stable prefix fits. To
make malformed external input deterministic, if the prefix itself reaches the limit the
formatter returns the complete scan-key prefix without title; preservation of agent and ID
takes precedence over the nominal cosmetic cap. This exception is documented and cannot be
reached by the repository's ticket naming convention.

Idle names are intrinsically below the bound.

## Rename gate and deduplication

Add `last_pane_names: HashMap<u32, String>` to `State`. It records the last name Lisa applied
to each physical pane without inflating every slot fixture. Add
`State::rename_slot(pane_id, desired_name)` which:

- Confirms the physical pane ID belongs to a discovered slot.
- Returns false without a host call when the cache equals the desired name.
- Stores the new value before invoking the host call.
- Calls `rename_terminal_pane` in production.
- Returns true when a rename was newly applied.

Updating before the host call makes repeated scheduler events idempotent and lets native
tests assert the operation without needing a Zellij host. The plugin API provides no rename
acknowledgement, so this is the strongest locally observable applied-state model available.

## Assignment ordering

In `schedule_ready_tickets`, resolve the route and fetch the parsed ticket before selecting
the I/O branch. After the safety gates and before any of these calls:

- `send_line_to_pane(adapter.launch_command(...))`
- `send_line_to_pane("/clear")`
- `send_line_to_pane(resident_adapter.exit_command())`

format and apply the assigned title. This covers fresh launch, same-provider reuse, and
cross-provider switch with identical ordering. The actual `route.agent` is already known.

The bound ticket and thread state may still be recorded after branch I/O as today; naming
does not need those records because the ticket and route are already local variables.

## Reuse and timeout behavior

No rename is required when `.cleared` arrives or a clear timeout injects the pending prompt:
the assignment name was applied before `/clear` began and must remain unchanged.

No rename is required when the cross-provider exit grace expires and the fresh incoming
client launches: the incoming assignment name was applied before `/exit` began.

Awaiting-human suppression leaves the existing assigned name intact because scheduling
returns before selecting/mutating the slot and transition handlers do not release it.

Stop/clear timeout recovery also leaves the assigned name intact. These paths repair client
interaction, not assignment ownership.

## Release behavior

`release_slot_for_ticket` remains the sole common slot-release operation. After it clears
the ticket binding, it derives the idle name from post-release session truth:

- `has_session && last_client = Some(client)` -> `<client> · idle`.
- otherwise -> `lisa · idle`.

The rename happens only when a matching assigned slot was found and actually released.
Commit failure never calls this helper, so it cannot produce an idle title. Verified
completion calls it only after durable Done verification.

Other true release paths, such as error or stale-session reclaim, also become idle because
the slot is genuinely made available for future scheduling. The acceptance criterion bars
false idle names on non-release paths, not honest idle names after recovery releases.

## Empty-shell behavior

At discovery, every newly recorded empty slot receives `lisa · idle`. This establishes the
initial operator view before any ticket is scheduled.

During `WaitingForExit` recovery, if the pending ticket disappears, the code clears
`has_session` and `last_client`; it then applies `lisa · idle`. This covers the only current
path that turns a known resident slot into an unassigned clean shell outside normal release.

## Test strategy

Pure formatter tests cover:

- Claude and Codex assigned formats.
- Control-character replacement and whitespace normalization.
- Empty sanitized titles.
- Exact-bound and over-bound Unicode truncation.
- Preservation of complete provider and ticket ID.
- Both idle forms.
- Invalid requested route falling back to the actual configured provider.

Scheduler tests cover:

- Rename cache deduplication.
- Initial empty-shell discovery state where practical through a slot helper.
- Fresh launch assigned name.
- Same-provider reuse assigned name.
- Cross-provider recycle assigned name.
- Verified completion release to resident idle.
- Failed completion retaining the assigned name.
- Clean-shell recovery to `lisa · idle`.

Existing lifecycle tests already exercise the underlying branches. New assertions should be
added to those tests where fixtures are stable, with focused helpers used to avoid relying on
Zellij host output.

## Live validation

The acceptance criterion requests a live mixed Claude/Codex Zellij sequence. A real result
requires an available Zellij session, provider binaries/authentication, and a build containing
the change. The implementation will inspect the local environment and run the validation if
those prerequisites are safely available. Review will distinguish observed evidence from any
environmental limitation; it will not claim simulated unit tests as live evidence.

## Decision summary

- Scheduler owns naming, not providers or UI.
- One formatter produces every title.
- One cached rename gate invokes Zellij.
- Assignment rename occurs before first lifecycle input.
- Common release derives idle name from resident-session state.
- Non-release transitions retain the assigned title.
- Maximum normal display length is 80 Unicode scalar values.
