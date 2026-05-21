# JQL recipes for jirac

Common JQL fragments to pair with `jirac issue list --jql`, `jirac issue bulk-transition --jql`, `jirac issue bulk-update --jql`, `jirac issue bulk-comment --jql`, and `jirac issue archive --jql`.

## My work

```
assignee = currentUser() AND resolution = Unresolved ORDER BY updated DESC
assignee = currentUser() AND status = "In Progress"
assignee = currentUser() AND sprint in openSprints()
reporter = currentUser() AND created >= -7d ORDER BY created DESC
```

## Project triage

```
project = PROJ AND status = "To Do" AND assignee is EMPTY
project = PROJ AND priority = Highest AND resolution = Unresolved
project = PROJ AND labels = "needs-triage"
project = PROJ AND status = "In Review" AND updated < -2d
```

## Sprint

```
project = PROJ AND sprint = openSprints()
project = PROJ AND sprint = "Sprint 24"
project = PROJ AND sprint in openSprints() AND status != Done
project = PROJ AND sprint in closedSprints() AND resolution = Done AND updated >= -14d
```

## Recency

```
updated >= -7d ORDER BY updated DESC
created >= -7d ORDER BY created DESC
resolutiondate >= -30d AND status = Done
```

## Cleanup / archive candidates

```
project = PROJ AND resolution = Done AND updated < -180d
project = PROJ AND status = "Won't Do" AND updated < -90d
project = PROJ AND status = Done AND fixVersion in releasedVersions() AND updated < -365d
```

## Linking and parents

```
issueLinkType = "blocks" AND project = PROJ
"Epic Link" = PROJ-100
parent = PROJ-50
```

## Combine with bulk flows

```bash
# Bulk-transition every unassigned "To Do" to "In Progress" in PROJ
jirac issue bulk-transition --jql 'project = PROJ AND status = "To Do" AND assignee is EMPTY' --to 'In Progress'

# Bulk-update priority on Low items
jirac issue bulk-update --jql 'project = PROJ AND priority = Low AND resolution = Unresolved' --priority Medium --force

# Bulk-comment on all open sprint items
jirac issue bulk-comment --jql 'project = PROJ AND sprint in openSprints()' --body 'Please post your standup before 10am.'

# Archive done items older than 180 days
jirac issue archive --jql 'project = PROJ AND resolution = Done AND updated < -180d'
```

## Quoting tips

- Wrap the entire JQL in single quotes in bash, use double quotes inside.
- PowerShell: wrap outside in double quotes, escape inner doubles with backtick (`` ` ``).
- Status names with spaces (`"In Progress"`, `"To Do"`) must be quoted in JQL.
- Use `currentUser()`, `openSprints()`, `closedSprints()`, `releasedVersions()`, `unreleasedVersions()` for dynamic values.
- Negate with `!=` or `NOT`. Range with `>=`, `<=`, `BETWEEN`.
