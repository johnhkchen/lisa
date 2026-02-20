# T-011-03 Structure: Feedback Document Layout

## File

Single output file: `docs/active/work/T-011-03/feedback.md`

## Sections (follows ticket template)

### 1. Header + Environment
- Device, OS, arch
- Rust version, Zellij version, Claude Code version

### 2. Build & Install
- What worked (cargo test, just check)
- What didn't (broken symlink, placeholder URLs)
- Suggested improvements (lisa doctor, better error messages)

### 3. Init & Validate
- What worked (scaffolding, detection)
- What didn't (minor gaps)
- Suggested improvements

### 4. Runtime (lisa loop)
Six subsections per template:
- Dashboard: Ralph naming in header, dead code warnings
- Scheduling: works correctly after S-005 fixes
- Transitions: event-driven hooks from S-008/S-010 work
- Session management: prompt delivery, context quality
- Hotkeys: pause, mark-done, reset, scroll
- Error handling: no dep checks before launch

### 5. Bugs Found Table
Columns: #, Severity, Description, Repro steps
Entries: broken symlink, ralph naming, dead code warnings, missing dep checks

### 6. QoL Improvement Ideas Table
Columns: #, Category, Idea, Effort estimate
Entries mapped from S-013 through S-016 findings

### 7. Priorities for S-012
Top 5 items ranked by impact, aligned with S-012 ticket list

## No Other Files Modified
This is a pure documentation task. No code changes.
