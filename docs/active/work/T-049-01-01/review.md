# Review — T-049-01-01 seal types and resolution

## Disposition

Pass.

The ticket's implementation is complete, committed, formatted, and verified.

Both acceptance criteria are satisfied within this ticket's declared boundary.

Tier 1 transaction semantics were not changed.

Journal-sealed completion mechanics remain correctly deferred to T-049-03-01.

## What changed

### `crates/lisa-core/src/completion.rs`

Added `CompletionSeal` with `Commit` and `Journal` runtime tiers.

Added `CompletionSealMode` with `Auto`, `Commit`, and `Journal` configured
stances.

Added stable lowercase parsing, display, and Serde representations.

Added typed commit-support and unavailable-reason vocabulary.

Added immutable `ResolvedCompletionSeal`, retaining why auto selected journal
without treating deliberate journal as a fallback.

Added a typed explicit-commit resolution error.

Added the pure resolution matrix.

### `crates/lisa-core/src/types.rs`

Added the pinned completion seal to `PluginConfig`.

The legacy/default plugin value is commit.

Runtime parsing accepts commit and journal only.

Missing, auto, and malformed map values remain commit-sealed.

This preserves the pre-ticket completion gate for old layouts and fails closed
for invalid native-to-plugin configuration.

### `crates/lisa-cli/src/config.rs`

Added `.lisa.toml` `[guards] completion` input.

Added configured `completion_mode` to `ResolvedConfig`.

The default is auto.

Unknown completion values are actionable validation errors that name the field
and all valid values.

Unknown guard keys remain warnings, matching existing configuration behavior.

The generated configuration includes a commented, inert guard example.

### `crates/lisa-cli/src/completion_seal.rs`

Added the native one-shot environment resolution adapter.

It checks repository discovery, configured `user.email`, a resolvable `HEAD`,
and the repository metadata directory used by the current transaction.

The probe is read-only.

The `FnOnce` seam makes one probe invocation the maximum for a resolution call.

Explicit journal bypasses the probe entirely.

Explicit commit formats a named hard failure and includes:

`git config user.name "You"`

`git config user.email you@example.com`

The per-run wrapper retains the pinned core resolution and Git root from the
same probe.

### `crates/lisa-cli/src/main.rs`

Registered the new internal completion-seal module.

No new CLI command was added.

### `crates/lisa-cli/src/doctor.rs`

Made the shared loop dependency set conditional on whether the pinned tier
requires Git.

Commit loops still require Git.

Journal loops require only their configured agent provider.

Doctor continues checking Git as before; doctor seal wording belongs to the
dependent visibility/doctor tickets.

### `crates/lisa-cli/src/loop_cmd.rs`

Added the real-run pinning boundary immediately after cheap project/protocol
checks.

One immutable result now drives Git dependency selection, Git-root reuse, and
the layout tier.

Removed real-run Git root re-discovery.

Generated KDL now contains `completion_seal "commit"` or
`completion_seal "journal"`, never auto.

Explicit journal is honored in Git-capable environments because it is resolved
before and independently of dependency checks.

Repo-less journal runs use the canonical project root for the existing path
transport slot.

Dry-run remains probe-free.

### `crates/lisa-cli/tests/zellij_version_preflight.rs`

Updated the missing-Git integration contract.

The old assertion required every loop to abort without Git.

The new regression proves an unconfigured auto loop proceeds journal-sealed and
does not emit the prior Git dependency failure.

Doctor's missing-Git diagnosis test remains unchanged and passing.

## Acceptance review

### Environment resolution matrix

Repo/identity/transaction support available resolves commit under auto.

Repository missing resolves journal under auto.

Identity missing resolves journal under auto.

Transaction path unavailable resolves journal under auto and retains the typed
reason.

Identity missing under explicit commit returns a hard error.

The CLI error test asserts the named preflight, configured commit field, and
both remedy command lines.

Explicit journal resolves journal even when commit support is available.

Invalid config values fail validation and list `auto, commit, journal`.

The missing-Git integration test exercises the real process boundary and proves
the unconfigured default no longer resolves to an unavailable requirement.

### Pin once and expose

The core resolution result is immutable and exposes the pinned tier.

The core result also exposes the retained automatic-fallback reason for later
doctor/status consumers.

The CLI uses one `FnOnce` probe to create one `RunCompletionSeal`.

Invocation-count tests prove auto/commit invoke the probe once and explicit
journal invokes it zero times.

`run_loop` stores one result and reuses it for every startup consumer.

The plugin receives a concrete `CompletionSeal` via KDL and cannot re-probe.

No auto runtime variant exists, preventing delayed/mid-run selection.

## Test coverage

Core tests cover type strings, defaults, parsing, serialization-compatible
lowercase values, every resolution branch, fallback reason retention, and hard
explicit-commit failure.

Plugin-config tests cover valid tiers and legacy/malformed fail-closed behavior.

CLI config tests cover all documented values, default auto, generated template,
unknown key warning, and actionable unknown value.

CLI resolver tests cover probe cardinality, explicit journal bypass, fallback
reason, pinned root, and exact identity remedy.

Doctor tests cover commit-versus-journal dependency sets.

Loop tests cover both emitted pinned tier strings and absence of auto.

The Zellij preflight integration suite covers real missing-Git auto behavior.

Full workspace verification covers the existing atomic transaction, completion
state machine, recorded livelock, hostile ordering, nested repository, provider,
claim, scheduler, and UI regressions.

## Verification results

`cargo fmt --all -- --check` passed.

`cargo test --workspace` passed.

`just check` passed:

- `cargo check -p lisa-plugin --target wasm32-wasip1`;
- complete workspace tests.

The existing real-Zellij integration remained ignored because it requires an
external live environment; this ticket does not change that test's policy.

## Commit and ownership review

Four Lisa-managed ticket commits were created:

- `4526486` — core seal domain and plugin transport;
- `a0cba73` — config parsing and native resolver;
- `a6a6c2c` — loop pinning and conditional dependency integration;
- `b7f93d5` — missing-Git auto integration contract.

Every commit used `lisa commit-ticket` with exact repository-relative includes.

No ordinary index command or ordinary commit was used.

No ticket-owned source remains staged, modified, or untracked.

The remaining worktree entries are Lisa-managed ticket/provenance/publication
state or another ticket's artifacts.

## Open concerns and deferred work

No blocking concern remains for this ticket.

The transaction-readiness probe checks today's static prerequisites without
creating a probe commit. A later filesystem race can still make the real
transaction fail; that transaction remains fail-closed, and bounded parking is
owned by S-049-04.

An unborn repository resolves journal because today's transaction requires
`HEAD`. Supporting root commits would change Tier 1 behavior and is intentionally
out of scope.

Journal completion is only typed and routed here. Hashing, journal authorization,
and journal-sealed status publication remain T-049-03-01.

Doctor/status copy and ledger seal fields remain T-049-01-02/T-049-02-02.

History initialization and project-local identity remain S-049-02.

These are declared dependency boundaries rather than gaps in this delivery.
