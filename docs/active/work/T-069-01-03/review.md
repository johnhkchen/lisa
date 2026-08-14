# T-069-01-03 — a release is promoted into nightly once, not judged on every box

Soak moved from every machine's clock to the publisher's. A scheduled job now
reads the release list, applies `channel.rs`'s own nightly rule, writes the
promotion pointer the tap and the apt suites are built from, and republishes
both — and does none of that when the answer has not changed.

## What changed

**The decision (Rust, shares one soak window with `channel.rs`)**

- `crates/lisa-cli/src/promote.rs` *(new)* — reads a releases-API response, calls
  `channel::resolve(Channel::Nightly, …)` with `channel::DEFAULT_SOAK_HOURS`, and
  returns one of three actions: `promote`, `retire`, `unchanged`. There is no
  second window and no second superseded rule; the promotion runs the same
  function a curl-installed box runs.
- `crates/lisa-cli/src/main.rs` — hidden `lisa promote-nightly`
  (`--releases`, `--pointer`, `--write`, `--json`, `--now`). It writes nothing
  without `--write`, and writes nothing at all when the pointer already says the
  answer.
- `crates/lisa-cli/tests/promote_nightly_cli.rs` *(new)* — the command end to end.
- `docs/knowledge/flag-audit.md` — the five new flags, all working defaults.

**The publishing (one definition, two callers)**

- `.github/workflows/publish-apt-repository.yml` *(new)* — the apt build-and-sign
  steps, lifted out of `release.yml` unchanged except for two things: it takes the
  tree to build from as an input, and it reads the promotion pointer from `main`
  rather than from that tree. It also emits a suite table into the run summary.
- `.github/workflows/publish-homebrew-tap.yml` *(new)* — the same move for the
  tap. It now takes both formulae from **release assets** instead of the release
  run's artifact store, which is what lets a promotion — which has no build of its
  own — use the identical path a release does.
- `scripts/commit-tap-formulae.sh` *(new)* — style, then commit only the formulae
  whose contents moved, with an optional reason line in the commit body.
- `.github/workflows/release.yml` — its two publish jobs are now `uses:` calls to
  the above. 251 lines deleted, 19 added; no behaviour intended to change.

**The promotion**

- `.github/workflows/promote-nightly.yml` *(new)* — hourly and on demand. Decides,
  writes the pointer to `main` when it moved, then calls both publish workflows.
  A run with nothing to promote stops after the decision: no commit, no rewritten
  formula, no re-signed suite. The pointer commit carries `[skip ci]` on purpose —
  `auto-release.yml` watches CI runs on `main`, and a promotion must never cut a
  release.
- `scripts/verify-nightly-promotion.sh` *(new)*, wired into `ci.yml` — rehearses
  the shell half against a throwaway tap.

**The docs**

- `docs/knowledge/nightly-promotion.md` *(new)* — the rule, the one window, the
  scoping answer for client-side soak, how to read where nightly stands without
  ssh, how to force or rehearse a promotion, and the failure mode.
- `packaging/apt/README.md`, `docs/knowledge/release-checklist.md`, `README.md`.

## Acceptance criteria

| criterion | where it is met |
| --- | --- |
| A scheduled job promotes soaked releases into the formula and the suite | `promote-nightly.yml`: decide → write pointer → call both publish workflows |
| "Superseded" is defined and tested | The newest release is the only candidate. `two_releases_inside_one_window_promote_neither` (unit **and** CLI), `a_superseded_release_is_never_promoted_even_after_it_soaks` |
| A promotion with nothing to do changes nothing | Three layers: `decide` returns `unchanged`, `write_pointer` skips identical contents, and the publish jobs do not run at all. `a_promotion_with_nothing_to_do_does_not_touch_the_pointer` asserts the file's mtime; `verify-nightly-promotion.sh` asserts the tap gains no commit |
| A yanked or deleted release is never promoted | The list is read at promotion time; drafts and half-uploaded assets are dropped. `a_yanked_release_is_never_promoted`, `a_release_still_uploading_is_not_promoted`, plus `retire`, which walks a dangling pointer back to `stable` |
| One stated window, agreeing with `channel.rs` | `channel::DEFAULT_SOAK_HOURS` through `channel::resolve` — the same code path, not a copied number. `the_window_judged_is_the_one_channel_rs_states` |
| Promotion is visible | `packaging/apt/nightly-tag.txt` plus its `git log`; the promotion commit message; a run summary on every run including no-ops; a reason line in the `lisa-nightly` tap commit (`verify-nightly-promotion.sh` step 4) |
| Client-side soak retired or scoped | Scoped, and said plainly in `nightly-promotion.md`: packaged boxes get the promotion's answer, curl and source boxes still compute it themselves from the same window. One rule in two places, because a package manager has no clock and an unmanaged box has no publisher |

