# Plan — T-045-02-02 zellij-injects-launcher-only

## Implementation strategy

Implement one atomic transport unit spanning the adapter contract and all scheduler
fresh-launch call sites.
Develop against focused native tests first.
Then run package, workspace, and WASM verification.
Commit only the two ticket-owned source files through Lisa's isolated transaction.

## Step 1 — snapshot the baseline

Before editing source:

1. confirm the ordinary index is empty;
2. record unrelated modified and untracked paths;
3. run the prerequisite launcher integration test;
4. run focused adapter tests;
5. run the nearest existing scheduler launch tests.

Expected baseline:

- `launch-codex` argv capture passes;
- Codex adapter tests still expect direct `codex` in the plugin script;
- current fresh-dispatch test proves assignment body is stored separately;
- unrelated `.lisa` ledgers and planning material remain outside ticket ownership.

No baseline failure will be hidden.
If a focused test is already failing, diagnose ownership before changing assertions.

## Step 2 — make assignment publication explicit at the adapter boundary

Edit `crates/lisa-plugin/src/adapter.rs`.

1. Change `AgentAdapter::launch_command` to accept `assignment_path: &Path`.
2. Update its documentation to require an already-published path.
3. Update `ClaudeCodeAdapter::launch_command` with `_assignment_path`.
4. Preserve the exact `build_claude_command` call.
5. Change `CodexAdapter::interactive_line` to accept the path.
6. Change `CodexAdapter::launch_command` to forward the path.
7. Do not alter `SpawnContext`.
8. Do not alter assignment text or reuse methods.

Verification after the mechanical interface edit:

```text
cargo check -p lisa-plugin
```

The check is expected to identify every unconverted call site.
Use that list together with `rg` to ensure none is missed.

## Step 3 — construct the native launcher command in `CodexAdapter`

Within `interactive_line`:

1. retain all five lifecycle environment values;
2. shell-quote `lisa_bin` for the environment value;
3. shell-quote `lisa_bin` again as the invoked executable;
4. emit `launch-codex` as a fixed subcommand;
5. retain the optional shell-quoted `--model` fragment;
6. emit fixed ` -- `;
7. shell-quote the exact pane-side assignment path;
8. retain the pane error-marker fallback.

Do not include:

- assignment body bytes;
- a sentence around the path;
- direct Codex safety flags;
- an environment-only assignment reference;
- a fallback to bare `codex`.

The direct Codex flags now belong exclusively to the native launcher's argv builder.

## Step 4 — update adapter unit tests

Add one stable assignment-path fixture for adapter tests.
Pass it to every `launch_command` call.

Update assertions in these categories:

- Claude exact command tests;
- route-resolution command test;
- mixed-route model test;
- Codex launch shape;
- provider context/launch separation;
- Codex model forwarding;
- pending tagged delivery versus launch;
- bare Lisa fallback.

Add or strengthen exact assertions for:

- `'/abs/lisa' launch-codex`;
- `-- '<assignment-path>'`;
- absence of direct ` codex --dangerously`;
- absence of full assignment body terms;
- unchanged `.error` marker.

Run:

```text
cargo test -p lisa-plugin adapter::tests
```

The adapter suite must pass before scheduler changes proceed.

## Step 5 — retain the exact publication in same-pane startup relaunch

Edit `State::acknowledge_shell_ready` in `lib.rs`.

1. Replace the success-only publication check with a retained result.
2. Preserve the existing recovery-failure call and early return on error.
3. Compute the pane-side path with `strip_host_prefix`.
4. Pass it to `adapter.launch_command`.
5. Preserve launch script publication.
6. Preserve lease marker publication order.
7. Preserve `Starting` state, relaunch count, activity, and slot mutation.

Run the exact startup recovery tests that cover this function.
Inspect their script assertions for provider-specific expectations.

## Step 6 — retain the exact publication in primary scheduling

Edit `State::schedule_ready_tickets`.

1. Bind the `AssignmentRef` returned by `prepare_assignment`.
2. Keep current lease revocation and logging on failure.
3. Translate its path once before fresh/recycle branching.
4. Pass the same translated reference into every `launch_command` call:
   recycle, `FreshExec`, and empty-pane fresh launch.
5. Leave `ClearHandshake` reuse untouched.
6. Leave the full assignment bytes out of `launch_cmd` and pane input.
7. Preserve all seat, readiness, thread, and route state.

Use `rg "launch_command\\("` after editing.
Every production call must now have an assignment-path argument.

Run focused scheduling tests:

```text
cargo test -p lisa-plugin test_fresh_dispatch_requires_start_then_chat_ack_for_both_providers
cargo test -p lisa-plugin test_pane_title_fresh_launch_uses_actual_fallback_route
```

Update only expectations made obsolete by the intended transport change.
Do not weaken ownership or lease assertions.

## Step 7 — retain the exact publication after `/exit`

Edit the exit-ready branch of `State::check_transition_timeouts`.

1. Bind the new `AssignmentRef`.
2. Preserve recovery versus non-recovery error reporting.
3. Translate the returned path.
4. Pass it to `launch_command`.
5. Preserve `prepare_fresh_launch` and `send_line_to_pane`.
6. Preserve slot transition cleanup and session fields.
7. Preserve recovery `Starting` and readiness classification.

Run existing consecutive Codex and dropped-ack fixtures.
They must retain their current state-machine outcomes because this ticket changes
only the launch command transport.

## Step 8 — add the two-ticket stub-pane acceptance fixture

Add a focused native test in `lib.rs`.

Setup:

1. create two ready Codex tickets;
2. create two empty pane slots;
3. configure a recognizable Lisa binary;
4. schedule both tickets once.

For each active assignment:

