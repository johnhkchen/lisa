# Plan: implement catalog-backed config upserts

## Step 1: add the metadata model

Modify `crates/lisa-cli/src/config.rs`.

- Define `ConfigKey` with dotted path, section, key, default, and description.
- Define `CONFIG_KEYS` with all 17 fixed CLI-parsed configuration keys.
- Use valid TOML right-hand-side text for every default.
- Use direct, one-line, outcome-focused descriptions.
- Keep the catalog crate-visible for `init.rs` and T-050-02-02.

Verification:

- `cargo fmt --all -- --check` parses the new declarations.
- A focused config test can enumerate the catalog.

## Step 2: centralize key recognition

Modify `validate_config` in `config.rs`.

- Add lookup helpers for dotted path and section/key membership.
- Replace fixed manual known-key arrays with catalog lookups.
- Preserve warning wording.
- Preserve phase-name and provider-name child validation.
- Preserve every semantic validation branch.

Verification:

- Existing unknown-section tests pass.
- Existing unknown-key tests for dirs, agent, runtime, guards, triage, and
  scheduling pass.
- Existing valid-config tests produce no warnings.

## Step 3: render the fresh template from metadata

Modify `default_config_toml` in `config.rs`.

- Keep active version, directory, and max-thread values.
- Render every optional key with its catalog description/default.
- Use empty inline tables for the two empty-map defaults.
- Keep all optional assignments commented.
- Keep the established top-level section ordering.

Verification:

- The generated document parses as `LisaConfig`.
- Existing fresh-template inertness tests pass.
- Resolved defaults remain unchanged.
- The generated text contains every catalog description and assignment.

## Step 4: add catalog completeness tests

Extend the `config.rs` test module.

- Build a TOML fixture containing every fixed field.
- Enumerate the fixture's fixed dotted paths.
- Compare the sorted fixture paths with sorted catalog paths.
- Assert catalog paths and section/key pairs are unique.
- Parse each `key = default` fragment in its section.
- Assert descriptions contain no newline.
- Assert descriptions end in a period.
- Assert descriptions begin with an approved direct verb.
- Assert descriptions avoid the repository's banned voice terms.

Verification:

- A missing catalog row reports the missing path in assertion output.
- A duplicated row reports its path.
- A malformed default reports its path and parse error.
- A voice failure reports the path and description.

## Step 5: make scheduling upserts catalog-backed

Modify `upsert_missing_config_keys` in `init.rs`.

- Remove the local hard-coded scheduling tuple list.
- Remove the special phase-timeout block.
- Iterate the scheduling catalog rows in stable order.
- For each absent leaf, insert its description and commented default at the
  current end of `[scheduling]`.
- Continue recognizing active and commented assignments as present.

Verification:

- Existing active-value preservation test passes.
- Existing commented-value preservation test passes.
- Missing phase-timeouts and provider-caps parents become inert assignments.
- A repeated upsert is byte-identical.

## Step 6: append absent sections

Extend `upsert_missing_config_keys` in `init.rs`.

- Add a byte-preserving block append helper.
- Iterate `agent`, `guards`, and `triage` in that order.
- Skip active or commented section headers already present.
- Build each absent block from catalog rows.
- Comment the section header and every assignment.
- Separate appended blocks cleanly without trimming old bytes.

Verification:

- A legacy dirs+scheduling file gains all three sections.
- The original file remains an exact output prefix.
- Files with no trailing newline remain unchanged before the appended separator.
- No setting becomes active.

## Step 7: add legacy and customization fixtures

Extend the `init.rs` test module.

- Create a current-version dirs+scheduling-only fixture.
- Include custom directory values, max threads, optional timeout, and a user
  comment.
- Run pure upsert and full init planning.
- Assert one planned `.lisa.toml` update.
- Assert each missing catalog stub exactly once.
- Assert custom text exactly once.
- Compare resolved configuration before and after.
- Run a second upsert and assert byte equality.

Verification:

- The fixture directly covers the first acceptance criterion.
- Failure messages identify the missing or duplicated path.

## Step 8: pin current-file no-op behavior

Use `config::default_config_toml()` as the canonical current fixture.

- Run the pure upsert and require exact byte equality.
- Run `plan_init_actions` and require a `.lisa.toml` `NoOp`.
- Add a customized current fixture containing active agent, guard, and triage
  values plus user comments.
- Require exact equality when every relevant section/key is already present.

Verification:

- No comments, values, or ordering change.
- No duplicate section is appended.

## Step 9: run focused formatting and tests

Commands:

```text
cargo fmt --all -- --check
cargo test -p lisa-cli config::tests
cargo test -p lisa-cli init::tests
```

If the package test filters do not match all intended tests, run the full
package suite instead.

Acceptance gate:

- All new tests pass.
- Existing config and init tests pass.
- Formatting is clean.

## Step 10: inspect the ticket-owned diff

Commands:

```text
git diff -- crates/lisa-cli/src/config.rs crates/lisa-cli/src/init.rs
git diff --check -- crates/lisa-cli/src/config.rs crates/lisa-cli/src/init.rs
git status --short
```

Inspection criteria:

- Only the two intended source files contain ticket source changes.
- No unrelated dirty file is included.
- No ticket-owned path is staged.
- Descriptions are present only in the catalog.
- Existing resolution logic is unchanged.
- Existing bytes are never trimmed in the merger.

## Step 11: commit the meaningful source unit through Lisa

The catalog and its init consumer are one compiling, meaningful unit. Commit
them through the isolated transaction:

```text
lisa commit-ticket \
  --ticket-id T-050-02-01 \
  --message "Make init discover every config section" \
  --include crates/lisa-cli/src/config.rs \
  --include crates/lisa-cli/src/init.rs
```

Do not use ordinary `git add`, `git commit`, or the ordinary index.

Commit gate:

- Lisa reports a successful ticket commit.
- Both source paths are clean afterward.
- Other tickets' dirty paths remain untouched.

## Step 12: run aggregate verification

Commands:

```text
cargo test -p lisa-cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Run the workspace suite in proportion to available time. Any repository-wide
failure must be classified as ticket-caused or pre-existing with concrete
evidence.

Acceptance gate:

- CLI tests pass.
- Workspace tests pass.
- Clippy passes with warnings denied.
- Formatting remains clean.

## Step 13: maintain `progress.md`

Create the implementation artifact before editing source.

Update it after:

- catalog implementation;
- init merger implementation;
- focused tests;
- Lisa source commit; and
- aggregate verification.

Record deviations before taking a changed implementation direction.

## Step 14: review the final repository state

Inspect:

- the source commit diff;
- exact catalog coverage;
- focused and aggregate test evidence;
- `git status --short`;
- absence of ticket-owned staged, modified, or untracked source files.

Map each acceptance criterion to a test or code path.

## Step 15: write Review artifacts

Write `review.md` in the private attempt work directory with:

- change summary;
- files modified;
- behavior and preservation guarantees;
- test coverage and command results;
- acceptance-criteria evidence;
- open concerns or limitations;
- source commit identity.

Write exactly this pass disposition when all gates are green:

```json
{"disposition":"pass","reason":null}
```

Otherwise write an actionable block disposition conforming to the assignment.

## Step 16: validate disposition and stop

Run:

```text
lisa check-disposition T-050-02-01
```

Correct every reported issue. Do not change ticket phase/status, publish
artifacts, start a dependent ticket, or release the seat. Lisa owns completion.
