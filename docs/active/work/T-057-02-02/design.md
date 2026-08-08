# Design — T-057-02-02 init-retires-what-it-once-wrote

## 1. The seam: detection is shared, remedy is not

`currency.rs` gains one public function that reads only the filesystem:

```rust
pub fn retirements(root: &Path) -> Vec<Retirement>

pub struct Retirement {
    pub kind: RetirementKind,      // WorkflowDocument | ContextFile | ConfigKey | TicketPhase
    pub subject: String,           // repo-relative path, or ".lisa.toml [scheduling] auto_advance"
    pub detail: String,            // one sentence for a human
    pub disposition: Disposition,
}

pub enum Disposition {
    /// Bytes prove Lisa wrote this file and nobody has touched it. Init may delete it.
    RemoveFile(PathBuf),
    /// `.lisa.toml` sets a key Lisa no longer reads, and the line can be lifted out
    /// without disturbing a byte around it.
    DropConfigKey { path: PathBuf, section: &'static str, key: &'static str },
    /// Init must leave this exactly as it is. The string is the reason, already
    /// worded for the `--dry-run` preview.
    Preserve { path: PathBuf, reason: String },
}
```

`retirements` calls nothing in `init`. `plan_init_actions` calls `retirements`. `inventory` calls
both. The call graph is acyclic; the module reference is not, which Rust allows.

This is the only structure that satisfies "does not re-derive staleness on its own" without the
mutual recursion `inventory → plan_init_actions → inventory` would create. Every consent rule in the
story now has exactly one home, and it is not inside init.

`inventory` keeps deriving each remedy by reading init's plan, unchanged from T-057-02-01: a planned
`RemoveFile` at the path, or a planned `RetireConfigKey`, means `Remedy::Init`; otherwise
`Remedy::Clean`. It could now read the `Disposition` directly — but reading the plan is what makes
doctor's promise true of the code that will actually run, and this ticket is the first time the two
answers differ from each other's defaults.

`init::plan_retired_template` is deleted. Its rule and its exact reason strings move into
`retirements`, so the rdspi document is retired by the same path as everything else — the ticket's
"inventory-driven action rather than a special case inside the rename".

## 2. Consent, per subject

| Subject | Init may remove when | Otherwise |
|---|---|---|
| `docs/knowledge/rdspi-workflow.md` | bytes ∈ `templates::LEGACY_WORKFLOWS` | `Preserve` — a path Lisa created, so always reported |
| `CLAUDE.md` | `is_generated_claude_md` **and** §3 permits | `Preserve` if it still bears a frozen preamble; otherwise **silence** |
| `AGENTS.md` | `is_generated_agents_md` | `Preserve` if it still carries Lisa's pointer sentence; otherwise **silence** |
| `scheduling.auto_advance` | §4 says the line lifts out cleanly | `Preserve` — reported, never reformatted |
| a ticket at a retired phase | never | `Preserve`, always |

**The reportable-but-not-removable signal.** The acceptance criterion asks that a `CLAUDE.md` with
one edited line be *preserved byte-identical and reported as preserved with a reason*. An edited
generation is invisible to `is_generated_claude_md` by construction, so a second, weaker signal
decides whether to *speak*, and it never authorizes removal:

- `CLAUDE.md` — the file still starts with one of `legacy_context::CLAUDE_HEADERS`, the frozen
  preambles that run from `# CLAUDE.md` to `## Project`.
