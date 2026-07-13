# Review: authorized Codex field report

## Disposition

BLOCK.

The ticket is not ready for Lisa's completion transaction.

The exact release CLI and embedded WASM were exercised in isolated disposable
Zellij repositories with live Codex seats.
The pre-ownership case fully passed.
The blocking Review case generated the required live disposition and matching
acknowledgement, but the harness exited before retaining the final no-Done,
dependent, provenance, and commit snapshots.

That gap is material and is correctly treated as blocking rather than inferred
from deterministic proof or hidden by a redundant metered rerun.

## What changed

No product source file was created, modified, or deleted.

Attempt-private artifacts created:

- `research.md`;
- `design.md`;
- `structure.md`;
- `plan.md`;
- `live-field-harness.sh`;
- `live-evidence/` raw receipts and snapshots;
- `progress.md`, the canonical field report;
- `review.md`;
- `review-disposition.json`.

The private harness was adjusted only for its own portability and evidence flow.
No scheduler, CLI, core type, workflow template, or repository ticket was patched.

## Exact artifact binding

The observed executable was:

```text
target/release/lisa
sha256 498134e92f43ea5a3d834c5cb22afdf5d6ad180e2543ae543b4ae84588addfe9
```

The observed instantiated WASM was:

```text
sha256 053c48c6176987d90d64979c0593001b2a6d70196c7e689ef3e8de87a41a896f
```

These match T-040-03-03 exactly.
Both fixture layouts named the pinned executable and extracted a plugin with
the expected WASM digest.

## Deterministic coverage

No deterministic command was rerun.

The review relies on the admitted predecessor record for:

- optimized release builds;
- fresh build-script embedding;
- formatting and native/WASM Clippy;
- 794 passing tests;
- canonical `just check`;
- the named blocking Review regression;
- the named pre-ownership producer/CLI regression.

The report labels these facts as inherited deterministic proof.

## Live blocking Review coverage

Observed and retained:

- external disposable fixture;
- exact build identity;
- attempt-1 lease and started signals;
- real Codex generation-1 `UserPromptSubmit` acknowledgement;
- model and session/turn identifiers;
- six Codex-authored phase artifacts;
- exact valid block disposition;
- blocking Review content;
- terminal snapshots showing artifact verification;
- fixture and credential-home removal.

Missing because the harness asserted a sampled UI label before common capture:

- final ticket frontmatter;
- final dependent frontmatter and attempt absence;
- final provenance ledger;
- final Git HEAD/log/tree/status;
- durable post-stop proof of no completion.

The deterministic regression covers those semantics internally,
but the ticket requires live evidence too.
This is the blocking gap.

## Live pre-ownership coverage

The second case is complete.

It observed a real Codex pane `terminal_0` and closed it before ownership.
The scheduler appended exactly one schema-v3 assignment-transition record:

```text
ticket:       T-LIVE-PREOWN
attempt:      1
pane:         0
provider:     openai
state:        delivery-failed
reason:       provider did not acknowledge the bounded chat assignment
duration:     40 seconds
```

The pinned CLI reconstructed the same state, reason, provider, and timestamps.
No execution outcome existed.
The ticket stayed open in Research.
Baseline and final HEAD were identical.
No Done commit occurred.

## Teardown review

Both external fixture roots are absent.
Both ephemeral Codex homes are absent.
Both live Zellij processes were killed.

An audit found exited session metadata remained after `kill-session`.
Both named sessions were then deleted explicitly.
The final session listing contains no `l40*` entry.

Physical teardown is complete,
but the initial receipt's immediate session-absence check was too optimistic.
The field report documents that limitation.

## Harness setup issues

Three pre-observation private-harness issues were corrected:

- Zellij session name length;
- optional-signal strict-shell return status;
- macOS Bash associative-array incompatibility.

They occurred before provider behavior under test and did not change Lisa.

The post-turn UI assertion is not categorized as harmless setup.
It lost required evidence after a live Codex turn and therefore blocks.

## Repository hygiene

No ticket-owned product source remains staged, modified, or untracked.
No ordinary index operation was used for ticket work.
No source transaction was needed.

Lisa-managed ticket/provenance mutations and published artifact copies remain
outside this ticket's direct ownership.
Pre-existing plugin-relative fixture residue was preserved.

## Open concern and required action

The live blocking Review must be observed once more under explicit authorization
with final negative-outcome capture performed before UI assertions.
That run must retain:

- ticket and dependent final frontmatter;
- dependent attempt absence;
- final provenance ledger;
- final Git HEAD/log/tree/status;
- explicit session deletion receipt.

Until that evidence exists, Done would overstate the field gate.
