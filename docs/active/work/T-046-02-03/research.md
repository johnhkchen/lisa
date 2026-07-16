# Research: XDG-aware Zellij pre-grant and cache

## Ticket scope

- Ticket: `T-046-02-03`.
- Title: `xdg-aware-pregrant-and-cache`.
- Current phase at assignment start: Research.
- The reported failure is specific to Lisa's Zellij cache-directory lookup.
- The observable symptom is an unexpected Zellij permission prompt.
- The affected environment is Linux with `XDG_CACHE_HOME` set.
- The required behavior is explicitly tied to Zellij 0.43 semantics.
- Existing macOS and Linux fallback paths are compatibility constraints.
- Regression coverage is required for both configured and unconfigured cache environments.
- Both permission pre-grant and plugin-cache cleanup are in scope.

## Repository and module context

- Lisa is a Rust workspace.
- The CLI package is `crates/lisa-cli`.
- The Zellij plugin package is `crates/lisa-plugin`.
- `crates/lisa-cli/src/main.rs` declares the private `doctor` module.
- `crates/lisa-cli/src/loop_cmd.rs` orchestrates startup of a Lisa loop.
- `crates/lisa-cli/src/doctor.rs` owns dependency checks and runtime preparation helpers.
- The relevant code is native CLI code, not WASM plugin code.
- The workspace uses package-local unit tests extensively.
- `doctor.rs` contains its tests in a trailing `#[cfg(test)]` module.
- `tempfile` is already available as a CLI dev-dependency.

## Runtime startup flow

- `run_loop` in `loop_cmd.rs` prepares the runtime before executing Zellij.
- It writes the embedded WASM plugin to a content-hashed temporary path.
- A new filename is used to avoid stale path-based cache behavior.
- It calls `crate::doctor::clean_zellij_plugin_cache()`.
- It then calls `crate::doctor::pregrant_plugin_permissions(&wasm_path)`.
- Both wrappers resolve their own cache path through the same private helper.
- The cleanup runs before pre-grant.
- Zellij is executed after both best-effort preparation steps.
- Neither preparation wrapper returns an error to `run_loop`.
- Failure therefore falls through to Zellij's normal runtime behavior.

## Doctor command flow

- `run_doctor` checks required binaries and project configuration.
- It also reports a Zellij plugin-cache check.
- That check calls the same private `zellij_cache_dir()` helper.
- It passes the resolved path to `clean_zellij_plugin_cache_in`.
- A missing resolved directory produces a diagnostic message.
- An existing directory with no Lisa entries produces a clean result.
- Removed cache entries are counted and reported.
- The doctor path is another consumer of the same resolver.
- Changing the resolver affects both loop startup and doctor cleanup.
- No command-line override for the Zellij cache path exists.

## Current cache resolver

- `zellij_cache_dir()` is private to `doctor.rs`.
- It reads `HOME` with `std::env::var`.
- If `HOME` is absent or non-Unicode, it returns `None`.
- On macOS it appends `Library/Caches/org.Zellij-Contributors.Zellij`.
- On other targets it appends `.cache/zellij`.
- The implementation labels the non-macOS branch as Linux / other Unix.
- It does not read `XDG_CACHE_HOME`.
- It does not reject a relative cache override because it never observes one.
- It does not use the same directory library as Zellij.
- All cache consumers inherit these properties.

## Zellij 0.43 cache resolution

- The resolved local dependency source includes `zellij-utils-0.43.1`.
- Its `consts.rs` imports `directories::ProjectDirs`.
- It constructs project directories with qualifier `org`.
- It uses organization `Zellij Contributors`.
- It uses application `Zellij`.
- `ZELLIJ_CACHE_DIR` is the resulting `ProjectDirs::cache_dir()`.
- `ZELLIJ_PLUGIN_PERMISSIONS_CACHE` appends `permissions.kdl` to that path.
- Session cache, plugin artifacts, and release-note state share that base.
- The construction is lazy and process-global inside Zellij.
- The relevant `directories` release in the lockfile is 5.0.1.

## `directories` 5.0.1 Linux behavior

- On Linux, the application name is normalized to lowercase `zellij`.
- The qualifier and organization do not appear in the Linux project suffix.
- The base cache directory first considers `XDG_CACHE_HOME`.
- Only an absolute `XDG_CACHE_HOME` is accepted.
- An unset override falls back to the user's home plus `.cache`.
- A relative override also falls back to the home cache directory.
- The project suffix `zellij` is appended to that base.
- The normal fallback is therefore `$HOME/.cache/zellij`.
- An absolute override produces `$XDG_CACHE_HOME/zellij`.
- Home discovery is delegated to the directory library's system helper.

## `directories` 5.0.1 macOS behavior

