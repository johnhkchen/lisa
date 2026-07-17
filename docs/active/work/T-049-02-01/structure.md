# Structure: init-history-offer

## Change inventory

Modify:

- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/tests/help_surface.rs`

Create:

- `crates/lisa-cli/tests/init_history.rs`

Do not modify:

- `crates/lisa-cli/src/commit_transaction.rs`
- `crates/lisa-cli/src/completion_seal.rs`
- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/doctor.rs`
- any ticket phase/status frontmatter
- any shared `docs/active/work/T-049-02-01` artifact path

## `main.rs` command interface

Extend `Commands::Init` with two boolean fields:

```rust
with_history: bool
no_history: bool
```

Clap metadata:

- Long flags are `--with-history` and `--no-history`.
- Each flag conflicts with the other.
- Help copy uses `project history`, not mechanism terminology.
- Both remain optional so terminal calls can prompt.

Dispatch destructures both fields with `dry_run` and `path`.

Dispatch maps flags to `init::HistoryPreference`:

```text
with_history = true  -> WithHistory
no_history = true    -> NoHistory
neither              -> Ask
```

Clap prevents the impossible both-true state before dispatch.

Call shape changes from:

```rust
init::run_init(&path, dry_run)
```

to:

```rust
init::run_init(&path, dry_run, preference)
```

No other subcommand signatures change.

## `init.rs` constants

Add private constants near the imports:

- history offer copy;
- decline consequence copy;
- project history identity name;
- project history identity email;
- initial commit message.

Keeping copy centralized makes exact assertions simple and avoids slightly divergent terminal, dry-run, and fixture strings.

## `init.rs` public preference type

