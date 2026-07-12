# Structure — T-036-01-01: about-line and operator/internal grouping

## Files touched

| File | Change | Nature |
|------|--------|--------|
| `crates/lisa-cli/src/main.rs` | modify | about-line string + per-variant `#[command(...)]` attributes |

No files created or deleted. No new modules. No test files (deferred to
T-036-01-03). This is the only ticket-owned source file.

## Exact edit points in `crates/lisa-cli/src/main.rs`

### 1. Top-level about-line (lines 18–23)

`struct Cli`'s `#[command(...)]`:

```rust
#[command(
    name = "lisa",
    about = "Runs your coding agents through a project's tickets.",
    version
)]
```

Only the `about = "…"` value changes. `name` and `version` stay.

### 2. Per-variant grouping attributes on `enum Commands` (lines 29–174)

Add exactly one `#[command(...)]` attribute to each variant. The variants keep
their current declaration order and their current `///` doc comments verbatim;
only the grouping attribute is added. Where a variant already has no
`#[command(...)]`, add one; none currently carries a conflicting one.

Attribute per variant (kebab name in parentheses):

- `Init` (`init`)            → `#[command(display_order = 0)]`
- `Validate` (`validate`)    → `#[command(display_order = 1)]`
- `Status` (`status`)        → `#[command(display_order = 2)]`
- `SetupGuide` (`setup-guide`) → `#[command(hide = true)]`
- `HooksGuide` (`hooks-guide`) → `#[command(hide = true)]`
- `Doctor` (`doctor`)        → `#[command(display_order = 3)]`
- `Version` (`version`)      → `#[command(hide = true)]`
- `AgentExec` (`agent-exec`) → `#[command(display_order = 20)]`
- `CaptureUsage` (`capture-usage`) → `#[command(display_order = 21)]`
- `CommitTicket` (`commit-ticket`) → `#[command(display_order = 22)]`
- `CompleteTicket` (`complete-ticket`) → `#[command(display_order = 23)]`
- `Loop` (`loop`)            → `#[command(display_order = 4)]`

Placement: the attribute goes on its own line between the variant's `///` doc
comment and the variant name, e.g.:

```rust
    /// Initialize a project for lisa-loop completion
    #[command(display_order = 0)]
    Init {
        ...
    },
```

The `///` text is left byte-for-byte unchanged (that copy is T-036-01-02's
domain). The variant field lists (`{ dry_run, path }` etc.) are untouched.

## Interfaces / boundaries

- **Public CLI surface (behavioral):** unchanged. Every kebab command name and
  every flag is identical; `hide`/`display_order` affect only help rendering.
  clap still parses all 12 subcommands.
- **`fn main()` dispatch (main.rs:176–329):** not touched. Match arms bind by
  variant name, which is unchanged.
- **Module boundary:** all edits are within the `Cli`/`Commands` type
  declarations at the top of `main.rs`; no `mod` list change, no imports change.

## Ordering of changes

Single logical edit, but applied as two coherent hunks for a clean commit:

1. About-line string swap (struct `Cli`).
2. The twelve grouping attributes (enum `Commands`).

Both land in one file and one `lisa commit-ticket` unit — they are one
indivisible surface change (the about-line and grouping are the ticket's single
deliverable) and splitting them would leave an intermediate state that only half
satisfies the AC.

## What stays out

- No variant reordering in source (ordering is attribute-driven).
- No `///` copy edits.
- No new heading text (infeasible per Research; not attempted).
- No changes to `crates/lisa-cli/Cargo.toml`, `build.rs`, or any other crate.
- No test file (T-036-01-03).

## Post-edit invariants to hold

- `enum Commands` still has exactly 12 variants; `main()` still has a match arm
  per variant (compiler enforces exhaustiveness — a dropped/renamed variant
  would fail to build).
- `lisa --help` shows 9 commands + `help`; 3 are hidden.
- Hidden commands still resolve and run.