- On macOS, the cache base is the home directory plus `Library/Caches`.
- Project identity is represented as a bundle identifier.
- Spaces in the organization become hyphens.
- The resulting suffix is `org.Zellij-Contributors.Zellij`.
- The normal path is `$HOME/Library/Caches/org.Zellij-Contributors.Zellij`.
- This matches Lisa's current macOS hardcoded path.
- The macOS implementation does not consult `XDG_CACHE_HOME`.
- Project-directory construction still depends on discovering a home directory.
- The ticket requires this current macOS path to remain unchanged.

## Permission pre-grant behavior

- `pregrant_plugin_permissions` is the environment-resolving wrapper.
- `pregrant_plugin_permissions_in` is the filesystem-operation seam.
- The inner function appends `permissions.kdl` to the supplied cache directory.
- The plugin key is the quoted full path of the generated WASM.
- Existing exact-path grants are detected line-by-line.
- Existing unrelated KDL entries are preserved.
- Missing parent directories are created.
- The write is best-effort and represented as a boolean.
- The permission set mirrors the plugin's `request_permission` call.
- The set contains `WriteToStdin`.
- The set contains `ChangeApplicationState`.
- The set contains `ReadApplicationState`.
- The set contains `RunCommands`.
- Existing tests cover content, idempotence, and preservation.
- Existing tests inject a cache directory directly.
- They do not exercise environment-based resolution.

## Cache-cleanup behavior

- `clean_zellij_plugin_cache` is the environment-resolving wrapper.
- `clean_zellij_plugin_cache_in` is the filesystem-operation seam.
- The inner function recursively traverses the supplied directory.
- It removes files or directories whose names contain `lisa-plugin`.
- It descends into all other directories.
- It tolerates unreadable and nonexistent directories.
- It counts successfully removed matching entries.
- Existing tests cover no matches.
- Existing tests cover a deeply nested Lisa cache entry.
- Existing tests cover a nonexistent cache root.
- Existing tests inject a cache directory directly.
- They do not exercise environment-based resolution.

## Existing test constraints

- Unit tests in `doctor.rs` can access the private resolver and wrappers.
- The resolver currently has no injected environment abstraction.
- Environment variables are process-global state.
- Rust tests within one test binary may execute concurrently.
- `doctor.rs` already has a test that mutates `CODEX_HOME`.
- No current test in this module mutates `XDG_CACHE_HOME`.
- No current test in this module tests the Zellij cache resolver.
- Filesystem effects can be isolated with `tempfile::TempDir`.
- A resolver test can distinguish candidate roots by placing fixtures selectively.
- Platform-specific assertions are needed because Linux and macOS semantics differ.

## Dependency state

- `crates/lisa-cli/Cargo.toml` has no direct `directories` dependency.
- The workspace lockfile already contains `directories` 5.0.1.
- That package is present transitively through existing workspace dependencies.
- `lisa-cli`'s current lockfile dependency list does not include `directories`.
- A direct use by CLI code requires declaring it in the CLI manifest.
- The workspace currently builds with Rust 2021 edition settings.
- No feature gate is present for cache resolution.

## Ownership and boundaries

- The ticket's likely source ownership is confined to CLI cache resolution.
- `loop_cmd.rs` already calls stable wrappers and has no path logic.
- The plugin permission request itself already matches the pre-granted set.
- The recursive cleanup algorithm is independent of path selection.
- KDL serialization is independent of path selection.
- Project configuration does not participate in Zellij's cache path.
- Zellij environment variables for config files are unrelated to this cache root.
- The relevant external contract is the `directories` project identity tuple.
- The artifact directory is private to attempt 1 and must not be published directly.

## Worktree and workflow constraints

- The initial worktree contains Lisa-managed modified and untracked files.
- Those files predate this ticket execution and are unrelated to source ownership.
- Ticket source changes must use exact repository-relative include paths.
- Ordinary `git add` and `git commit` are prohibited for implementation work.
- Meaningful source units must be committed with `lisa commit-ticket`.
- Phase artifacts remain under `.lisa/attempts/T-046-02-03/1/work`.
- Ticket frontmatter phase and status are controlled by Lisa.
- Review requires both Markdown and the exact disposition JSON shape.

## Acceptance boundary

- The configured Linux case must target the cache root Zellij actually uses.
- Permission pre-grant must place `permissions.kdl` under that root.
- Cleanup must remove Lisa entries under that root.
- The unconfigured Linux path must remain `$HOME/.cache/zellij`.
- The macOS path must remain `$HOME/Library/Caches/org.Zellij-Contributors.Zellij`.
- Regression tests must make both environment states observable.
- The implementation must preserve best-effort behavior when no directory resolves.
