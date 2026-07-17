# Research — T-049-03-02

## Ticket boundary

- The ticket has two related honesty requirements.
- The first is executable guard behavior around completion seal selection.
- The second is a standing manual field leg for repository-less completion.
- The ticket begins in Research and requires all remaining RDSPI phases.
- Phase artifacts belong in the attempt-private work directory.
- Lisa, not this agent, advances ticket phase/status and publishes admitted artifacts.
- Source changes must use `lisa commit-ticket` with exact repository-relative paths.

## Completion vocabulary and types

- `crates/lisa-core/src/completion.rs` owns the shared seal vocabulary.
- `CompletionSealMode` represents configured intent: `Auto`, `Commit`, or `Journal`.
- `CompletionSeal` represents the resolved runtime tier: `Commit` or `Journal`.
- `CommitSealSupport` represents the result of probing the environment.
- `CommitSealUnavailable` distinguishes a missing repository, missing identity, and an
  unavailable transaction with detail.
- `resolve_completion_seal` is the pure intent-plus-capability resolver.
- Explicit `Journal` always resolves to journal sealing.
- `Auto` resolves to commit when support is available and journal otherwise.
- Explicit `Commit` resolves only when commit support is available.
- Explicit `Commit` plus unavailable support returns a resolution error.
- `CompletionSealReceipt` is typed evidence for either a commit id or a sorted set of
  path-bound content hashes.
- Receipt construction prevents an empty commit id and malformed journal evidence.

## Native preflight resolution

- `crates/lisa-cli/src/completion_seal.rs` adapts the pure resolver to a real project.
- `resolve_for_run` calls `probe_commit_support` for `auto` and `commit` modes.
- Explicit journal mode deliberately skips the Git probe.
- The probe first runs `git rev-parse --show-toplevel`.
- A failed repository discovery is represented as `RepositoryMissing`.
- A discovered root is canonicalized and retained in `RunCompletionSeal`.
- The probe checks `git config --get user.email` for a nonempty identity.
- Missing email becomes `IdentityMissing`.
- The probe verifies that `HEAD` exists for the isolated transaction.
- It also verifies the absolute Git metadata directory is available.
- Other failures become `TransactionUnavailable` with a diagnostic detail.
- `resolve_for_run_with` exists as an injectable unit-test boundary.
- The resolved `RunCompletionSeal` contains both the resolution and optional Git root.
- `format_preflight_failure` names `[guards].completion = "commit"` explicitly.
- The failure includes commands to configure identity and an alternative `lisa init`
  history-offer remedy.
- Unit coverage already asserts that explicit commit plus missing identity fails after
  one probe and includes the complete remedy.

## Loop pinning boundary

- `crates/lisa-cli/src/loop_cmd.rs` resolves the seal once for a non-dry run.
- This happens before runtime resolution, dependency checks, layout publication, or
  Zellij launch.
- Commit sealing retains the discovered Git root.
- Journal sealing canonicalizes the project directory itself as the host root.
- Dependency preflight receives whether the pinned seal is commit.
- `generate_layout` serializes the resolved seal into the plugin configuration.
- The plugin receives a concrete `CompletionSeal`, not the original auto/commit/journal
  intent.
- Therefore an auto run that resolves commit has the same completion boundary as an
  explicit commit run after launch.
- No downstream component re-runs environment resolution.
- A dry run uses explicit intent where available and defaults auto display to commit;
  it does not perform the real-run environment probe.

## Doctor and status visibility

- `crates/lisa-cli/src/doctor.rs` loads config and calls `resolve_for_run`.
- Doctor prints a dedicated completion section.
- `visibility_line` supplies the stable plain-language seal description.
- Commit is described as `commit-sealed — finished work lands as history`.
- Journal is described as `journal-only — finished work is recorded but not undoable`.
- Auto plus missing identity may resolve journal and still print the identity reason and
  remedies.
- Explicit commit resolution errors are printed in the completion section.
- Doctor returns an error if completion resolution failed.
- `crates/lisa-cli/src/status.rs` uses the read-only inspection resolver and the same
  visibility line.
- `crates/lisa-cli/tests/seal_visibility.rs` exercises doctor/status as compiled CLI
  fixtures with absent, identityless, and identity-bearing repositories.
- The compiled doctor fixture already covers explicit commit plus missing identity.

## Plugin completion boundary

- `crates/lisa-plugin/src/lib.rs` stores the pinned tier in
  `PluginConfig.completion_seal`.
- `dispatch_completion` builds a native completion command only for commit sealing.
- It persists `Requested` and `CommandInFlight` transitions before launching work.
- Journal mode calls `complete_pending_journal_seal` directly.
- Commit mode launches the isolated native completion transaction.
- `handle_completion_result` accepts only exit zero plus commit-shaped stdout as a
  successful commit result.
