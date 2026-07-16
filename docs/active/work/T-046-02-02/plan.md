# Plan — T-046-02-02 pinned fetch, verify, and store

## 1. Preserve assignment boundaries

Keep unrelated modified and untracked paths untouched.

Do not change ticket phase or status frontmatter.

Write every phase artifact only beneath the current attempt work directory.

Use exact include paths for every ticket source commit.

Verification: compare final `git status --short` with the initial inventory.

## 2. Add native acquisition dependencies

Edit `crates/lisa-cli/Cargo.toml`.

Add Ureq with default features disabled and TLS enabled.

Add SHA-2, Flate2, and Tar with the selected feature sets.

Run Cargo check to resolve `Cargo.lock`.

Inspect the graph for accidental native TLS or xattr dependencies.

Verification: `cargo check -p lisa-cli` reaches source compilation.

## 3. Define pinned production metadata

Add the private managed-release value to `runtime.rs`.

Add the x86_64 Linux no-web static-musl URL and baked SHA-256.

Add the aarch64 Linux no-web static-musl URL and baked SHA-256.

Add compile-time target selection.

Return a named error for targets without a ticket-specified artifact.

Verification: selector tests and host compilation.

## 4. Add cache-first acquisition entry point

Add the final executable cache predicate.

Reject an existing invalid final directory without overwriting it.

Create the runtime root only after a genuine cache miss.

Add a unique sibling temporary-directory allocator.

Add cleanup-on-drop semantics.

Verification: a preinstalled executable returns before network setup; temp
directories clean up after failures.

## 5. Implement bounded download

Build one Ureq GET using the exact release URL.

Set a Lisa user-agent header.

Copy the response to `download.tar.gz` in the temporary directory.

Flush and sync the archive file.

Do not retry status, transport, or body-read failures.

Format failures with a stable category, exact URL, and expected digest.

Verification: the local fixture counter observes one request.

## 6. Implement SHA-256 verification

Hash the complete downloaded archive.

Format the digest as lowercase hexadecimal.

Compare it with the compiled expected string.

On mismatch, include expected and actual values in the named error.

Allow the cleanup guard to remove the archive and directory.

Verification: good fixture passes; bad expectation leaves no partial state.

## 7. Implement constrained extraction

Decode the verified gzip stream and iterate tar entries.

Accept exactly one regular top-level `zellij` entry.

Reject all unexpected paths and types.

Copy it into the temporary install root.

Flush and sync it, then set executable mode.

Require the expected entry.

Verification: the installed fixture runs `--version`.

## 8. Publish atomically

Remove the downloaded archive from the candidate directory.

Rename the complete sibling temporary directory to the version directory.

Disarm cleanup only after rename succeeds.

If a concurrent publisher won, accept its executable and clean the losing
candidate.

Do not remove or replace an existing version directory.

Verification: the final directory contains only a usable `zellij`.

## 9. Integrate managed resolution

Change the Managed resolver arm to derive and ensure the final path.

Select production metadata only on a cache miss.

Pass the result through existing canonicalization and version inspection.

Leave System and Pinned behavior unchanged.

Verification: all predecessor runtime mode tests remain green.

## 10. Build fixture-server support

Create deterministic gzip/tar bytes containing a version-printing shell stub.

Create a bounded localhost HTTP/1.1 server.

Track accepted requests atomically.

Support complete and interrupted response modes.

Ensure server threads terminate without hanging the suite.

Verification: fixture digest and request counting are deterministic.

## 11. Test successful fetch, verification, and store

Serve the valid archive into an empty versioned runtime location.

Inject its URL and digest.

Call the same acquisition path used by managed resolution.

Inspect the installed executable through normal runtime inspection.

Assert one request, correct version, executable mode, no archive, and no temp
sibling.

## 12. Test checksum mismatch

Serve a complete archive and inject a different expected digest.

Assert the checksum-mismatch category.

Assert exact URL, expected digest, and actual digest are printed.

Assert one request and no final or temporary directory.

## 13. Test interrupted download

Serve a body prefix with a larger advertised content length.

Close without completing the body.

Assert a named download error includes exact URL and expected digest.

Assert one request and no final or temporary directory.

## 14. Test offline with no cache

Reserve and release a localhost port so no listener remains.

Inject its URL and a fixture expected digest.

Call acquisition once.

Assert the named error includes URL and digest.

Assert the final runtime directory remains absent.

The lack of a retry wrapper establishes bounded behavior.

## 15. Test zero network on second resolution

Serve exactly one valid response.

Run installation and normal inspection once.

Run the same resolution operation against the same data root again.

Ensure the second call uses the final executable cache branch.

Assert the fixture request count remains exactly one.

## 16. Format and run focused checks

Run `cargo fmt --all` and its check form.

Run the runtime test module.

Keep fixture resources isolated so normal parallel tests are safe.

Document commands and results in `progress.md`.

## 17. Commit the source unit

Inspect diffs for `Cargo.toml`, `Cargo.lock`, and `runtime.rs`.

Confirm no ticket-owned file is ordinarily staged.

Run `lisa commit-ticket` with ticket T-046-02-02, a focused message, and exact
includes for those three paths.

Do not use ordinary `git add` or `git commit`.

Verification: inspect the created commit and confirm exact path ownership.

## 18. Run broad verification

Run `cargo test -p lisa-cli`.

Run `cargo test --workspace`.

Run `just check` for the repository's aggregate WASM/native gate.

If Linux cross targets are installed, compile both supported selectors;
otherwise rely on cfg-complete review and CI for non-host targets.

Record results and environmental limitations.

## 19. Audit acceptance criteria

Map the success test to fetch, digest, extraction, permission, and rename.

Map checksum mismatch to its named error and absent partial install.

Map interruption to an absent torn runtime directory.

Map offline behavior to one bounded error with URL and SHA-256.

Map second resolution to an observed request count of one.

Search for network retry loops, checksum fetches, and PATH fallback.

## 20. Complete Review artifacts

Write `review.md` in the private attempt work directory.

Summarize exact committed source files and behavior.

List focused and broad test results.

State platform support and future checksum-update responsibility.

Document any concern or verification gap.

Write `review-disposition.json` with exactly the required pass or block shape.

Confirm ticket-owned source paths are committed and clean.

Remain on T-046-02-02 and stop after Review.
