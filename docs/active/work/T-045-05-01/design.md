# Design — T-045-05-01 real Codex/Zellij field harness

## Decision summary

Create a dedicated live shell harness and companion runbook.

The harness will run two isolated real-Codex/real-Zellij cases in sequence.

The `legacy` case uses an explicitly supplied pre-E-045 Lisa executable.

The `current` case uses a freshly built release CLI and embedded WASM from HEAD.

Both cases begin with the same synthetic Review ticket and existing canonical review evidence.

Both use an ephemeral authenticated Codex home with hooks disabled.

Both record rather than synthesize pane signals.

The legacy case is successful only when it observes the expected false delivery failure.

The current case is successful only when it observes claim evidence and avoids delivery failure.

The later T-045-05-02 ticket can strengthen the scaffold with every epic-level assertion.

## Option 1 — extend `live_provider_startup.sh`

The existing harness already owns most PTY, Zellij, evidence, and cleanup machinery.

Extending it would minimize duplicated shell helpers.

It would also mix two independent field contracts.

The existing script compares providers, starts in Research, and requires hooks.

This ticket compares Lisa delivery generations, starts in Review, and forbids hooks.

Its existing `verify_launch_contract` rejects exactly the new assignment-path launcher.

Its Codex state verifier requires an ack signal that this ticket deliberately removes.

Adding modes would put incompatible assumptions behind many conditionals.

That would make the older field proof harder to reproduce and review.

Decision: reject modification of the existing harness.

## Option 2 — add a mode to the deterministic stub harness

`real_zellij_delivery_boundary.sh` already demonstrates missing acknowledgement.

It has excellent bounded polling and failure diagnostics.

A real-Codex switch could reuse most fixture functions.

The script's provider is intentionally a local stub named `claude`.

Its event ledger is produced by that stub, not observed from installed provider behavior.

Mixing real and stub evidence risks treating the stub's exact timestamps as Codex facts.

Its purpose is model-free regression coverage and it runs through an ignored Cargo test.

The new story explicitly says stub acknowledgement cannot close acceptance.

Decision: keep it unchanged and borrow only its small shell patterns.

## Option 3 — dedicated `live_codex_review_boundary.sh`

A dedicated script gives the field run one coherent contract.

Its name exposes both the live provider and Review recovery boundary.

It can require explicit old/new executable identity without affecting other controls.

It can encode metering and authentication warnings at the entry point.

It can retain raw evidence for the dependent assertion ticket.

It duplicates some generic Bash functions, but those functions are short and stable.

The isolation cost is preferable to coupling incompatible test semantics.

Decision: choose this option.

## Old-path subject selection

The harness cannot reconstruct the old behavior from current source without reverting code.

Reverting in the shared checkout would violate ticket isolation and risk concurrent work.

A Git worktree build would be possible but expensive and would need a precise historical commit.

The 2026-07-13 incident involved an installed rc.8 executable.

The current environment has that installed executable and its launch shape is directly observable.

The harness will therefore require `LEGACY_LISA_BIN`.

It defaults to the current `command -v lisa` only when explicitly executable.

The current subject defaults to freshly rebuilt `target/release/lisa`.

The script records absolute paths, SHA-256 hashes, versions, and launch-command capabilities.

It refuses identical old/new hashes.

This makes the comparison an explicit binary-boundary experiment rather than a version-string claim.

## Current-path build decision

The canonical run builds release WASM first and release CLI second via `just build-cli`.

This refreshes the CLI's embedded plugin before any fixture starts.

`SKIP_BUILD=1` is available only with an explicit `CURRENT_LISA_BIN`.

Preparation mode still performs build and non-provider validation.

The generated layout is copied into case evidence.

The extracted WASM hash is compared with the freshly built target WASM for the current case.

The legacy case records its generated layout and extracted plugin hash without comparing to HEAD.

## Fixture topology

Each case uses its own external `mktemp` repository.

Each repository contains one story and one ticket.

The ticket has `status: open` and `phase: review`.

It names Codex explicitly.

Canonical `docs/active/work/<ticket>/review.md` already exists.

That document states that prior phases produced no source changes.

The fixture Git baseline commits all initial content.

Lisa then sees an existing Review ticket on first plugin load.

This matches the recovery form of the reported T-014/T-015 failures without copying private work.

## Claim-first timing design

