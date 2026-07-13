# Research: Nested-monorepo path regression

## Ticket boundary

T-042-01-06 is a regression-only bug ticket following T-042-01-05.
Its single acceptance criterion joins two already implemented production
boundaries: completion command construction in `lisa-plugin` and the isolated
`complete-ticket` transaction in `lisa-cli`.

The field topology is one Git repository with a Lisa project two directories
below it at `games/midsummer`. Ticket and work files therefore live below
`games/midsummer/docs/active`, while unrelated root-level `docs` entries may
also exist.

## Completion command builder

`crates/lisa-plugin/src/lib.rs` owns `State::build_completion_command`.
The method is the command-construction boundary used by the completion effect
executor. It returns an argv vector and a Zellij command context map.

`State` retains two distinct roots after T-042-01-05:

- `project_root` is the Lisa project root and host-command cwd;
- `git_root` is the enclosing repository root and Git pathspec base.

The builder requires a configured `lisa_bin`, a non-empty project root, a
ticket id, and the ticket file path. It maps the ticket and configured work
directory through `completion_repository_relative_path`.

That mapper interprets `/host/...` paths under the Lisa project root, keeps host
absolute paths absolute, and interprets relative paths under the project root.
It lexically normalizes both the candidate and Git root, rejects candidates
outside the Git root, and returns a Git-root-relative path.

For `/repo/games/midsummer` nested under `/repo`, the builder emits:

- `--path /repo`;
- `--ticket-file games/midsummer/docs/active/tickets/<id>.md`;
- `--work-dir games/midsummer/docs/active/work/<id>`.

The existing plugin test
`completion_command_uses_git_root_and_nested_repository_paths` asserts that
argv exactly. It does not execute the transaction.

Before T-042-01-05, the builder used `project_root` for `--path` and stripped
the project prefix from ticket and work paths. In the field representation this
was `--path games/midsummer` with `docs/...` arguments. The CLI discovered the
enclosing Git root but Git interpreted the retained arguments from that root.

## Complete-ticket transaction

`crates/lisa-cli/src/commit_transaction.rs` owns `complete_ticket`.
`CompleteTicketRequest` carries repository root, ticket id, message, ticket
file, and work directory.

The transaction canonicalizes the requested root, asks Git for the actual
top-level root, and maps both requested paths with
`completion_repo_relative_path`. It validates that the ticket is Markdown and
that ticket and work paths are distinct.

It reads the original ticket bytes, prepares Done frontmatter, and calls the
isolated `commit_ticket` transaction with exactly the ticket and work includes.
On failure it restores the original ticket bytes. On success it returns the
old and new commit ids plus the concrete committed paths.

The isolated transaction uses an alternate index, serializes through a lock,
and reconciles committed paths without consuming unrelated ordinary-index
entries. Its result reports concrete files, including files found recursively
under an included work directory.

The existing CLI test
`complete_ticket_normalizes_nested_project_paths_to_git_root` builds a nested
fixture and executes the transaction. It supplies project-relative `docs/...`
paths with the nested project as `repo_root`; it does not consume builder argv.

## Crate boundary

`lisa-plugin` is currently a `cdylib` crate with native unit tests. It depends
on `lisa-core`, not `lisa-cli`.

`lisa-cli` is currently binary-only. Its `commit_transaction` module and the
request/result types are crate-private. The CLI entry point constructs the same
request types directly after Clap parsing.

A plugin native test cannot currently call the CLI transaction through a crate
API. Invoking a presumed `target/debug/lisa` would depend on build order and an
external artifact. Starting nested Cargo from a Cargo test would add lock and
toolchain coupling.

The transaction module has no dependency on the CLI entry point. It depends on
`lisa-core`, `fs2`, and the standard library, all already dependencies of the
CLI package. This makes it independently exportable from a library target.

## Fixture and assertions

The repository test helper in `commit_transaction.rs` already initializes a
temporary Git repository, configures an identity, writes files, creates a base
commit, and queries Git.

The required connected fixture needs to create:

- `games/midsummer/docs/active/tickets/<id>.md` in Review;
- `games/midsummer/docs/active/work/<id>/review.md`;
- a root-level `docs/...` sentinel;
- a base commit before the work artifact is added.

The pre-fix argv must be represented as the historical field shape and rejected
by an assertion before it can mutate the fixture. The fixed argv must come from
the real `State::build_completion_command`, be decoded as the real
`complete-ticket` argument contract, and be passed to the real transaction.

Success is observable through one returned commit id, a one-commit HEAD advance,
the exact committed path list, Done frontmatter at the nested ticket, the nested
review artifact in the commit, and unchanged root-level docs content.

## Repository constraints

The ordinary worktree already contains Lisa-managed ticket/provenance changes
and an unrelated untracked `crates/lisa-plugin/docs/` tree. They must be
preserved. Ticket source changes must be committed only with `lisa
commit-ticket` and exact paths. Attempt artifacts remain private and are not
included in the implementation commit.
