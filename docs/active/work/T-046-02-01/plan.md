# Plan — T-046-02-01 runtime resolver and config

## 1. Establish implementation tracking

Create the private `progress.md` artifact.

Record the planned source paths and the active overlap with T-046-01-02.

Verification:

- the artifact exists only in this attempt directory;
- shared ticket phase and status remain untouched.

## 2. Re-read concurrent integration files

Inspect Git status, recent commits, `doctor.rs`, and `loop_cmd.rs`.

Determine whether T-046-01-02 has committed its enforcement contract.

Do not edit its private artifacts or frontmatter.

Verification:

- the working tree versions match current HEAD;
- no ticket-owned source path already has foreign modifications.

## 3. Add runtime request and result vocabulary

Create `crates/lisa-cli/src/runtime.rs`.

Define the request enum, mode enum, resolved structure, and managed version.

Implement stable mode display strings.

Verification:

- types compile with the binary crate;
- equality assertions can express every mode.

## 4. Implement managed data-path derivation

Snapshot XDG data home and HOME as OS-native values.

Accept an absolute usable `XDG_DATA_HOME` first.

Fall back to absolute `$HOME/.local/share`.

Append `lisa/runtime/zellij-0.43.1/zellij`.

Return a named error when no usable data root exists.

Verification:

- XDG case yields the exact required path;
- HOME fallback yields the exact required path;
- missing roots fail without creating directories.

## 5. Implement system PATH lookup

Split the supplied PATH in order.

Find the first executable Zellij candidate.

Canonicalize it to an absolute path.

Return a system-mode-specific failure when absent.

Verification:

- two fixtures prove first-entry precedence;
- the returned path is absolute;
- an empty PATH fails.

## 6. Implement executable inspection

Run the selected path with `--version`.

Capture complete stdout and exit status.

Use `classify_zellij_version_output` and the shared range display.

Accept in-range versions.

Reject below-floor output with detected version, range, mode, and remedy.

Reject unparseable output with a distinct named error.

Verification:

- 0.40.1 is refused with every required diagnostic element;
- 0.43.x and 0.44.x pass;
- malformed and nonzero stubs fail closed.

## 7. Implement all-mode resolution

Map managed requests to the managed data path.

Map system requests to PATH lookup.

Map pinned requests directly to their absolute path.

Normalize paths and inspect the exact chosen executable.

Return mode, canonical version, and absolute path together.

Verification:

- all three request variants produce the matching mode;
- pinned mode ignores a competing PATH stub;
- managed mode ignores a competing PATH stub;
- system mode uses the PATH winner.

## 8. Extend `.lisa.toml` parsing

Add `RuntimeConfig` and the defaulted top-level field.

Add `ZellijRuntimeRequest` to `ResolvedConfig`.

Map absent/managed/system/absolute-path values in `resolve_config`.

Add `runtime` and `zellij` to unknown-key validation tables.

Reject relative pinned values semantically.

Verification:

- absent value resolves managed;
- explicit managed resolves managed;
- system resolves system;
- absolute pin resolves pinned;
- unknown runtime keys warn without failing;
- invalid relative pins produce actionable errors.

## 9. Document configuration defaults

Add a `[runtime]` block to `default_config_toml`.

Keep managed behavior active by absence or explicit documented value.

Show system and absolute pin alternatives as comments.

Verification:

- generated TOML parses;
- generated config resolves managed;
- examples do not accidentally select system or a host-specific path.

## 10. Declare the runtime module

Add `mod runtime;` to `main.rs`.

Keep the command surface unchanged.

Verification:

- `cargo check -p lisa-cli` finds the module;
- no help output changes unexpectedly.

## 11. Run the first focused test set

Run runtime module tests.

Run config module tests.

Run formatting and diff checks for the first unit.

Verification:

- all new pure and stub-based tests pass;
- existing config tests pass;
- only planned paths differ.

