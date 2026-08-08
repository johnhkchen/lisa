# Design — T-057-02-01 doctor-knows-what-stale-means

## Shape

```
crate::currency::inventory(root: &Path) -> ProjectCurrency
```

Filesystem reads only. No printing, no writes, no process exit. Returns:

```rust
struct ProjectCurrency {
    recorded_version: RecordedVersion,   // NoProject | Unreadable | PreVersioning | Behind | Current
    findings: Vec<CurrencyFinding>,      // ordered: behind → retired → stale content, then by subject
    current_version: &'static str,
}

struct CurrencyFinding { kind: CurrencyKind, subject: String, detail: String, remedy: Remedy }
enum CurrencyKind { Behind, Retired, StaleContent }
enum Remedy { Init, Clean, Operator(String) }
```

`Remedy` has no `None` variant. That is the ticket's rule expressed as a type: a finding cannot be
constructed without something for the reader to do.

## The three decisions that make this module worth having

**1. Behind is not computed here — it is asked of init.**
`plan_init_actions(root)` already decides which Lisa-owned files have a newer form, using
`plan_owned_template`'s byte comparison. Every `InitAction::UpdateFile` in that plan *is* a behind
finding. The alternative — a second staleness comparison living in `currency.rs` — is exactly the
disagreement the ticket forbids, and it would drift the first time either side changed.

**2. The remedy is read back off the same plan.**
A finding's remedy is not chosen from its category. If init's plan would resolve it — a planned
`UpdateFile`, a `RemoveFile` at that exact path, or a planned `.lisa.toml` that no longer sets the
key — the remedy is `lisa init`. Otherwise removal is the only fix, so the remedy is `lisa clean`.
When T-057-02-02 teaches init to retire context files, those findings move from `lisa clean` to
`lisa init` on their own, with no edit here and no window in which doctor names a command that then
reports nothing to do.

**3. Silence is the default for paths Lisa does not own.**
`docs/knowledge/rdspi-workflow.md` is a path Lisa created, so its presence is always reported; the
bytes only decide the remedy. `CLAUDE.md` and `AGENTS.md` are the project's paths, so they are
reported *only* when their bytes match a generation Lisa shipped. A hand-written one produces no
finding at all — not a softer finding, none.

## Recognising a generation from bytes

`AGENTS.md` interpolated nothing: exact byte comparison against the two shipped generations.

`CLAUDE.md` interpolated project name, type label, build commands and source layout. What gets
frozen is the generator's *shape*: a sequence of `Lit(&'static str)` spans and `Slot` holes. A file
matches when every literal appears in order, the first literal starts the file, the last literal
ends it, and the pattern consumes the whole file. Consequences, all intended:

- rewriting any of Lisa's prose → no match → the file is the operator's;
- appending a section → the closing literal no longer ends the file → no match;
- editing a build command *inside a hole* → still matches. This is the one deliberate imprecision.
  The holes are drawn as tightly as the format strings allow (fences, `# Build`/`# Run tests`/
  `# Lint` labels and all prose are literal), so what remains in a hole is a command string or a
  directory listing.

The project-type labels are a closed set (`Rust`, `Node.js`, `Go`, `Python`, `unknown type`), so
they are matched literally rather than through a hole.

## Version handling

`.lisa.toml` absent → `NoProject`, no findings; there is nothing to be current with. Present but
unparseable → `Unreadable`, no version finding (doctor's existing config check owns that failure).
Present with no `version` key → `PreVersioning`: reported as *behind by an unknown distance*, with
`lisa init` as the remedy, and nothing that reaches `has_failures`.

`.lisa.toml` gets at most one line. The version finding and the "settings Lisa added since are
missing" finding are the same upgrade seen twice; the settings line is emitted only when the version
finding had nothing to say.

## Rendering

`doctor.rs` gains one function, `format_project_currency`, and one section:

```
Checking project currency...

  behind   .lisa.toml
    Set up by Lisa 0.4.0; this Lisa is 0.5.0.
    Remedy: run `lisa init`
```

Same shape as every existing doctor check: subject, verdict, remedy. A current project gets exactly
one line saying so. The renderer makes one judgment of its own and it is not about staleness: how
much to show — five findings per kind, then `... and N more like the above`, because a board with
two hundred retired-phase tickets is a section people scroll past.

The section is informational. Nothing in it reaches `has_failures`, so `lisa doctor`'s exit code is
unchanged: a project three versions behind still runs, and refusing to work is not doctor's job.

## Rejected alternatives

- **A `stale: bool` per file.** Collapses behind and retired, which is the distinction the rest of
  the story is built on.
- **A trait per category with its own remedy.** Puts the remedy next to the category, which is
  precisely the hard-coding that lets doctor and init disagree.
- **Skipping the frozen generator outputs and matching a header prefix.** A prefix match claims
  every file that starts with `# CLAUDE.md`, including the operator's.
