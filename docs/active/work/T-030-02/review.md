# Review: append-only ignore and mutation report

## Outcome

`lisa init` now upgrades `.lisa/.gitignore` without replacing it. Every readable
existing byte remains an exact prefix, and only missing Lisa-required rules are
appended. The vend regression retains `hooks/ntfy-topic`, adds `claude/` and
`codex/`, preserves the customized workflow and hooks, and proves the secret path
still matches `git check-ignore`.

Init output now makes planner intent and real effects auditable. Planned actions
distinguish creates, updates, ordinary no-ops, and safety skips. A successful
real run prints a separate `Files changed` section containing exactly the files
whose contents it created or updated, with no directories, skipped paths, or
unchanged paths.

No ticket or story frontmatter was edited.

## Production changes

### Append-only nested gitignore

`crates/lisa-cli/src/init.rs` adds `plan_append_only_gitignore` and routes only
`.lisa/.gitignore` through it.

The helper:

- creates the current template when the path is absent;
- reads existing text without performing a planning-time write;
- derives required rules from `templates::LISA_GITIGNORE`;
- compares trimmed logical lines so harmless surrounding spacing is accepted;
- identifies missing nonempty rules in template order;
- returns a no-op when every required rule is already present;
- clones the original content without reconstruction;
- inserts one separator newline only when a nonempty file lacks a trailing one;
- appends each missing rule once with a trailing newline;
- safety-skips unreadable or non-UTF-8 content instead of replacing it.

The previous `LEGACY_LISA_GITIGNORES` whole-file registry entry was removed from
`templates.rs` because append-only permission no longer depends on historical
ownership. The historical one-line file still upgrades naturally by appending
the two missing current rules.

### Ownership policy remains centralized

T-030-01's `plan_owned_template` continues to govern the workflow and all five
plain-text hook targets. Unknown or unreadable static content is still preserved.
Known prior exact bytes can still upgrade to the current template. Current exact
bytes remain unchanged.

No second static-template classifier or persistent ownership database was
introduced. The gitignore helper is a format-specific preserving merge, parallel
to the existing TOML and JSON merge policies.

### Explicit action semantics

`InitAction` now has separate `NoOp` and `SafetySkip` variants.

- `no-op` means the target already satisfies its policy or is intentionally
  preserve-if-present.
- `skip` means init declined a possible update because ownership, readability,
  or structured validity was insufficient.

Creates and updates keep their existing labels. Dry-run and real-run plans share
the same action vector and display implementation, so both modes expose all four
categories consistently.

### Exact real-run mutation record

The execution loop records a `FileMutation` only after a successful `fs::write`.

- `CreateFile` records `created`.
- `UpdateFile` records `updated`.
- `CreateDir`, `NoOp`, and `SafetySkip` record nothing.

After a successful run, `Files changed` prints the deterministic record in action
order. A fully idempotent run prints `none`. The report is therefore the content
write set, not a repetition of the broader plan.

`run_init` retains its public signature and delegates to a private
writer-injected implementation. Tests can capture the actual command output
without a separate binary harness, while normal CLI execution still writes to a
locked stdout handle.

### Hook permission boundary

The old post-pass chmodded every active hook that existed, even when its content
was a safety skip. Init now sets `0755` only for active hook scripts created or
updated by the current run. This keeps fresh and known-prior hooks executable
while leaving a skipped project-owned hook's bytes and mode untouched.

`on-notify.sample` remains excluded and non-executable.

### Operator documentation

README's `lisa init` reference now states:

- static templates update only with exact known Lisa ownership evidence;
- customized, unreadable, or unclassifiable files are preserved as safety skips;
- TOML and JSON retain their preserving structured merges;
- `.lisa/.gitignore` is append-only;
- project ignore rules are never deleted, reordered, or rewritten;
- dry and real plans distinguish all action categories;
- a successful real run reports the exact file content write set;
- operators should inspect those reported files before their next commit.

The command's printed next steps carry the same pre-commit inspection reminder.

## Files changed

### Production and documentation

- `README.md` — documents ownership, append-only, reporting, and inspection
  contracts.
- `crates/lisa-cli/src/init.rs` — action categories, append-only planner,
  writer-injected execution, mutation report, permission boundary, and tests.
- `crates/lisa-cli/src/templates.rs` — removes the obsolete legacy gitignore
  ownership constant.

### RDSPI artifacts

- `docs/active/work/T-030-02/research.md`
- `docs/active/work/T-030-02/design.md`
- `docs/active/work/T-030-02/structure.md`
- `docs/active/work/T-030-02/plan.md`
- `docs/active/work/T-030-02/progress.md`
- `docs/active/work/T-030-02/review.md`

No production file was created or deleted. One obsolete internal constant was
deleted. Unrelated dirty and untracked worktree content was preserved and not
included in ticket commits.

## Acceptance criteria assessment

