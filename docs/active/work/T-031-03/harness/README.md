# T-031-03 atomic provider-contract harness

This deterministic harness exercises Lisa's real isolated Git commands in a
temporary repository outside the Lisa checkout. It runs five Codex-routed
tickets on one logical reused seat, a dependent Codex ticket, and a final Claude
ticket through the same transaction driver.

The harness uses real Git repositories, real `lisa init`, real `lisa validate`,
real `lisa commit-ticket`, and real `lisa complete-ticket` processes. It does not
launch a model or Zellij. Provider route, one-seat reuse, and dependency-start
events are deterministic fixture inputs; T-031-02's plugin tests independently
cover the live pending-seat and scheduler publication state machine.

## Run

Build Lisa and point the harness at it:

```bash
cargo build -p lisa-cli
LISA_BIN="$PWD/target/debug/lisa" \
  docs/active/work/T-031-03/harness/run.sh
```

Retain a successful fixture and its evidence:

```bash
LISA_BIN="$PWD/target/debug/lisa" \
  docs/active/work/T-031-03/harness/run.sh --keep
```

Or choose the external destination explicitly:

```bash
LISA_BIN="$PWD/target/debug/lisa" \
  docs/active/work/T-031-03/harness/run.sh --keep --root /tmp/lisa-t03103
```

Failures are always retained and print their root/evidence paths. Successful
runs clean up unless `--keep` is supplied. The Cargo integration test supplies
its just-built Lisa executable and runs this script automatically.

## Assertions

- Five Codex ticket starts reuse `seat-1`.
- A Claude ticket uses the same transaction flow.
- The dependent starts only after its prerequisite completion commit exists and
  is an ancestor of `HEAD`.
- Each implementation unit uses exact-path `lisa commit-ticket`.
- Each ticket's Done frontmatter first appears in its completion commit.
- All six work artifacts and final ticket source exist in the completion tree.
- No loop-owned source, ticket, or artifact residue remains.
- A foreign ordinary-index entry keeps the exact same staged mode/blob tuple
  across every implementation and completion commit.
- The foreign path enters no ticket commit.
- Exactly one completion/provenance receipt is recorded per ticket.

## Evidence

Evidence is outside the fixture repository so diagnostics cannot become loop
residue:

- `activity.jsonl`: route, seat, start, implementation, pending, confirmation;
- `provenance.jsonl`: final provider/seat/outcome/commit receipts;
- `commits.txt`: implementation and completion hashes;
- `index.before`, `index.current`, `index.after`: ordinary-index stage tuples;
- `status.final`: expected foreign staged entry and nothing else;
- `trees/<ticket>.txt`: complete commit-tree path listings;
- `<ticket>.ticket.done`: committed Done-frontmatter blobs;
- `init.txt` and `validate.txt`: fixture setup evidence.

These files are sufficient to distinguish a provider-attribution/order failure,
a missing completion receipt, a commit-tree omission, and ordinary-index drift.
