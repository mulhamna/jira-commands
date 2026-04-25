# Interactive TUI

```bash
jirac tui
jirac tui -p PROJ
```

The TUI provides a full-screen terminal interface for browsing and managing Jira issues.

![jirac TUI preview](assets/readme/sample_tui.jpeg)

## Keyboard shortcuts

| Key         | Action                                                              |
| ----------- | ------------------------------------------------------------------- |
| `j` / `k`   | Navigate up / down                                                  |
| `Enter`     | View issue                                                          |
| `C`         | Open column settings popup                                          |
| `c`         | Create issue                                                        |
| `e`         | Edit issue                                                          |
| `a`         | Open native assignee popup and assign issue                         |
| `t`         | Transition issue                                                    |
| `w`         | Add worklog                                                         |
| `l`         | Manage labels                                                       |
| `m`         | Open native component popup and set project-scoped components       |
| `u`         | Upload attachment                                                   |
| `o`         | Open in browser                                                     |
| `r`         | Refresh issue list                                                  |
| `/`         | JQL search                                                          |
| `?`         | Show in-app help                                                    |
| `q` / `Esc` | Quit or go back, depending on context                               |

## Column settings

| Key           | Action                       |
| ------------- | ---------------------------- |
| `Space`       | Toggle selected column       |
| `a`           | Select all available columns |
| `r`           | Reset to default columns     |
| `s` / `Enter` | Save preferences             |
| `Esc`         | Cancel without saving        |

## Assignee popup

| Key         | Action |
| ----------- | ------ |
| Type        | Filter assignee list from Jira |
| `j` / `k`   | Move selection |
| `Enter`     | Assign selected assignee |
| `Esc`       | Cancel |

## Component popup

| Key         | Action |
| ----------- | ------ |
| Type        | Filter project components |
| `j` / `k`   | Move selection |
| `Space`     | Toggle selected component |
| `Enter`     | Save selected components |
| `Esc`       | Cancel |
