use crossterm::event::KeyCode;

use super::app::{App, AppAction};
use super::column::AVAILABLE_COLUMNS;
use super::mode::Mode;
use super::panel::Focus;
use super::prefs::TuiPreferences;

pub(super) fn handle_key(app: &mut App, code: KeyCode) -> AppAction {
    match &app.mode {
        Mode::Browse => {
            if app.focus == Focus::Detail {
                handle_view_key(app, code)
            } else {
                handle_browse_key(app, code)
            }
        }
        Mode::Search => handle_search_key(app, code),
        Mode::Transition => handle_transition_key(app, code),
        Mode::ColumnPicker => handle_column_picker_key(app, code),
        Mode::AssigneePicker => handle_assignee_picker_key(app, code),
        Mode::ComponentPicker => handle_component_picker_key(app, code),
        Mode::Help => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        Mode::SavedJqlPicker => handle_saved_jql_key(app, code),
        Mode::ThemePicker => handle_theme_picker_key(app, code),
        Mode::ServerInfo | Mode::ConfigView => {
            if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                app.mode = Mode::Browse;
            }
            AppAction::None
        }
    }
}

fn handle_browse_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => AppAction::Quit,
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_issue();
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_issue();
            AppAction::None
        }
        KeyCode::Enter => {
            if app.selected_issue().is_some() {
                app.open_detail();
                app.clear_status();
                return AppAction::WarmActiveTab;
            }
            AppAction::None
        }
        KeyCode::Char('r') => AppAction::Refresh,
        KeyCode::Char('t') => AppAction::FetchTransitions,
        KeyCode::Char('o') => AppAction::OpenBrowser,
        KeyCode::Char('/') => {
            app.search_input = app.jql.clone();
            app.search_cursor = app.search_input.chars().count();
            app.mode = Mode::Search;
            AppAction::None
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            AppAction::None
        }
        KeyCode::Char('C') => {
            app.mode = Mode::ColumnPicker;
            AppAction::None
        }
        KeyCode::Char('p') => {
            app.mode = Mode::SavedJqlPicker;
            AppAction::None
        }
        KeyCode::Char('T') => {
            app.mode = Mode::ThemePicker;
            AppAction::None
        }
        KeyCode::Char('S') => {
            app.mode = Mode::ServerInfo;
            AppAction::LoadServerInfo
        }
        KeyCode::Char('g') => {
            app.mode = Mode::ConfigView;
            AppAction::LoadConfigView
        }
        KeyCode::Char('c') => AppAction::CreateIssue,
        KeyCode::Char('e') => app
            .selected_issue_key()
            .map(AppAction::EditIssue)
            .unwrap_or(AppAction::None),
        KeyCode::Char('a') => app
            .selected_issue_key()
            .map(AppAction::OpenAssigneePicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char(';') => app
            .selected_issue_key()
            .map(AppAction::AddComment)
            .unwrap_or(AppAction::None),
        KeyCode::Char('w') => app
            .selected_issue_key()
            .map(AppAction::AddWorklog)
            .unwrap_or(AppAction::None),
        KeyCode::Char('l') => app
            .selected_issue_key()
            .map(AppAction::EditLabels)
            .unwrap_or(AppAction::None),
        KeyCode::Char('m') => app
            .selected_issue_key()
            .map(AppAction::OpenComponentPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('u') => app
            .selected_issue_key()
            .map(AppAction::UploadAttachment)
            .unwrap_or(AppAction::None),
        _ => AppAction::None,
    }
}

