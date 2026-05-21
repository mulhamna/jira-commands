---
description: Manage Jira issue links (blocks, relates, duplicates, etc.) with jirac — list link types, add a link, or delete an existing link
---

Manage links between Jira issues using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Determine the operation from the user's request:
   - **List link types**: `jirac issue link list-types`
   - **Add link**: needs link type (e.g. Blocks, Relates, Duplicates), outward key (source), inward key (target).
   - **Delete link**: needs the link ID (obtain it from `jirac issue view <KEY>`).
3. For unfamiliar link type names, run `jirac issue link list-types` first.
4. Run the appropriate command and report the resulting link.

Notes:
- The **outward** issue is the "source" of the link (the blocker, the duplicate). The **inward** issue is the "target" (the blocked, the original).
- Common link type names: `Blocks`, `Relates`, `Duplicates`, `Clones`, `Cloners`. Casing matters — match what `list-types` returns.
- Adding the same link twice is allowed by Jira and produces duplicate entries — check existing links via `jirac issue view <KEY>` before adding.

Examples:
- "what link types are available" → `jirac issue link list-types`
- "PROJ-1 blocks PROJ-2" → `jirac issue link add --link-type Blocks PROJ-1 PROJ-2`
- "PROJ-3 is related to PROJ-4" → `jirac issue link add --link-type Relates PROJ-3 PROJ-4`
- "PROJ-5 duplicates PROJ-6" → `jirac issue link add --link-type Duplicates PROJ-5 PROJ-6`
- "add a comment when linking" → `jirac issue link add --link-type Blocks --comment 'blocked by infra change' PROJ-1 PROJ-2`
- "delete link 10042" → `jirac issue link delete 10042`
