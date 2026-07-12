# Plan — T-036-01-02: plain, verb-forward command help

One file, one atomic copy change. The plan is short because the work is small and
the risk is that a stray edit disturbs an attribute or match arm — so the steps
are ordered to keep the diff provably comment-only and to verify the jargon ban.

## Step 1 — Apply the twelve `///` rewrites

Edit `crates/lisa-cli/src/main.rs`, replacing exactly the doc-comment lines named
in structure.md, in one pass:

- Operator five: Init, Validate, Status, Doctor, Loop.
- Hidden three: SetupGuide, HooksGuide, Version.
- Hook four: AgentExec (first line only, keep body), CaptureUsage, CommitTicket,
  CompleteTicket.

Touch only `///` text. Leave every `#[command(...)]`, field, and match arm
untouched.

**Verify:** `git diff crates/lisa-cli/src/main.rs` shows only `///`-line changes
(no attribute, field, or dispatch lines in the diff).

## Step 2 — Build and read the rendered help

```
cargo build -p lisa-cli --release
./target/release/lisa --help
for c in init validate status doctor loop; do ./target/release/lisa $c --help; done
./target/release/lisa agent-exec --help   # confirm body still present
```

**Verify:**
- The parent command list shows the five new operator short lines.
- Each operator `<cmd> --help` opens with its new sentence.
- Loop's line reads "Start a run: … in parallel where they don't collide."
- AgentExec's long help still carries the env-var / codex-exec body.

## Step 3 — Jargon-ban check on operator strings

Grep the operator help for banned terms:

```
./target/release/lisa --help | sed -n '/Commands:/,/^$/p' \
  | grep -Ei 'dag|orchestrat|scheduling|leverage|solutions' && echo "FAIL" || echo "clean"
```

Also eyeball each operator `--help` opening line for the same terms.

**Verify:** zero banned-term hits in the five operator lines. (Hidden/hook
commands are not jargon-gated, but they were de-jargoned in Step 1 too.)

## Step 4 — No-regression: command set intact + tests green

```
for c in init validate status doctor loop agent-exec capture-usage \
         commit-ticket complete-ticket setup-guide hooks-guide version; do
  ./target/release/lisa $c --help >/dev/null 2>&1 && echo "$c ok" || echo "$c BROKEN"
done
cargo test --workspace
```

**Verify:** all 12 subcommands resolve; `cargo test --workspace` stays green (no
test reads help yet — T-036-01-03 adds that lock).

## Step 5 — Commit through Lisa's isolated transaction

Single meaningful unit (the whole copy change is one atomic edit to one file):

```
lisa commit-ticket --ticket-id T-036-01-02 \
  --message "T-036-01-02: plain verb-forward per-command help" \
  --include crates/lisa-cli/src/main.rs
```

Only the ticket-owned path is included. No ordinary `git add`/`git commit`. After
the commit, `git status` must show no ticket-owned file left staged, modified, or
untracked (the work/ artifacts are published by Lisa, not committed here).

## Testing strategy

- **Unit/integration tests:** none added by this ticket — the story assigns the
  help-surface regression test to T-036-01-03. Adding it here would collide with
  that ticket's file ownership.
- **Verification is by running the built binary** (Steps 2–4): the help text is
  the artifact, so reading the rendered `--help` output *is* the test, plus the
  grep-based jargon check and the resolve-all-12 check.
- **Regression guard:** `cargo test --workspace` confirms nothing else broke,
  since the edit is comment-only and no existing test asserts help text.

## Rollback

If any step fails, the change is a set of one-line comment edits in one file;
revert the file and re-derive from structure.md. No data, config, or interface
migration is involved.
