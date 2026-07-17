# Structure — T-049-01-01 seal types and resolution

## Modified: `crates/lisa-core/src/completion.rs`

Add the completion-seal domain vocabulary near the top of the existing pure
completion module, before aggregate identity/state types.

### `CompletionSeal`

Public enum with variants `Commit` and `Journal`.

Derives debug, clone/copy, equality/order/hash, Serde serialize/deserialize.

Serde representation uses lowercase variant names.

Implements `Default` as `Commit` only for legacy runtime compatibility.

Exposes `VALID`, `as_str`, strict `parse`, and `Display`.

### `CompletionSealMode`

Public enum with variants `Auto`, `Commit`, and `Journal`.

Derives the same value traits and lowercase Serde support.

Implements `Default` as `Auto` for unconfigured `.lisa.toml` intent.

Exposes `VALID`, `as_str`, strict `parse`, `explicit_seal`, and `Display`.

`explicit_seal` returns `None` for auto and the corresponding runtime seal for
the other two values.

### `CommitSealUnavailable`

Public typed reason enum:

- `RepositoryMissing`;
- `IdentityMissing`;
- `TransactionUnavailable { detail: String }`.

Implements `Display` with stable, operator-usable prerequisite descriptions.

### `CommitSealSupport`

Public enum containing either `Available` or
`Unavailable(CommitSealUnavailable)`.

This is the pure adapter input to resolution.

### `ResolvedCompletionSeal`

Public immutable value with private fields:

- `seal: CompletionSeal`;
- `commit_unavailable: Option<CommitSealUnavailable>`.

Exposes `seal()` and `commit_unavailable()` getters.

No setter or re-resolution method exists.

### `CompletionSealResolutionError`

Public typed error containing the unavailable reason for an explicitly required
commit seal.

Exposes a `reason()` getter and an error display naming the unsatisfied commit
seal.

### `resolve_completion_seal`

Public pure function accepting `CompletionSealMode` and `CommitSealSupport`.

Returns `ResolvedCompletionSeal` or `CompletionSealResolutionError` according
to the design matrix.

### Tests

Add a dedicated nested test group covering:

- default/parse/display strings;
- auto with available support;
- auto with each unavailable environment;
- explicit commit success;
- explicit commit hard failure and retained reason;
- explicit journal with available and unavailable support;
- automatic fallback reason versus deliberate journal absence of reason.

## Modified: `crates/lisa-core/src/types.rs`

Import `CompletionSeal` from the completion module.

Add public `completion_seal: CompletionSeal` to `PluginConfig` near `git_root`,
because both values are native-startup facts consumed by completion adapters.

Initialize it to `CompletionSeal::Commit` in `PluginConfig::new`.

Parse the `completion_seal` map key leniently in `from_config_map`.

Only `commit` and `journal` are accepted at runtime.

Missing, `auto`, or unknown values retain the commit default.

Add round-trip and malformed/legacy fallback tests.

No scheduler behavior changes in this file.

## Modified: `crates/lisa-cli/src/config.rs`

Import `CompletionSealMode`.

Add `GuardsConfig` with optional raw-string `completion`.

Add `guards: GuardsConfig` to `LisaConfig` with a Serde default.

Add `completion_mode: CompletionSealMode` to `ResolvedConfig`.

Set its default to auto.

Resolve the validated `[guards].completion` string in `resolve_config`, with a
defensive fallback to auto.

Extend validation metadata:

- `guards` in known top-level sections;
- `completion` in known `[guards]` keys;
- warning loop for unknown guard keys.

Perform strict semantic validation through `CompletionSealMode::parse`.

Extend `default_config_toml()` with an inert `[guards]` example.

Add tests for default, each valid value, unknown actionable value, unknown key
warning, and default template.

## Created: `crates/lisa-cli/src/completion_seal.rs`

This module owns the native environment adapter and the per-run pinned wrapper.

### `RunCompletionSeal`

Crate-visible immutable struct containing:

- core `ResolvedCompletionSeal`;
- optional canonical `git_root` discovered during the same probe.

Expose `seal()`, `commit_unavailable()`, and `git_root()` getters.

### `CommitProbeOutcome`

