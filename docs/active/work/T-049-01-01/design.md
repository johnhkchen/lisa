# Design — T-049-01-01 seal types and resolution

## Decision summary

Represent configured completion intent and the runtime seal as different core
types.

Resolve them through a pure core function fed by typed environment support.

Perform the real Git probe once in the native CLI at real loop startup.

Carry the immutable resolved result and the Git root discovered by that same
probe through the remainder of startup.

Pass only the pinned `CompletionSeal` into the plugin layout.

Keep legacy plugin layouts commit-sealed so this ticket does not weaken Tier 1.

## Domain vocabulary

Add `CompletionSeal` with `Commit` and `Journal` variants.

Its stable external strings are `commit` and `journal`.

Derive Serde with lowercase names so later ledger fields can use the same type.

Add `CompletionSealMode` with `Auto`, `Commit`, and `Journal` variants.

This type represents `[guards].completion`, not an active runtime tier.

Its default is `Auto`.

Both types expose strict parsing with an error that lists every accepted value.

Add `CommitSealSupport`, a typed description of the strongest commit contract
the startup probe could establish.

Support is either available or unavailable for one stable reason:

- repository missing;
- commit identity missing;
- transaction path unavailable.

The transaction-unavailable reason retains an owned detail string so a hard
preflight failure can name the actual failed prerequisite.

Add `ResolvedCompletionSeal`, the immutable/pinned resolution value.

It exposes the selected seal and, for an automatic journal fallback, the typed
reason commit was unavailable.

Explicit journal has no unavailable reason because it is a deliberate stance,
not an inferred degradation.

Explicit commit returns a typed resolution error instead of a value when commit
support is unavailable.

## Pure resolution rules

`auto + available` resolves commit.

`auto + unavailable(reason)` resolves journal and retains `reason`.

`commit + available` resolves commit.

`commit + unavailable(reason)` returns a hard typed error.

`journal + any support` resolves journal and records no fallback reason.

The CLI will not invoke the environment probe at all for explicit journal.

The pure resolver still accepts a support value for totality and testability;
the CLI adapter supplies an arbitrary unavailable value on the short path.

## Environment probe

Create a small CLI module dedicated to completion-seal startup resolution.

The adapter first checks the configured mode.

For explicit journal it immediately returns a pinned journal resolution and no
repository root.

For auto or explicit commit it runs Git at the requested project directory.

Repository presence is established with `git rev-parse --show-toplevel`.

The returned root is canonicalized once and retained for layout construction.

Identity is established with `git config --get user.email` at the discovered
repository root.

A successful command with empty output is treated as missing identity.

Transaction readiness is established against prerequisites used by today's
isolated transaction.

The current transaction requires a resolvable `HEAD` before it can read the
base tree or create a parented commit.

The probe therefore runs `git rev-parse --verify HEAD`.

It also resolves `--absolute-git-dir`, matching transaction repository
discovery and ensuring that the transaction metadata path exists.

These checks are read-only; loop startup must not create a dangling commit or
advance a ref merely to select a guard tier.

The later transaction remains fail-closed for I/O races and failures that occur
after startup.

## Why not make a probe commit

`git commit-tree` would test object creation and identity more directly.

It would also write an unreachable object during a nominally diagnostic startup
step.

It still could not prove the later compare-and-swap `update-ref` succeeds after
agents have worked for some time.

The present ticket's pinning rule means later environmental drift must not cause
a silent tier switch anyway.

Checking the exact static prerequisites and letting the real transaction remain
the authoritative fail-closed operation preserves a clean startup boundary.

## Hard-failure text

The core error remains typed and product-neutral.

The CLI formats explicit-commit refusal as a named completion-seal preflight
failure.

The first line identifies `[guards].completion = "commit"` and the missing
requirement.

The remedy always includes these two project-local commands on separate lines:

`git config user.name "You"`

`git config user.email you@example.com`

For a missing repository or unavailable transaction path, the failure also
names that prerequisite before the identity remedy.