- Failed native commits are classified and passed through bounded failure handling.
- Identity/history failures retry once, then park with an operator-owned ask.
- Repository permission and stale-lock failures follow the same retry-then-park shape.
- Transient contention exhausts command launches and waits for reconciliation deadline.
- Unrecognized failures park immediately with the raw ask preserved.
- `finish_successful_completion` additionally checks that receipt seal equals the
  pinned plugin seal.
- A mismatched receipt is rejected and cannot release scheduler state.
- Durable Done frontmatter is verified before confirmation is journaled.
- There is no failure branch from a commit result into journal completion.

## Completion journal evidence

- `crates/lisa-plugin/src/completion_journal.rs` owns the append-only aggregate journal.
- Every record carries an explicit `seal` field.
- Transitions include requested, command-in-flight, failure-observed, rejected, and
  confirmed.
- Confirmed commit rows carry `commit_id` and no content hashes.
- Confirmed journal rows carry content hashes and no commit id.
- Deserialization rejects mixed or malformed evidence.
- Journal sealing hashes the final ticket and every nested admitted artifact.
- The journal completion publisher writes Done content before returning the receipt.
- Existing tests exercise repository-less completion and hash binding.

## Existing failure fixtures

- `completion_failure_fixture` in plugin tests creates a Review-phase ticket, current
  attempt lease, private Review artifacts, seat ownership, journal, and provenance.
- Its default `PluginConfig` retains `CompletionSeal::Commit`.
- Tests simulate identity and unborn-history errors through `handle_completion_result`.
- They assert bounded launches, blocked Review status, released seats, structured
  operator asks, journal failure observations, and parking provenance.
- The preserved Chromebook incident replay covers a real unborn identityless Git
  failure and verifies lock cleanup.
- Existing tests do not name the auto-at-start then broken-at-completion scenario.
- Existing failure assertions count failure rows but do not explicitly assert the
  absence of journal-sealed confirmation/content-hash rows for that ticket.

## Chromebook protocol and fixture

- `docs/knowledge/chromebook-install-test.md` is the manual acceptance instrument.
- The protocol is intentionally metered and requires a fresh authenticated agent CLI.
- `/cbt/prepare` fetches the selected README, extracts the install section, writes the
  tested instruction, applies optional variants, and snapshots disk state.
- Current prepare flags cover pinned README, ancient Zellij, XDG cache, and discovery.
- `/cbt/run` records agent/version/auth facts, stamps time, and launches one agent.
- `/cbt/grade` independently checks installation, doctor, init, validate, dry-run,
  resource bounds, prohibited compilers, and apt actions.
- The current grader creates `~/demo` after the agent exits.
- It opportunistically initializes Git when present, while the base fixture asserts
  that Git is absent.
- The current measured instruction asks only for installation and green doctor.
- It does not ask the agent to execute a ticket through a real Lisa loop.
- The current record captures only a representative doctor Zellij line.
- It does not quote the completion-seal line.
- It does not verify a completion-journal row or its file hashes.
- The Dockerfile installs curl, procps, sudo, Node, and the two agent CLIs.
- It explicitly fails the image build if Git, Rust, compilers, or make are present.
- Operator scripts are copied to `/cbt`, kept off PATH, and not disclosed to the tested
  agent.
- `just cbt-collect` copies selected sanitized evidence from a finished container.

## Shell constraints in the fixture

- The scripts use POSIX `sh` and `set -eu` (the grader uses `set -u`).
- The image has `sha256sum` through Debian core utilities.
- Python currently appears as a NodeSource dependency but is not a protocol invariant.
- Hash verification should therefore use ordinary shell tools rather than Python.
- No-Git behavior must not install Git as a convenience because repository absence is
  the subject of the leg.
- The tested agent must not receive the hidden grading rubric or `/cbt` paths.
- Manual execution remains outside this ticket; only protocol and fixture support ship.

## Ownership and concurrent work

- The worktree already contains unrelated modified ledger, ticket, and source files.
- `crates/lisa-cli/src/commit_transaction.rs` is modified by another active ticket.
- `docs/active/work/T-049-04-02/` and its ticket state are also unrelated.
- This ticket must not stage, revert, include, or otherwise consume those paths.
- Candidate owned paths are the seal visibility test, plugin boundary test, Chromebook
  prepare/grader scripts, and Chromebook runbook.
- The phase artifacts themselves remain private until Lisa publishes them.

## Research conclusion

- The production architecture already encodes the required hard preflight and pinned
  completion tier.
- The missing executable evidence is a scenario-shaped assertion at the compiled
  preflight and plugin completion boundaries.
- The field protocol lacks a named no-Git flag, tailored measured instruction, seal
  capture, and hash-verifiable journal grading.
- No production fallback from commit to journal was found.
