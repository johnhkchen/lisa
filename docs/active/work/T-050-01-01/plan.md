# Plan: init-history-default

## Delivery strategy

Implement the ticket in two ticket-owned commits:

1. history default behavior plus unit and integration fixtures;
2. README and Chromebook runbook guidance.

Each commit uses `lisa commit-ticket` with exact repository-relative paths. The
ordinary index is never used. Private RDSPI artifacts remain uncommitted in the
attempt directory for Lisa to admit during completion.

## Step 1: Establish the positive copy contract

Modify `crates/lisa-cli/src/init.rs`.

- Add `HISTORY_KEPT` beside the existing history copy constants.
- Use the exact ticket sentence, including the em dash and final period.
- Extend the copy-focused unit test to pin the literal.
- Preserve the offer wording unchanged.
- Preserve the assertion that the offer contains no `git` token.
- Preserve the decline consequence wording unchanged.

Verification:

- the source has one positive announcement literal;
- unit tests compare the constant exactly;
- no prompt-copy regression occurs.

## Step 2: Represent unavailable history tooling

Extend private `RepositoryState` in `init.rs`.

- Add `Unavailable { reason: String }`.
- Keep `Missing`, `Unborn`, and `Born` meanings unchanged.
- Map `io::ErrorKind::NotFound` from the initial Git spawn to unavailable.
- Give command-not-found a stable reason naming Git.
- Map other launch failures to unavailable with contextual reasons.
- Preserve ordinary not-a-repository results as `Missing`.
- Map unexpected Git inspection failures to unavailable.
- Map empty repository-root output to unavailable.
- Map follow-up HEAD/symbolic-ref launch or inspection failures to unavailable.
- Do not execute any mutating command in repository discovery.

Verification:

- missing repository with Git remains distinguishable from missing Git;
- successful born/unborn discovery remains byte-for-byte behaviorally identical;
- all enum matches compile exhaustively.

## Step 3: Replace the no-flag demand with resolution

Refactor `resolve_history_action` without changing its public boundary.

- Return `None` immediately for born repositories.
- Keep `NoHistory` mapped to `Decline` for relevant states.
- Map explicit `WithHistory + Unavailable` to a named error.
- Include the preserved underlying reason in that error.
- Include instructions to install or repair Git and retry.
- Include `--no-history` as the explicit journal remedy.
- Preserve interactive prompting for `Ask`.
- Map interactive rejection to `Decline`.
- Map interactive acceptance plus unavailable state to `Decline`.
- Map interactive acceptance plus missing/unborn to existing creation actions.
- Map non-interactive `Ask + Missing` to repository creation.
- Map non-interactive `Ask + Unborn` to initial commit creation.
- Map non-interactive `Ask + Unavailable` to `Decline`.
- Remove the obsolete flag-required error.

Verification:

- resolver tests cover every unavailable/preference distinction;
- non-interactive no-flag with usable Git chooses history;
- interactive yes with unavailable Git returns journal fallback;
- explicit with-history with unavailable Git is the only unavailable-state error.

## Step 4: Align dry-run behavior

Remove the early special case that prints the offer and future flag demand.

- Ensure dry run never calls `prompt_for_history`.
- Resolve no-flag dry run automatically from observed capability.
- Report that project history would be kept for creation actions.
- Report the existing consequence for decline.
- Preserve no-mutation behavior.
- Keep born repositories free of irrelevant history copy.

Verification:

- dry-run unit test uses empty input and does not fail;
- output does not contain the prompt;
- output does not instruct callers to choose flags;
- `.git` and scaffold files remain absent.

## Step 5: Announce actual decisions

Update `run_init_with_io` action execution output.

- Run existing repository bootstrap before printing positive copy.
- Print exact `HISTORY_KEPT` after successful missing-repository setup.
- Print exact `HISTORY_KEPT` after successful unborn initial commit.
- Retain exact `HISTORY_DECLINED` for explicit decline and fallback.
- Preserve blank-line grouping and the later init summary.
- Do not print decision copy for born repositories.

Verification:

- failures cannot print a false positive announcement;
- black-box stdout contains the exact required sentence;
- fallback stdout contains the exact existing consequence.

## Step 6: Update unit tests

In `init.rs` tests:

- retain offer default/retry parsing coverage;
- retain end-of-input rejection coverage;
- replace the explicit-flag-required test;
- test bare non-interactive usable-history success;
- test unavailable non-interactive fallback at resolver level;
- test unavailable interactive-accept fallback with `yes` input;
- test unavailable explicit-with-history actionable failure;
- test unavailable explicit-no-history decline;
- update dry-run expectations to automatic preview;
- retain all unrelated init tests unchanged.

Verification command:

```bash
cargo test -p lisa-cli --lib init::tests
```

Expected result: all init unit tests pass with no global environment mutation.

## Step 7: Extend compiled-CLI fixtures

Modify `crates/lisa-cli/tests/init_history.rs`.