Private adapter value combining `CommitSealSupport` with the optional discovered
repository root.

### `resolve_for_run`

Crate-visible production entry point accepting project root and configured mode.

Delegates to a generic/private `resolve_for_run_with` using the real probe.

Formats core explicit-commit errors into the named CLI preflight failure.

### `resolve_for_run_with`

Accepts a `FnOnce(&Path) -> CommitProbeOutcome`.

The one-shot function type documents and enforces one probe invocation within
the startup resolution operation.

Explicit journal bypasses the closure.

Auto and explicit commit invoke it exactly once.

### `probe_commit_support`

Runs read-only Git commands in prerequisite order.

Repository command:

`git -C <project> rev-parse --show-toplevel`

Identity command:

`git -C <repo> config --get user.email`

Transaction commands:

`git -C <repo> rev-parse --verify HEAD`

`git -C <repo> rev-parse --absolute-git-dir`

Failures map to typed unavailable reasons, preserving concise stderr in the
transaction detail.

### Failure formatter

Produce a named `Completion seal preflight failed` message.

Include the typed missing prerequisite.

Include exactly the two identity remedy command lines.

### Tests

Use the `FnOnce` seam for the full mode/environment matrix at the CLI boundary.

Count invocations to prove auto/commit probe once and explicit journal probes
zero times.

Assert missing identity explicit commit contains the named preflight and both
remedy lines.

Use disposable repositories for focused real-probe tests if stable isolation
from global identity can be established without process-global test races.

## Modified: `crates/lisa-cli/src/main.rs`

Register `mod completion_seal;` beside config/loop modules.

No command-line surface is added.

## Modified: `crates/lisa-cli/src/doctor.rs`

Parameterize the required dependency builder/helper with `require_git: bool`.

Doctor's `build_checks` passes `true`, retaining its existing surface in this
ticket.

Loop's helper accepts the pinned-tier decision.

The selected agent dependency remains required regardless of seal.

Update existing helper tests and add one asserting the no-Git set contains only
the selected agent.

## Modified: `crates/lisa-cli/src/loop_cmd.rs`

Import `CompletionSeal` as needed for dry-run layout rendering.

On the real-run branch, resolve the completion seal once immediately after
cheap project/protocol checks and before runtime/dependency side effects.

Retain the returned `RunCompletionSeal` binding through layout generation.

Pass `require_git = pinned.seal() == CompletionSeal::Commit` into each provider
dependency check.

Remove the unconditional second Git-root discovery.

Use `pinned.git_root()` when present; otherwise canonicalize the project root.

Pass the pinned seal explicitly into `generate_layout`.

Render `completion_seal "commit|journal"` beside `git_root`.

Dry-run chooses the explicit configured seal or commit for auto and does not
invoke the real environment probe.

Update the layout helper/tests to pass a seal.

Add assertions that commit and journal layouts carry the exact tier and parse
back through `PluginConfig` behavior.

## Unchanged files

`crates/lisa-cli/src/commit_transaction.rs` is unchanged; Tier 1 behavior stays
exactly as it is.

`crates/lisa-plugin/src/completion_journal.rs` is unchanged; journal seal
mechanics are not implemented here.

`crates/lisa-plugin/src/lib.rs` receives the new config field through its
existing cloned `PluginConfig`, with no completion branching in this ticket.

Doctor/status output and ledger schemas are unchanged for T-049-01-02.

Ticket frontmatter is unchanged; Lisa owns phase/status transitions.

## Implementation and commit boundaries

Unit 1: core seal domain plus plugin configuration transport.

Exact source includes:

- `crates/lisa-core/src/completion.rs`;
- `crates/lisa-core/src/types.rs`.

Unit 2: `.lisa.toml` guard parsing and native one-shot resolution.

Exact source includes:

- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/completion_seal.rs`;
- `crates/lisa-cli/src/main.rs`.

Unit 3: loop/dependency integration and pinned layout transport.

Exact source includes:

- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`.

Each unit is formatted, tested proportionally, and committed with
`lisa commit-ticket --ticket-id T-049-01-01` and only those exact include paths.

Phase artifacts are excluded from ticket source commits because Lisa publishes
them after lease verification.