| Criterion | Result |
|---|---|
| Preserve every existing ignore line | Met: original content is the exact merged prefix |
| Add only missing Lisa rules | Met: trimmed-line membership and ordered append |
| Never delete/reorder/rewrite project rules | Met: existing content is never reconstructed |
| Idempotent without trailing newline | Met: separator added once; second run is no-op |
| Idempotent with surrounding spacing | Met: comparisons use `line.trim()` |
| Retain `hooks/ntfy-topic` | Met in combined vend fixture |
| Secret remains ignored | Met with real `git check-ignore` invocation |
| Distinguish four action outcomes | Met via explicit enum variants and captured output |
| Real run reports exact file write set | Met via success-point record and before/after test |
| Skipped/unchanged absent from report | Met in output regression |
| Vend workflow customization covered together | Met in the same ignore/secret fixture |
| CLI contract documented | Met in README and command next steps |
| Focused and full suites pass | Met |

## Test coverage

### Append-only behavior

`test_append_only_gitignore_handles_spacing_newlines_and_idempotence` covers:

- a legacy rule file with no trailing newline;
- exact separator and append output;
- original-prefix preservation;
- a second plan producing a no-op;
- harmless spaces and tabs around all required rules;
- no content rewrite for the spaced current file.

`test_append_only_gitignore_preserves_unreadable_content` covers invalid UTF-8,
specific safety classification, and byte preservation.

### Field regression

`test_init_preserves_vend_customizations_and_secret_ignore_rule` uses one upgrade
fixture containing:

- committed-style Story Layer/read-the-story workflow additions;
- a customized historical stop hook;
- a customized notification sample;
- `.lisa/.gitignore` with `signals/` and `hooks/ntfy-topic`;
- the corresponding `.lisa/hooks/ntfy-topic` secret file.

It asserts read-only planning, safety skips for static customizations, the exact
planned append, real-run byte preservation, exact upgraded ignore content, and a
successful `git check-ignore .lisa/hooks/ntfy-topic` result.

### Reporting and permission behavior

`test_init_output_categories_and_mutation_report_match_write_set` creates a
single fixture with:

- a missing file create;
- a gitignore update;
- current no-op targets;
- an unknown hook safety skip.

It captures dry-run output, proves dry-run makes no changes, snapshots every file
action before a real run, compares after-state bytes, and asserts the report
equals the actual changed set and categories exactly. It also proves workflow and
skipped hook paths are absent from the report, a skipped hook retains mode
`0640` on Unix, and the second real run reports `none`.

### Existing coverage retained

- fresh Rust and Node initialization;
- init/validate round trips for Claude and Codex hooks;
- static current/known-prior/unknown ownership cases;
- non-UTF-8 workflow and hook preservation;
- TOML version and missing-key merges;
- JSON hook merges and malformed-input safety;
- create-only context-file preservation;
- fresh active-hook executable bits and notification-sample opt-in mode;
- validation diagnostics and CLI setup behavior.

## Verification results

- Focused init suite: 68 passed, 0 failed.
- Full CLI suite: 251 passed, 0 failed.
- Full workspace: 630 passed, 0 failed
  (251 CLI, 145 core, 234 plugin; doc-tests passed).
- `just check`: passed, including `cargo check` for the plugin on
  `wasm32-wasip1` and the full workspace suite.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

Warning-strict CLI clippy reaches one pre-existing needless-borrow diagnostic in
an older stale-version test near `init.rs:2032`, already documented in T-030-01.
The one new test-style diagnostic found during review was fixed; no strict clippy
finding points to T-030-02 code or tests.

## Commits

- `a2a85fc` — RDSPI research, design, structure, plan, and initial progress.
- `8796cca` — append-only merge, action/reporting implementation, regression
  tests, and operator documentation.
- A final scoped handoff commit contains the test-only clippy cleanup plus final
  progress and review artifacts.

## Open concerns and limitations

### Invalid UTF-8 is preserved, not merged

The action model stores text content, so a non-UTF-8 gitignore cannot be safely
inspected. Init visibly safety-skips it. This is conservative and avoids data
loss, but operators must add missing Lisa rules manually.

### Equivalence is intentionally narrow

Only exact rules with harmless surrounding whitespace are considered equal.
The merge does not infer equivalence between alternate glob spellings, inline
comments, or negation sequences. Existing duplicate project rules are retained.
This protects ordering and avoids interpreting project intent.

### Appended line endings use LF

Existing bytes and line endings remain untouched. If an existing file uses CRLF,
the appended suffix uses the LF format of the embedded template. This produces a
mixed-ending suffix but honors the stronger no-rewrite contract.

### Reports describe completed successful runs

Init remains non-transactional. If execution fails after earlier writes, the
command returns an error before the final success report. The successful-run
report is exact; rollback or partial-failure journaling is outside this ticket.

### Directories are excluded

`Files changed` intentionally reports files whose contents were written, matching
the ticket's operator-facing file set. Created directories remain visible in the
planned action list.

## Human review focus

- Confirm trimmed exact-line equality is the intended harmless-spacing boundary.
- Confirm absolute paths in the final report are consistent with existing plan
  output and desirable for operator inspection.
- Confirm limiting chmod to written hooks is the intended resolution of the
  prior ownership concern.
- Review the combined vend fixture as the permanent 0.3.0 to 0.4.0-rc.5 safety
  regression.

## Final assessment

All ticket acceptance criteria are implemented and covered. The original secret
exposure path is protected both structurally and through Git's own ignore
evaluation, customized workflow content remains intact, repeated init is
idempotent, and the command now provides a precise operator-visible record of
what it wrote and what it declined to touch. No critical issue remains open.
