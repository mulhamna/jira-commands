# Interactive TUI

```bash
jirac tui
jirac tui -p PROJ
```

The TUI provides a full-screen terminal interface for browsing and managing Jira issues.

![jirac TUI preview](assets/readme/sample_tui.jpeg)
![jirac TUI Split preview](assets/readme/sample_tui_split.jpeg)

## Keyboard shortcuts

### Issue list

| Key         | Action                                                        |
| ----------- | ------------------------------------------------------------- |
| `j` / `k`   | Navigate up / down                                            |
| `Enter`     | Open split detail view                                        |
| `p`         | Open saved JQL queries                                        |
| `V`         | Browse project fix versions and preview one version backlog   |
| `T`         | Open theme picker                                             |
| `S`         | Show server summary                                           |
| `g`         | Show config summary                                           |
| `C`         | Open column settings popup                                    |
| `c`         | Create issue                                                  |
| `e`         | Edit issue                                                    |
| `y`         | Change issue type in a native Jira move modal                 |
| `M`         | Move issue to another project in a native Jira modal          |
| `a`         | Open native assignee popup and assign issue                   |
| `t`         | Transition issue                                              |
| `;`         | Add comment                                                   |
| `w`         | Add a single worklog                                          |
| `b`         | Add a bulk/range worklog                                      |
| `l`         | Manage labels                                                 |
| `m`         | Open native component popup and set project-scoped components |
| `v`         | Open native fix version popup and set project-scoped versions |
| `s`         | Add issue to an active/future sprint                          |
| `u`         | Upload attachment                                             |
| `o`         | Open in browser                                               |
| `r`         | Refresh issue list                                            |
| `n`         | Scan and open Jira mention notifications                      |
| `R`         | Mark selected notification issue as read                      |
| `/`         | JQL search                                                    |
| `?`         | Show in-app help                                              |
| `q` / `Esc` | Quit or go back, depending on context                         |

### Detail view

| Key         | Action                          |
| ----------- | ------------------------------- |
| `Esc` / `q` | Close detail and return to list |
| `←` / `→`   | Switch detail tabs              |
| `Tab`       | Switch detail tabs              |

Detail tabs:
- Summary
- Versions
- Comments
- Worklog
- Attachments
- Subtasks
- Links

## Worklog flows

- `w` opens the single-worklog modal with time spent, optional date, optional start time, and comment.
- `b` opens the bulk-worklog modal with time spent, from/to date, optional start time, optional weekend exclusion, and comment.
- Bulk submission requires confirmation: press `Ctrl+S` once to review the summary, then `Ctrl+S` again to create the worklogs. Editing any field resets the confirmation state.

### Column settings

| Key           | Action                       |
| ------------- | ---------------------------- |
| `Space`       | Toggle selected column       |
| `a`           | Select all available columns |
| `r`           | Reset to default columns     |
| `s` / `Enter` | Save preferences             |
| `Esc`         | Cancel without saving        |

### Assignee popup

| Key       | Action                         |
| --------- | ------------------------------ |
| Type      | Filter assignee list from Jira |
| `j` / `k` | Move selection                 |
| `Enter`   | Assign selected assignee       |
| `Esc`     | Cancel                         |

### Component popup

| Key       | Action                    |
| --------- | ------------------------- |
| Type      | Filter project components |
| `j` / `k` | Move selection            |
| `Space`   | Toggle selected component |
| `Enter`   | Save selected components  |
| `Esc`     | Cancel                    |

### Fix version popup

| Key       | Action                     |
| --------- | -------------------------- |
| Type      | Filter project fix versions |
| `j` / `k` | Move selection             |
| `Space`   | Toggle selected version    |
| `Enter`   | Save selected versions     |
| `Esc`     | Cancel                     |

### Saved queries popup

| Key       | Action                   |
| --------- | ------------------------ |
| `j` / `k` | Move selection           |
| `Enter`   | Run selected saved query |
| `Esc`     | Cancel                   |

### Project versions browser

| Key       | Action                              |
| --------- | ----------------------------------- |
| Type      | Filter project fix versions         |
| `j` / `k` | Move selection                      |
| `Enter`   | Refresh backlog preview for version |
| `e`       | Edit start/release dates and status |
| `Esc`     | Close browser                       |

### Version edit modal

| Key          | Action                           |
| ------------ | -------------------------------- |
| `Tab`        | Next field                       |
| `Shift+Tab`  | Previous field                   |
| `Ctrl+S`     | Save version metadata            |
| `Enter`      | Submit when focused on one-line field |
| `Esc`        | Cancel                           |

### Theme picker

| Key       | Action                        |
| --------- | ----------------------------- |
| `j` / `k` | Move selection                |
| `Enter`   | Apply and save selected theme |
| `Esc`     | Cancel                        |

### Server and config popups

| Key         | Action      |
| ----------- | ----------- |
| `Esc` / `q` | Close popup |
