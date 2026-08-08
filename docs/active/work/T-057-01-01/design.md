# Design — T-057-01-01 four-phases-become-one

Four decisions to make. Everything else is compiler-directed mechanics.

---

## D1 — Where the forward mapping lives

`research | design | structure | plan → Phase::Implement` must hold at **both** parse entry
points: `ticket::parse_phase` (frontmatter) and `Phase::from_name` (`phase_timeout_*` layout
keys). The ticket's own framing is "two independent phase parsers, which must not drift".

**Option A — duplicate the four arms in each table.**
Both functions grow the same four `=> Implement` arms. Straightforward, and the AC's
"one test covering both entry points" catches drift.
Rejected: it *documents* the drift risk instead of removing it, and the drift the ticket is
worried about is precisely a future edit that touches one table and forgets the other. A test
catches that only if someone re-reads the test; the type system catches it never.

**Option B — `parse_phase` delegates to `Phase::from_name`. (chosen)**

```rust
fn parse_phase(value: &str) -> Result<Phase, TicketError> {
    Phase::from_name(&value.to_lowercase()).ok_or_else(|| TicketError::InvalidField { .. })
}
```

One table. Drift becomes structurally impossible rather than test-detected. The behaviours that
distinguish the two entry points are both preserved exactly:

- `parse_phase` lowercases before delegating, so `RESEARCH` and `Design` keep working
  (`ticket.rs` 745–746 pins that).
- `from_name` stays case-sensitive on its own, so `Phase::from_name("Research") == None` — the
  existing `types.rs` 1754 assertion survives untouched, and a `phase_timeout_Research` layout
  key keeps being ignored exactly as today.
- The `TicketError::InvalidField` shape, field name, and `reason` string stay in `parse_phase`,
  where the caller-facing error belongs. The `reason` drops the four retired spellings from its
  enumeration: it describes what to *write*, and those are no longer written.

The AC still gets its test — one `#[test]` asserting both entry points on all four retired
spellings. With delegation that test is cheap insurance rather than the only line of defence.

**Rejected variant:** move the lowercasing into `from_name` so both are case-insensitive. That
widens what `phase_timeout_*` accepts, which nobody asked for, and would break `types.rs` 1754.

---

## D2 — `phase_timeouts` collision

`Phase` is a `HashMap` key (`PluginConfig::phase_timeouts`). With the forward mapping, a layout
carrying both `phase_timeout_research = 300` and `phase_timeout_implement = 1800` collapses two
entries onto one key, and the winner is whichever the `BTreeMap` iteration reaches last —
deterministic (`implement` sorts after `research`), but not obviously so.

**Decision: accept it, do not add machinery.** Two reasons.

1. It is the honest consequence of the mapping the ticket already decided. `research` *means*
   `implement` now; a board that timed Research at 300s is stating a budget for the work that
   phase named, and that work is now called Implement.
2. Any alternative (first-wins, max-wins, reject-on-collision) is new behaviour in a subsystem
   the story explicitly puts out of slice, and the config path is lenient by construction —
   `from_config_map` never errors, it skips.

Not pinned by a test, because pinning an incidental `BTreeMap` ordering would make it a promise.
Noted here so the next reader finds the reasoning rather than rediscovering the collision.

**`config.rs` `known_phases` (538–556) is left alone.** It is an accept-list for
`[scheduling.phase_timeouts]` keys and it lists all six work phases. All six are still
*accepted* — the four retired ones map forward. Reducing the list would emit a fresh warning at
every upgraded board for a key that still works, which is the opposite of the ticket's stance.
Routing it through `Phase::from_name` was considered and rejected: `from_name` also accepts
`ready` and `done`, which are not sensible timeout keys, so that swap would widen the accept-list
in a direction nobody asked for.

---

## D3 — What `Implement` returns from `artifact_filename()`

`None`, per the ticket. Worth confirming it is safe rather than taking it on faith, because two
plugin sites read that function:

- `check_artifact_advances` (`lib.rs` ~6000–6008) computes
  `if current_phase == Phase::Implement { "review.md" } else { artifact_filename() }`. The `if`
  shadows the `None` completely. **No behaviour change.**
- `check_idle_signals` (`lib.rs` ~6624) reads it only inside the
  `Research | Design | Structure | Plan | Review` arm. `Implement` has its own arm above it. After
  the collapse that arm's pattern is just `Phase::Review`, and `Review.artifact_filename()` is
  still `Some("review.md")`. **No behaviour change.**

