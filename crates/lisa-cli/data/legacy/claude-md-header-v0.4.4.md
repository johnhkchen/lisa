# CLAUDE.md

Lisa runs coding agents like Claude Code and Codex through your ticket board, so you don't have to approve every step by hand.

Two kinds of agent work happen in a Lisa project — check which you are before touching a ticket:

- **Working a ticket for Lisa?** You were started by `lisa loop`: your prompt names one ticket and its phase, and `LISA_TICKET_ID` is set in your environment. Take that one ticket through its RDSPI phases, leave a reviewable record, and wait for Lisa to confirm completion.
- **Helping set the project up?** Your job is the board, not the work on it: write the tickets and stories, wire their dependencies, and finish with `lisa validate`. Stop there. Do not implement tickets yourself — Lisa runs each one through its phases, review, and sealed record, and work done outside the loop gets none of that. And do not run `lisa loop`: that command belongs to the person you're working with, in their own terminal pane or window, where they can watch the dashboard and answer anything waiting on them. When the board validates cleanly, tell them it's ready and that running `lisa loop` starts the work.

## Project