## 12. Reconcile T-046-01-02

Re-read HEAD and working-tree status.

If the adjacent ticket has committed, adapt to its current check/report API.

If it remains active, avoid copying obsolete generic-check structures.

Document any plan adjustment in `progress.md` before editing shared files.

Verification:

- no committed floor-enforcement behavior is removed;
- no foreign uncommitted source edit is consumed.

## 13. Integrate loop resolution

Resolve the configured runtime on real loop runs before side effects.

Preserve dry-run's no-runtime behavior.

Avoid redundant PATH-only Zellij checks.

Pass the absolute runtime path through to launch.

Print the chosen mode, version, and path before exec.

Verification:

- managed and pinned command builders use their exact absolute paths;
- system uses the frozen discovered path;
- dry-run tests remain independent of installed Zellij.

## 14. Refactor the launch seam

Add a path parameter to Unix and non-Unix launch functions.

Construct commands from the path rather than the string `zellij`.

Retain layout argument and project working directory.

Include the selected path in errors.

Verification:

- command inspection tests assert program, argument, and current directory;
- no bare Zellij launch remains in `loop_cmd.rs`.

## 15. Integrate doctor reporting

Load the full resolved config.

Resolve the same runtime request used by loop.

Report mode, canonical version, and absolute path.

Represent resolution failure as a required doctor failure.

Ensure the generic dependency list does not separately check PATH Zellij.

Verification:

- managed, system, and pinned fixtures render all three fields;
- resolution failure makes doctor fail;
- agent checks, project checks, cache cleanup, and Codex trust remain intact.

## 16. Run integration-focused tests

Run runtime, config, doctor, and loop module tests.

Run any adjacent T-046-01-02 tests added to the same modules.

Fix only ticket-owned regressions.

Verification:

- below-floor system behavior retains its named error;
- supported system fixtures pass;
- doctor report includes selected runtime identity;
- pinned and managed launches target absolute paths.

## 17. Format and inspect

Run `cargo fmt --all` and then the check form.

Run `git diff --check`.

Inspect exact diffs for all planned source paths.

Inspect ordinary index and status without modifying unrelated paths.

Verification:

- Rust formatting is stable;
- no whitespace errors exist;
- no unrelated worktree path is included;
- ordinary index remains empty.

## 18. Run package verification

Run `cargo test -p lisa-cli`.

Run `cargo test -p lisa-core version` to protect the consumed policy.

Record counts and ignored live tests in progress.

Verification:

- CLI unit and integration tests pass;
- core Zellij-version tests pass;
- live-only ignored tests remain intentionally ignored.

## 19. Run workspace verification

Run `just check`.

This covers the WASM target check and full workspace tests.

Record any environment limitation precisely.

Verification:

- plugin WASM check passes;
- all executed workspace tests pass;
- no ticket-owned warning or failure remains.

## 20. Commit source through Lisa

Use `lisa commit-ticket` only.

Provide ticket ID `T-046-02-01`.

Pass each ticket-owned source path with its own exact `--include`.

Use one or two meaningful commits according to the final overlap state.

Never use ordinary `git add` or `git commit`.

Verification:

- commit succeeds through the isolated transaction;
- the commit contains only exact owned paths;
- every owned source path is clean afterward;
- ordinary index remains empty.

## 21. Final implementation audit

Search for bare `Command::new("zellij")` launch sites.

Search for duplicate managed path/version literals.

Review acceptance criteria against tests and source.

Update progress with final commit and verification evidence.

Verification:

- loop launch is path-driven;
- doctor names mode, version, and path;
- all modes and precedence are covered;
- below-floor system refusal is covered with named remediation.

## 22. Review and disposition

Write private `review.md` summarizing source, tests, and concerns.

Write the exact pass JSON if all criteria and cleanliness checks pass.

Otherwise write the exact block JSON with an actionable reason.

Remain on this ticket after both artifacts exist.
