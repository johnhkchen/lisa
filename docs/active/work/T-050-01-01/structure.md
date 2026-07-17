# Structure: init-history-default

## Change map

The implementation remains inside the existing init history boundary. No new modules,
dependencies, public commands, or configuration fields are introduced.

Modified source and test files:

- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/tests/init_history.rs`

Modified public documentation:

- `README.md`
- `docs/knowledge/chromebook-install-test.md`

Attempt-private artifacts:

- `research.md`
- `design.md`
- `structure.md`
- `plan.md`
- `progress.md`
- `review.md`
- `review-disposition.json`

No files are created outside the private artifact directory. No repository file is
deleted or renamed.

## `crates/lisa-cli/src/init.rs`

### Copy constants

Add one private constant beside the existing offer and decline strings:

```text
HISTORY_KEPT = "Keeping project history — finished work will be undoable."
```

Responsibilities:

- provide one exact positive announcement surface;
- avoid duplicated literals across action branches;
- allow direct copy-focused unit assertions;
- remain private because it is CLI output, not a library contract.

Keep `HISTORY_OFFER`, `HISTORY_DECLINED`, identity, email, and commit-message constants
unchanged.

### `RepositoryState`

Extend the private enum with:

```rust
Unavailable { reason: String }
```

The variant represents inability to inspect or create project history before mutation.
The reason is retained for explicit-request diagnostics. Other variants keep their
current meaning:

- `Missing`: no repository found and Git commands are available;
- `Unborn { root }`: repository exists with symbolic but unresolved `HEAD`;
- `Born`: repository exists with resolved `HEAD`.

This enum remains private and does not affect CLI parsing or external APIs.

### `repository_state`

Keep the existing single `rev-parse --show-toplevel` entry probe.

Change result mapping:

- executable not found -> `Unavailable` with a stable Git-not-available reason;
- other process-launch error -> `Unavailable` with existing contextual detail;
- ordinary “not a git repository” -> `Missing` unchanged;
- unexpected nonzero inspection -> `Unavailable` retaining formatted failure;
- empty successful root -> `Unavailable` retaining the existing no-result detail;
- `HEAD` verification success -> `Born` unchanged;
- valid symbolic `HEAD` -> `Unborn { root }` unchanged;
- errors spawning or interpreting follow-up probes -> `Unavailable`.

The function still returns `Result<RepositoryState, String>` unless mechanical cleanup
shows the error arm is unreachable. Keeping the signature minimizes caller churn and
allows truly internal formatting failures to remain representable.

No history mutation occurs in this function.

### Explicit unavailable error helper

Add a small private formatter if it improves match clarity. It accepts the preserved
reason and returns one stable error containing:

- that project history was requested;
- why it cannot be honored;
- instruction to install or repair Git;
- `lisa init --with-history` as the retry;
- `lisa init --no-history` as the journal override.

The formatter must not be used by the offer path, so the offer retains its no-`git`
wording rule.

### `resolve_history_action`

Retain the existing parameters and result type. Reorder logic around state and
preference rather than using one accepted boolean that loses capability information.

Boundary rules:

1. `Born` returns `None` before prompting or inspecting preference.
2. Dry-run never reads input.
3. `WithHistory + Unavailable` returns the actionable error.
4. `NoHistory` returns `Decline` for every non-born state.
5. `Ask + interactive` reads the existing prompt.
6. An interactive rejection returns `Decline`.
7. An interactive acceptance plus `Unavailable` returns `Decline`.
8. `Ask + non-interactive + Unavailable` returns `Decline`.
9. `Ask + non-interactive + Missing` returns `CreateRepository`.
10. `Ask + non-interactive + Unborn` returns `CreateInitialCommit`.
11. Explicit or interactive acceptance in usable states maps identically.

One local helper may translate usable repository states to history actions. If used,
it stays private and exhaustive.

Remove the obsolete non-interactive “choose a flag” error and the dry-run “choose a
flag later” instruction.

### `run_init_with_io` history reporting

Keep repository resolution before project detection and scaffolding.

Dry-run action reporting:

- creation actions -> a prospective keep-history line;
- decline -> existing `HISTORY_DECLINED`;
- none -> no history line.

Real action reporting:

- after `initialize_project_history` succeeds -> exact `HISTORY_KEPT`;
- after `create_initial_history_commit` succeeds -> exact `HISTORY_KEPT`;
- decline -> exact existing consequence line;
- none -> no added line.

Keep blank-line placement compatible with current output shape.

### Unit tests

Preserve the prompt-copy and parsing tests.

Replace `noninteractive_init_requires_an_explicit_history_flag` with a test proving
bare non-interactive init creates history when the executable is usable. It should use
the real temporary-directory path and verify `.git` plus announcement output.

Add resolver-focused tests for `Unavailable`:

- no-flag non-interactive -> `Decline`;
- no-flag interactive accept -> `Decline` after writing the offer;
- explicit `WithHistory` -> actionable error with Git repair and no-history remedy;
- explicit `NoHistory` -> `Decline`.

Update dry-run assertions to prove no flag previews the automatic keep decision and
does not prompt. Retain the no-mutation assertion.

The copy test adds the exact positive constant assertion and retains the offer’s
no-`git` guarantee.

## `crates/lisa-cli/tests/init_history.rs`

### Constants

Add a test-local `HISTORY_KEPT` literal matching the ticket’s exact sentence. Keep
existing offer and decline constants unchanged.

### Fixture command environment

Add a helper that runs Lisa with a controlled `PATH` lacking Git. The helper should
not mutate the process-global environment. It should reuse the fixture’s isolated
`HOME`, system-config, and global-config setup.

Because Lisa itself is invoked through `CARGO_BIN_EXE_lisa`, clearing `PATH` does not
prevent launching Lisa. Init’s child `git` lookup will fail predictably.

If platform runtime commands are required by any subcommand, use an empty temporary
directory as `PATH` rather than relying on host directories. The init and status paths
under test should require no external executable in journal mode.

### Fresh usable-history fixture

Rename the accepted bare-folder test to communicate that no flag is the contract.
Remove `--with-history` from its Lisa invocation.

Retain every existing assertion for:

- exit zero;
- `.git` directory;
- local Lisa identity;
- unchanged global config;
- resolved 40-character `HEAD`;
- root commit author/email/message;
- empty root tree;
- later exact-path `commit-ticket` success;
- commit-sealed status.

Add an exact `HISTORY_KEPT` assertion.

### No-Git default fixture

Add a black-box test that runs bare init with Git absent from `PATH`.

Assertions:

- exit status is success;
- stdout includes exact `HISTORY_DECLINED`;
- `.git` remains absent;
- scaffold output exists;
- a fixture ticket can be written;
- status succeeds under the same no-Git environment;
- status reports the existing journal-only seal line.

### Explicit-with-history no-Git fixture

Add a black-box test using the same environment and `--with-history`.

Assertions:

- exit status is failure;
- stderr names Git as unavailable;
- stderr contains install/repair guidance;
- stderr contains the `--no-history` alternative;
- `.git` is absent;
- scaffold files were not created.

### Existing flag and offer coverage

Change the current flag-contract test so it no longer expects bare failure. Keep:

- conflicting flag rejection through Clap;
- dry-run offer/copy checks only if still applicable;
- the exact no-`git` offer wording rule.

If dry-run no longer prints the offer, move that copy assertion to unit coverage rather
than manufacturing interactive integration input.

### Born and unborn snapshots

Do not remove or weaken any snapshot assertion. Keep explicit flags where they isolate
the prior safety contract. The tests must continue proving:

- born repository metadata tree unchanged;
- local config unchanged;
- global config unchanged;
- `HEAD` unchanged;
- no nested `.git` directory;
- unborn decline config/index unchanged and `HEAD` unresolved;
- unborn acceptance config/index unchanged;
- staged user file remains staged;
- initial tree remains empty.

## `README.md`

### Quick Start

Keep the bare command block. Replace the script/agent flag requirement with automatic
decision prose:

- available project history is kept and announced;
- unavailable history falls back to Lisa’s journal and is announced;
- interactive sessions still offer the choice;
- flags are overrides, not prerequisites.

Keep examples for both flags, labeled as explicit overrides.

### CLI reference

Update command comments so bare init is the normal automatic path. Describe history
flags as forcing yes/no outcomes. Remove the sentence that scripts and agents must
pass one. Preserve conservative rerun and existing-repository guidance.

## `docs/knowledge/chromebook-install-test.md`

### No-Git completion leg

Change the generated instruction from `lisa init --no-history` to bare `lisa init`.
Explain that this intentionally exercises automatic journal fallback with Git absent.

### Snapshot/start leg

Delete the designed-error comment for bare init. Change the command from
`lisa init --no-history` to bare `lisa init`. Explain that the default keeps history
when the prepared environment has Git and falls back otherwise.

Mention history flags only as deliberate overrides for a test that specifically needs
to force a branch.

## Commit boundaries

Commit implementation and fixtures as one meaningful source unit because the behavior
and its acceptance tests form one contract:

```text
crates/lisa-cli/src/init.rs
crates/lisa-cli/tests/init_history.rs
```

Commit public documentation as a second meaningful unit:

```text
README.md
docs/knowledge/chromebook-install-test.md
```

Private attempt artifacts are not committed directly; Lisa publishes admitted work
after lease verification.

## Verification boundaries

- format changed Rust files;
- run init unit tests;
- run `init_history` integration tests;
- run help-surface tests to prove no accidental CLI drift;
- run all `lisa-cli` tests;
- run workspace tests or `just check` if time and environment permit;
- inspect ticket-owned status for no uncommitted source or docs;
- validate final review disposition through Lisa.
