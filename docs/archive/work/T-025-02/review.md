# T-025-02 Review — AGENTS.md generation + toggle documentation

Handoff document. What changed, how it's tested, and what a human reviewer should
know.

## Summary

Codex auto-loads `AGENTS.md` while Claude Code auto-loads `CLAUDE.md`. This ticket
(1) makes `lisa init` scaffold an `AGENTS.md` that cannot drift from `CLAUDE.md`,
(2) makes the Codex ticket prompt point at `AGENTS.md` where the Claude prompt
points at `CLAUDE.md`, (3) documents the Codex client toggle and prerequisites in
the README, and (4) confirms `lisa validate` accepts a project with both context
files. Claude remains the default; the no-opt-in path is byte-for-byte unchanged.

## Files changed

- **`crates/lisa-core/src/client.rs`** — added `AgentClient::context_file()`
  (`Claude → "CLAUDE.md"`, `Codex → "AGENTS.md"`), the shared vocabulary accessor
  both crates use. +1 test.
- **`crates/lisa-plugin/src/lib.rs`** — `ticket_prompt` gained a
  `context_file: &str` parameter (replaces the hardcoded `CLAUDE.md`);
  `build_claude_command` passes the Claude context file (output unchanged); added
  the `AgentClient` import. +1 test, 1 test updated.
- **`crates/lisa-plugin/src/adapter.rs`** — `ClaudeCodeAdapter::reuse_prompt`
  passes the Claude context file; `CodexAdapter::agent_exec_line` passes the Codex
  context file. +1 parity test, 2 tests updated.
- **`crates/lisa-cli/src/templates.rs`** — added the `AGENTS_MD` pointer const.
  +1 test.
- **`crates/lisa-cli/src/init.rs`** — `plan_init_actions` scaffolds `AGENTS.md`
  (skip-if-exists, unconditional). +2 tests, 2 tests updated (count 18→19).
- **`README.md`** — Prerequisites note, `agent.client` config row, new "Codex
  client (experimental)" section, CLI-reference updates. Docs only.

No files created or deleted (besides `docs/active/work/T-025-02/` artifacts). No
public API removed; `AgentClient::context_file()` is additive and
`ticket_prompt` is `pub(crate)`.

## Design decisions (see design.md)

- **Pointer file, not a rendered twin or symlink.** `AGENTS.md` duplicates no
  project body, so drift is structurally impossible; it names `CLAUDE.md` as the
  source of truth and keeps the RDSPI reference. Mirrors the existing `CLAUDE.md`
  init pattern (one `InitAction`).
- **Unconditional emission.** Emitted regardless of selected client so switching
  to Codex is a one-line `.lisa.toml` edit with no re-scaffold; inert and harmless
  for Claude-only projects. Handled like `CLAUDE.md` (never overwrites authored
  content).
- **No new required-file check in validate/loop.** Requiring `AGENTS.md` would
  regress every existing Claude-only project; instead a test locks in that a
  project with both files validates clean.
- **Context filename lives on `AgentClient`**, keeping the plugin free of magic
  strings and consistent with the "one shared vocabulary" seam from T-025-01.

## Acceptance-criteria trace

| AC | Status | Evidence |
|----|--------|----------|
| `lisa init` + templates generate `AGENTS.md`, shared-source (no drift), incl. RDSPI ref | ✅ | `AGENTS_MD` pointer; `test_agents_md_points_to_claude`, `test_run_init_creates_files` |
| Codex prompt → `AGENTS.md`, Claude prompt → `CLAUDE.md` (`ticket_prompt`, lib.rs) | ✅ | `codex_prompt_references_agents_not_claude`, `test_ticket_prompt_uses_given_context_file` |
| README documents toggle, Codex prereqs (binary, version pinning, trust), wrapper behaviour, Claude default unchanged | ✅ | README "Codex client (experimental)" |
| Docs don't imply support beyond the two natives (ACP future) | ✅ | Section states "only supported clients … ACP is future work, not available yet" |
| `lisa validate` accepts both context files | ✅ | `test_validate_accepts_both_context_files` |
| Version-pinning caveat + `lisa doctor` reports installed codex version (Notes) | ✅ | README prereqs; doctor behaviour shipped in T-025-01 |

## Test coverage

- **New/changed unit tests**: core `context_file_per_client`; plugin
  `test_ticket_prompt_uses_given_context_file`,
  `codex_prompt_references_agents_not_claude` (+ 3 updated); templates
  `test_agents_md_points_to_claude`; init
  `test_run_init_never_overwrites_agents_md`,
  `test_validate_accepts_both_context_files` (+ 2 updated).
- **No-regression proofs retained**: `test_build_claude_command` and
  `native_reuse_prompt_matches_free_fn` still assert `CLAUDE.md`, so the Claude
  default path is byte-identical.
- **Full suite**: `cargo test --workspace` → 532 tests pass (218 cli / 118 core /
  196 plugin). WASM release build succeeds. `cargo clippy` clean on all three
  crates.

## Gaps / open concerns

- **`setup_guide.rs` not updated** — its LLM onboarding text still names only
  `CLAUDE.md`. The AC is met by the README; updating setup-guide was optional per
  Design and deferred to keep the diff tight. Low priority follow-up.
- **Docs are not automatically tested.** The README's accuracy about the Codex
  mechanism rests on the T-025-01 implementation (doctor/trust/plumb-through),
  which is itself test-covered; the prose was verified by read-through against the
  ACs.
- **Pointer vs native expectations.** A Codex agent is told to read `AGENTS.md`,
  which points it to `CLAUDE.md`. This relies on the agent following the one-line
  pointer — deliberate (it's the anti-drift guarantee) but worth noting if a
  future Codex version treats `AGENTS.md` as authoritative-and-complete rather
  than a jumping-off point.

## Reviewer notes

Nothing here changes persisted formats, the config schema, or the layout config
map (client selection plumb-through already shipped in T-025-01). The riskiest
line is the `ticket_prompt` signature change; it is `pub(crate)` and every caller
+ test was updated in the same change, with the Claude output asserted unchanged.
No human action required to merge beyond normal review.
