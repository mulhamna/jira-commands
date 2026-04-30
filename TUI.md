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
| `T`         | Open theme picker                                             |
| `S`         | Show server summary                                           |
| `g`         | Show config summary                                           |
| `C`         | Open column settings popup                                    |
| `c`         | Create issue                                                  |
| `e`         | Edit issue                                                    |
| `a`         | Open native assignee popup and assign issue                   |
| `t`         | Transition issue                                              |
| `;`         | Add comment                                                   |
| `w`         | Add a single worklog                                          |
| `b`         | Add a bulk/range worklog                                      |
| `l`         | Manage labels                                                 |
| `m`         | Open native component popup and set project-scoped components |
| `u`         | Upload attachment                                             |
| `o`         | Open in browser                                               |
| `r`         | Refresh issue list                                            |
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

### Saved queries popup

| Key       | Action                   |
| --------- | ------------------------ |
| `j` / `k` | Move selection           |
| `Enter`   | Run selected saved query |
| `Esc`     | Cancel                   |

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
