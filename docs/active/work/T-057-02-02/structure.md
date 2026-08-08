# Structure — T-057-02-02 init-retires-what-it-once-wrote

## Files

| File | Change | Role after |
| --- | --- | --- |
| `crates/lisa-cli/src/currency.rs` | +detector, inventory rewritten to consume it | Owns *detection* for all four retirements (`retirements`), and separately renders them as doctor findings (`inventory`). |
| `crates/lisa-cli/src/init.rs` | `plan_retired_template` deleted; one new `InitAction` variant; retirement group appended to the plan | Owns *action* — mapping each disposition to a plan line, and folding the config-key removal into the single `.lisa.toml` write. |
| `crates/lisa-cli/src/config.rs` | +`RetiredKeyRemoval`, +`remove_retired_scheduling_key` | Owns the surgical line-lift, beside `CONFIG_KEYS` which both writers already read. |
| `crates/lisa-cli/src/legacy_context.rs` | +`bears_lisa_claude_marks`, +`bears_lisa_agents_marks` | Owns both the strong (removal) and weak (reportable) byte signals, side by side so neither can be mistaken for the other. |

No new module, no new file, no new data. Four edits.

## The seam

```
init::plan_init_actions ──> currency::retirements        (pure; reads bytes, returns dispositions)
                       └──> config::remove_retired_scheduling_key   (only when a disposition authorized it)

currency::inventory ───────> init::plan_init_actions      (remedy: what will actually run)
                    └──────> currency::retirements        (kind, subject, detail)
```

`retirements` calls nothing in `init`. That is the whole reason it exists: the literal reading of
"use the inventory" — `plan_init_actions` calling `inventory` — recurses forever through
`inventory`'s own call to the plan. The module reference `init → currency` closes a module cycle,
which Rust permits; the *function* call graph is acyclic.

## New surface

### `currency` (crate-private, beside the public `inventory`)

```rust
pub(crate) fn retirements(root: &Path) -> Vec<Retirement>

pub(crate) struct Retirement { kind: RetirementKind, subject: String, disposition: Disposition }

pub(crate) enum RetirementKind {
    WorkflowDocument,
    /// `proven_generation` is the strong signal: bytes match a shipped generator
    /// output. False means only the weak mark matched — reportable by init,
    /// never a removal warrant and never an inventory finding.
    ContextFile { proven_generation: bool },
    ConfigKey,
    TicketPhase { phase: String },
}

pub(crate) enum Disposition {
    RemoveFile { path: PathBuf, reason: String },
    DropConfigKey { path: PathBuf, section: &'static str, key: &'static str, reason: String },
    Preserve { path: PathBuf, reason: String },
}
```

`subject` is repository-relative (`CLAUDE.md`, `.lisa.toml [scheduling] auto_advance`); the
`PathBuf` inside each disposition is absolute, because that is what init acts on and prints.

**Detection is shared; presentation is not.** `retirements` carries the *reason* init prints in its
preview. The doctor-facing *detail* sentence and the `Remedy` stay in `inventory`, derived from
`RetirementKind` plus init's plan — they are rendering choices for a different reader, and pushing
them into the detector would make one string serve two audiences badly.

### `config`

```rust
pub(crate) enum RetiredKeyRemoval { Absent, Removed(String), NotSurgical(&'static str) }
pub(crate) fn remove_retired_scheduling_key(existing: &str) -> RetiredKeyRemoval
```

### `legacy_context`

```rust
pub(crate) fn bears_lisa_claude_marks(content: &str) -> bool   // starts with a frozen CLAUDE_HEADERS preamble
pub(crate) fn bears_lisa_agents_marks(content: &str) -> bool   // contains Lisa's pointer sentence
```

### `init`

```rust
InitAction::RetireConfigKey { path, section, key, reason }
//   Display: "  remove  {path} [{section}] {key} ({reason})"
```

Reporting only. The bytes ride in the `.lisa.toml` `UpdateFile` — one file, one write — so the
execute loop's arm for it is empty. It exists because `update  .lisa.toml` does not tell an operator
which key is about to disappear, and the preview is what they read before consenting.

## Control flow inside `plan_init_actions`

1. `let retired = currency::retirements(root);` — once, at the top.
2. Directories, workflow document, hooks, settings — unchanged.
3. `.lisa.toml`: version → `upsert_missing_config_keys` → **and then**, *only if* `retired` holds a
   `DropConfigKey` for this path, `config::remove_retired_scheduling_key`. One `UpdateFile`.
