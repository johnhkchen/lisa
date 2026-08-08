# Plan — T-057-02-02 init-retires-what-it-once-wrote

Six commits, each one `just check`-green on its own. Ordered so the detector exists before anything
consumes it and so no commit leaves init able to delete something the inventory cannot explain.

## 1. The weak marks — `legacy_context.rs`

`bears_lisa_claude_marks` (starts with any `CLAUDE_HEADERS` entry) and `bears_lisa_agents_marks`
(contains the pointer sentence, lifted to a named constant so the frozen-generation test can assert
both files still carry it).

Tests: every frozen generation bears its mark; a one-line edit of one still bears it; the
hand-written files already in that module's tests do not.

Nothing consumes these yet. Green.

**Include:** `crates/lisa-cli/src/legacy_context.rs`

## 2. The surgical removal — `config.rs`

`RetiredKeyRemoval` + `remove_retired_scheduling_key`, next to `CONFIG_KEYS`. Procedure exactly as
design §4:

1. parse; not parsing, or not setting `scheduling.auto_advance` → `Absent`
2. collect candidate lines: an `auto_advance =` inside the active `[scheduling]` table, or a
   top-level `scheduling.auto_advance =`; skip commented lines
3. not exactly one candidate → `NotSurgical`
4. drop that one line; re-parse; the result must parse, must not set the key, and must equal the
   original table with that one key removed — any failure is `NotSurgical`

Tests: interleaved comments and custom values preserved byte-for-byte; unrelated and unknown keys
survive; `Absent` on a clean file and an unparseable one; `NotSurgical` on
`scheduling = { auto_advance = true }` and on two candidates; `[scheduling]` reopened later in the
file still finds its key.

Nothing consumes it yet. Green.

**Include:** `crates/lisa-cli/src/config.rs`

## 3. The detector — `currency.rs`

Add `retirements` + its three types. Move the four detections into it verbatim in behaviour,
including the reason strings `plan_retired_template` prints today (`superseded by
docs/knowledge/lisa-workflow.md` — pinned by an existing init test, so it must survive the move
unchanged).

Rewrite `inventory`'s four `retired_*`/`stale_*` helpers to map over `retirements` instead of
re-reading the filesystem. Behaviour must be unchanged at this commit: every existing `currency` and
`doctor` test passes untouched. That is the check on the move.

The context-pair ordering rule and the `proven_generation` flag land here too, but with nothing in
init to act on them the observable inventory is identical.

**Include:** `crates/lisa-cli/src/currency.rs`

## 4. The action verb — `init.rs`

`InitAction::RetireConfigKey` + its `Display` + its (empty) execute arm. Delete
`plan_retired_template`; drive the whole retirement group off `currency::retirements`, appended at
the end of `plan_init_actions`. Gate `config::remove_retired_scheduling_key` on a `DropConfigKey`
disposition being present for that path.

This is the commit where behaviour changes: `CLAUDE.md`, `AGENTS.md` and `auto_advance` become
things init removes. The existing `currency` assertion `Remedy::Clean` for a generated `CLAUDE.md`
flips to `Remedy::Init` in the same commit — it is a marker left for this ticket, and leaving it
stale for a commit would mean shipping a doctor that names the wrong command.

Watch for: `test_plan_init_actions_empty_dir` and the action-count assertions — the retirement group
is appended, so any test asserting a fixed plan length or a positional index needs reading before
assuming it still holds.

**Include:** `crates/lisa-cli/src/init.rs`, `crates/lisa-cli/src/currency.rs`

## 5. The acceptance tests

Written against the finished behaviour, in `init.rs`'s test module, on a shared 0.4.4 fixture that
carries all five subjects at once (old version, `rdspi-workflow.md`, generated `CLAUDE.md` +
`AGENTS.md`, `auto_advance` interleaved with comments, a ticket at `phase: structure`):

1. `--dry-run` names every retirement and leaves the tree byte-identical (recursive path+bytes snapshot)
2. one `lisa init` leaves no `Behind` and no `Retired` finding — only the ticket rows, each with an
   `Operator` remedy
3. a second consecutive run plans no mutating action and changes no byte
4. the retired-phase ticket file is byte-identical after the run
5. the `.lisa.toml` after the run: `auto_advance` gone, every comment, every unrelated key, and the
   key order intact
6. an unremovable `.lisa.toml` (inline table) is byte-identical afterward and reported
7. the four context-pair cases, asserted on removal *and* on disk after the run

**Include:** `crates/lisa-cli/src/init.rs`

## 6. Docs, if the run turns any up

`docs/knowledge/` says nothing about init's action vocabulary today; expect nothing. If step 5
surfaces operator-facing copy that now lies, fix it here rather than folding it into a code commit.

## Risks, and what each costs

| Risk | Cost if it lands | Guard |
| --- | --- | --- |
| Prefix match claims an operator's `CLAUDE.md` | P1 violation — their standing instructions deleted | The weak mark never authorizes removal and never becomes a doctor finding (structure §"weak mark stops at init's preview"); removal stays anchored at both ends |
| `.lisa.toml` comes back reformatted | Worse than the dead key, per the ticket | Line splice + parse-equivalence post-condition; `NotSurgical` refuses rather than guesses |
| Dangling `AGENTS.md → CLAUDE.md` | A repository that reads as broken | `AGENTS.md` planned first; `CLAUDE.md` reads that decision |
| Silent removal with no preview line | The preview stops being trustworthy | Init calls the remover only when a disposition authorized it |
| Existing tests encode the pre-ticket answer | A "regression" that is actually the point | Commit 4 flips them deliberately, with the reason in the diff |

## Definition of done

Every acceptance criterion has a named test above. `just check` green: `cargo check -p lisa-plugin
--target wasm32-wasip1`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test
--workspace`. Verified by exit code, not by reading output.