1. identify pane and exact lease;
2. load the retained assignment reference;
3. read the assignment body;
4. read `.lisa-launch-<pane>.sh`;
5. locate the matching `SessionLaunch` command;
6. verify a pending Enter exists for that pane.

Assertions:

- exactly two ticket/pane pairs exist;
- assignment paths are distinct;
- launch script paths are distinct;
- each pane line is only `sh <script>`;
- no pane line includes the body or context instructions;
- each script invokes `lisa launch-codex`;
- each script names its own exact quoted assignment path;
- no script contains assignment body text;
- no script directly invokes Codex;
- both slots are fresh Codex `Starting` sessions;
- neither slot uses `WaitingForClear`.

Run the fixture alone by exact test-name filter.
Its output must be deterministic and require no environment variable.

## Step 9 — reconcile existing script assertions

Search for tests reading `.lisa-launch-*.sh`.
Classify each by route:

- Codex fresh script: exact assignment path is now expected;
- Claude fresh script: path remains absent;
- generic publication helper: arbitrary payload behavior is unchanged.

Keep all assertions that assignment body bytes are absent.
Keep all assertions that `LISA_ASSIGNMENT` markers are absent from fresh launch scripts.
Where an assertion says “bare launch,” update its wording to “bounded launcher invocation”
without weakening the actual condition.

Run:

```text
cargo test -p lisa-plugin codex
cargo test -p lisa-plugin startup
cargo test -p lisa-plugin consecutive
```

## Step 10 — format and inspect the diff

Run the formatter, then inspect only ticket-owned changes:

```text
cargo fmt --all
git diff -- crates/lisa-plugin/src/adapter.rs crates/lisa-plugin/src/lib.rs
git diff --check -- crates/lisa-plugin/src/adapter.rs crates/lisa-plugin/src/lib.rs
```

Review for:

- accidental Claude command changes;
- missed launch call sites;
- duplicate filename construction;
- loss of error handling;
- assignment-body interpolation;
- direct Codex invocation remaining in production adapter code;
- unrelated test rewrites.

Update `progress.md` with completed steps and any deviations before committing.

## Step 11 — focused verification

Run in this order:

```text
cargo test -p lisa-plugin adapter::tests
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
cargo test -p lisa-cli --test codex_launcher
cargo test -p lisa-plugin
```

The CLI launcher test proves the path remains one native child argv element.
The plugin fixture proves the exact file path reaches that launcher without body paste.
Together they cover both halves of story acceptance without a real provider.

If a failure concerns an old state-machine assertion, preserve its semantic check and
adjust only the launch-shape expectation.
If a failure exposes a duplicate prompt or wrong attempt path at initial dispatch,
fix it before committing.

## Step 12 — workspace and WASM verification

Run:

```text
cargo fmt --all -- --check
cargo test --workspace
just check
```

`just check` includes the `wasm32-wasip1` plugin check and workspace tests.
No real Zellij/Codex test will be enabled or claimed.
Record exact counts or notable ignored tests in `progress.md`.

Because the worktree is shared, any failure in a foreign uncommitted path will be
inspected and attributed before deciding whether it blocks this ticket.

## Step 13 — isolated ticket commit

Confirm the ordinary index is still empty.
Confirm only the two intended source paths contain ticket changes.

Create one isolated commit:

```text
lisa commit-ticket \
  --ticket-id T-045-02-02 \
  --message "feat(plugin): launch Codex with exact assignment reference" \
  --include crates/lisa-plugin/src/adapter.rs \
  --include crates/lisa-plugin/src/lib.rs
```

Do not use `git add`, `git commit`, or a broad include.
After the command:

1. inspect the new commit stat and diff;
2. run `git show --check` on it;
3. verify both ticket-owned paths are clean relative to HEAD;
4. verify no ordinary index entries exist;
5. preserve unrelated runtime and planning files.

## Step 14 — post-commit verification

Rerun at minimum:

```text
cargo test -p lisa-plugin codex_stub_panes_receive_only_fresh_per_ticket_launcher_lines
cargo test -p lisa-cli --test codex_launcher
git status --short
```

If the isolated commit rebases or serializes over concurrent work, confirm the exact
ticket diff remains present and tests still compile on the resulting HEAD.

## Step 15 — Review artifacts

Write `progress.md` with:

- each completed implementation step;
- commit hash and exact includes;
- test commands and outcomes;
- deviations and rationale;
- source ownership audit;
- remaining story boundaries.

Then write `review.md` covering:

- adapter interface change;
- scheduler publication flow;
- two-ticket fixture evidence;
- prerequisite argv evidence;
- Claude compatibility;
- test coverage and limitations;
- open concerns for later claim/field stories.

Write `review-disposition.json` exactly as pass only if:

- all ticket-owned source is committed and clean;
- focused, package, workspace, and WASM checks pass;
- the stub-pane fixture proves both tickets receive fresh launcher invocations;
- assignment bodies do not appear in pane lines or launcher scripts;
- no critical concern remains.

Otherwise write a block disposition with a specific actionable reason.
Do not alter ticket phase/status or publish shared work artifacts.

## Verification criteria summary

Implementation is complete when all of the following are true:

1. `CodexAdapter` invokes `lisa launch-codex`.
2. The exact returned assignment path is supplied after `--`.
3. Direct Codex child argv construction remains in the native CLI only.
4. All fresh launch sites retain and pass the published reference.
5. Zellij pane input contains only the atomic script invocation.
6. Assignment content remains only in the immutable assignment file.
7. Two fixture tickets produce two distinct launch invocations and TUIs.
8. Claude launch and reuse tests retain existing behavior.
9. Existing scheduler state-machine tests remain green.
10. CLI argv-capture, plugin package, workspace, and WASM checks pass.
11. The isolated commit contains exactly the two owned source paths.
12. Both Review artifacts are valid in the attempt directory.