So `None` for `Implement` is not a regression; it is the type finally agreeing with the two
places that already routed around it.

---

## D4 — How far into `lisa-plugin` this ticket reaches

The tension: T-057-01-02 owns "make the plugin compile against the four-variant `Phase`", but
this ticket's own last acceptance criterion is `just check` green — which runs
`cargo check -p lisa-plugin --target wasm32-wasip1` and `cargo test --workspace`. A shrunk enum
that leaves the plugin uncompilable fails this ticket.

**Decision: this ticket makes the plugin compile; it does not subtract plugin machinery.**
The line between the two:

| This ticket (T-057-01-01) | T-057-01-02 |
|---|---|
| Collapse `Phase` match arms in `ui.rs` display impls | — |
| Narrow the idle-signal arm to `Phase::Review` (compile-forced) | — |
| `Ready` spawn sentinel advances to `Implement`, not `Research` | — |
| Update tests whose fixtures name retired variants | — |
| — | Delete the `progress.md` durability-admission block (~5981–5998) |
| — | Delete the `if current_phase == Phase::Implement` artifact special case (~6000–6008) |
| — | Whatever else the collapse leaves unreachable |

Rule of thumb applied: **if the compiler demands it, it is mine; if only a human reading the
code would notice it is now dead, it is T-057-01-02's.** That keeps the boundary mechanical and
keeps `just check` green at both ends.

Every `match` over `Phase` in the tree is exhaustive with no `_` arm, so the compiler enumerates
this ticket's plugin work exhaustively. There is no judgement call about what to catch.

---

## D5 — Removing `auto_advance` without breaking its two mirrors

`CONFIG_KEYS` is the hub of two **bidirectional** consistency tests found in Research:

- `config.rs` `verify_readme_config_table` — every catalog entry needs a README row *and* every
  README row needs a catalog entry.
- `main.rs` `flag_audit_tests` — exact set equality between `config:{path}` ids and
  `docs/knowledge/flag-audit.md` rows.

So the catalog entry, README:201, and flag-audit.md:126 must be deleted in **one** change or the
tests fail from either side. This is a hard ordering constraint, recorded in the plan as a single
step.

### Should an upgraded `.lisa.toml` warn?

The AC: "A `.lisa.toml` containing `auto_advance = true` under `[scheduling]` still loads and is
ignored." Two ways to get there.

**Option A — add a retired-keys allow-list so the key is silently accepted.**
Rejected: it is new machinery for a dead key, and the story's whole thesis is subtraction. It
also has to be maintained forever, or removed later by another ticket doing exactly this work
again.

**Option B — let it fall into the existing unknown-key path. (chosen)**
`SchedulingConfig` has no `deny_unknown_fields`, so serde ignores it and the file parses. The
existing `validate_config` walk pushes `Unknown key in [scheduling]: auto_advance` into
`warnings`, and `main.rs` prints warnings to stderr and keeps going. **Loading succeeds; the key
is ignored; the operator is told once, in words, that it does nothing.** That is a better
outcome than silence — an operator who set `auto_advance = true` believed it did something.

The ticket's constraint is "an unknown key is not a reason to refuse a project", and a warning
is not a refusal. Nothing in the codebase turns a warning into a non-zero exit.

The test pins all three halves: the load succeeds, the resolved config is unaffected, and the
warning names the key.

### The three shell fixtures

`crates/lisa-cli/tests/fixtures/{live_provider_startup,live_codex_review_boundary,
real_zellij_delivery_boundary}.sh` write `auto_advance` into a generated `.lisa.toml`. **Left
as-is deliberately** — they are now unmodified specimens of a pre-0.5 board, and the thing this
ticket promises is that such a board still runs. Editing them would delete the evidence.

---

## What is explicitly not decided here

- The assignment prompt at `lib.rs` ~146–148, which still recites six phases — T-057-01-04.
- `crates/lisa-cli/data/rdspi-workflow.md` and its `legacy/` copies — T-057-01-05.
- This repo's own `.lisa.toml` commented `# auto_advance = false` stub. It is a comment, produces
  no warning, and the file is not owned by this ticket; `lisa init` stops emitting the stub
  regardless, which is the part that matters for new projects.
- The `RDSPIR`/`RDSPI` doc-comment vocabulary scattered through `lisa-core` and `lisa-plugin`
  headers. Doc comments that *enumerate the phases* are corrected here because they become
  factually wrong (`types.rs` 117, 207; `dag.rs` 289; `lib.rs` 6519). The workflow's *name* is
  T-057-01-05's to retire.
