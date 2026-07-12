# Research: T-038-02-01 cargo-fmt-clean

## Ticket scope

- The ticket is `T-038-02-01`, titled `cargo-fmt-clean`.
- It belongs to story `S-038-02`.
- Its stated goal is to bring the workspace to canonical Rust formatting.
- Its only acceptance criterion is that `cargo fmt --all -- --check` exits zero.
- The criterion additionally constrains any ticket diff to formatting-only changes.
- The ticket begins in the Research phase.
- Its three dependencies are complete in the current history.
- Those dependencies are `T-038-01-01`, `T-038-01-02`, and `T-038-01-03`.
- The current `HEAD` is the completion commit for `T-038-01-01`.
- The other two dependency completion commits are also ancestors of `HEAD`.

## Repository shape

- The root is a Cargo workspace.
- The workspace manifest is `Cargo.toml`.
- The workspace uses Cargo resolver version 2.
- The workspace member glob is `crates/*`.
- There are three current Rust crates under that glob.
- `crates/lisa-core` contains shared domain and scheduling types.
- `crates/lisa-cli` contains the command-line application.
- `crates/lisa-plugin` contains the Zellij WASM plugin.
- The workspace uses Rust edition 2021 through workspace package metadata.
- Formatting is therefore a workspace-wide concern rather than a single-crate concern.

## Rust source inventory

- `crates/lisa-cli/build.rs` is a Rust build script.
- `crates/lisa-cli/src/agent_exec.rs` is CLI source.
- `crates/lisa-cli/src/capture_usage.rs` is CLI source.
- `crates/lisa-cli/src/commit_transaction.rs` is CLI source.
- `crates/lisa-cli/src/config.rs` is CLI source.
- `crates/lisa-cli/src/detect.rs` is CLI source.
- `crates/lisa-cli/src/doctor.rs` is CLI source.
- `crates/lisa-cli/src/hooks_guide.rs` is CLI source.
- `crates/lisa-cli/src/init.rs` is CLI source.
- `crates/lisa-cli/src/loop_cmd.rs` is CLI source.
- `crates/lisa-cli/src/main.rs` is the CLI entry point.
- `crates/lisa-cli/src/setup_guide.rs` is CLI source.
- `crates/lisa-cli/src/status.rs` is CLI source.
- `crates/lisa-cli/src/templates.rs` is CLI source.
- `crates/lisa-cli/tests/atomic_provider_contract.rs` is an integration test target.
- `crates/lisa-cli/tests/help_surface.rs` is an integration test target.
- `crates/lisa-cli/tests/real_zellij_delivery_boundary.rs` is an integration test target.
- `crates/lisa-core/src/client.rs` is shared core source.
- `crates/lisa-core/src/dag.rs` is shared core source.
- `crates/lisa-core/src/diagnostics.rs` is shared core source.
- `crates/lisa-core/src/lib.rs` is the core crate root.
- `crates/lisa-core/src/provenance.rs` is shared core source.
- `crates/lisa-core/src/route.rs` is shared core source.
- `crates/lisa-core/src/ticket.rs` is shared core source.
- `crates/lisa-core/src/types.rs` is shared core source.
- `crates/lisa-plugin/src/adapter.rs` is plugin source.
- `crates/lisa-plugin/src/codex_ack.rs` is plugin source.
- `crates/lisa-plugin/src/lib.rs` is the plugin crate root.
- `crates/lisa-plugin/src/pane_name.rs` is plugin source.
- `crates/lisa-plugin/src/ui.rs` is plugin source.

## Formatting configuration

- No `rustfmt.toml` file is present in the repository file inventory.
- No `.rustfmt.toml` file is present in the repository file inventory.
- The repository therefore relies on the installed stable rustfmt defaults.
- Cargo discovers each workspace package through the root member glob.
- `cargo fmt --all` requests formatting for every workspace package.
- The `--` delimiter passes subsequent flags to rustfmt.
- The `--check` rustfmt flag performs validation without rewriting files.
- This exact command matches the acceptance criterion verbatim.

## Current observed state

- `cargo fmt --all -- --check` was run from the repository root.
- The command exited with status 0.
- The command emitted no formatting diff.
- This means every currently discovered Rust target is already canonical.
- `git status --short` reports two modified files.
- `.lisa/provenance.jsonl` is modified by Lisa workflow activity.
- `docs/active/tickets/T-038-02-01.md` is modified by Lisa phase handling.
- Neither modified file is Rust source.
- Neither modified file is owned as an implementation change by this ticket.
- The ticket diff changes `phase: ready` to `phase: research`.
- The assignment explicitly forbids manually changing ticket phase or status.
- The existing ticket change must therefore remain under Lisa's ownership.

## Historical context

- Recent commits include completion publications for all three prerequisites.
- Their completion commits contain ticket state and admitted work artifacts.
- The dependency completion commits do not themselves contain Rust source edits.
- Earlier ticket-specific commits contain the implementation work those artifacts describe.
- The current formatter result observes the aggregate tree after those changes.
- Formatting cleanliness can therefore be satisfied by the inherited tree.
- A ticket does not require a source diff when the acceptance state already exists.

## Relevant project commands

- `cargo build -p lisa-plugin --target wasm32-wasip1 --release` builds the plugin.
- `cargo build -p lisa-cli --release` builds the CLI.
- `cargo test --workspace` runs native workspace tests.
- `just check` combines a WASM check with tests according to `CLAUDE.md`.
- None of those commands is the ticket's explicit acceptance command.
- Formatting does not exercise runtime behavior or compilation semantics.
- The exact formatter check is the direct and proportionate verification.

## Workflow boundaries

- Phase artifacts belong in the attempt-private work directory.
- The private directory is `.lisa/attempts/T-038-02-01/1/work/`.
- Artifacts must not be written directly to `docs/active/work/T-038-02-01/`.
- Lisa publishes admitted artifacts after checking the attempt lease.
- Ticket-owned source changes must use `lisa commit-ticket`.
- The command requires exact repository-relative include paths.
- Ordinary `git add` and `git commit` are prohibited for ticket work.
- No commit transaction is necessary when there is no source change.
- Workflow-managed ticket and provenance changes must not be included.
- Review must confirm that no ticket-owned files remain dirty.

## Constraints and assumptions

- The formatter installed in the active toolchain defines canonical output.
- The acceptance command is authoritative for which targets are checked.
- A zero exit on the current tree is evidence of formatter cleanliness.
- Running rustfmt in write mode on an already-clean tree should produce no changes.
- Avoiding a write-mode run also avoids needless interaction with shared work.
- The shared working tree may contain concurrent Lisa metadata activity.
- That activity is expected in this repository's multi-ticket workflow.
- Existing unrelated modifications cannot be claimed by this ticket.
- Any implementation decision must preserve the current zero-diff result.

## Research conclusion

- The relevant surface is all Rust source discovered by the Cargo workspace.
- Canonical formatting is governed by default rustfmt configuration.
- The exact acceptance check already passes at the start of this attempt.
- There is currently no Rust formatting delta to apply.
- The remaining phases must decide and document how to handle that clean baseline.
