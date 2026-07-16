# Review — T-046-02-01 runtime resolver and config

## Disposition

Pass.

The ticket now provides one runtime resolver for managed, system, and pinned
Zellij selection.

Loop and doctor consume the same resolved mode, version, and absolute path.

Both acceptance criteria are implemented and covered by focused plus workspace
verification.

## Source summary

The ticket's source change spans exactly five paths:

- `crates/lisa-cli/src/runtime.rs`;
- `crates/lisa-cli/src/config.rs`;
- `crates/lisa-cli/src/main.rs`;
- `crates/lisa-cli/src/doctor.rs`;
- `crates/lisa-cli/src/loop_cmd.rs`.

No manifest, lockfile, plugin, core, ticket-frontmatter, or shared workflow
artifact was changed by this ticket's source commits.

## Runtime module

`runtime.rs` is a new binary-local native module.

It separates configured intent from resolved executable identity.

`ZellijRuntimeRequest` has managed, system, and pinned-path variants.

`ZellijRuntimeMode` supplies stable lowercase mode labels.

`ResolvedZellijRuntime` retains mode, canonical version, and absolute path.

The resolver does not own tickets, layouts, caches, downloads, or archives.

This keeps the T-046-02-02 acquisition seam focused.

## Managed mode

Managed mode is the default when `[runtime].zellij` is absent.

The exact managed release is declared as Zellij 0.43.1.

That patch belongs to the plugin's compiled 0.43 SDK family.

With an absolute `XDG_DATA_HOME`, the path is:

`$XDG_DATA_HOME/lisa/runtime/zellij-0.43.1/zellij`

Without a usable XDG value, the fallback is:

`$HOME/.local/share/lisa/runtime/zellij-0.43.1/zellij`

Relative XDG values fall back rather than being treated as valid roots.

Missing or relative HOME plus unusable XDG produces a named error.

The resolver never silently falls back from managed to system mode.

Directory creation and managed binary acquisition remain in dependent ticket
T-046-02-02 as intended.

## System mode

`[runtime] zellij = "system"` explicitly opts into PATH lookup.

PATH entries are searched in order.

The first executable candidate is canonicalized to an absolute path.

Lisa invokes that exact path for `--version` inspection.

The selected path is frozen for later loop execution.

An absent PATH or absent executable produces a named system-resolution error.

The resolver classifies command output through `lisa-core`'s version contract.

Zellij 0.40.1 is rejected with detected version, required range, and managed-
runtime remedy.

Zellij 0.43 patch and 0.44 minor fixtures pass.

Unparseable output and nonzero `--version` exits fail closed.

## Pinned mode

Any non-symbolic `[runtime].zellij` value denotes a pinned path.

Configuration validation requires that path to be absolute.

Pinned resolution never consults PATH.

The file is canonicalized and inspected through its exact absolute path.

Pinned binaries receive the same compatibility validation as other modes.

This prevents an explicit but incompatible binary from reaching the plugin host.

## Configuration behavior

`LisaConfig` now has a defaulted `RuntimeConfig` section.

The raw optional string follows the repository's existing semantic-validation
pattern.

`ResolvedConfig` contains the typed request rather than a magic string.

Absence and explicit `managed` both resolve managed.

Explicit `system` resolves system.

An absolute string resolves pinned and therefore has explicit path precedence.

`runtime` is a known top-level section.

`zellij` is the only known key inside it.

Unknown runtime keys warn and do not fail parsing.

Relative pinned values fail with an actionable accepted-forms message.

The default `.lisa.toml` template documents all three choices while leaving the
managed default inert and portable.

Existing config files require no textual migration because absence is managed.

## Loop behavior

Real loop runs resolve Zellij before plugin and layout side effects.

Dry-run behavior remains free of external runtime requirements.

Agent and optional WASM dependency checks retain their existing machinery.

The previous generic PATH-only Zellij check is no longer duplicated.

Startup output names Zellij mode, version, and path.

Both Unix and non-Unix launch functions receive the resolved path.

Their command constructor uses `Command::new(zellij_path)`.

There is no remaining bare `Command::new("zellij")` launch site.

Launch errors include the selected path.

## Doctor behavior

Doctor loads the same resolved configuration as loop.

It resolves exactly one Zellij runtime.

Its required Zellij report names:

- configured/resolved mode;
- canonical detected version;
- supported range;
- absolute executable path.

Resolution or compatibility failure becomes a required unsupported result.

Doctor preserves the structured unsupported formatting introduced by
T-046-01-02.

Agent checks, project protocol reporting, cache cleanup, and Codex trust remain
in their existing order and behavior.

## Test coverage

Nine runtime unit tests cover:

- XDG managed path;
- HOME fallback;
- invalid data roots;
- ordered system PATH selection;
- absolute system path normalization;
- pinned-over-PATH precedence;
- managed-over-PATH precedence;
- 0.40.1 rejection and remedy;
- 0.43.x and 0.44.x acceptance;
- unparseable and nonzero failures.

Four new config assertions cover default, explicit managed, system, and pinned
resolution plus unknown-key and relative-path validation.

Doctor tests assert all three mode labels with version, range, and path.

Loop tests assert managed and pinned command programs are the exact supplied
absolute paths, with the expected layout argument and working directory.

The adjacent built-CLI suite explicitly selects system mode and proves 0.40.1
refusal, supported 0.43/0.44 passage, successful doctor output, and unparseable
doctor refusal.

## Verification evidence

`cargo fmt --all -- --check` passed.

`git diff --check` passed.

Focused runtime tests passed 9 of 9.

Focused config tests passed 57 of 57.

Focused doctor tests passed 44 of 44.

Focused loop tests passed 22 of 22.

The system-runtime black-box suite passed 4 of 4.

`cargo test -p lisa-cli` passed all executed package tests.

The package run included 295 binary unit tests and all 13 executed integration
tests.

The real-Zellij delivery harness remained ignored under its declared live-host
requirements.

`just check` passed.

Its WASM target check passed.

Workspace tests passed 19 CLI library, 295 CLI binary, 207 core, and 395 plugin
unit tests plus integration and doc-test targets.

No executed test failed.

## Commit evidence

The resolver/config unit was committed through Lisa:

`8fd4781b4c9119bb1998cf9a134d36d3c53fc67a`

It contains exactly runtime.rs, config.rs, and main.rs.

The command integration unit was committed through Lisa:

`c67f2355a64e0694dc904aec36b746ec282b32ce`

It contains exactly doctor.rs and loop_cmd.rs.

The ordinary Git index was never used and is empty.

All five ticket-owned source paths are clean.

## Concurrency handling

T-046-01-02 completed floor enforcement while this ticket was implementing its
non-overlapping runtime module.

This ticket re-read and preserved that committed structured failure behavior.

T-046-01-02 retained ownership of its black-box test and committed its own
runtime-aware fixture adaptation as `79a2888`.

No foreign uncommitted path entered either T-046-02-01 transaction.

## Open concerns and boundaries

Managed acquisition is intentionally not implemented here.

Until T-046-02-02 lands, a missing default managed binary produces a named error
at its exact expected path.

That is the planned dependent-ticket seam, not a silent system fallback.

The exact managed version must move with future SDK/release updates.

The constant and its documentation make that maintenance point visible.

Live network download and real managed installation are not tested by this
ticket because they belong to T-046-02-02 and the Chromebook field protocol.

No correctness issue blocks this ticket's resolver/config acceptance criteria.

## Recommendation

Admit the Review artifacts and allow Lisa to prepare the completion commit.
