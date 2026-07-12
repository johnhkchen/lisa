# Structure: evidence-only rebuild and dogfood

## Change inventory

Create or complete exactly six attempt-private workflow artifacts:

- `.lisa/attempts/T-038-04-01/1/work/research.md`
- `.lisa/attempts/T-038-04-01/1/work/design.md`
- `.lisa/attempts/T-038-04-01/1/work/structure.md`
- `.lisa/attempts/T-038-04-01/1/work/plan.md`
- `.lisa/attempts/T-038-04-01/1/work/progress.md`
- `.lisa/attempts/T-038-04-01/1/work/review.md`

Do not create, modify, or delete product source files.

Do not create, modify, or delete maintained fixture files.

Do not write any artifact directly under:

`docs/active/work/T-038-04-01/`

Do not modify:

- `docs/active/tickets/T-038-04-01.md`;
- `.lisa/provenance.jsonl`;
- any lease, signal, or attempt metadata;
- Cargo manifests or lockfiles;
- `Justfile`;
- release-readiness artifacts owned by `T-038-04-02`.

Build outputs under `target/` are generated artifacts and are not repository
source changes.

Temporary fixture repositories live outside the checkout and clean themselves
on successful execution.

## Existing inputs

### Canonical build recipe

Read and execute the existing `Justfile` recipe:

`just build-cli`

The recipe owns build order.

No replacement build script or wrapper is introduced.

### Atomic transaction fixture

Execute the maintained script:

`docs/active/work/T-031-03/harness/run.sh`

Supply one environment input:

`LISA_BIN=<canonical absolute release CLI path>`

Do not pass `--keep` on the normal successful run.

The harness owns its temporary repository, evidence directory, and cleanup.

### Real-Zellij delivery fixture

Execute the maintained script:

`crates/lisa-cli/tests/fixtures/real_zellij_delivery_boundary.sh`

Supply one required environment input:

`LISA_BIN=<canonical absolute release CLI path>`

Leave `KEEP_LISA_ZELLIJ_FIXTURES` at its default zero for the normal run.

The harness owns Zellij session names, fixture roots, signals, events, and
cleanup.

## Artifact identity record

The implementation evidence will define one build identity block containing:

- source commit: `git rev-parse HEAD`;
- observation timestamp in UTC;
- CLI repository-relative path;
- CLI canonical absolute path;
- CLI version output;
- CLI byte count;
- CLI SHA-256;
- WASM repository-relative path;
- WASM byte count;
- WASM SHA-256;
- Rust, Cargo, Just, and Zellij versions.

Use platform-available read-only commands:

- `git rev-parse HEAD`;
- `date -u`;
- `wc -c`;
- `shasum -a 256`;
- executable `--version` commands.

Record the fingerprint values in `progress.md` after the build.

Recompute the two sizes and hashes after both fixtures.

The post-run block will state whether they match the pre-run values.

## `progress.md` organization

### Scope and source state

Record:

- ticket and attempt identity;
- starting source head;
- starting worktree state;
- explicit preservation of Lisa-managed pre-existing changes;
- confirmation that no source implementation is planned.

### Build observation

Record:

- exact command;
- exit status;
- duration;
- output paths;
- build classification;
- artifact fingerprint block.

### Fixture result matrix

Use one row per maintained fixture.

Columns:

- fixture;
- exact release artifact used;
- observed scenarios/boundaries;
- exit status;
- receipt;
- duration;
- result.

The atomic fixture remains one fixture even though it models six tickets.

The real-Zellij fixture remains one fixture with four named scenarios.

### Detailed observations

For the atomic fixture, record observations for:

- fixture initialization and validation;
- five Codex and one Claude logical ticket;
- dependency gating;
- exact-path ticket commits;
- completion publication;
- foreign ordinary-index preservation;
- final PASS receipt.

Only claim details guaranteed by the harness assertions.

For the real-Zellij fixture, record observations for:

- success;
- suppressed process start;
- suppressed acknowledgement;
- dquote recovery;
- final PASS receipt.

Only claim scenario success when the full script exits zero.

### Boundary statement

State that:

- all work was local;
- provider behavior came from deterministic shell fixtures;
- no Codex or Claude model was invoked;
- no provider token usage or live authentication was involved;
- the atomic fixture did not load WASM;
- the real-Zellij fixture did load the CLI's embedded WASM through `lisa loop`;
- live installed-provider validation remains outside this ticket.

### Deviations and source commits

Record either:

- no deviations and no source commits; or
- each deviation, affected exact path, verification, and Lisa transaction hash.

The expected branch is no deviation.

### Final ownership state

Record the final `git status --porcelain` classification.

Separate pre-existing Lisa-managed paths from any ticket-owned source paths.

State explicitly whether ticket-owned source residue exists.

## `review.md` organization

### Outcome

Lead with whether the acceptance criterion is satisfied.

List independent status for:

- release rebuild;
- atomic fixture;
- real-Zellij fixture;
- source cleanliness.

### Change summary

State that no product, test, manifest, or maintained documentation source was
changed.

List the six private phase artifacts as the only ticket-authored files.

Do not count generated `target/` outputs as source changes.

### Test and dogfood coverage

Explain what each fixture proves and what it does not prove.

Include exact reproduction commands and stable receipts.

### Commit review

State why no `lisa commit-ticket` source transaction was needed if no source
path changed.

Confirm ordinary Git staging and committing were not used.

### Open concerns

Preserve the honest limitations:

- deterministic stubs are not installed providers;
- real process timing can still vary by host load;
- the atomic fixture is native-only;
- hashes identify files but do not independently extract embedded bytes;
- the runtime fixture is the proof that embedding produced a loadable plugin.

### Handoff

Point the downstream report ticket to `progress.md` for exact values and
commands.

Remain on the current ticket after Review.

Do not publish Done or begin `T-038-04-02`.

## Command boundaries

### Build unit

One command:

`just build-cli`

Verification immediately after:

- exit status zero;
- CLI exists and is executable;
- WASM exists and is nonempty;
- CLI reports version;
- sizes and hashes can be read.

### Fixture unit 1

One script invocation against the canonical CLI.

Verification immediately after:

- exit status zero;
- stdout contains the six-ticket PASS receipt.

### Fixture unit 2

One script invocation against the same canonical CLI.

Verification immediately after:

- exit status zero;
- stdout names all four scenarios;
- stdout contains the final real-Zellij PASS receipt.

### Integration identity unit

After fixtures:

- recompute artifact sizes and hashes;
- compare them with the pre-run values;
- inspect repository status;
- inspect the ordinary index without modifying it.

## Source commit boundaries

There are no planned source commit boundaries.

Attempt workflow artifacts are not included in `lisa commit-ticket`.

Generated build outputs are ignored and are not included in any commit.

If implementation unexpectedly changes one maintained path, it becomes one
meaningful ticket unit and must be committed with:

`lisa commit-ticket --ticket-id T-038-04-01 --message <message> --include <exact-path>`

Multiple unrelated paths must not be bundled merely because they support the
same run.

No ordinary `git add` or `git commit` command is part of this structure.

## Explicit non-structure

No new fixture runner is introduced.

No shell abstraction is added.

No CI job is added.

No fixture output directory is checked in.

No release artifact is checked in.

No live-provider evidence is created.

No size, startup, or memory baseline report is authored here; that aggregation
belongs to `T-038-04-02`.

No source cleanup is folded into this evidence ticket.

No ticket phase or status frontmatter is changed manually.
