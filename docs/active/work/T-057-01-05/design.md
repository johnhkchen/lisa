# T-057-01-05 — Design

Four decisions, each with what was rejected.

---

## D1. The migration mechanism: a sixth `InitAction`, not a rename

**Decision.** Add `InitAction::RemoveFile { path, reason }` and a planner
`plan_retired_template(path, known_generations)` that mirrors `plan_owned_template`'s arms:

```rust
!path.exists()                       => (no action at all)
Ok(existing) if known.contains(..)   => RemoveFile   // an unmodified Lisa template
Ok(_)                                => SafetySkip   // the operator's file now
Err(_)                               => SafetySkip   // unreadable
```

The new path is planned independently, by the existing `plan_owned_template` call with its path
changed. A project therefore sees two lines: `create docs/knowledge/lisa-workflow.md` and either
`remove docs/knowledge/rdspi-workflow.md` or `skip … (superseded by …/lisa-workflow.md)`.

**Why two independent decisions rather than one rename.** The two questions are genuinely
different. "Does this project have a current `lisa-workflow.md`?" is answered by exact-bytes
ownership on the new path. "Is the old file safe to remove?" is answered by exact-bytes ownership
on the old path. Fusing them into an `fs::rename` couples them: a project that already has a
correct `lisa-workflow.md` *and* a stale `rdspi-workflow.md` (re-running init, or a partially
applied upgrade) is a normal state, and the rename framing has no arm for it. Two planners have
one each, and `lisa init` stays idempotent — the second run plans nothing, because the old file
is gone and the new one matches.

**Why `RemoveFile` is worth a variant.** Deletion is the one thing Lisa's init has never done. It
should be visible in `--dry-run` before it happens, printable with a reason, and recorded as a
`FileMutation` afterwards — all three come free from being an action rather than a side effect
inside a planner.

**Rejected: leave the old file.** It is the failure the ticket is named for. A 0.4 project would
carry two contract documents that disagree, and the stale one is the one old prompts, old shell
history, and the operator's muscle memory point at.

**Rejected: delete unconditionally.** It would be the first thing Lisa ever destroyed that a
person wrote. The same rule that protects a modified `on-stop.sh` protects this.

**Rejected: `fs::rename` when known, leave when modified.** Renaming the *old* bytes to the new
path leaves a 0.4.4 document sitting at `lisa-workflow.md` until the next init pass upgrades it —
a window where the new path holds the old workflow. Delete-and-create-current has no such window.

## D2. Removal eligibility is exactly the legacy list, and 0.4.4 joins it

