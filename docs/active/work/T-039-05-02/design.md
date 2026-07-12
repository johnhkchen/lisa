# Design: typed atomic publication boundary

## Objective

Create one explicit plugin-owned abstraction for sibling-temp publication while
preserving every behavior characterized by `T-039-05-01`. Represent differing
temporary naming and execution sides as typed options. Keep authority,
serialization, directory creation, and result interpretation at each call site.

## Rejected: generic `write_atomic(path, bytes)`

A minimal helper could synthesize one temp name and return `io::Result`.

- It would remove repeated write/rename code.
- It would flatten three distinct temp-name families.
- It could not preserve deterministic artifact collisions.
- It would discard site-specific operator diagnostics.
- It could not express the shell execution side honestly.

Decision: reject because it shares more policy than the sites have in common.

## Rejected: closure-driven engine

A generic engine could accept closures for naming, writing, moving, cleanup, and errors.

- It could reproduce every behavior.
- Its signature would obscure the finite policy set.
- Arbitrary closures would make exhaustive audit difficult.
- Callers would still contain most mechanics.

Decision: reject because flexibility conflicts with an explicit boundary.

## Rejected: core-wide utility

Putting the helper in `lisa-core` appears to join plugin, provenance, and CLI.

- Provenance is append-only and must not use replacement rename.
- Git transactions publish refs with compare-and-swap and rollback semantics.
- Shell readiness needs plugin quoting and host-path rules.
- Only the plugin has sibling-temp rename callers.

Decision: reject; the cross-crate seam is semantic rather than mechanical.

## Chosen: typed plugin module

Add a private `publication` module with a finite temp-name policy, Rust-side
writer/renamer, and shell-side command renderer.

- It centralizes exactly the duplicated mechanism.
- Naming variants are exhaustive and reviewable.
- Caller-owned serialization and authority remain visible.
- Rust cleanup and shell `mv` semantics remain separate.
- Error vocabulary stays site-specific.
- No unjustified public API is added.

## Type model

`TemporaryName` has three variants:

- nonce-bearing sibling with an explicit prefix;
- attempt-and-nonce sibling with a prefix and attempt ID;
- exact deterministic sibling filename.

`PublicationPath` contains the final destination and typed temporary policy.
Resolution joins the temp name to the destination parent, structurally preserving
same-directory publication. The module owns wall-clock nonce generation.

`PublicationErrors` has named `write` and `publish` labels. Named fields prevent
positional inversion while preserving existing literal messages.

`RustPublication` contains path options, serialized bytes, and error labels.
Its operation is exactly write, rename, best-effort cleanup on rename failure,
and return of the final path on success.

`ShellPublication` contains path options and serialized text. It returns the
exact quoted `printf > temp && mv temp destination` command. It deliberately
adds no cleanup, destination guard, or `mv` option that could alter collisions.

## Serialization ownership

Serialization remains at call sites:

- launch wraps payload with shebang and newlines;
- assignment passes raw instructions;
- lease marker uses compact JSON bytes;
- admitted artifact uses raw staged bytes;
- readiness uses compact JSON text.

This keeps schemas and serialization errors at the sites that understand them.

## Directory and authority ownership

Callers retain parent creation because each directory error differs. Shell
readiness intentionally performs no host-side creation. `admit_artifact` retains
exact lease validation and staged path selection. The boundary receives no
scheduler state and cannot mint or weaken authority.

## Site mapping

- Launch: nonce prefix `.lisa-launch-{pane}.sh.tmp.` and launch labels.
- Assignment: nonce prefix `.assignment.md.tmp.` and assignment labels.
- Lease: attempt-nonce prefix `pane-{pane}.lease.tmp.` and marker labels.
- Artifact: exact `.{artifact}.attempt-{id}.tmp` and artifact labels.
- Readiness: attempt-nonce prefix `pane-{pane}.shell-ready.tmp.` and shell rendering.

## Shell quoting

Move canonical shell quoting into the publication module. Preserve the existing
crate-visible `shell_quote` surface through a re-export so adapter and test call
sites require no unrelated changes.

## Collision and error preservation

- `fs::write` continues overwriting regular temporaries.
- Rust rename continues replacing regular destinations.
- Rust directory-destination failures still clean the generated temp.
- Exact artifact temp directory collisions still fail at write.
- Shell directory destinations still receive the temporary as a child.
- OS error tails remain untouched.
- Existing per-site prefix and displayed-path choices remain exact.

## Provenance and Git boundary

Do not route provenance append or Git ref publication through this helper.
Verify their existing suites unchanged. The module documents that it is a
sibling-temp file replacement boundary, not a universal durability abstraction.

## Testing decision

- Run predecessor publication characterization before and after.
- Keep those tests source-identical.
- Run core provenance and CLI transaction tests unchanged.
- Direct hostile tests of the new module belong to successor `T-039-05-03`.
- Run full workspace, Clippy, formatting, and repository gates.

## Expected source inventory

- Create `crates/lisa-plugin/src/publication.rs`.
- Modify `crates/lisa-plugin/src/lib.rs`.
- Do not modify manifests, core, CLI, fixtures, or schemas.