4. Every retirement, appended as one group at the end, so the preview closes on what is about to be
   destroyed rather than burying it under twenty no-ops.

**Step 3's gate is the invariant that matters.** Init never calls the remover on its own judgment,
so "a key disappears without a preview line naming it" is impossible by construction. The reverse
(a line whose key survives) would require `remove_retired_scheduling_key` to succeed on the raw file
and fail on the upserted one; the upsert only ever inserts *commented* lines and appends commented
sections, and commented lines are not removal candidates, so it cannot. A test pins it anyway.

## Mapping tables

Disposition → plan line:

| Disposition | `InitAction` |
| --- | --- |
| `RemoveFile` | `RemoveFile { path, reason }` |
| `DropConfigKey` | `RetireConfigKey { path, section, key, reason }` |
| `Preserve` | `SafetySkip { path, reason }` — the existing verb for "this is yours" |

Every preserved reason keeps the `preserved:` prefix `plan_owned_template` already uses, so a
preserved ticket cannot be misread as a ticket Lisa declined to schedule.

Retirement → doctor finding:

| Kind | Finding |
| --- | --- |
| `WorkflowDocument` | `Retired`, remedy read off the plan |
| `ContextFile { proven_generation: true }` | `Retired`, remedy read off the plan |
| `ContextFile { proven_generation: false }` | **no finding** — see below |
| `ConfigKey` | `Retired`, remedy read off the plan |
| `TicketPhase { phase }` | `StaleContent`, `Remedy::Operator(...)` naming the edit |

### The weak mark stops at init's preview

An edited `CLAUDE.md` that still bears a frozen preamble gets an init `skip` line and **no doctor
finding**. This is a deliberate narrowing of the weak signal beyond what design §2 spells out, and it
is where this ticket's P1 risk actually lives:

- as a preview line, a false positive costs one informational sentence;
- as a doctor finding, its remedy would be `run \`lisa clean\`` — and T-057-02-03 is a command that
  deletes. One prefix match would then be two keystrokes from destroying an operator's standing
  instructions to every model that reads their repository.

Design §2 says the weak signal "never authorizes removal". Keeping it out of the inventory is what
makes that true of the whole system rather than only of init. A byte-exact generation still reaches
doctor and still reaches `lisa clean`, because there the warrant is the strong signal.

## Ordering rule for the context pair

`retirements` plans `AGENTS.md` first and `CLAUDE.md` reads that decision. File order decides nothing.

`CLAUDE.md` is removed only when its bytes are a proven generation **and** nothing will be left at
`AGENTS.md` pointing at it — that is, `AGENTS.md` does not exist, does not contain the string
`CLAUDE.md`, or is itself being removed in the same plan. Otherwise `Preserve`, reason
`preserved: AGENTS.md still points at it`.

The pointer test is `contains("CLAUDE.md")` against the real file, not a marks test: a hand-written
`AGENTS.md` that names `CLAUDE.md` dangles exactly as badly as a generated one.

## Tests, and where each lives

`currency.rs` — one existing assertion flips (`Remedy::Clean` → `Remedy::Init` for a generated
`CLAUDE.md`; its comment "init does not retire context files yet" is the marker this ticket removes):

- the four context-pair cases (generated/generated, generated/edited, edited/generated, absent/generated)
- an edited-but-marked `CLAUDE.md` is invisible to `inventory` and visible to the plan
- a hand-written context file is invisible to both

`config.rs` — `remove_retired_scheduling_key` in isolation:

- interleaved comments, custom values, key order: every byte but the one line survives
- `Absent` when the key is not set, when the file does not parse
- `NotSurgical` for an inline `scheduling = { auto_advance = true }` and for two candidate lines
- the parse-equivalence post-condition: every other key survives the edit

`legacy_context.rs` — the weak matchers recognise every frozen generation and every one-line edit of
one, and reject hand-written files.

`init.rs` — the plan and the run:

- `--dry-run` against the 0.4.4 fixture leaves the tree byte-identical, and names every retirement
- the 0.4.4 fixture becomes current through one `lisa init`, then reports no `Behind` and no
  `Retired` finding
- a second consecutive run plans no mutation and changes no byte
- tickets at retired phases are reported and their frontmatter is untouched
- an unremovable `.lisa.toml` is left byte-identical and reported

## What does not change

`--dry-run` itself. It already prints the plan and returns before the history step and the execute
loop, so "changes nothing on disk" is an existing property this ticket pins with a test rather than
one it has to build.