fn handle_view_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
            app.close_detail();
            AppAction::None
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let next = app.active_tab.prev();
            app.set_active_tab(next);
            AppAction::WarmActiveTab
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
            let next = app.active_tab.next();
            app.set_active_tab(next);
            AppAction::WarmActiveTab
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_detail_down(1);
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_detail_up(1);
            AppAction::None
        }
        KeyCode::PageDown => {
            app.scroll_detail_down(8);
            AppAction::None
        }
        KeyCode::PageUp => {
            app.scroll_detail_up(8);
            AppAction::None
        }
        KeyCode::Home => {
            app.reset_detail_scroll();
            AppAction::None
        }
        KeyCode::Char('t') => AppAction::FetchTransitions,
        KeyCode::Char('o') => AppAction::OpenBrowser,
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            AppAction::None
        }
        KeyCode::Char('e') => app
            .selected_issue_key()
            .map(AppAction::EditIssue)
            .unwrap_or(AppAction::None),
        KeyCode::Char('a') => app
            .selected_issue_key()
            .map(AppAction::OpenAssigneePicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char(';') => app
            .selected_issue_key()
            .map(AppAction::AddComment)
            .unwrap_or(AppAction::None),
        KeyCode::Char('w') => app
            .selected_issue_key()
            .map(AppAction::AddWorklog)
            .unwrap_or(AppAction::None),
        KeyCode::Char('u') => app
            .selected_issue_key()
            .map(AppAction::UploadAttachment)
            .unwrap_or(AppAction::None),
        _ => AppAction::None,
    }
}

fn handle_search_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Enter => {
            let jql = app.search_input.trim().to_string();
            app.mode = Mode::Browse;
            if jql.is_empty() {
                AppAction::None
            } else {
                AppAction::ExecuteSearch(jql)
            }
        }
        KeyCode::Left => {
            if app.search_cursor > 0 {
                app.search_cursor -= 1;
            }
            AppAction::None
        }
        KeyCode::Right => {
            if app.search_cursor < app.search_input.chars().count() {
                app.search_cursor += 1;
            }
            AppAction::None
        }
        KeyCode::Home => {
            app.search_cursor = 0;
            AppAction::None
        }
        KeyCode::End => {
            app.search_cursor = app.search_input.chars().count();
            AppAction::None
        }
        KeyCode::Backspace => {
            if app.search_cursor > 0 {
                app.search_cursor -= 1;
                let byte_pos = app
                    .search_input
                    .char_indices()
                    .nth(app.search_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.search_input.len());
                let char_len = app.search_input[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.search_input.drain(byte_pos..byte_pos + char_len);
            }
            AppAction::None
        }
        KeyCode::Delete => {
            let len = app.search_input.chars().count();
            if app.search_cursor < len {
                let byte_pos = app
                    .search_input
                    .char_indices()
                    .nth(app.search_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.search_input.len());
                let char_len = app.search_input[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.search_input.drain(byte_pos..byte_pos + char_len);
            }
            AppAction::None
        }
        KeyCode::Char(c) => {
            let byte_pos = app
                .search_input
                .char_indices()
                .nth(app.search_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.search_input.len());
            app.search_input.insert(byte_pos, c);
            app.search_cursor += 1;
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn handle_transition_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            app.transitions.clear();
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_transition();
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_transition();
            AppAction::None
        }
        KeyCode::Enter => {
            if let Some(idx) = app.transition_list_state.selected() {
                if let Some((id, _)) = app.transitions.get(idx) {
                    let action =
                        AppAction::ExecuteTransition(app.transition_issue_key.clone(), id.clone());
                    app.mode = Mode::Browse;
                    return action;
                }
            }
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn handle_column_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app
                .column_picker_state
                .selected()
                .map(|i| (i + 1).min(AVAILABLE_COLUMNS.len() - 1))
                .unwrap_or(0);
            app.column_picker_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app
                .column_picker_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.column_picker_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = app.column_picker_state.selected() {
                let column = AVAILABLE_COLUMNS[idx];
                if app.visible_columns.contains(&column) {
                    if app.visible_columns.len() > 1 {
                        app.visible_columns.retain(|c| *c != column);
                    } else {
                        app.set_status("Keep at least one visible column", true);
                    }
                } else {
                    app.visible_columns.push(column);
                }
            }
            AppAction::None
        }
        KeyCode::Enter | KeyCode::Char('s') => {
            app.mode = Mode::Browse;
            AppAction::SaveColumnPreferences
        }
        KeyCode::Char('a') => {
            app.visible_columns = AVAILABLE_COLUMNS.to_vec();
            app.set_status("Selected all available columns", false);
            AppAction::None
        }
        KeyCode::Char('r') => {
            app.visible_columns = TuiPreferences::default().visible_columns;
            AppAction::ResetColumnPreferences
        }
        _ => AppAction::None,
    }
}