The disposable `AGENTS.md` supplies the field-only timing protocol.

Codex reads project instructions automatically before its first tool action.

The first tool action is one shell command.

That command searches only the exact current attempt work directory.

If it finds `assignment-<attempt>-<nonce>.md`, it derives the nonce from that filename.

It sleeps for a bounded current-path delay and invokes inherited `$LISA_BIN claim`.

Only after that command returns may Codex inspect Review work and write disposition artifacts.

If no nonce assignment exists, the same first command sleeps for a longer legacy delay.

The legacy branch never fabricates a claim.

The current delay intentionally exceeds the first ack timeout but not the passive claim deadline.

This makes `delivered-awaiting-claim` observable before the claim arrives.

The legacy delay intentionally exceeds both old delivery windows.

This gives the hook-dependent path time to reach its false terminal failure before output appears.

The delays are harness environment variables with validated positive defaults.

They are not production scheduler changes.

## Hooks-disabled design

Each case creates a separate ephemeral `CODEX_HOME`.

The operator's existing `auth.json` is linked, never copied.

No hooks file is installed.

`config.toml` contains `[features] hooks = false`.

Lisa may append its project-trust table to the file.

The harness records a small receipt proving the false setting and trusted canonical root.

Cleanup removes the whole home and authentication symlink on success or failure.

Expected hook signal counts are observations, not injected controls.

## Evidence capture design

The evidence root has build metadata and one directory per case.

A background sampler runs every 100 ms.

It captures dashboard and terminal snapshots with UTC timestamps.

It records first observation of scheduler vocabulary.

It copies every newly observed pane signal into a case-local signal capture directory.

Each capture filename includes timestamp, source basename, and a sequence number.

It records a TSV row with timestamp, basename, byte count, and digest.

It samples relevant `ps` rows for `launch-codex`, Codex, and the case root.

This supplies launcher-spawn observation independent of terminal rendering.

At finalization it copies launch scripts, assignment files, ticket, prior/current work, provenance,
completion journal, layout, Git log, status, pane manifest, and final screens when present.

On failure it prints bounded diagnostics and preserves evidence.

## Case outcome boundary

The legacy harness waits for a dashboard failure associated with the fixture ticket.

It requires no captured `.claim` signal.

It requires no admitted current-attempt Review artifact before failure.

It captures and terminates the case immediately after the false failure observation.

This prevents the still-live model from later obscuring the reproduction.

The current harness waits for a copied `.claim` signal or claim activity evidence.

It waits for `owned` and then durable ticket completion.

It rejects any dashboard/terminal snapshot containing delivery failure.

It records the current launch script and nonce assignment.

The complete assertions for stale claim, exactly-one completion, and clean successor TUI belong to
T-045-05-02, which consumes this scaffold and raw capture format.

## Preparation mode

`PREPARE_ONLY=1` must not start Codex or Zellij sessions.

It checks dependencies and authentication.

It builds the current subject unless skipped.

It proves old and new binaries differ.

It creates and validates both disposable repositories.

It checks shell syntax and expected fixture contents.

It records a `PREPARED` receipt.

Fixture and evidence retention make preparation useful for a separately authorized operator run.

## Documentation decision

Add `docs/knowledge/live-codex-review-boundary.md`.

The runbook will lead with metering and explicit authorization.

It will describe prerequisites, canonical invocation, safe preparation, overrides, evidence layout,
and interpretation limits.

It will state that same version strings do not prove binary identity.

It will distinguish observed live facts from deterministic predecessor tests.

It will explain that a legacy failure is expected and is not a harness process failure.

## Rejected production changes

Do not add a scheduler-only test pretending to be live evidence.

Do not add hook shims when hooks are disabled.

Do not alter claim admission or assignment timeout constants.

Do not teach production assignment text about field delays.

Do not store provider authentication in the repository or evidence tree.

Do not make the harness a default Cargo test.

Do not publish captured live transcripts as committed fixtures in this ticket.

## Verification standard

Non-metered verification consists of Bash syntax, optional ShellCheck, preparation mode, and the
existing focused Rust suites for launcher, claim, scheduler wait, and completion boundaries.

The live acceptance run is separately identifiable and uses installed Codex/Zellij.

If the live run produces an unexplained state, the harness exits nonzero and retains evidence.

Passing this ticket means the scaffold exists and its authorized comparison is reproducible.
