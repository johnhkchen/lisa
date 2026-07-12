# Design: publication-site characterization

## Objective

Add tests that make the current publication contracts explicit before the next
ticket centralizes their mechanics. The tests must cover all five rename sites,
their distinct serialization and naming policies, collision outcomes, cleanup,
operator diagnostics, hostile paths, and provenance append integrity. No
production behavior or public interface changes.

## Option 1: rely on existing tests

Existing plugin tests already exercise large launch payloads, hostile assignment
bytes, a shell-readiness command, current-lease artifact admission, and lease
markers through lifecycle fixtures. Existing provenance tests cover normal
append and record attribution.

Advantages:

- no source changes;
- broad success behavior is already represented;
- lifecycle fixtures cover real state-machine callers.

Disadvantages:

- collision replacement is mostly implicit;
- temp-name contracts are not systematically asserted;
- rename failure cleanup and error text are unpinned;
- admitted artifacts lack isolated publication-site coverage;
- provenance failure integrity and its operator event are untested;
- acceptance explicitly asks for characterization and hostile-path tests.

Decision: reject. The current suite is a useful baseline but does not bracket the
planned publication refactor tightly enough.

## Option 2: introduce the boundary while adding tests

Extract an atomic writer now, inject nonce generation, and test it directly.

Advantages:

- deterministic temp names become easy to inspect;
- one helper could exercise all failure modes;
- less filesystem-specific setup in tests.

Disadvantages:

- this performs the next ticket's production refactor early;
- tests would characterize the new abstraction rather than the current sites;
- helper options could accidentally flatten distinct contracts;
- acceptance requires tests passing on the unmodified production tree.

Decision: reject. This ticket is deliberately the left-hand characterization
bracket around `T-039-05-02`.

## Option 3: static source-shape assertions

Read `lib.rs` with `include_str!` and assert format strings, rename calls, and
error prefixes.

Advantages:

- exact temp format strings are easy to pin;
- no dependence on filesystem collision behavior;
- concise test implementation.

Disadvantages:

- source spelling is not runtime behavior;
- the next refactor is supposed to change source shape;
- such tests would obstruct the intended abstraction even if behavior survived;
- serialization, cleanup, and integrity remain unproved.

Decision: reject. Characterization should survive internal reorganization.

## Option 4: black-box private-helper filesystem characterization

Use inline native tests to call the five private helpers. Exercise regular-file
destination collisions for replacement behavior and hostile filesystem shapes
for deterministic failures. Inspect published bytes, residual temporaries,
returned commands, and error prefixes.

Advantages:

- executes current production code unchanged;
- observes externally meaningful contracts;
- remains useful after helper extraction;
- inline tests can access private sites without visibility changes;
- destination directories reliably force rename failures on Unix;
- overlong leaf names expose nonce-bearing temporary paths in write errors;
- shell execution tests cover quoting and shell-side residue.

Disadvantages:

- Unix rename and filename-length behavior is platform-specific;
- wall-clock nonce values cannot be predicted exactly;
- tests must validate name families rather than a literal nonce;
- shell readiness failure cleanup differs intentionally from Rust sites.

Decision: choose this option. Lisa and Zellij target the same Unix-like runtime
where these publication contracts operate, and the assertions focus on stable
behavioral properties.

## Option 5: subprocess fault injection

Run helpers under a subprocess with permissions, mounts, or syscall interposition
to force write and rename failures.

Advantages:

- arbitrary failure points can be selected;
- exact operator diagnostics can be covered.

Disadvantages:

- substantially more harness complexity;
- permission behavior changes under root;
- syscall interposition is platform/toolchain dependent;
- unnecessary for the named contracts.

Decision: reject. Filesystem shape fixtures produce the required failures with
less machinery.

## Chosen plugin test organization

Add two publication-focused tests in the existing `lib.rs` inline test module.

### Success and collision catalog

`publication_sites_preserve_serialization_and_collision_contracts` exercises:

- fresh launch replacing an existing regular destination;
- assignment replacing an existing regular destination;
- lease marker replacing an existing regular destination;
- admitted artifact replacing both a pre-existing deterministic temporary and
  a pre-existing canonical destination;
- shell readiness replacing a pre-existing temporary and destination.

