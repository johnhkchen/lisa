# Research — T-049-01-01 seal types and resolution

## Ticket boundary

The ticket introduces completion-seal vocabulary and resolves a configured guard
stance against the environment at loop startup.

The two seal tiers are `commit` and `journal`.

`commit` names the existing isolated atomic Git transaction.

`journal` is only a contract in this ticket; its completion mechanics belong to
T-049-03-01.

The configured values are `auto`, `commit`, and `journal`, with `auto` as the
unconfigured default.

The acceptance boundary requires environment-matrix tests, actionable config
validation, a hard failure for unsatisfied explicit commit, and one pinned result
per run.

Doctor/status wording and ledger fields belong to dependent ticket T-049-01-02.

Journal hashing and journal-driven completion belong to T-049-03-01.

History initialization and identity setup belong to S-049-02.

Completion-failure parking belongs to S-049-04.

## Core completion domain

`crates/lisa-core/src/completion.rs` is the pure completion-domain module.

It contains typed completion, attempt, correlation, deadline, state, event,
effect, rejection, and retryability vocabulary.

The module deliberately performs no I/O and has no CLI, Git, scheduler, or
Zellij dependency.

This is the existing natural home for a `CompletionSeal` domain type.

`crates/lisa-core/src/lib.rs` exposes `completion` as a public module.

Core already depends on Serde and `thiserror`, so seal values can support stable
serialization and typed resolution errors without adding dependencies.

Existing core enums commonly derive `Debug`, `Clone`, `Copy`, equality, hashing,
and Serde traits where they cross storage or configuration boundaries.

Stable lowercase strings are generally supplied through `Display` or explicit
parse methods rather than inferred by callers.

## CLI configuration

`crates/lisa-cli/src/config.rs` owns `.lisa.toml` parsing, validation, defaults,
and CLI/file/default precedence.

`LisaConfig` mirrors top-level TOML sections as optional/raw input.

The current top-level sections are `version`, `dirs`, `scheduling`, `agent`, and
`runtime`.

There is no `[guards]` section yet.

Unknown top-level sections and unknown keys inside known sections currently
produce warnings.

Known enum-like values use raw strings in the input structure and semantic
validation after TOML deserialization.

`[agent].client` is the clearest convention: it remains a `String`, is parsed by
a domain parser, and unknown values become actionable validation errors.

`[runtime].zellij` follows a similar semantic-validation path with custom text.

`ResolvedConfig` applies defaults but currently contains only configuration that
can be resolved without inspecting the project environment.

`resolve_config` is called by loop, doctor, status, unblock, and init paths.

Those calls happen before loop-specific external dependency checks.

The generated default `.lisa.toml` text comes from `default_config_toml()` in
the same module.

Config tests are colocated in `config.rs` and cover parsing, defaults, unknown
keys, semantic failures, and generated-template behavior.

## Loop startup and pinning boundary

`crates/lisa-cli/src/loop_cmd.rs::run_loop` is the single native loop-start
boundary.

It first checks project structure and protocol version.

Dry-run returns through `run_dry` before external dependency or runtime checks.

A real run then resolves Zellij exactly once and keeps the resolved executable
for reporting and launch.

This existing runtime pattern demonstrates the repository convention for a
configured intent that becomes an immutable per-run choice.

Provider dependencies are checked after Zellij resolution.

Git is currently an unconditional required dependency for every loop.

The loop currently discovers a Git root unconditionally and fails when the
project is not inside a repository.

That root is passed into the generated KDL layout as `git_root`.

The layout is the native-to-WASM configuration boundary.

`generate_layout` receives already-resolved native values and renders string
keys for `PluginConfig::from_config_map`.

There is no existing completion-seal key in the layout.

Dry-run uses the project directory as a fallback Git root so it can remain
useful for uncommitted projects.

## Plugin configuration

`crates/lisa-core/src/types.rs::PluginConfig` is parsed from the Zellij string
configuration map.

It contains `git_root`, work directories, scheduling values, client routing,
the Lisa binary path, and provider caps.

`PluginConfig::from_config_map` is deliberately lenient; native CLI validation
is the gate, and malformed plugin-map values fall back rather than panic.

`PluginConfig::new` supplies defaults for legacy layouts and direct tests.

The plugin copies `config.git_root` into scheduler state.

The existing completion command builder uses that Git root to construct exact
repository-relative paths for `complete-ticket`.

Dependent tickets will need the pinned seal on this side of the KDL boundary so
doctor/status/ledger writers and completion mechanics do not re-probe.

## Existing Git transaction requirements

`crates/lisa-cli/src/commit_transaction.rs` owns the isolated transaction.

Repository discovery uses `git rev-parse --show-toplevel` and
`--absolute-git-dir`.

The transaction acquires `.lisa-commit.lock` and reserves an alternate index.

It snapshots the ordinary index, initializes the alternate index from `HEAD`,
stages only exact includes, writes a tree, creates a commit with `commit-tree`,
and atomically advances `HEAD` with `update-ref`.

The ordinary index is reconciled after the ref advance.

The transaction explicitly runs `git rev-parse --verify HEAD` before it can
initialize its alternate index.

Consequently, an unborn repository is not satisfiable for the current commit
seal even when Git and an identity exist.

`commit-tree` relies on Git author/committer identity.

The ticket explicitly names `git config user.email` as the identity probe.

Existing transaction tests configure both `user.name` and `user.email` in
fixtures.

No reusable preflight API currently reports whether this transaction is
satisfiable without changing repository state.

## Dependency and doctor behavior

`crates/lisa-cli/src/doctor.rs` builds dependency checks shared by doctor and
loop preflight.

Git and the selected agent client are both currently required.

`check_required_deps` returns rendered failures for real loop startup.

Doctor adds Zellij, embedded-WASM, optional WASM-target, project-version, cache,
and Codex-trust diagnostics.

The dependent T-049-02-02 explicitly owns richer doctor identity diagnostics.

For journal mode to be honored without Git, the loop dependency boundary must
be able to require the provider while not requiring Git.

## Tests and fixtures

Core unit tests are colocated within source modules.

CLI config and loop tests are also colocated.

`tempfile` is available to both core tests and CLI tests.

Many Git-facing tests create a temporary repository, configure a deterministic
identity, create an initial commit, and then exercise the behavior.

Loop layout tests assert exact rendered configuration fragments.

The requested environment matrix can be separated into pure core resolution
tests and CLI probe tests using disposable repositories.

Pure tests can prove all resolution branches without depending on the executing
machine's global Git configuration.

CLI probe tests must force repository-local identity state to avoid inheriting
developer-global config.

Git's `--local` configuration alone does not suppress inherited global identity;
tests can isolate `HOME`/global configuration or invoke Git with explicit config
environment when needed.

## Constraints and assumptions

The resolved seal must not be recomputed by the plugin or completion writers.

The configured stance and the pinned seal are distinct concepts.

`auto` is a request, not a runtime seal and should not appear in ledger rows.

Explicit `journal` must not unnecessarily execute Git probes.

Explicit `commit` must never silently fall back to journal.

Auto may fall back to journal for a missing repository, missing identity, or an
unsatisfied transaction prerequisite.

The current transaction's required `HEAD` is an observable transaction-path
prerequisite and can be checked without creating a commit.

The ticket requires the two-line identity remedy to remain in the explicit
commit failure text when identity is absent.

Ticket-owned implementation files must be committed with exact include paths
through `lisa commit-ticket`.

Attempt artifacts remain private under
`.lisa/attempts/T-049-01-01/1/work/` and are not ticket source includes.
