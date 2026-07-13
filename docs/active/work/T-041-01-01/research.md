# Research: completion domain types

## Ticket boundary

- T-041-01-01 establishes vocabulary for a completion aggregate in `lisa-core`.
- The ticket starts in Research and requires all remaining RDSPI phases.
- The acceptance criterion names the module and its public type families.
- It does not ask for a reducer, reconciliation, plugin integration, or persistence.
- T-041-01-02 depends on this ticket and adds the total reducer.
- T-041-01-03 follows it and adds level-triggered reconciliation.
- Story S-041-01 explicitly keeps Zellij and WASM imports outside the module.
- The story also leaves existing plugin completion call sites untouched.

## Workspace organization

- The workspace contains `lisa-core`, `lisa-cli`, and `lisa-plugin` crates.
- `crates/lisa-core/src/lib.rs` declares each core module with `pub mod`.
- Core modules are publicly addressable beneath the `lisa_core` crate root.
- `lisa-core` currently contains client, DAG, diagnostics, disposition,
  provenance, route, ticket, and general types modules.
- There is no existing `completion.rs` module.
- `lisa-core` has no dependency on `lisa-plugin` or Zellij APIs.
- Its production dependencies are serde, serde_yaml_ng, and serde_json.
- `tempfile` is its only direct development dependency.
- The workspace lockfile already contains transitive thiserror versions.
- `lisa-core` does not yet declare thiserror directly.

## Existing identity vocabulary

- `types.rs` defines `TicketId` as a `String` alias.
- `types.rs` defines `AttemptLease` as a ticket ID plus a raw `u64` attempt ID.
- Attempt leases are serializable and compare by their complete pair.
- `AttemptLease::mint` makes the first generation 1.
- Minting checks ticket agreement and numeric overflow.
- The lease is the current attempt-authority boundary used by the plugin.
- There is no standalone attempt identity newtype.
- There is no completion identity type.
- There is no command correlation identity type.

## Existing completion behavior

- Completion behavior currently lives in the plugin rather than core.
- The plugin defines private completion source and authority enums.
- The plugin tracks pending completions in a map keyed by ticket ID.
- A pending entry carries source and authority data.
- Existing request paths return booleans and launch external commands.
- Artifact, idle, stopped, observed-Done, and manual paths can request work.
- Existing command results are handled in plugin state.
- Current behavior therefore combines domain decisions and adapter effects.
- This ticket does not migrate or remove that behavior.

## Disposition boundary

- `disposition.rs` defines `ReviewDisposition`.
- Its variants are `Pass`, `Block { reason }`, and `Invalid { reason }`.
- Parsing is fail-closed.
- Only an exact pass document grants completion authority.
- Block and invalid states retain operator-visible reasons.
- Story S-041-01 says the completion domain consumes this as typed input.
- This ticket does not duplicate the disposition parser.

## Core module conventions

- Core public data types commonly derive `Debug`, `Clone`, `PartialEq`, and `Eq`.
- Persisted types additionally derive serde traits where persistence is needed.
- Public types and fields have rustdoc documentation.
- Unit tests are colocated under `#[cfg(test)]` modules.
- Pure vocabulary modules do not import plugin types.
- Existing error types often implement Display and Error manually.
- The ticket specifically requires thiserror for completion rejection errors.

## Named acceptance vocabulary

- `CompletionState` must represent Eligible.
- It must represent Requested.
- It must represent CommandInFlight with a correlation identity.
- It must represent Rejected with a reason and retryability.
- It must represent Confirmed.
- `CompletionEvent` must provide typed aggregate inputs.
- Attempt and completion identities must be newtypes.
- A correlation ID must be a separate identity.
- `EffectCommand` represents an external action without executing it.
- `Transition` represents an accepted state change and optional effect.
- `CompletionRejection` represents refused transitions without a boolean.

## Rejection vocabulary

- Already-pending must be a distinct variant.
- Stale-lease must be a distinct variant.
- Disposition-blocked must be a distinct variant.
- Dependency-blocked must be a distinct variant.
- Launch-failed must be a distinct variant.
- Launch failure has an underlying adapter error suitable for error chaining.
- The rejected lifecycle state separately records retryability.
- The module therefore needs an explicit retryability type.
- The acceptance criterion rules out collapsing these outcomes into flags.

## Constraints

- A command-in-flight value without correlation must be unrepresentable.
- No default or optional correlation can satisfy that invariant.
- The module must compile on the native workspace test target.
- It must remain compatible with the plugin's WASM build boundary.
- No Zellij or WASM dependency can be introduced into `lisa-core`.
- External effects must remain data only.
- The reducer belongs to the next ticket, not this one.
- Level-triggered durable-input derivation belongs to the ticket after that.
- Existing unrelated working-tree changes must be preserved.
- Ticket-owned source changes must use `lisa commit-ticket` with exact paths.

## Verification surface

- Unit tests can prove newtype values remain distinct at the type level.
- Construction tests can cover every lifecycle state.
- A state pattern match can demonstrate correlation is mandatory in flight.
- Error tests can prove every rejection has a distinct Display message.
- A source test can prove launch failure exposes its underlying error.
- Transition tests can prove effects are optional and singular.
- `cargo test -p lisa-core` provides fast focused verification.
- `cargo test --workspace` is the ticket-wide required verification.
- A wasm target check can confirm the pure dependency boundary remains sound.

## Observed repository state

- The ticket frontmatter and Lisa provenance file are modified by orchestration.
- `crates/lisa-plugin/docs/` is untracked and unrelated to this ticket.
- Those paths must not be included in this ticket's source commit.
- The private attempt directory already contains assignment and launch files.
- Phase artifacts belong only in that private attempt work directory.