- Add test-local `HISTORY_KEPT`.
- Add a Lisa helper with an empty controlled `PATH`.
- Remove `--with-history` from the fresh usable-history invocation.
- Assert exact positive announcement.
- Retain local/global identity assertions.
- Retain empty-root and later transaction assertions.
- Retain commit-seal status assertion.
- Add bare no-Git init success fixture.
- Assert exact decline consequence and absent `.git`.
- Write a fixture ticket and prove journal-only status in no-Git mode.
- Add explicit with-history/no-Git failure fixture.
- Pin the named install/repair and no-history remedy.
- Assert failure occurs before scaffold writes.
- Keep conflicting-flags validation.
- Retain born repository full metadata snapshot.
- Retain unborn config and index byte snapshots.

Verification command:

```bash
cargo test -p lisa-cli --test init_history
```

Expected result: all former safety fixtures and new default fixtures pass.

## Step 8: Format and inspect the source unit

- Run `cargo fmt --all -- --check`.
- If formatting is needed, run the repository formatter.
- Inspect only the two source-unit diffs.
- Confirm no weakened assertions or unrelated cleanup.
- Run targeted unit and integration commands again after formatting.
- Run `cargo test -p lisa-cli --test help_surface` to catch accidental CLI drift.

Inspect with:

```bash
git diff -- crates/lisa-cli/src/init.rs crates/lisa-cli/tests/init_history.rs
```

## Step 9: Commit the source unit through Lisa

Use exactly:

```bash
lisa commit-ticket \
  --ticket-id T-050-01-01 \
  --message "Make init choose the strongest history default" \
  --include crates/lisa-cli/src/init.rs \
  --include crates/lisa-cli/tests/init_history.rs
```

If the installed CLI requires an explicit project path, add only the repository root
argument supported by its help. Do not broaden include paths.

Post-commit checks:

- neither file is modified, staged, or untracked;
- unrelated Lisa-owned ticket-file changes remain untouched;
- ordinary index contents are unchanged.

## Step 10: Update README guidance

Modify only relevant Quick Start and CLI reference paragraphs.

- Keep bare `lisa init` as the first and normal command.
- Say Lisa keeps history automatically when available.
- Say Lisa falls back to its journal automatically when unavailable.
- Note that interactive runs retain the offer.
- Label `--with-history` and `--no-history` as overrides.
- Remove language requiring scripts or agents to choose a flag.
- Preserve existing-repository safety language.

Verification:

- `rg` finds bare init in Quick Start and CLI reference;
- no README prose says scripts must pass a history flag;
- both flags remain documented.

## Step 11: Update Chromebook runbook

Modify `docs/knowledge/chromebook-install-test.md`.

- Change no-Git leg instruction to bare `lisa init`.
- Explain that automatic journal fallback is part of the measurement.
- Change fresh-container init command to bare `lisa init`.
- Remove the 0.4.4 designed-error note.
- Remove claims that non-interactive input requires a flag.
- State that flags are deliberate test overrides only.
- Do not change unrelated grader mechanics.

Verification:

```bash
rg -n "lisa init|with-history|no-history|designed behavior|non-interactive" \
  README.md docs/knowledge/chromebook-install-test.md
```

Expected result: normal paths use bare init; flags appear only as overrides.

## Step 12: Commit the documentation unit through Lisa

Use exactly:

```bash
lisa commit-ticket \
  --ticket-id T-050-01-01 \
  --message "Teach bare init as the automatic history path" \
  --include README.md \
  --include docs/knowledge/chromebook-install-test.md
```

Post-commit checks mirror the source unit: both files must be clean and unrelated
working-tree changes must remain untouched.

## Step 13: Run broad verification

Run, in increasing scope:

```bash
cargo test -p lisa-cli --lib init::tests
cargo test -p lisa-cli --test init_history
cargo test -p lisa-cli --test help_surface
cargo test -p lisa-cli
cargo test --workspace
cargo fmt --all -- --check
```

If workspace tests are prohibitively slow or an unrelated concurrent failure occurs,
record the exact command and evidence in `progress.md` and `review.md`. Do not weaken
ticket tests to obtain a pass.

## Step 14: Audit acceptance evidence

Map each criterion to executable evidence:

- fresh + Git + no flag -> compiled-CLI fixture;
- repository/local identity/root commit -> same fixture;
- subsequent commit seal -> transaction and status assertions;
- fresh + no Git + no flag -> no-path fixture;
- consequence line + exit zero -> exact output/status assertions;
- explicit with-history + no Git -> failure fixture with remedy assertions;
- interactive accept + no Git -> injected-I/O resolver test;
- born/unborn byte safety -> retained snapshot fixtures;
- README bare path and override docs -> reviewed diff and `rg`;
- runbook bare path and removed error note -> reviewed diff and `rg`.

## Step 15: Write progress and Review artifacts

Maintain `progress.md` during implementation with:

- completed steps;
- exact test commands and outcomes;
- commit identifiers returned by Lisa;
- deviations and rationale;
- remaining work.

After all source/docs are clean and verification is green:

- write `review.md` with change summary, test evidence, and concerns;
- write exactly `{"disposition":"pass","reason":null}` to disposition JSON;
- run `lisa check-disposition T-050-01-01`;
- fix every reported issue;
- remain on this ticket without editing ticket phase/status or starting other work.