**Decision.** `LEGACY_WORKFLOWS` becomes three entries — `rdspi-workflow-v0.2.md`,
`-v0.4.md`, and a new `-v0.4.4.md` holding the *rendered* 0.4.4 text (purpose paragraph included,
byte-identical to today's `docs/knowledge/rdspi-workflow.md`). That one list serves both jobs:
`known_prior` for upgrading a `lisa-workflow.md`, and removal eligibility for a stale
`rdspi-workflow.md`.

Capturing the rendered form matters: `plan_owned_template` compares against
`LISA_WORKFLOW.as_str()`, which is `PURPOSE_PARAGRAPH + "\n\n" + include_str!(…)`. The two
existing legacy files are stored that way; the third must match or the 0.4.4 upgrade — the exact
case criterion 3 names — silently falls through to `SafetySkip`.

**Rejected: a separate `RETIRED_WORKFLOWS` list for the old path.** It would be the same three
strings under a second name, and the failure mode of the duplicate (a generation added to one
list and not the other) is precisely the "told its file is unrecognised" outcome criterion 3
forbids.

**Note on naming.** The constants become `LISA_WORKFLOW` and `LEGACY_WORKFLOWS`. Criterion 3
names `LEGACY_RDSPI_WORKFLOWS`, but criterion 7 forbids `rdspi` outside archive paths and the
legacy template data — and a Rust identifier in `templates.rs` is neither. The list itself is
what criterion 3 is about; it keeps its job and gains its third entry. The legacy *data files*
keep their `rdspi-workflow-v*.md` names: they are the legacy template data criterion 7 exempts,
and they are named after what they are.

## D3. What the document becomes

**Shape** (target ~95 rendered lines, against 146):

```
purpose paragraph                      (injected, unchanged)
## How a ticket moves                  ready → implement → review → done, four entries
### Implement                          commit discipline, verbatim from ~41–47
### Review                             review.md + the three disposition shapes, verbatim
    check-execution contract           five clauses, verbatim
    the ask, the counter-example       verbatim
    wait for Lisa                      verbatim
## If a session dies                   NEW — the honest limitation
## Rules                               phase transitions; seal-gated completion  (was 1–6, now 2)
## Ticket Format                       phase list trimmed to four values
## Concurrency                         verbatim
```

**What is deleted:** the four phase chapters, and Phase Rules 1, 2, 4, and 5. Rule 3 (Lisa
detects the artifact and advances the ticket) and Rule 6 (completion is seal-gated) survive —
Rule 3 because criterion 4 asks for it in so many words, Rule 6 because it is the boundary the
whole completion path depends on.

**What is added — two paragraphs and no more.** A four-item phase list, and a short section
saying: a session that dies mid-work resumes from its commits, because `lisa commit-ticket` put
them on the branch; a journal-sealed project has no commits, so a ticket that dies mid-work
restarts from the beginning. Stated where the person it affects reads it, in the same plain
register as the rest.

**Rejected: keep a per-phase "artifact" heading for Implement.** There is no Implement artifact —
`Phase::artifact_filename` returns `None` and `progress.md` is no longer published. Naming one
would re-create the writing duty criterion 4 removes.

**Rejected: a "Migration from RDSPI" section.** The document is what a project has *now*. The
rename belongs in release notes and in the `SafetySkip` reason an operator actually sees, not in
a contract that every future agent reads forever.

## D4. Scope of the surface sweep

**Decision.** Change every file that *describes the running system*; leave every file that
*records history*.

Changed: the two documents, `templates.rs`, `init.rs`, `setup_guide.rs`, `check_run.rs`,
`disposition.rs`, `types.rs`, `ticket.rs`, `context.rs`, `adapter.rs`, `README.md`,
`CONTRIBUTING.md`, `CLAUDE.md`, `AGENTS.md`, `docs/ROADMAP.md`, `docs/PROMPT_CODEX.md`,
`aur/PKGBUILD`, `live_provider_startup.sh`, `docker/chromebook-test/bin/prepare`, the two
`codex_ack` fixtures, and the two fixture-board files that use "RDSPI" as an example of jargon.

Left alone: `docs/archive/**`, `docs/active/work/**`, and the board's own
tickets/stories/epics — a completed ticket is a record of what was true when it ran, and
rewriting ~300 of them would be forging the record. Also left alone: `docs/knowledge/` field
notes and runbooks, which S-057-01 puts out of this ticket's scope in as many words.

**Consequence for criterion 7, stated plainly.** `grep -ril rdspi` over the whole tracked tree
will still match those history files. The criterion's own preceding clause excludes
`docs/active/work/**` and `docs/archive/**`; this design reads the board's ticket/story files and
the knowledge-base field notes as the same kind of thing, and the *live* tree — everything that
tells an operator or an agent what Lisa does today — as the thing the criterion is about. The
verification command in `plan.md` states the exclusions explicitly rather than hiding them.

**`ROADMAP.md` is history too, but it is Lisa's own changelog and it names the path in the
present tense.** Its four lines are reworded to describe the document without naming the retired
file, so the record of what happened survives without pointing at a path that no longer exists.

## D5. Doc-comment citations

`check_run.rs:7` and `disposition.rs:26` cite the document by path. Both move to
`docs/knowledge/lisa-workflow.md`. `disposition.rs`'s citation is load-bearing in the sense that
the test named in criterion 6 (`the_documented_check_contract_matches_the_code_that_enforces_it`)
is the pin the comment is promising — that test keeps passing unchanged, because it asserts on
sentences, not on a path.

`ROLE_CONTRACT` (`context.rs:18`) tells an agent to take a ticket "through its RDSPI phases".
That becomes "through implementation and review" — the two phases an agent actually works. It is
a `pub const` asserted by `test_agent_contract_names_both_roles_and_both_prohibitions`, which
checks the two roles and the two prohibitions and not this clause, so the edit is safe.

## D6. Verification

- The byte-equality pin (`templates.rs`) is the strongest single check: it fails unless the
  repository's `docs/knowledge/lisa-workflow.md` is exactly what `lisa init` would install.
- Two new tests carry criterion 2 — the migration test the ticket is named for — one for the
  byte-exact 0.4.4 project and one for the edited one, asserting `RemoveFile` and `SafetySkip`
  respectively, and in both cases that `lisa-workflow.md` is created.
- One new test asserts the 0.4.4 text is in the legacy list and upgrades cleanly (criterion 3),
  which the existing `test_plan_init_updates_every_known_rdspi_template` loop already covers
  once the entry is added.
- One new test asserts the document's shape: four phase names, one artifact pair, shorter than
  146 lines, and the resume sentence present (criteria 4 and 5).
