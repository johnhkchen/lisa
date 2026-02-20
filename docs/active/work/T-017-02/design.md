# T-017-02 Design: Archive completed stories and tickets

## Approach Options

### Option A: `git mv` for tracked, `mv` for untracked
- Use `git mv` for the 2 tracked stories, 6 tracked tickets, and tracked work dirs
- Use `mv` for all untracked files
- Then `git add` everything
- Pro: Precise, preserves rename detection for tracked files
- Con: Two code paths, more commands

### Option B: `mv` everything, then `git add`
- Use plain `mv` for all files regardless of tracking status
- Then `git add -A docs/active/ docs/archive/`
- Git auto-detects renames (content-based similarity) at commit time
- Pro: Simple, uniform approach, same end result
- Con: Slightly less explicit about renames

### Option C: Stage all untracked first, then `git mv` everything
- `git add` all untracked files first
- Then `git mv` everything uniformly
- Pro: Uniform approach, explicit renames
- Con: Unnecessary staging of files we're immediately moving

## Decision: Option B

`mv` everything then `git add` is the simplest approach. Git's rename detection works on content similarity, so the history tracking result is identical. This avoids having to distinguish tracked vs untracked files in the move commands.

## Post-move cleanup
- Ensure .gitkeep remains in docs/active/work/ (it will, since T-017-* work dirs keep the directory non-empty)
- Ensure archive subdirectories exist before moving
- Verify final state matches acceptance criteria

## Rejected
- Option A: Unnecessarily complex for same result
- Option C: Extra staging step for no benefit
