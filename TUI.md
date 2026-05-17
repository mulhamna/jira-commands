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
| `:`         | Add the same comment to many issues by JQL or explicit keys   |
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

## Bulk comment flow

- `:` opens a bulk-comment modal.
- Use either the current/custom JQL field or an explicit issue-key list.
- Write one Markdown comment body that will be posted to every matched issue.
- Submission requires confirmation: press `Ctrl+S` once to review the target count, then `Ctrl+S` again to post.

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
| `n`       | Create a new project fix version    |
| `e`       | Edit name, description, dates, status |
| `Esc`     | Close browser                       |

### Version create / edit modal

| Key          | Action                                |
| ------------ | ------------------------------------- |
| `Tab`        | Next field                            |
| `Shift+Tab`  | Previous field                        |
| `Ctrl+S`     | Create or save version metadata       |
| `Enter`      | Submit when focused on one-line field |
| `Esc`        | Cancel                                |

Create/edit fields include name, description, start date, release date, released, and archived.

### Theme picker

Available presets include dark themes plus **GitHub Light** for a brighter, light-mode-friendly palette.

| Key       | Action                        |
| --------- | ----------------------------- |
| `j` / `k` | Move selection                |
| `Enter`   | Apply and save selected theme |
| `Esc`     | Cancel                        |

### Server and config popups

| Key         | Action      |
| ----------- | ----------- |
| `Esc` / `q` | Close popup |

## Mouse support

The TUI is fully keyboard-driven, but every pointer-friendly interaction is also wired up. Mouse capture is enabled automatically while the TUI is running; to copy text out of the terminal hold `Shift` (iTerm2 / most Linux terminals) or `Option` (Terminal.app) while selecting.

| Action                  | Result                                                                  |
| ----------------------- | ----------------------------------------------------------------------- |
| Click issue row         | Select that row                                                         |
| Double-click issue row  | Open split detail view (same as `Enter`)                                |
| Click detail tab        | Switch to that tab; lazy-fetches comments/worklog/links as needed       |
| Click detail pane       | Switch focus to detail when currently focused on the list               |
| Click picker option     | Apply for single-select pickers (transition, assignee, sprint, saved JQL, theme, project versions browser); toggle for multi-select pickers (components, fix versions, columns) |
| Click outside a popup   | Close popup (equivalent to `Esc`)                                       |
| Scroll wheel on list    | Previous / next issue                                                   |
| Scroll wheel on picker  | Move picker selection up / down                                         |

Notes:
- Modal forms (create / edit / bulk-comment) intentionally ignore outside clicks so partially filled forms aren't discarded. Use `Esc` to cancel.
- Double-click window is 400 ms on the same cell. Slower clicks count as two separate selects.
- Under tmux make sure `set -g mouse on` is set in `~/.tmux.conf` so mouse events are forwarded to the TUI.
