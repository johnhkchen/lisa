# Design — T-049-03-02

## Objective

Make both downgrade boundaries auditable without changing the established completion
architecture:

1. explicit commit intent must stop before loop launch when identity is absent;
2. a run pinned to commit must park when Git disappears instead of producing journal
   completion evidence;
3. the standing Chromebook protocol must contain a repeatable, scored, no-Git leg.

## Design constraints

- Preserve the single-probe, immutable-seal runtime model.
- Do not add a second Git capability probe in the plugin.
- Do not add a commit-to-journal recovery branch.
- Reuse S-049-04's existing completion failure and parking machinery.
- Keep normal Chromebook install legs byte-for-byte compatible where practical.
- Keep the no-Git leg manual, authenticated, and metered.
- Do not disclose `/cbt` tooling to the tested agent.
- Make pass evidence independently checkable from collected files.
- Avoid reliance on Git, jq, Python, or a compiler in the no-Git grader.
- Avoid unrelated dirty paths owned by concurrent tickets.

## Decision 1: production behavior versus boundary tests

### Option A — add new enforcement branches

The CLI could special-case explicit commit in `run_loop`, and the plugin could special-case
mid-run repository loss.

Advantages:

- The ticket would visibly add production conditionals.
- Error text could be tailored specifically to repository disappearance.

Disadvantages:

- Explicit commit already hard-fails through the pure resolver and native adapter.
- The plugin already has a single failure classification and parking path.
- A new plugin probe would weaken the pinned-tier architecture.
- A new fallback branch would create the silent downgrade the ticket forbids.
- Duplicate conditionals would increase drift between doctor and loop.

Decision: reject.

### Option B — assert the existing architectural boundaries with scenario tests

Use the compiled CLI fixture for explicit commit/missing identity and add a plugin scenario
that starts from `Auto + Available`, pins commit, removes the repository metadata, executes
the real native completion transaction, and feeds its failure through the plugin boundary.

Advantages:

- Tests the intended contract rather than duplicating implementation.
- Models the temporal distinction: capability at start, breakage at completion.
- Exercises the real transaction failure text and the real parking adapter.
- Can prove no journal receipt/content hashes were written.
- Keeps one production completion gateway.

Disadvantages:

- The auto resolver and plugin live in different crates, so the test composes their public
  types rather than launching an entire Zellij loop.
- Deleting `.git` in a test must be carefully restricted to a temporary directory.

Decision: choose Option B.

## Explicit commit test boundary

The existing compiled `seal_visibility` fixture is the correct level for the first acceptance
criterion. It creates an actual identityless repository, invokes `lisa doctor` with
`[guards].completion = "commit"`, checks nonzero status, and verifies:

- the named completion preflight failure;
- the exact explicit guard setting;
- the missing-identity reason;
- both operator remedies.

No duplicate unit test is needed. The implementation will retain and run this fixture as one
half of the enforcement test suite. The plugin scenario is the second half.

## Mid-run breakage scenario

The new test will use the existing Review completion fixture because it already supplies:

- a real ticket and work directory;
- an admitted current attempt;
- passing private Review artifacts;
- a physical seat and owned scheduler thread;
- completion journal and provenance paths.

The test setup will then:

1. resolve `CompletionSealMode::Auto` with `CommitSealSupport::Available` through the pure
   resolver;
2. copy the resulting commit seal into plugin config;
3. initialize a real Git repository with local test identity and an initial commit;
4. dispatch completion, proving a commit effect was launched;
5. remove only the temporary repository's `.git` directory;
6. invoke `lisa_cli::commit_transaction::complete_ticket` against that now-broken root;
7. pass the actual error to `handle_completion_result`.

Repository disappearance classifies conservatively as unrecognized and parks immediately.
That is acceptable: the contract requires a visible ask and parking, not a new classifier.

The assertions will cover:

- the pinned tier remains commit;
- no pending completion or scheduler thread remains;
- the physical seat is released;
- the ticket is Review/blocked rather than Done;
- the published disposition is a nonempty blocking ask;
- the journal records the failure and action-required rejection;
- every row stays labeled commit;
- no confirmed row, content hashes, or journal seal appears.

## Decision 2: no-Git fixture shape

### Option A — documentation-only operator recipe

Add prose telling an operator to create a directory and manually inspect results.

Advantages:

- Minimal code change.

Disadvantages:

- Does not satisfy the ticket's fixture-flag requirement.
- Reintroduces copy/paste drift into the scripted ritual.
- Hash verification would remain subjective.

Decision: reject.

### Option B — a separate `/cbt/no-git` script

Add a fifth operator script dedicated to the variant.

Advantages:

- Strong separation from install-only preparation.
- Easy to evolve independently.

Disadvantages:

- Expands the ritual surface unnecessarily.
- Duplicates README extraction, snapshots, metadata, and launch setup.
- The ticket specifically describes a fixture flag on `/cbt/prepare`.