This keeps the acceptance assertion stable while leaving the later doctor
ticket room to add the alternate Lisa-history offer.

## Configuration flow

Add a `GuardsConfig` input structure with raw optional `completion: String`.

Add `guards` to `LisaConfig` with `serde(default)`.

Add `[guards]` and `completion` to unknown-key validation tables.

Unknown keys remain warnings, matching existing config convention.

Unknown completion values are semantic validation errors, matching the enum-like
`[agent].client` convention and the ticket requirement.

Add `completion_mode` to `ResolvedConfig`.

The default is `CompletionSealMode::Auto`.

`resolve_config` parses the already-validated raw string and defensively falls
back to auto if it is called with an unvalidated structure.

Add an inert commented example to generated `.lisa.toml`:

`[guards]`

`# completion = "auto" # auto | commit | journal`

The section itself may be active because an empty table is semantically the
same as absence.

## Run pinning and layout flow

Call the real resolver exactly once on the non-dry-run branch of `run_loop`.

Store its return value in a local binding and never call the probe from layout,
plugin, dependency checks, or completion code.

Use the repository root retained by that result when available.

For journal mode without a repository, pass the canonical project root through
the existing `git_root` key for compatibility until journal mechanics introduce
their own path vocabulary.

Add a distinct `completion_seal` KDL key holding `commit` or `journal`.

Add `completion_seal: CompletionSeal` to `PluginConfig`.

Plugin map parsing accepts only the two runtime tiers and falls back to commit
for missing or malformed values.

The commit fallback preserves old layouts and direct plugin tests.

No plugin code performs an environment probe.

Dry-run is not a run and keeps its no-preflight behavior.

Its illustrative layout uses an explicitly configured tier when present and
uses commit for auto, the historical generated-layout behavior.

## Conditional Git dependency

Change the loop dependency helper so Git is required only when the pinned seal
is commit.

The selected agent remains required in both tiers.

Doctor's general checks continue to include Git in this ticket; visibility and
doctor-specific seal reporting are owned by dependent tickets.

This split prevents an explicit journal configuration from being contradicted
by an unconditional loop Git check.

An auto probe with no Git executable resolves journal and then uses the same
provider-only dependency path.

## Considered alternatives

### Put environment probing in lisa-core

Rejected because core completion code is intentionally pure and reusable by the
WASM plugin, where process-spawning Git is unavailable.

### Store `auto` in `PluginConfig` and let the plugin resolve

Rejected because it creates a second probe boundary, cannot run native Git
directly, and violates pin-once semantics.

### Replace `ResolvedConfig.completion_mode` with a seal immediately

Rejected because `resolve_config` has no project root or environment and is used
by status, init, unblock, doctor, and tests outside real loop startup.

### Mutate `ResolvedConfig` in place after probing

Rejected because it conflates configured intent with runtime fact and makes it
possible for callers to treat an unpinned default as resolved.

### Return only `CompletionSeal`

Rejected because the automatic journal reason is needed by the next visibility
and doctor work, and recomputing that reason would violate the no-reprobe rule.

### Treat any repository with identity as transaction-ready

Rejected because today's transaction cannot operate on an unborn branch; the
field incident and current `rev-parse --verify HEAD` code make that a real
capability boundary.

### Change the transaction to support unborn repositories

Rejected as outside this ticket. Tier 1 semantics are explicitly unchanged.

## Verification strategy

Core tests exhaust the three modes against available and each unavailable
support reason.

They assert explicit journal ignores available commit support.

They assert automatic fallback retains its reason.

They assert explicit commit fails without producing a journal result.

Config tests assert default auto, all valid strings, unknown-value actionable
failure, unknown guards-key warning, and generated-template parsing.

CLI resolution tests use an injected probe boundary or controlled command
fixtures to assert that explicit journal performs no probe.

Loop/layout tests assert the pinned runtime key round-trips into `PluginConfig`.

The explicit missing-identity failure test asserts both exact remedy command
lines.

Workspace tests and formatting provide the final regression check.
