# Progress — T-049-01-01 seal types and resolution

## Outcome

Implementation is complete and committed through Lisa's isolated transaction.

The completion seal ladder is now typed in lisa-core.

`.lisa.toml` accepts `[guards] completion = "auto" | "commit" | "journal"`.

The unconfigured default is auto.

A real loop probes commit support once, pins the result, and transports only the
runtime `commit` or `journal` tier into the plugin layout.

Explicit journal avoids Git probing and no longer requires Git as a loop
dependency.

Explicit commit fails rather than degrading and includes the required two-line
identity remedy.

## Completed — core domain

Added `CompletionSeal` with commit and journal tiers.

Added `CompletionSealMode` with auto, commit, and journal intent.

Added stable lowercase parsing, display, and Serde forms.

Added typed commit unavailability categories:

- repository missing;
- identity missing;
- transaction path unavailable with detail.

Added `CommitSealSupport` as the pure environment adapter input.

Added immutable `ResolvedCompletionSeal` with the pinned tier and optional auto
fallback reason.

Added typed `CompletionSealResolutionError` for explicit commit refusal.

Added the pure resolution matrix.

Added unit tests for every configured stance and environment support branch.

## Completed — plugin transport

Added `completion_seal` to `PluginConfig`.

Legacy/missing plugin configuration defaults to commit.

Only commit and journal are accepted in the runtime map.

`auto`, unknown, and malformed values retain commit, so an untrusted layout
cannot silently weaken the existing completion gate.

Added plugin-config tests for commit, journal, auto, unknown, and absent values.

## Completed — CLI configuration

Added `GuardsConfig` and the new top-level `[guards]` section.

Added resolved `completion_mode` to `ResolvedConfig`.

Defaulted missing configuration to auto.

Added semantic validation through the core parser.

Unknown completion values now name `[guards].completion` and list all accepted
values.

Unknown `[guards]` keys retain the repository's warning convention.

Added an inert commented completion setting to generated `.lisa.toml`.

Added config tests for defaulting, all valid values, actionable invalid values,
unknown keys, and generated template behavior.

## Completed — one-shot environment resolution

Created `crates/lisa-cli/src/completion_seal.rs`.

Added immutable `RunCompletionSeal`, which carries the core resolution and the
repository root learned by that same probe.

Used a `FnOnce` seam so one resolution operation cannot call its probe twice.

Explicit journal short-circuits without invoking the probe.

The real probe checks, in order:

1. `git rev-parse --show-toplevel`;
2. `git config --get user.email`;
3. `git rev-parse --verify HEAD`;
4. `git rev-parse --absolute-git-dir` and the resolved metadata directory.

The checks are read-only and match the static prerequisites of today's isolated
transaction.

The existing transaction remains the fail-closed authority for later I/O races.

Added invocation-count tests proving one probe for auto/commit and zero for
explicit journal.

Added exact assertions for both identity remedy commands.

## Completed — loop integration

Moved completion-seal resolution to the real-run branch immediately after cheap
project/protocol validation and before runtime side effects.

The result is held in one local binding for the rest of startup.

Removed real-run Git root re-discovery; the root from the seal probe is reused.

Repo-less journal runs use the canonical project root for the existing layout
path slot.

Made Git a conditional loop dependency: required for pinned commit and omitted
for pinned journal.

Doctor continues to check Git in this ticket; doctor seal visibility belongs to
the dependent work.

Added `completion_seal "commit|journal"` to generated KDL.

Dry-run remains environment-probe-free and never emits auto into runtime config.

Added layout and dependency tests for journal and commit.

## Commits

`452648630a1e5b2d8342df3c494196c15def9c70`

`Add completion seal domain types`

Exact includes:

- `crates/lisa-core/src/completion.rs`
- `crates/lisa-core/src/types.rs`

`a0cba73962386c8527fc9a1c16746c6942bae71e`

`Resolve completion seal configuration`

Exact includes:

- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/completion_seal.rs`
- `crates/lisa-cli/src/main.rs`

`a6a6c2c3fb8f05edabc6be7f37d5608ed931a0c6`

`Pin completion seal at loop startup`

Exact includes:

- `crates/lisa-cli/src/completion_seal.rs`
- `crates/lisa-cli/src/doctor.rs`
- `crates/lisa-cli/src/loop_cmd.rs`

`b7f93d59da39bd3391b12de36b5871eccdae2844`

`Update missing Git loop contract`

Exact include:

- `crates/lisa-cli/tests/zellij_version_preflight.rs`

## Verification

Focused core verification:

`cargo test -p lisa-core`

Passed 225 core unit tests, the generated completion state-machine test, the
recorded livelock regression, and doc tests.

Focused CLI verification:

- completion-seal tests: 3 passed;
- config tests: 60 passed;
- doctor tests: 49 passed;
- loop tests: 24 passed;
- Zellij preflight integration: 6 passed.

Full verification:

`cargo fmt --all -- --check`

Passed.

`cargo test --workspace`

Passed across lisa-cli, lisa-core, lisa-plugin, all integration regressions, and
doc tests. The explicitly environment-gated real-Zellij test remained ignored
by its existing contract.

`just check`

Passed the `wasm32-wasip1` plugin check and repeated the complete workspace test
suite successfully.

## Deviations from plan

The planned three source units became four commits.

Full workspace verification found an existing integration test that asserted
missing Git always aborts loop startup.

That assertion contradicted this ticket's new default-auto contract.

The test was updated in its own exact-path Lisa commit to prove a missing-Git
auto loop proceeds on the journal tier.

The loop integration commit also included a small cleanup to the newly created
resolver module: a crate-private fallback-reason getter was removed after only
the pinned tier and Git root proved necessary at the CLI call site. The core
`ResolvedCompletionSeal` still exposes the fallback reason for dependent
callers.

No transaction implementation was changed.

No journal hashing/completion behavior was implemented.

No doctor/status/ledger visibility work from dependent tickets was absorbed.

## Ownership and worktree

All ticket-owned source changes were committed with `lisa commit-ticket` and
exact repository-relative includes.

No ordinary `git add`, `git add -A`, or ordinary `git commit` was used.

No ticket-owned source path remains staged, modified, or untracked.

Remaining worktree changes are Lisa-managed metadata/ticket publication or
another ticket's work artifacts and were not touched or committed here.
