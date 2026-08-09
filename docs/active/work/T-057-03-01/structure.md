# Structure — T-057-03-01, release-0-5-0-rc-1

## Files this ticket owns

| Path | State | Role |
|---|---|---|
| `Cargo.toml` | modified, committed `09570c9` | workspace `version = "0.5.0-rc.1"`; three crates inherit |
| `Cargo.lock` | modified, committed `09570c9` | refreshed through Cargo, never hand-edited |
| `crates/lisa-cli/Cargo.toml` | modified, committed `09570c9` | internal `lisa-core` requirement tracks the bump |
| `crates/lisa-cli/src/config.rs` | modified, committed `18aa699` | `version_is_stale` test |
| `crates/lisa-cli/src/currency.rs` | modified, committed `18aa699` | two `.lisa.toml`-level tests |
| `docs/knowledge/release-checklist.md` | modified, committed `a242036` | re-parameterized; `WORKFLOW_GATE` appended |
| `docs/knowledge/release-0.5.0-rc.1-cut-record.md` | new, committed `d0b827f` | the cut record |

## Files this ticket does not own

`docs/active/tickets/T-057-03-01.md` and the four untracked story files are
Lisa's board. Lisa publishes them with the completion commit; an agent that
committed them would be writing outside its `--include` ownership. They stay
untracked, and `git status --short` showing them is correct, not dirty.

`.lisa/completion-journal.jsonl` and `.lisa/provenance.jsonl` are Lisa's own
runtime records, modified continuously by the loop that is running this session.
Never touched by hand.

## Where the version flows

```
Cargo.toml  version = "0.5.0-rc.1"
    │  version.workspace = true
    ├── lisa-core     0.5.0-rc.1
    ├── lisa-plugin   0.5.0-rc.1
    └── lisa-cli      0.5.0-rc.1
            │  env!("CARGO_PKG_VERSION")
            └── config::LISA_VERSION ──> version_is_stale(recorded, LISA_VERSION)
                                              │
                                              └── currency::inventory ──> RecordedVersion::Behind
                                                                              └── Remedy::Init
```

That chain is the reason the version-compare assertion is an acceptance
criterion rather than a nicety: S-057-02's upgrade path reads a `.lisa.toml`,
compares it against `LISA_VERSION`, and decides whether to offer a remedy. A
prerelease suffix that inverted or broke the compare would silently turn the
whole upgrade path off, and nothing else in the suite would notice.

## The checklist's gate structure

Four gates, declared in the version block and iterated in **both** `for gate in
...` loops (lines 118 and 400). All four verified ancestors of HEAD:

```
c08e755  E045_GATE      ancestor
fcdd293  MUSL_GATE      ancestor
6fcb2f2  SEAL_GATE      ancestor
e67491b  WORKFLOW_GATE  ancestor   ← appended this cut (completion of S-057-02)
```

Cumulative, append-only. A gate declared and never iterated proves nothing, so
adding the name in one place and not the other would be a silent regression of
the runbook — which is why both loops carry it.

## The two review artifacts

`review.md` and `review-disposition.json`, in this attempt's work directory.
Lisa publishes them to `docs/active/work/T-057-03-01/` after checking the lease.
Attempt 1 wrote both and was fenced before publication; that is the only reason
this attempt exists.
