# Design: T-035-03-01 Codex trust path canonicalization

## Goal

Ensure Lisa writes the same existing-project identity to Codex's trust
configuration that Codex derives from its resolved working directory.

The change must remain best-effort, preserve existing configuration, be
idempotent, and be provable without launching Codex.

## Decision criteria

The selected approach should:

- cover both `lisa loop` and `lisa doctor`;
- resolve macOS `/var` aliases to `/private/var`;
- avoid changing unrelated CLI path presentation and layout behavior;
- preserve trust seeding for synthetic or temporarily unresolvable paths;
- keep the current exact-header/idempotence contract;
- admit a deterministic filesystem unit test;
- avoid any provider, Zellij, scheduler, or network dependency.

## Option 1: canonicalize every CLI root in `resolve_path`

`main.rs::resolve_path` could call `std::fs::canonicalize` before dispatching
every subcommand.

### Benefits

- All commands would use a single physical root spelling.
- Loop trust seeding would receive the canonical path automatically.
- Generated layout and reporting would also use canonical paths.

### Costs

- The helper serves init, validate, status, setup guide, loop, usage capture,
  and commit transactions.
- `lisa init --path <new-path>` may intentionally target a path whose final
  structure does not yet exist.
- User-visible paths and repository paths would change outside the ticket's
  trust boundary.
- The ticket explicitly excludes broad infrastructure changes.

### Decision

Rejected.

Canonicalizing the entire CLI root is a much wider semantic change than needed
to align one external tool's project identity.

## Option 2: canonicalize only in `loop_cmd::run_loop`

The Codex branch in `run_loop` could canonicalize `root` before calling
`pregrant_codex_trust`.

### Benefits

- Directly fixes the fresh-loop scenario that exposed the defect.
- Leaves other loop paths and commands unchanged.
- Is easy to read at the call site.

### Costs

- `lisa doctor` would continue writing the alias path.
- The invariant would depend on every caller remembering to canonicalize.
- The trust function's documented contract would still accept and emit an
  identity that Codex may not use.
- A low-level regression would not prove all callers get the behavior.

### Decision

Rejected.

The shared trust writer, rather than a particular caller, owns the external
identity contract.

## Option 3: canonicalize in `pregrant_codex_trust`

The environment-resolving wrapper could canonicalize before delegating to
`pregrant_codex_trust_in`.

### Benefits

- Both doctor and loop use the corrected production wrapper.
- The low-level writer retains literal-path behavior for existing tests.
- Canonicalization failure can be handled at one production boundary.

### Costs

- Direct callers of `_in`, including the natural filesystem regression, would
  not exercise the invariant.
- The helper's behavior would differ depending on which public-in-crate entry
  point is used.
- The `_in` function is the actual owner of the project table header.

### Decision

Viable but rejected in favor of placing normalization next to header
construction.

## Option 4: canonicalize in `pregrant_codex_trust_in`

Resolve `work_tree` immediately before building the Codex project table header.

Use the canonical path when resolution succeeds and the supplied path when it
fails.

### Benefits

- Every caller gets the same identity rule.
- Header lookup and header append use the same normalized value.
- Existing synthetic/nonexistent inputs retain their current best-effort
  behavior through fallback.
- A unit test can pass a symlink alias directly and inspect exact TOML bytes.
- The implementation remains independent of Codex and Zellij.

### Costs

- The helper performs one filesystem lookup before reading the config.
- A previously seeded alias entry will remain in the file while a canonical
  entry is appended on the first corrected run.
- The Boolean return does not expose which path was chosen.

### Decision

Chosen.

The small filesystem lookup occurs only during doctor/loop preflight and is
negligible compared with external dependency checks and loop startup.

Leaving an obsolete alias table is safer than rewriting user configuration.
The canonical entry becomes the authoritative match and subsequent runs are
idempotent against it.

## Canonicalization failure policy

### Option A: fail trust pregrant

Return false if `std::fs::canonicalize` fails.

This gives a strict identity guarantee but turns a normalization enhancement
into a new failure mode for nonexistent paths, permission-limited paths, and
tests using synthetic paths.

### Option B: use the original path as fallback

Attempt canonicalization and preserve the supplied path if it cannot be
resolved.

This retains the existing best-effort contract while guaranteeing the desired
identity for the real fixture, which exists before loop startup.

### Decision

Choose Option B.

The acceptance scenario always has an existing fixture root. A fallback is
appropriate because the surrounding API intentionally treats trust seeding as
nonfatal and retains provider bypass behavior.

## Regression design

Create a temporary filesystem tree containing:

- a real project directory;
- a sibling symbolic-link path pointing at that directory;
- a separate temporary Codex home.

Pass the symlink spelling to `pregrant_codex_trust_in`.

Compute the expected cwd with `std::fs::canonicalize` on the same alias, which
models Codex's resolved cwd identity.

Read `config.toml` and assert it contains exactly the project header constructed
from that expected canonical path followed by `trust_level = "trusted"`.

Also assert the canonical path differs from the alias path so the fixture
actually exercises link resolution rather than only absolute-path formatting.

The test is Unix-specific because Rust's standard symbolic-link creation API is
platform-specific. Lisa's supported live environment and the macOS bug are
Unix-based, while the production canonicalization itself remains portable.

## Existing-test preservation

Keep the current literal `/work/tree` tests.

Because that path does not normally exist, canonicalization falls back and the
existing assertions remain meaningful for write, preservation, and
idempotence.

The new test adds the missing existing/symlinked-path branch.

## Documentation behavior

Update the trust helper's Rust documentation to state that existing project
paths are canonicalized to match Codex's cwd identity and that unresolvable
paths fall back to their supplied spelling.

No runbook edit is needed in this enabling ticket: T-035-03-02 owns the
committed fresh-loop harness/runbook extension and the live rerun.

The source-level contract and regression are sufficient for this ticket's
explicit free verification boundary.

## Scope decision

Modify only `crates/lisa-cli/src/doctor.rs`.

Do not change scheduler code, provider startup, CLI-wide path resolution,
ticket frontmatter, shared work publication, or live harness execution.

This forms one meaningful ticket-owned source unit: implementation,
documentation, and its regression test in the same module.