## How it is tested

- `cargo test --workspace` — green (checked at HEAD in a clean worktree). 16 new
  unit tests in `promote.rs`, 9 new CLI tests.
- `cargo fmt --all -- --check`, `cargo clippy -p lisa-core -p lisa-cli -- -D warnings` — clean.
- `scripts/verify-nightly-promotion.sh` — passes locally and now runs in CI.
- `actionlint` — clean on all four workflow files (the only findings in
  `release.yml` are pre-existing shellcheck info in cargo-dist's own steps).
- **Live rehearsal**, against the real release list today, with a copied pointer
  and no `--write`:

  ```
  action:  promote        why: v0.5.0-rc.2 is the newest release and has cleared
  nightly: v0.5.0-rc.2         its 24h soak window
  canary:  v0.5.0-rc.2    late: v0.5.0-rc.2 became promotable 112h ago
  ```

  So the first real run moves `nightly` off `stable` and onto v0.5.0-rc.2, and
  says it is five days late — which it is, because until now nothing was promoting.

## What still concerns me

1. **The GitHub Actions half cannot be executed here.** Reusable-workflow wiring,
   `secrets: inherit`, and the Pages deployment are validated by `actionlint` and a
   YAML parse only. The first tag and the first hourly run are the real test.
   Watch specifically:
   - the `github-pages` environment now receives a deployment whose run ref is
     `main` rather than a tag; if that environment has a deployment-branch rule,
     the promotion's apt publish is where it will show up;
   - the tap publish now reads `lisa.rb` from release assets. Verified that every
     recent release carries it (`v0.5.0-rc.2`, `v0.4.4`, `v0.4.4-rc.9` all do), but
     it is a different source than the artifact store it used yesterday.
2. **`[skip ci]` on the promotion commit** keeps `auto-release.yml` from cutting a
   release off a one-word data change. It also means that commit is never
   CI-verified. It is one word, written by a tested tool, and the alternative is a
   promotion that ships a release.
3. **Hourly is a judgement call.** It costs a cached Rust build per hour and keeps
   "a day later" honest. Every six hours would be cheaper and would stretch the
   real wait to as much as 30 hours.
4. **A promotion job that stops firing is still not fully caught.** Late runs and
   half-published releases warn; a schedule that silently stops on an active
   repository does not. The runbook names the two-line check that catches it and
   says plainly that this is the residual risk.
5. **Two files crossed T-069-01-04's work, which ran concurrently.**
   - The README paragraph I wrote about `lisa-nightly` following the promotion was
     swept into their commit `b8f1a81`. The content is on `main` and correct; only
     the attribution is theirs.
   - `crates/lisa-cli/tests/help_surface.rs` — their new `upgrade` after_help left
     the snapshot stale, so `cargo test --workspace` on `main` was failing after
     `Complete T-069-01-04`. I refreshed the snapshot in its own commit
     (`741a0f1`), named as a sweep. It is outside this ticket's ownership; I took
     it because their seat was already released and `main` was red for everyone.
6. **Not in scope, and named in the ticket:** nothing here promotes anything to
   `stable`. That is still a person dropping `-rc` from the workspace version.
