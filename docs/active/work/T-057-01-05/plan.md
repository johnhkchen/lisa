# T-057-01-05 — Plan

Five commits. Each is verifiable on its own except where the byte-equality pin forces a join,
which step 2 says explicitly.

---

### Step 1 — Capture the 0.4.4 generation

`cp docs/knowledge/rdspi-workflow.md crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.4.md`, add
it to the legacy list, rename the two constants (`LISA_WORKFLOW`, `LEGACY_WORKFLOWS`).

*Verify:* `cargo test -p lisa-cli templates` — the existing suite still passes with the data file
still named `rdspi-workflow.md`, proving the capture is byte-exact against the rendered template
before anything moves. `test_plan_init_updates_every_known_rdspi_template` asserts every legacy
entry differs from current and upgrades; a bad capture fails it immediately.

*Commit:* `--include crates/lisa-cli/data/legacy/rdspi-workflow-v0.4.4.md crates/lisa-cli/src/templates.rs`

### Step 2 — The document: write, rename, delete (one unit)

`git mv`-equivalent for both copies, write the new text, update the byte-equality pin and the
single-source path list, update the four `templates.rs` tests, add the shape test.

The data file, the checked-in copy, and the pin cannot be split: `test_workflow_document_embedded`
compares `LISA_WORKFLOW` to `docs/knowledge/lisa-workflow.md` byte-for-byte, so any two of the
three landing without the third leaves the tree red. `init.rs` still points at the old path after
this step and its tests still write `rdspi-workflow.md` fixtures — that is fine, because
`plan_owned_template` takes the path as an argument and the old path simply gets the new content
for one commit.

*Verify:* `cargo test -p lisa-cli`; the new shape test carries criteria 4 and 5.

*Commit:* `--include crates/lisa-cli/data/lisa-workflow.md crates/lisa-cli/data/rdspi-workflow.md docs/knowledge/lisa-workflow.md docs/knowledge/rdspi-workflow.md crates/lisa-cli/src/templates.rs`

### Step 3 — The migration

`InitAction::RemoveFile`, `plan_retired_template`, the install path, the validate check, the
executor arm, the ~40 test fixture paths, and the two new migration tests.

*Verify:* `cargo test -p lisa-cli init` — specifically the two new tests. The unmodified case
asserts create-plus-remove and then executes and checks the filesystem; the modified case asserts
create-plus-skip, that the skip reason names `lisa-workflow.md`, and that the edited bytes are
untouched after execution. Together they are criterion 2.

*Commit:* `--include crates/lisa-cli/src/init.rs`

### Step 4 — Doc comments and the guide

`check_run.rs`, `disposition.rs`, `types.rs`, `ticket.rs`, `context.rs`, `adapter.rs`,
`config.rs`, `setup_guide.rs`.

*Verify:* `cargo test --workspace` — `setup_guide`'s two renamed tests, and
`test_agent_contract_names_both_roles_and_both_prohibitions` proving the `ROLE_CONTRACT` edit
kept both roles and both prohibitions intact.

*Commit:* one, all eight files.

### Step 5 — Operator-facing prose and fixtures

`README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `AGENTS.md`, `docs/ROADMAP.md`,
`docs/PROMPT_CODEX.md`, `aur/PKGBUILD`, `.lisa.toml`, `live_provider_startup.sh`,
`docker/chromebook-test/bin/prepare`, the two fixture-board files, the two `codex_ack` fixtures.

*Verify:* `just check` green, plus the scoped grep below.

*Commit:* one.

---

## Testing strategy

**Unit / behavioural (new):**

| Test | Criterion |
|---|---|
| `an_unmodified_0_4_4_workflow_is_migrated_to_the_new_name` | 2 (first half) + 3 |
| `a_modified_workflow_is_left_where_the_operator_put_it` | 2 (second half) |
| `the_workflow_document_describes_the_board_lisa_actually_runs` | 4, 5 |

**Unit (unchanged, and that is the point):**
`test_review_disposition_contract_is_injected` and
`the_documented_check_contract_matches_the_code_that_enforces_it` keep every asserted string.
Criterion 6 is satisfied by their diffs containing no string literal changes at all.

**Existing, retargeted:** `test_workflow_document_embedded` (byte-equality against
`docs/knowledge/lisa-workflow.md`), the ~40 `init.rs` validate fixtures,
`test_plan_init_actions_empty_dir`'s create count.

**Gate:** `just check` — `cargo check -p lisa-plugin --target wasm32-wasip1`, `cargo fmt --check`,
`cargo clippy -- -D warnings`, `cargo test --workspace`. Judged by exit code, not by reading
output.

**Criterion 7 verification command** — the live tree, with the history exclusions design D4
argues for stated in the command itself rather than left implicit:

```sh
git ls-files | grep -v '^docs/archive/' | grep -v '^docs/active/' | grep -v '^docs/knowledge/' \
  | xargs grep -ril rdspi
```

Expected output: exactly the three `crates/lisa-cli/data/legacy/rdspi-workflow-v*.md` files.
`docs/knowledge/` is excluded for its field notes and runbooks; the one file in it this ticket
owns is verified separately by the byte-equality pin, and `ls docs/knowledge/ | grep rdspi` must
be empty.

## Risks

1. **A missed `RDSPI_WORKFLOW` reference outside `lisa-cli`.** Compile error, not a silent
   failure — the constant is `pub` but only `init.rs` reads it.
2. **The 0.4.4 capture drifting.** Step 1's ordering exists for this: capture and verify before
   the source of truth moves.
3. **The `phase:` frontmatter list in the document.** Trimming it to four values is right for the
   document but the *parser* still accepts the retired four by alias. The document describes what
   to write, not what is tolerated; the aliases stay undocumented on purpose, as T-057-01-01
   intended them to be invisible.
4. **`FileMutationKind` may need a `Removed` variant**, which could surface in whatever renders
   mutations. Checked while implementing; if the render is a match, it gets an arm.