Each subcase uses a path containing spaces, a quote, and shell metacharacters.
Rust-side sites must treat it literally. Shell-side sites must quote it safely.

Assertions lock the distinct serialized bytes:

- launch: shebang, payload, trailing newline;
- assignment: exact raw assignment bytes;
- lease marker: compact JSON bytes deserializing to the exact lease;
- admitted artifact: exact raw staged bytes;
- shell readiness: compact JSON string for the exact lease.

Assertions also lock:

- returned launch command remains bounded and quoted;
- returned assignment path is the canonical path;
- every regular destination collision is replaced;
- success leaves no temporary files;
- admitted artifact consumes neither the staged source nor its provenance;
- shell commands cannot create an injection sentinel.

### Hostile failure and diagnostics catalog

`publication_sites_preserve_temp_names_cleanup_and_operator_errors` exercises:

- overlong leaf directories for launch, assignment, and lease marker temp-write
  failures;
- destination directories for Rust-side rename failures;
- a directory occupying the admitted-artifact deterministic temporary;
- a directory occupying the shell-readiness destination.

Overlong-name errors expose the actual temporary path. The test asserts stable
prefix/suffix structure while treating the wall-clock nonce as opaque digits.
This pins the per-site name families without coupling to an exact instant.

Rename-failure assertions require:

- the documented operator prefix;
- final destination path in the message;
- no generated Rust-side temporary residue;
- the pre-existing destination directory remains intact.

Admitted-artifact temp collision asserts the exact deterministic temporary path
in its write error. Shell-readiness failed `mv` asserts nonzero status and the
currently intentional residual temp whose name includes pane and attempt IDs.

## Provenance test design

Two layers are useful because provenance has a core I/O boundary and plugin
operator behavior.

### Core append integrity

Add `append_serialization_failure_preserves_existing_ledger` in
`lisa-core/src/provenance.rs`.

- seed a ledger with a valid line;
- clone a record and set `cost_usd` to `NaN`;
- JSON serialization must return `InvalidData` before opening the ledger;
- assert the ledger bytes are exactly unchanged;
- use a parent path containing hostile shell characters to prove ordinary Rust
  path handling.

This is deterministic and avoids unreliable permission failures.

### Plugin operator-facing failure

Add a plugin test around `State::emit_provenance`:

- configure `ledger_path` as an existing directory with hostile characters;
- install a current attempt and thread;
- emit a Done provenance record;
- assert the method returns false;
- assert no directory contents were created;
- assert the activity log contains one Error event beginning with
  `provenance write failed for {ticket}:`.

This pins the swallowing/logging contract without changing teardown state.

## Temp-name assertion strategy

- Fixed prefixes and identity components are asserted exactly.
- Nanosecond nonce portions are asserted nonempty and ASCII numeric.
- Success directories are enumerated to prove no `.tmp` residue.
- Rust rename failure directories are enumerated for cleanup.
- Admitted artifact's deterministic name is asserted literally.
- Shell failure residue is expected and inspected rather than cleaned by the
  production command.
- Tests clean only through temporary-directory teardown.

## Collision semantics

The tests intentionally characterize current Unix behavior:

| Site | Existing regular destination | Existing temporary | Failed rename |
|---|---|---|---|
| launch | replaced | generated nonce usually avoids | temp removed |
| assignment | replaced | generated nonce usually avoids | temp removed |
| lease marker | replaced | generated nonce usually avoids | temp removed |
| admitted artifact | replaced | overwritten if regular | temp removed |
| shell readiness | replaced by `mv` | truncated by redirection | temp remains |
| provenance | appended, never replaced | not applicable | prior bytes retained when serialization fails |

The tests do not claim cross-platform replacement behavior beyond the project's
supported execution environment.

## Production-code decision

Do not modify:

- publication helper signatures;
- nonce generation;
- serialization;
- error strings;
- cleanup behavior;
- path validation;
- provenance schema;
- provenance teardown ordering.

Only test modules change.

## Verification decision

Run focused new tests first, then publication/provenance groups, the plugin and
core suites, the workspace suite, Clippy, formatting, and `just check`. Commit
the two test-bearing source files as one meaningful characterization unit via
Lisa's isolated transaction with exact include paths.