- `AGENTS.md` — the file still contains Lisa's pointer sentence, `This project's agent context lives
  in [CLAUDE.md](CLAUDE.md) — the single source of truth for every agent client`.

Two new predicates in `legacy_context.rs`, `bears_lisa_claude_marks` / `bears_lisa_agents_marks`,
sit beside the strict matchers so the strong and weak tests are read together and neither can be
mistaken for the other. T-057-02-01 rejected prefix matching *as a removal warrant*, where a false
positive claims the operator's file; used only to print a line, a false positive costs one
informational sentence and a false negative costs the silence that is already today's behaviour.

Everything else at those two paths produces no `Retirement` at all — not a softer one, none.

## 3. The mixed case: retire the pointer before its target, never the target alone

Both frozen `AGENTS.md` generations end by pointing at `CLAUDE.md`, and `legacy_context.rs` pins
that text with a test comment naming this ticket. The two directions are not symmetric:

- removing `AGENTS.md` and keeping `CLAUDE.md` removes a *pointer* — nothing dangles;
- removing `CLAUDE.md` while something at `AGENTS.md` still points at it is the dangling reference
  the ticket forbids.

**The rule:** `CLAUDE.md` is retired only when nothing will be left at `AGENTS.md` pointing at it —
that is, `AGENTS.md` does not exist, or does not mention `CLAUDE.md`, or is itself being removed in
the same plan. Otherwise `CLAUDE.md` becomes `Preserve` with the reason `preserved: AGENTS.md still
points at it`.

`AGENTS.md` is planned first; `CLAUDE.md` reads that decision. File order decides nothing.

The four cases, all reachable, all covered:

| `CLAUDE.md` | `AGENTS.md` | Outcome |
|---|---|---|
| generated | generated | both removed — the pair goes together |
| generated | edited / hand-written mentioning `CLAUDE.md` | **`CLAUDE.md` preserved**, reason names the pointer |
| edited / hand-written | generated | `AGENTS.md` removed, `CLAUDE.md` untouched |
| absent | generated | `AGENTS.md` removed |

The conservative direction is deliberate: an operator who wants the pair gone gets it in one run
once their `AGENTS.md` no longer names `CLAUDE.md`, and `lisa clean` (T-057-02-03) is where explicit
consent lives.

## 4. `.lisa.toml`: lift one line out, or do nothing

New in `config.rs`, next to the key catalog both callers already read:

```rust
pub(crate) enum RetiredKeyRemoval {
    Absent,
    Removed(String),
    NotSurgical(&'static str),   // the reason, for the preserved line
}
pub(crate) fn remove_retired_scheduling_key(existing: &str) -> RetiredKeyRemoval
```

Line surgery, in the manner of `upsert_missing_config_keys` — no `toml::Value` round trip, so no
comment, no key order, and no whitespace is reserialized. The procedure:

1. Parse. If the file does not parse, or does not set `scheduling.auto_advance`, return `Absent`.
   (An unparseable `.lisa.toml` already produces no currency finding; doctor's config check owns it.)
2. Find every line that *could* be that assignment: an `auto_advance = …` inside the active
   `[scheduling]` section, or a top-level `scheduling.auto_advance = …`. Commented lines do not
   count — step 1 already established the key is live.
3. Require exactly one candidate. Zero means the key is set some other way (an inline
   `scheduling = { auto_advance = true }`); more than one means the file is ambiguous. Either is
   `NotSurgical`.
4. Delete that one line, verbatim, and verify the result: it still parses, it no longer sets the
   key, and its parsed table equals the original's with that one key removed. Any failure —
   a value continuing onto the next line, say — is `NotSurgical`.

The post-condition is what makes this safe rather than clever: **every other byte in the file is
untouched by construction, and every other key survives by assertion.** When the check fails, init
leaves the file alone and says so, which the ticket ranks above a stripped-comment rewrite.

Init folds the removal into the single `.lisa.toml` write it already plans:

```rust
let updated = remove_retired(upsert_missing_config_keys(update_version_in_toml(existing)));
```

so there is one `UpdateFile` for that path and no ordering hazard between two writers.

## 5. Init's plan, and the one new variant

`InitAction` gains exactly one variant:

```rust
/// A `.lisa.toml` key Lisa no longer reads, named so `--dry-run` shows it.
///
/// Reporting only. The bytes ride in the `.lisa.toml` `UpdateFile` planned
/// above — one file, one write — so execution does nothing for this action.
/// It exists because "update .lisa.toml" does not tell an operator which key
/// is about to disappear, and the preview is what they read before consenting.
RetireConfigKey { path: PathBuf, section: &'static str, key: &'static str, reason: String }
```

`Display`: `  remove  {path} [{section}] {key} ({reason})`. One verb for the destructive class;
the variant is what makes it a distinct action.

Every retirement is appended in one group at the **end** of `plan_init_actions`, after the creates
and updates, so the preview ends on what is about to be destroyed rather than burying it:

```
  update  /p/.lisa.toml
  ...
  remove  /p/docs/knowledge/rdspi-workflow.md (superseded by docs/knowledge/lisa-workflow.md)
  remove  /p/CLAUDE.md (generated by Lisa and unedited since; Lisa no longer writes agent-context files)
  remove  /p/.lisa.toml [scheduling] auto_advance (Lisa stopped reading this setting in 0.5.0)
  skip    /p/AGENTS.md (preserved: edited since Lisa generated it, so it is yours now)
```

Mapping is one match arm per disposition: `RemoveFile → RemoveFile`, `DropConfigKey →
RetireConfigKey`, `Preserve → SafetySkip`. `SafetySkip` is the existing verb for "this is yours and
I am not touching it", already used by `plan_owned_template`; every preserved reason keeps the
`preserved:` prefix those lines use, so a ticket left alone cannot be misread as a ticket Lisa
declined to schedule.

**The one presentation judgment:** retired-phase tickets are the only unbounded group, and a board
with two hundred of them would push the removals off the top of the preview. Init lists five, then
one aggregate line — `preserved: N more tickets record a retired phase; \`lisa doctor\` lists them`.
The cap lives in init, not in `retirements`, because the inventory must stay complete for doctor;
this is a decision about a preview, and it belongs to the thing rendering the preview.

Nothing else about `--dry-run` changes. It already prints the plan and returns before both the
history step and the execute loop, so "changes nothing on disk" is an existing property this ticket
pins with a test rather than a property it has to build.

## 6. What is deliberately not here

- **Init does not rewrite ticket frontmatter.** The ticket forbids it and the currency inventory
  already routes those to `Remedy::Operator`. Init reports them and stops.
- **No migration framework.** One shared detector, one new enum variant, one new config helper. No
  version-to-version step registry, no ordering graph, no rollback.
- **No new consent prompt.** Removal consent is bytes, as it has been since T-057-01-05. A file init
  cannot prove it wrote is never destroyed by an upgrade, and the operator is never asked to
  adjudicate a diff at a prompt.

## 7. Rejected alternatives

- **`plan_init_actions` calling `currency::inventory`.** The literal reading of "use the inventory",
  and it recurses forever through `inventory`'s own call to the plan.
- **Two `UpdateFile` actions for `.lisa.toml`, the second removing the key.** Executes correctly
  (last write wins) but prints `update .lisa.toml` twice and never names the key — which is the
  whole point of the preview.
- **Round-tripping `.lisa.toml` through `toml::Value` and reserializing.** Removes the key with ten
  lines of code and returns the operator's file with its comments gone. The ticket names this as the
  worse outcome, and it is right.
- **Retiring `CLAUDE.md` and `AGENTS.md` independently.** Simpler, and it produces exactly the
  dangling pointer the ticket names.
- **Treating an unrecognised `CLAUDE.md` as "probably Lisa's, ask the operator".** Turns a P1
  guarantee into a prompt, and an upgrade that interrogates you about your own files is worse than
  one that says nothing.
