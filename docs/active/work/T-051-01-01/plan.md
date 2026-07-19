# Plan — T-051-01-01 defang-the-timing-flake

## Step 1 — Apply the test edit

- Edit `crates/lisa-cli/src/triage_agent.rs`: remove the
  `assert!(started.elapsed() < Duration::from_secs(3));` line from
  `bounded_runner_kills_timeout_near_the_configured_deadline`; add the rationale
  comment (see structure.md for exact text).
- Verify: `cargo test -p lisa-cli --bins bounded_runner` → 2 passed.
- Commit: `lisa commit-ticket --ticket-id T-051-01-01 --message "..." --include crates/lisa-cli/src/triage_agent.rs`.

Atomic and independently verifiable. This is the only source change.

## Step 2 — Prove the negative fixture (mutation, not committed)

Purpose: satisfy AC2 — a broken kill path must still turn the test red.

- Temporarily mutate the production poll loop so the deadline is not enforced.
  Minimal mutation: change the timeout guard
  `Ok(None) if started.elapsed() >= deadline =>` to `Ok(None) if false =>`
  (deadline branch dead). The child then runs `sleep 30` to completion; empty
  stdout → `extract_candidate` fails → `Err(Failed(...))`.
- Run `cargo test -p lisa-cli --bins bounded_runner_kills_timeout_near_the_configured_deadline`.
- Expected red: `assert_eq!(error, TimedOut)` fails with `left: Failed(...) right: TimedOut`.
  (Runtime ~30s because the child sleeps out — acceptable for a one-off.)
- Record the exact failure output in `progress.md`.
- **Revert** the mutation (`git checkout -- crates/lisa-cli/src/triage_agent.rs`
  would also drop Step 1, so instead re-apply by hand / re-edit the single guard
  line back). Confirm the file matches the committed Step 1 state:
  `git diff crates/lisa-cli/src/triage_agent.rs` shows no uncommitted delta.

Testing strategy note: this is a manual mutation demonstration, not an
added/committed test. Nothing about the mutation is committed; only its observed
red result is recorded in `progress.md`.

## Step 3 — Gate: `just check`

- Run `just check` (fmt + clippy + workspace tests). Judge by exit code, not by
  grepping output (per repo memory: never read a gate's pass/fail from scraped
  text). Must exit 0.

## Step 4 — 20 consecutive full-parallel workspace runs (AC1)

- Loop `cargo test --workspace` 20 times at full default parallelism (do not pass
  `--test-threads=1`; the point is to reproduce contention). Capture each run's
  exit code and whether `bounded_runner_kills_timeout_near_the_configured_deadline`
  failed.
- Success = 20/20 exit 0 with zero bounded-runner failures.
- Record the tally in `progress.md`. If any run fails for an *unrelated*
  pre-existing flaky test, note it distinctly — AC1 is specifically about the
  bounded-runner test not failing.

## Step 5 — Review artifacts

- Write `review.md` (changes, test coverage, open concerns).
- Write `review-disposition.json`:
  `{"disposition":"pass","reason":null}` if Steps 1–4 all succeeded.
- Run `lisa check-disposition T-051-01-01`; fix anything it reports.

## Verification criteria summary

| AC | Verified by |
|----|-------------|
| 1. 20× workspace green, no bounded-runner failure | Step 4 tally |
| 2. Broken kill path still red | Step 2 mutation demonstration |
| 3. Comment on load-immunity; no retry/sleep constructs | Step 1 diff review |
| 4. No production change unless runner causes spread | Diff is test-only; research documented the runner is correct |

## Rollback

Single-file, single-assertion change. If any gate regresses unexpectedly, revert
`crates/lisa-cli/src/triage_agent.rs` to its pre-ticket state and re-open Design.

## Commit discipline

- Exactly one `lisa commit-ticket` in Step 1, `--include crates/lisa-cli/src/triage_agent.rs`.
- No ordinary `git add`/`git commit` for ticket work. The Step 2 mutation is
  never staged or committed. After Step 2, no ticket-owned file is left modified
  or staged.