fn handle_assignee_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app
                .assignee_state
                .selected()
                .map(|i| (i + 1).min(app.assignee_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.assignee_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app
                .assignee_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.assignee_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Left => {
            if app.assignee_cursor > 0 {
                app.assignee_cursor -= 1;
            }
            AppAction::None
        }
        KeyCode::Right => {
            if app.assignee_cursor < app.assignee_query.chars().count() {
                app.assignee_cursor += 1;
            }
            AppAction::None
        }
        KeyCode::Backspace => {
            if app.assignee_cursor > 0 {
                app.assignee_cursor -= 1;
                let byte_pos = app
                    .assignee_query
                    .char_indices()
                    .nth(app.assignee_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.assignee_query.len());
                let char_len = app.assignee_query[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.assignee_query.drain(byte_pos..byte_pos + char_len);
                return AppAction::RefreshAssigneeOptions;
            }
            AppAction::None
        }
        KeyCode::Char(c) => {
            let byte_pos = app
                .assignee_query
                .char_indices()
                .nth(app.assignee_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.assignee_query.len());
            app.assignee_query.insert(byte_pos, c);
            app.assignee_cursor += 1;
            AppAction::RefreshAssigneeOptions
        }
        KeyCode::Enter => {
            if let Some(idx) = app.assignee_state.selected() {
                if let Some(option) = app.assignee_options.get(idx) {
                    app.mode = Mode::Browse;
                    return AppAction::AssignIssue(option.value.clone());
                }
            }
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn handle_saved_jql_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app
                .saved_jql_state
                .selected()
                .map(|i| (i + 1).min(app.prefs.saved_jqls.len().saturating_sub(1)))
                .unwrap_or(0);
            app.saved_jql_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app
                .saved_jql_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.saved_jql_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Enter => {
            if let Some(jql) = app.selected_saved_jql().map(|saved| saved.jql.clone()) {
                app.mode = Mode::Browse;
                return AppAction::ApplySavedJql(jql);
            }
            AppAction::None
        }
        KeyCode::Char('c') => AppAction::CreateSavedJql,
        KeyCode::Char('e') => app
            .selected_saved_jql_index()
            .map(AppAction::EditSavedJql)
            .unwrap_or(AppAction::None),
        KeyCode::Char('d') => app
            .selected_saved_jql_index()
            .map(AppAction::DeleteSavedJql)
            .unwrap_or(AppAction::None),
        _ => AppAction::None,
    }
}

fn handle_theme_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app
                .theme_state
                .selected()
                .map(|i| (i + 1).min(super::theme::ThemeName::ALL.len().saturating_sub(1)))
                .unwrap_or(0);
            app.theme_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app
                .theme_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.theme_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
            AppAction::SaveTheme
        }
        _ => AppAction::None,
    }
}

fn handle_component_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app
                .component_state
                .selected()
                .map(|i| (i + 1).min(app.component_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.component_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app
                .component_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.component_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Left => {
            if app.component_cursor > 0 {
                app.component_cursor -= 1;
            }
            AppAction::None
        }
        KeyCode::Right => {
            if app.component_cursor < app.component_query.chars().count() {
                app.component_cursor += 1;
            }
            AppAction::None
        }
        KeyCode::Backspace => {
            if app.component_cursor > 0 {
                app.component_cursor -= 1;
                let byte_pos = app
                    .component_query
                    .char_indices()
                    .nth(app.component_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.component_query.len());
                let char_len = app.component_query[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.component_query.drain(byte_pos..byte_pos + char_len);
                return AppAction::RefreshComponentOptions;
            }
            AppAction::None
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = app.component_state.selected() {
                if let Some(option) = app.component_options.get(idx) {
                    if app.component_selected.contains(&option.value) {
                        app.component_selected.remove(&option.value);
                    } else {
                        app.component_selected.insert(option.value.clone());
                    }
                }
            }
            AppAction::None
        }
        KeyCode::Char(c) => {
            let byte_pos = app
                .component_query
                .char_indices()
                .nth(app.component_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.component_query.len());
            app.component_query.insert(byte_pos, c);
            app.component_cursor += 1;
            AppAction::RefreshComponentOptions
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
            AppAction::EditComponents(app.component_issue_key.clone())
        }
        _ => AppAction::None,
    }
}
