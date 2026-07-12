# Structure: bounded cleanup blueprint

## File inventory

Modify exactly three ticket-owned source files:

- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/adapter.rs`
- `crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Create the required private attempt artifacts:

- `.lisa/attempts/T-038-03-02/1/work/research.md`
- `.lisa/attempts/T-038-03-02/1/work/design.md`
- `.lisa/attempts/T-038-03-02/1/work/structure.md`
- `.lisa/attempts/T-038-03-02/1/work/plan.md`
- `.lisa/attempts/T-038-03-02/1/work/progress.md`
- `.lisa/attempts/T-038-03-02/1/work/review.md`

Delete no files.

Do not modify:

- ticket frontmatter;
- shared `docs/active/work/T-038-03-02/` artifacts;
- Lisa provenance or lease data;
- historical admitted harnesses;
- lifecycle-hook templates;
- any other scheduler, adapter, or harness file.

## `crates/lisa-plugin/src/lib.rs`

### New private helper

Add one module-level function near the scheduler state definitions and before
its first consumers:

`fn pane_id_from_signal_filename(filename: &std::ffi::OsStr, suffix: &str) -> Option<u32>`

Internal sequence:

1. Convert `filename` to UTF-8 using `to_str()`.
2. Strip the exact `pane-` prefix.
3. Strip the exact supplied suffix.
4. Parse the remaining component as `u32`.
5. Return `None` on any failed step.

The function is private and pure. It has no access to `State`, paths,
directories, signal payloads, leases, clocks, or transition effects.

### Direct consumer replacements

Replace only the repeated filename parse chains in:

- `check_heartbeat_signals` using `.heartbeat`;
- `check_process_start_signals` using `.started`;
- `check_shell_ready_signals` using `.shell-ready`;
- `check_codex_ack_signals` using `.ack`;
- `check_awaiting_signals` using `.awaiting`;
- `check_error_signals` using `.error`.

Each consumer continues to:

- own `read_dir` and flattened iteration;
- obtain an entry path;
- choose its exact suffix at the call site;
- continue on an invalid filename;
- preserve its payload and deletion behavior;
- preserve state effects and logging.

### Transition consumer replacement

Keep the current two-arm `.stopped` / `.cleared` recognition structure.

For `.stopped`:

1. Recognize the suffix in the same branch position.
2. Remove the file in the same position.
3. call `pane_id_from_signal_filename(filename.as_ref(), ".stopped")`;
4. continue on `None`;
5. bump activity and handle the stopped signal as before.

For `.cleared`, use the same structure with `.cleared` and the existing clear
handler.

The exact local representation may use `OsStr::new(&filename)` or avoid the
owned UTF-8 `String`, provided non-UTF-8 names remain ignored and recognized
suffix deletion timing is unchanged.

Do not change `check_idle_signals`; its legacy ticket-id fallback remains
explicit and is not part of C-01.

### Parser tests

Add focused tests inside the existing `#[cfg(test)]` module.

One table-driven test asserts valid and invalid UTF-8 names. Each case contains:

- a filename;
- the requested suffix;
- the expected `Option<u32>`.

The case set includes numeric boundaries, leading zeroes, wrong delimiters,
wrong suffixes, empty/non-numeric ids, whitespace, negative values, and
overflow.

Add a Unix-only test that imports `std::os::unix::ffi::OsStringExt` locally,
constructs a filename containing an invalid UTF-8 byte, and expects `None`.
The production function remains portable because it uses only `OsStr::to_str`.

Do not refactor existing consumer regression tests or fixture builders.

## `crates/lisa-plugin/src/adapter.rs`

### Trait defaults

Change the existing `AgentAdapter` method declarations in place.

`reset_strategy` gains a body returning:

`ResetStrategy::ClearHandshake`

`follow_up` gains a body returning:

`FollowUp::TypeIntoPane(finish_up_prompt(ctx.ticket_dir, ctx.work_dir, ctx.ticket_id))`

Update method comments to describe these as native interactive defaults and
state that non-native transports override them.

### Concrete implementation removals

Remove exactly these duplicate methods:

- `ClaudeCodeAdapter::reset_strategy`;
- `ClaudeCodeAdapter::follow_up`;
- `CodexAdapter::reset_strategy`;
- `CodexAdapter::follow_up`.

Retain every other concrete method:

- launch command;
- assignment text;
- assignment reference where inherited;
- reuse prompt;
- exit command where inherited;
- signal capabilities;
- readiness mode.

Retain both enum alternatives `FreshExec` and `SpawnCommand`.

### Tests

Do not consolidate the per-provider expectations. Existing tests remain:

- `native_reset_is_clear_handshake`;
- `codex_reset_is_clear_handshake`;
- `native_follow_up_is_type_into_pane`;
- `codex_follow_up_is_typed_into_live_tui`;
- resolver and mixed-route reset assertions.

These tests now exercise inherited trait methods on both concrete types and
through resolved trait objects.

## Deterministic real-Zellij harness

### New script-local primitive

Add `event_count()` immediately before the two comparison helpers.

Inputs:

- positional argument 1: event kind.

Output:

- `0` when `$CURRENT_ROOT/evidence/events.log` does not exist;
- otherwise, the existing `awk` count of tab-delimited rows whose first field
  equals the requested kind.

Exit behavior:

- normal successful counting prints one integer;
- existing strict Bash mode remains enabled;
- no new external dependency is introduced because `awk` is already used.

### Comparison helpers

`event_count_is` retains:

- `kind` and `expected` locals;
- an `actual` local populated with `actual=$(event_count "$kind")`;
- the existing string equality comparison.

`event_count_at_least` retains:

- `kind` and `expected` locals;
- the same `actual` population;
- the existing arithmetic lower-bound comparison.

No harness call site changes. No shared shell library is introduced.

## Commit boundaries

### Source unit 1: signal filename parser

Include only:

`crates/lisa-plugin/src/lib.rs`

Commit after formatter and focused parser/plugin verification pass.

### Source unit 2: native adapter defaults

Include only:

`crates/lisa-plugin/src/adapter.rs`

Commit after formatter and focused adapter tests pass.

### Source unit 3: harness event counter

Include only:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Commit after shell syntax validation and the explicitly run ignored integration
test pass with its receipt.

No artifact path is included in ticket source commits; Lisa owns phase artifact
admission and the final completion transaction.

## Verification architecture

Focused proofs:

- parser test names selected through `cargo test -p lisa-plugin` filters;
- adapter test module or named tests under `cargo test -p lisa-plugin`;
- `bash -n` for shell syntax;
- `cargo test -p lisa-cli --test real_zellij_delivery_boundary -- --ignored --nocapture`.

Integrated proofs:

- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Final ownership proof:

- inspect ordinary index entries separately;
- inspect working-tree status;
- confirm none of the three ticket-owned source paths is staged, modified, or
  untracked;
- leave Lisa-managed ticket and provenance changes untouched.

## Explicit non-structure

- No scanner abstraction or signal enum is added.
- No scheduler module split occurs.
- No failure or timeout policy is unified.
- No atomic publication helper is added.
- No cross-harness library is created.
- No historical artifact is edited.
- No hook schema is introduced.
- No scheduler fixture builder migration occurs.
- No provider assignment construction is shared.
- No compatibility assertion is deduplicated.
