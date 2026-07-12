# Plan: cross-policy deadline regression

## 1. Establish baseline and ownership

- Inspect `git status --short`.
- Confirm only Lisa-managed paths are already modified.
- Confirm `crates/lisa-plugin/src/deadline.rs` is clean and unstaged.
- Run existing evaluator and characterization tests.

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
cargo test -p lisa-plugin characterizes_ --no-fail-fast
```

Verification: all pass before editing and ticket source has no prior work.

## 2. Add exact-action regression

Modify the inline test module in `deadline.rs` and add:

```text
cross_policy_deadline_actions_remain_distinct
```

Within one fixed-clock test:

- assert expired acknowledgement pane and captured state;
- assert all three exact transition variants in order;
- assert review ticket and pane action;
- assert health previous/current observation;
- assert exact session reclaim deadline payload;
- assert exact stale action identity.

Focused verification:

```text
cargo test -p lisa-plugin cross_policy_deadline_actions_remain_distinct --no-fail-fast
```

## 3. Add exemption/action matrix regression

Add:

```text
cross_policy_activity_and_human_exemptions_remain_distinct
```

Cover acknowledgement:

- expired candidate produces an action;
- comment records absence of activity/human exemption inputs.

Cover transitions:

- recent exit still fires;
- awaiting-human exit still fires;
- recent stop and clear are suppressed;
- awaiting-human stop and clear are suppressed.

Cover Review:

- recent activity suppresses prompt action;
- awaiting-human suppresses prompt action;
- quiet non-human input fires.

Cover health:

- recent activity produces Healthy;
- quiet activity produces Stuck even for the state-layer human case;
- document awaiting-human is intentionally not evaluator input.

Cover session:

- recent activity produces Warn;
- awaiting-human produces Warn;
- quiet non-human produces Reclaim;
- assert identities and deadline payloads.

Cover stale:

- recent activity suppresses action;
- awaiting-human suppresses action;
- quiet non-human produces the exact action.

Focused verification:

```text
cargo test -p lisa-plugin cross_policy_activity_and_human_exemptions_remain_distinct --no-fail-fast
```

## 4. Run focused regression group

```text
cargo test -p lisa-plugin cross_policy_ --no-fail-fast
```

Verification: both new tests pass and neither is accidentally filtered.

## 5. Format source

```text
cargo fmt --all
cargo fmt --all -- --check
```

Inspect status. Only `deadline.rs` may change as ticket-owned source.

## 6. Run evaluator and characterization coverage

```text
cargo test -p lisa-plugin deadline::tests --no-fail-fast
cargo test -p lisa-plugin characterizes_ --no-fail-fast
```

Verification:

- evaluator unit tests pass;
- all six state-layer characterizations remain green;
- no characterization test was edited.

## 7. Run complete plugin suite

```text
cargo test -p lisa-plugin --no-fail-fast
```

Verification: all executed plugin tests pass; environment-gated ignores remain
reported rather than treated as failures.

## 8. Run workspace suite

```text
cargo test --workspace --no-fail-fast
```

Verification: plugin, CLI, and core tests pass.

## 9. Run explicit Clippy gate

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

This is required by acceptance. No warning is permitted, including test code.

## 10. Run repository check

```text
just check
```

Verification: WASM plugin check and repository-defined tests pass. If a host
prerequisite is absent, record the exact limitation rather than masking it.

## 11. Inspect final diff

```text
git diff -- crates/lisa-plugin/src/deadline.rs
git diff --check
git status --short
git diff --cached --name-only
```

Review for:

- exactly two new test names;
- no production behavior change;
- all policy families represented;
- all transition action variants represented;
- session warning/reclaim distinction represented;
- no generic action abstraction;
- exact payload assertions;
- no unrelated formatting;
- empty ordinary index for ticket source.

## 12. Update progress

Write attempt-private `progress.md` recording:

- baseline results;
- implementation contents;
- per-policy matrix coverage;
- exact commands and counts;
- deviations;
- commit readiness.

Do not write to shared active work.

## 13. Commit meaningful source unit

Use Lisa's isolated transaction:

```text
lisa commit-ticket --ticket-id T-039-04-03 \
  --message "test(plugin): lock cross-policy deadline contracts" \
  --include crates/lisa-plugin/src/deadline.rs
```

Do not use ordinary `git add` or `git commit`, and do not broaden the include.

Verification:

- Lisa reports success and a commit hash;
- commit contains exactly `deadline.rs`;
- source path is clean and unstaged afterward;
- Lisa-managed paths remain outside the source commit.

## 14. Post-commit verification

```text
git show --stat --oneline HEAD
git status --short
git diff --cached --name-only
cargo test -p lisa-plugin cross_policy_ --no-fail-fast
```

Verification: commit matches plan, focused tests remain green, and no owned
source remains modified, staged, or untracked.

## 15. Review

Write attempt-private `review.md` with:

- outcome summary;
- file inventory;
- exact action coverage;
- exemption matrix coverage;
- gate coverage;
- commit hash;
- limitations and open concerns;
- acceptance assessment.

Do not edit ticket phase/status, publish artifacts, or begin another ticket.
Stop after review and wait for Lisa's completion transaction.

## Atomicity assessment

Both tests belong in one commit because together they express the contract:

- one locks what each policy does;
- one locks when activity and awaiting-human alter or suppress that action.

Splitting them would leave acceptance partially represented. No production
change is needed between them.

## Success criteria

- both cross-policy tests exist and pass;
- existing characterizations pass unchanged;
- plugin and workspace suites pass;
- Clippy passes with warnings denied;
- repository check passes;
- source commit contains the exact owned path;
- no ticket-owned source remains dirty or staged;
- progress and review accurately hand off the result.
