# Operator notes — XDG variant + tour rematch (shared container)

- XDG pre-grant field-confirmed: with XDG_CACHE_HOME seeded for the agent
  session, `~/.xdg-cache/zellij/permissions.kdl` exists (T-046-02-03's fix
  observed in the wild). The default `~/.cache/zellij/` also has entries —
  attributable to operator-shell invocations without the seeded env; the
  agent-session path is the one under test.
- Deviation: the tour rematch ran in THIS container (post-leg, lisa already
  installed) rather than a fresh one. Session context was fresh (new claude
  invocation); filesystem was not. Recorded per the landing-probes README's
  one-axis-at-a-time rule; next entry should restore the fresh-container
  condition.