Add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPreference {
    Ask,
    WithHistory,
    NoHistory,
}
```

Visibility is limited to the binary crate's module graph. No library API or cross-crate type changes.

## `init.rs` repository state

Add a private enum:

```rust
enum RepositoryState {
    Missing,
    Unborn { root: PathBuf },
    Born,
}
```

`Born` does not need to retain the root because no history operation follows. `Unborn` retains the discovered top-level root so an invocation below the repository root does not initialize nested metadata.

Add a probe helper:

```rust
fn repository_state(root: &Path) -> RepositoryState
```

It runs `rev-parse --show-toplevel`; failure means missing/unavailable repository. On success it resolves the output path and probes `rev-parse --verify HEAD` at the repository root.

The helper does not write files or config.

## `init.rs` command runner

Add a checked command helper specialized for project-history setup:

```rust
fn run_history_command(command: &mut Command, action: &str) -> Result<(), String>
```

Responsibilities:

- spawn the command;
- require successful exit;
- include action context;
- include non-empty stderr when present;
- never print success before the command succeeds.

This helper is private and does not replace existing command helpers elsewhere.

## `init.rs` initial commit helper

Add:

```rust
fn create_initial_history_commit(repo_root: &Path) -> Result<(), String>
```

It runs an empty commit at the repository root with:

- `--allow-empty`;
- quiet output;
- the stable initial message;
- scoped author name/email;
- scoped committer name/email.

It does not stage paths and does not touch the ordinary index.

## `init.rs` new-repository helper

Add:

```rust
fn initialize_project_history(root: &Path) -> Result<(), String>
```

Ordering:

1. initialize repository metadata at `root`;
2. write local name;
3. write local email;
4. call `create_initial_history_commit(root)`.

Only this helper writes config. It is called exclusively for `RepositoryState::Missing` after acceptance.

## `init.rs` prompt helper

Add a prompt loop over generic buffered input and output:

```rust
fn prompt_for_history(input: &mut impl BufRead, out: &mut impl Write) -> Result<bool, String>
```

It writes and flushes the offer, reads one line, and returns:

- `true` for empty, `y`, or `yes`;
- `false` for `n` or `no`;
- retries for other input;
- a readable error on EOF or I/O failure.

The generic boundary permits deterministic unit tests without a terminal.

## `init.rs` orchestration boundary

Change public `run_init` to accept `HistoryPreference`.

`run_init` owns terminal facts:

- lock stdin and stdout;
- determine whether both are terminals;
- call a new internal I/O-aware function.

Introduce:

```rust
fn run_init_with_io(
    root: &Path,
    dry_run: bool,
    preference: HistoryPreference,
    interactive: bool,
    input: &mut impl BufRead,
    out: &mut impl Write,
) -> Result<(), String>
```

Preserve a small writer-only test wrapper if it reduces churn:

```rust
fn run_init_with_writer(...)
```

That wrapper uses non-interactive input and must receive an explicit preference in tests.

## `init.rs` execution ordering

The I/O-aware orchestration runs in this order:

1. Validate root existence.
2. Detect project and print the existing project summary.
3. Probe repository state.
4. Resolve or prompt for the history decision if state is missing/unborn.
5. Plan and print existing scaffold actions.
6. If dry-run, print no-change completion and return.
7. Apply accepted history bootstrap.
8. Or print the exact decline consequence.
9. Apply scaffold actions with existing mutation tracking.
10. Preserve hook permission behavior.
11. Print existing completion and next-step output.

Dry-run output can show the offer or selected history action, but must not prompt and must not create `.git`.

Born repositories skip steps 4, 7, and 8 entirely.

## Existing test-call updates

Every direct `run_init` call in `init.rs` receives `HistoryPreference::NoHistory` unless the test explicitly targets history.

Every direct `run_init_with_writer` call receives an explicit no-history preference.

This keeps legacy scaffold tests deterministic and prevents test input reads.

Add focused unit tests for:

- offer constant contains benefits and no `git` token;
- prompt accepts default/yes/no and retries invalid input;
- non-interactive `Ask` returns an actionable flag requirement;
- dry-run with accepted history does not create repository metadata.

## `help_surface.rs`

Update only the init operator snapshot.

The expected options list gains:

- `--with-history` description;
- `--no-history` description.

The snapshot remains the exact output of the built binary. The existing operator-jargon test continues to cover the flag descriptions.

Add a small black-box assertion that supplying both flags fails and identifies the conflict if the snapshot suite does not already cover conflict semantics.

## `init_history.rs` fixture helpers

Create helpers for:

- launching the built `lisa` binary with an isolated config environment;
- running repository commands at a fixture path;
- returning trimmed stdout;
- reading local config values;
- recursively snapshotting repository metadata;
- asserting command success with stdout/stderr context.

Use a fixture struct holding temporary directories long enough for every subprocess.

Environment isolation:

- set `HOME` to a fixture directory;
- set `GIT_CONFIG_NOSYSTEM=1`;
- set `GIT_CONFIG_GLOBAL` to a known fixture file;
- retain normal `PATH` so the real repository executable is available.

## `init_history.rs` acceptance fixture

Flow:

1. Create bare project folder and isolated global identity fixture.
2. Run `lisa init --with-history --path <root>`.
3. Assert success and accepted copy.
4. Assert `.git` exists.
5. Assert exact local identity.
6. Assert global config bytes unchanged.
7. Assert `HEAD` resolves and its tree is empty.
8. Write one completion-owned file.
9. Run real `lisa commit-ticket` with one exact include.
10. Assert a second commit exists and contains the file.
11. Run `lisa status`.
12. Assert commit-sealed visibility.

## `init_history.rs` decline fixture

Flow:

1. Create bare project folder.
2. Run `lisa init --no-history`.
3. Assert success.
4. Assert exact consequence sentence.
5. Assert no `.git` path.
6. Run `lisa status`.
7. Assert journal-only visibility.

## `init_history.rs` existing-repository fixture

Flow:

1. Create repository root and an initial commit.
2. Create a nested Lisa project folder.
3. Record `HEAD`, local config bytes, global config bytes, and recursive `.git` snapshot.
4. Run init in the nested folder with `--with-history`.
5. Assert no nested `.git` path.
6. Assert all recorded repository facts and config bytes are identical.

This fixture proves explicit acceptance is idempotent in a born repository rather than causing another initial commit.

## `init_history.rs` unborn fixture

Use two isolated existing repositories:

- Decline case: run with `--no-history`; `HEAD` remains unresolved and config bytes are unchanged.
- Acceptance case: run with `--with-history`; `HEAD` resolves, the commit is empty, and config bytes are unchanged.

The acceptance commit identity is inspected from the commit object and must equal the Lisa project-history identity even though configuration stays absent or unchanged.

## Verification boundaries

Fast verification:

- targeted init unit tests;
- `init_history` integration test;
- `help_surface` integration test;
- `cargo fmt --check`.

Full verification:

- `cargo test --workspace`.

Final ownership check:

- source paths are clean after `lisa commit-ticket`;
- unrelated active-ticket and Lisa journal changes remain untouched;
- ordinary index remains as found.
