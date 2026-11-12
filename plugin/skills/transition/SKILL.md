---
name: transition
description: Transition a Jira issue to a workflow status or choose an interactive transition.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

Run `jirac issue transition <KEY> '<STATUS>'` when the target status or
transition ID is known. Without a target, the command opens an interactive
picker; in a non-interactive session, ask the user for the target instead.
Report the transition result.
