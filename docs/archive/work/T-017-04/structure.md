# T-017-04 Structure: Push and Verify CI Green

## Files to Stage (Commit)

### Source code changes (from T-017-01 fmt/clippy + improvements)
- `crates/lisa-cli/build.rs`
- `crates/lisa-cli/src/config.rs`
- `crates/lisa-cli/src/detect.rs`
- `crates/lisa-cli/src/doctor.rs`
- `crates/lisa-cli/src/init.rs`
- `crates/lisa-cli/src/loop_cmd.rs`
- `crates/lisa-cli/src/main.rs`
- `crates/lisa-cli/src/setup_guide.rs`
- `crates/lisa-cli/src/status.rs`
- `crates/lisa-cli/src/templates.rs`
- `crates/lisa-core/src/dag.rs`
- `crates/lisa-core/src/diagnostics.rs`
- `crates/lisa-core/src/ticket.rs`
- `crates/lisa-core/src/types.rs`
- `crates/lisa-plugin/src/lib.rs`
- `crates/lisa-plugin/src/ui.rs`

### Ticket status updates
- `docs/active/tickets/T-017-01-fmt-and-clippy.md`
- `docs/active/tickets/T-017-03-commit-pending-work.md`
- `docs/active/tickets/T-017-04-verify-ci.md`

### RDSPI work artifacts
- `docs/active/work/T-017-01/` (entire directory)
- `docs/active/work/T-017-03/progress.md`
- `docs/active/work/T-017-04/` (research.md, design.md, structure.md, plan.md, progress.md)

## Files to EXCLUDE
- `.lisa.toml` — runtime config
- `.lisa/hooks/on-clear.sh` — runtime hook
- `.lisa/hooks/on-stop.sh` — runtime hook

## No Files Created or Deleted
This ticket modifies existing files and adds work artifacts only.

## Ordering
1. Stage all files listed above
2. Commit with message referencing S-017
3. Push to origin/main
4. Monitor CI
5. Fix and re-push if needed
