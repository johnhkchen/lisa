# prompts/

Reusable prompts that produce a specific effect with lisa. Hand one to an agent —
paste it into a `claude` session, drop it into a ticket body, or give it to a lisa
loop agent — to get the described setup without re-explaining it each time.

| Prompt | Effect |
|--------|--------|
| [setup-ntfy-notifications.md](setup-ntfy-notifications.md) | Wire lisa's `on-notify` hook to push phone/desktop notifications via ntfy.sh, using `lisa hooks-guide` |

Each file has a **## Prompt** section (the exact text to give the agent) and, where
useful, a **## Expected result** showing what the agent should produce. Prompts assume
the project has been set up with `lisa init` (so `.lisa/hooks/` and the hook bindings
exist) and that the `lisa` CLI is on `PATH`.
