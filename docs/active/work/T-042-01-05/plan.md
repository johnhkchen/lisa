# Plan: Git-root-aware completion command

## Step 1: configuration carrier

Add `PluginConfig.git_root` with an empty default and lenient map parsing.

Verify with a focused core unit test that the default is empty and an absolute
layout value round-trips exactly.

## Step 2: native discovery and layout

Implement Git-root discovery in `loop_cmd.rs` using
`git -C <project> rev-parse --show-toplevel`, status checking, UTF-8-safe trimmed
output handling, and canonicalization.

Call it once in real loop startup and once in dry-run. Pass the result into
layout generation and emit an absolute `git_root` plugin setting.

Update all direct layout test calls. Add a temporary nested repository test and
assert the layout exposes the repository root rather than the nested project.

Verification:

`cargo test -p lisa-cli loop_cmd --no-fail-fast`

## Step 3: plugin root retention and mapping

Add `State.git_root`. In `load`, copy it from parsed configuration while retaining
`project_root` from the initial cwd.

Implement lexical normalization and completion-specific mapping:

- map `/host` to the project root;
- anchor relative paths at the project root;
- preserve host absolute paths;
- normalize components;
- reject escape and outside-root candidates;
- strip the Git root.

Use the mapper for ticket and work paths. Pass `git_root` to `--path`; retain the
project root as the host command cwd.

## Step 4: command regressions

Create a nested fixture with Git root `/repo`, project root
`/repo/games/midsummer`, and sandbox ticket/work paths. Assert exact argv values:

- `--path /repo`;
- `--ticket-file games/midsummer/docs/active/tickets/<id>.md`;
- `--work-dir games/midsummer/docs/active/work/<id>`.

Add an outside absolute ticket path and assert the error contains the stable
name `completion path outside Git root`.

Verification:

`cargo test -p lisa-plugin --lib completion_command --no-fail-fast`

## Step 5: integrated verification

Run formatting and focused packages:

`cargo fmt --all -- --check`

`cargo test -p lisa-core -p lisa-cli -p lisa-plugin --no-fail-fast`

Run the WASM compile/lint boundary:

`cargo clippy -p lisa-plugin --target wasm32-wasip1 -- -D warnings`

If focused verification exposes unrelated existing failures, record them with
evidence and continue with the strongest in-scope checks available.

## Step 6: isolated source commit

After `progress.md` records implementation and focused verification, commit the
single coupled source unit with:

`lisa commit-ticket --ticket-id T-042-01-05 --message "fix(plugin): build completion commands from Git root" --include crates/lisa-core/src/types.rs --include crates/lisa-cli/src/loop_cmd.rs --include crates/lisa-plugin/src/lib.rs`

Use the repository-built CLI if the installed binary lacks the command. Do not
use the ordinary index.

Confirm the three source paths are clean and the ordinary index remains empty.

## Step 7: review

Run a final diff/log/status inspection. Write `review.md` with the source commit,
acceptance mapping, verification results, and remaining concerns. Write the
strict pass/block JSON disposition. Remain on this ticket afterward.

## Completion checklist

- Both roots are explicit and retained.
- Nested command argv uses Git-root-relative includes.
- Outside paths fail with a named visible error.
- Existing CLI validation remains intact.
- Ticket-owned source is committed only through `lisa commit-ticket`.
- Review artifacts exist only in the attempt-private work directory.
