# T-017-04 Design: Push and Verify CI Green

## Problem
7 commits are ready to push, but there are also uncommitted changes (fmt/clippy fixes, ticket status updates, work artifacts) that need to be committed first.

## Options

### Option A: Single commit for all remaining changes, then push
Commit all uncommitted source changes + ticket updates + work artifacts in one commit, then push everything.

**Pros**: Simple, fast, gets to green quickly.
**Cons**: Lumps unrelated changes (source fixes, ticket status, RDSPI artifacts) into one commit.

### Option B: Two commits — source changes, then ticket/artifact updates
1. Commit source code changes (fmt/clippy fixes, feature work) with a descriptive message
2. Commit ticket status updates + RDSPI work artifacts
3. Push all

**Pros**: Cleaner history — separates code from meta.
**Cons**: More complex, and the source changes are really from T-017-01 which is already done.

### Option C: Single commit referencing completed tickets, then push
One commit that references both T-017-01 and T-017-03 as the completing commit for their remaining work.

**Pros**: Honest — this IS the final commit for that work. Simple.
**Cons**: Still mixes code and meta.

## Decision: Option A — Single commit
The remaining changes are all part of S-017 alpha release prep. The fmt/clippy fixes, ticket status updates, and work artifacts are all outputs of completed tickets. A single commit with a clear message keeps it simple and gets us to the push step quickly. The commit message will reference the ticket IDs.

## Excluded files
- `.lisa.toml` — runtime config, not tracked
- `.lisa/hooks/on-clear.sh` — runtime hook
- `.lisa/hooks/on-stop.sh` — runtime hook

## Push strategy
- Direct push to `origin/main` (no PR needed — these are local-only commits)
- Monitor CI via `gh run list` or GitHub Actions URL
- If CI fails, fix locally and push again

## Risk assessment
- **Low risk**: All 6 CI checks pass locally with the uncommitted changes applied
- **Potential risk**: CI environment differences (Ubuntu vs macOS) — unlikely since all checks are pure Rust with no platform-specific behavior
