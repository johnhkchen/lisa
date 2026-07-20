# Plan — T-051-01-03 defang-the-checksum-flake

Six steps. Steps 1–2 are the change; 3–7 are verification. Only Step 2 produces
a commit.

---

## Step 1 — Repair `FixtureServer` (the actual fix)

Edit `crates/lisa-cli/src/runtime.rs`, `FixtureServer::start` accept loop:

- Insert `stream.set_nonblocking(false).unwrap();` as the first statement of the
  `Ok((mut stream, _))` arm, before `read_request_headers`.
- Add the rationale comment specified in structure.md (names the BSD `accept()`
  inheritance, the unread-request → RST chain, and why this is not a retry).

Leave the listener non-blocking. Leave `read_request_headers`, `write_response`,
`FixtureResponse`, `request_count`, `join`, and `Drop` untouched.

**Verify:** `cargo test -p lisa-cli --bins runtime::tests` → exit 0.

Not committed yet — Step 2 lands with it as one logical unit.

---

## Step 2 — Self-diagnosing assertions + commit

Edit the same file, `checksum_mismatch_is_named_and_leaves_no_partial_install`:

- Add the contract comment above the assertion block.
- Attach `{error}` (and `request_count` on the category assertion) to the
  assertion messages. Keep all seven checks; weaken none.

**Verify:** `cargo test -p lisa-cli --bins runtime::tests` → exit 0, and
`cargo fmt --check` + `cargo clippy` clean for this crate.

**Commit** — the only commit this ticket makes:

```
lisa commit-ticket --ticket-id T-051-01-03 \
  --message <message> \
  --include crates/lisa-cli/src/runtime.rs
```

Message avoids backticks (T-051-01-01 recorded shell substitution eating a
backticked phrase from a commit message). Confirm the commit landed and that
`git status --porcelain crates/lisa-cli/src/runtime.rs` is empty afterward.

---

## Step 3 — Mutation M1: mismatch not detected (AC3, primary)

Temporarily edit production code at `runtime.rs:471`:

```rust
if actual_sha256 != release.sha256 {   →   if false {
```

**Expect red.** The guard no longer rejects the bad archive, so
`ensure_managed_zellij` returns `Ok(())` and the test's `.unwrap_err()` panics.

Run `cargo test -p lisa-cli --bins checksum_mismatch`, capture the exact failure
output into progress.md, then **revert**.

**Post-revert check:** `git diff crates/lisa-cli/src/runtime.rs` must be empty
(Step 2 already committed the intended change), and `grep "if false"` clean.
The mutation is never staged and never passed to `lisa commit-ticket`.

---

## Step 4 — Mutation M2: partial install left behind (AC3, second half)

Temporarily disarm `TempInstall`'s cleanup so the temporary install directory
survives the failed install (set `published: true` at construction, or make
`Drop` a no-op).

**Expect red** at `assert_no_install_temporary_directories` — or at
`assert!(!executable.parent().unwrap().exists())`, whichever the leftover trips
first. Either proves the "leaves no partial install" half of the test's name is
load-bearing.

Capture output, **revert**, re-verify `git diff` is empty.

---

## Step 5 — `just check`

Run `just check` (fmt + clippy + workspace tests). **Judge by exit code, never
by grepped output** — a scraped pipeline previously masked a real CI failure in
this workspace.

Caveat to apply when reading the result: `just check` builds the whole
workspace, including `crates/lisa-plugin`, which sibling ticket T-051-02-01 was
observed actively editing on this shared branch. A `lisa-plugin` **compile**
failure is a confounder, not a regression from this ticket — it must be
identified as such and disclosed, not silently absorbed. Any `lisa-cli` test
failure, by contrast, is this ticket's problem.

---

## Step 6 — 32× concurrent run (the flake-retirement evidence)

Re-run the exact harness that reproduced the ticket's 4/32:

```
cargo build --tests -p lisa-cli
32 concurrent invocations of the compiled lisa-cli test binary
```

**Expect: 0** `checksum_mismatch_is_named_and_leaves_no_partial_install`
failures, versus 4 before the fix.

Also record `second_managed_resolution_performs_zero_network_calls` (3 before
the fix) — the harness repair should retire it too, which is corroborating
evidence that the diagnosis was right rather than that the timing merely shifted.

Expected to still fail here and to be disclosed, not fixed:
`triage_agent::tests::bounded_runner_returns_valid_proposal_and_surfaces_failure`
(32/32 before) and `unblock::tests::timeout_is_bounded_and_kills_the_shell_group`
(1/32 before). Both are other files, other tickets.

---

## Step 7 — 20 consecutive `cargo test -p lisa-cli` runs (AC2)

The ticket's literal tally. Twenty consecutive runs, each judged by exit code,
recorded as a per-run pass/fail list plus a total in progress.md.

**Pass condition for AC2:** zero `checksum_mismatch_is_named_and_leaves_no_partial_install`
failures across all twenty. Any non-checksum failure is tallied separately and
disclosed the way T-051-01-01 disclosed its `lisa-plugin` compile confounders —
named, attributed, and shown to be outside this ticket's file.

---

## Testing strategy

- **What gets a test:** nothing new. This ticket repairs an existing test and
  the helper beneath it; adding a test for the fixture server would test the
  harness rather than the product.
- **What proves the fix:** the before/after failure counts under the identical
  reproduction harness (4/32 → expected 0/32), plus the 1500-trial controlled
  probe already recorded in research.md (5/1500 → 0/1500).
- **What proves the test still works:** mutations M1 and M2, each observed red
  and reverted.
- **Regression surface:** `just check` by exit code.

## Deviation policy

If Step 6 shows any checksum failure, the diagnosis is incomplete: stop, do not
retry or widen the assertion, and record the failure in progress.md before
re-entering analysis. If Step 5 or 7 surfaces a failure inside
`crates/lisa-cli` that is not the checksum test, treat it as in-scope
information — investigate and disclose it rather than tallying around it.

If the installer turns out to race after all (contradicting research), that is a
review.md finding with a proposed follow-up ticket, per AC5 / N4 — not a quiet
patch here.