Decision: reject.

### Option C — `--no-git` mode in prepare and conditional grade

Extend `/cbt/prepare` with a `--no-git` flag. It creates an empty, explicitly repository-less
project directory and tailors the measured instruction. Extend `/cbt/grade` to recognize the
metadata flag and add no-Git completion checks.

Advantages:

- Fits the existing variant model.
- Preserves one prepare/run/grade ritual.
- Metadata makes the grader deterministic.
- Normal legs keep their current assertions.
- The collected run record becomes self-describing.

Disadvantages:

- The tested instruction is longer than the install-only instruction.
- A real Lisa loop can consume substantially more tokens/time than installation alone.
- The grader must safely parse and validate JSONL hashes.

Decision: choose Option C.

## No-Git measured instruction

The no-Git flag will retain the selected README install section, then ask the agent to:

- install Lisa;
- keep Git absent;
- use the prepared `~/no-git-demo` directory;
- initialize with `lisa init --no-history`;
- create one ordinary ready ticket;
- run `lisa loop` until that ticket is Done;
- finish only when project-local `lisa doctor` passes.

The instruction names public Lisa commands and the visible project directory, but never names
the hidden `/cbt` tools or grading implementation. It deliberately requires a real loop rather
than a dry run because the claim being scored is completion.

The prepared directory starts empty. The agent owns scaffold creation and ticket creation. The
grader will require the canonical ticket id `T-NOGIT-001`; this makes journal lookup stable.

## No-Git grader contract

Normal installation checks remain. In no-Git mode, the grader additionally:

- refuses any available `git` command;
- requires no `.git` entry in or above the project fixture boundary;
- runs `lisa doctor` from the no-Git project;
- extracts and records the exact `completion seal:` line;
- requires the exact journal-only sentence;
- requires the named ticket to have `status: done` and `phase: done`;
- locates the last confirmed journal row for that ticket;
- requires `seal == "journal"`, no commit id, and a nonempty content-hash array;
- recomputes SHA-256 for every path binding under the project root;
- rejects absolute paths, `..` traversal, missing files, duplicate paths, and mismatches.

Node will perform JSON parsing and SHA-256 verification. Node is a fixture invariant because
both tested agent CLIs are npm-installed and the Docker build verifies their versions. This is
safer and clearer than parsing JSON with grep/sed and avoids adding jq.

The verifier prints one compact summary containing the confirmed row identity and verified
binding count. The grader copies that summary into the run record. Together with the collected
journal, ticket, and work artifacts, a reviewer can independently reproduce it.

## Evidence collection

The current collector omits project completion evidence. Extend it so a no-Git result retains:

- `.lisa/completion-journal.jsonl`;
- `docs/active/tickets/T-NOGIT-001.md`;
- `docs/active/work/T-NOGIT-001/`.

Collection remains path-specific and excludes authentication state. The destination remains
the existing Chromebook closing-evidence directory because the protocol already uses it for
all fixture legs.

## Timing and thresholds

The existing ten-minute success threshold was calibrated for install-only legs. A full RDSPI
loop with a nested agent session is a different measured workload. The runbook will state that
the no-Git leg retains the protocol's 20-minute hard stop but records the install threshold
separately from completion rather than pretending the old ten-minute install score covers the
full loop.

The script currently records only one start/end interval, so the no-Git leg will be scored under
the existing wall bound unless the protocol explicitly changes it. To avoid weakening existing
install claims, the implementation will keep the 600-second bound for normal legs and assign a
1200-second bound only when `no_git: 1` is present. The record will name the applied bound.

## Compatibility

- Existing prepare invocations remain valid.
- Existing metadata gains one additive `no_git` line.
- Existing grader records gain completion-seal and optional journal-evidence lines.
- The Docker image gains no package.
- Normal legs do not create the no-Git project.
- `just cbt-collect` gains only explicitly selected no-Git evidence paths.
- No completion schemas or public Rust APIs change.

## Risks and mitigations

- Nested agent execution may exceed the normal install window: use a separately named 20-minute
  no-Git bound without changing normal leg scoring.
- An agent may create the wrong ticket id: make the id exact in the instruction and grader.
- An agent may install Git: fail the leg explicitly.
- A row may claim hashes without matching bytes: recompute every binding.
- A malicious path could escape the fixture: reject absolute and parent-traversal paths before
  reading.
- Post-completion mutation can invalidate hashes: grade immediately after the agent exits and
  fail on mismatch.
- Concurrent ticket changes could contaminate commits: include only exact owned paths.

## Chosen design summary

No new production fallback or enforcement branch is warranted. Add a real temporal boundary
test for auto-pinned commit breakage, preserve the existing compiled explicit-commit preflight
test, and extend the Chromebook scripted ritual with a `--no-git` flag, exact journal-only
grading, reproducible content-hash verification, and sanitized evidence collection.
