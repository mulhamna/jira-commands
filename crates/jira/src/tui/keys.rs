use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;

use super::app::{App, AppAction};
use super::column::default_column_ids;
use super::modal::{handle_modal_key, ModalOutcome};
use super::mode::Mode;
use super::panel::Focus;
use super::prefs::TuiPreferences;

pub(super) fn handle_key(app: &mut App, event: KeyEvent) -> AppAction {
    let code = event.code;
    match &app.mode {
        Mode::Modal => {
            let Some(modal) = app.modal.as_mut() else {
                app.close_modal();
                return AppAction::None;
            };
            match handle_modal_key(modal, event) {
                ModalOutcome::Cancel => AppAction::CancelModal,
                ModalOutcome::Submit => AppAction::SubmitModal,
                ModalOutcome::Continue => AppAction::None,
                ModalOutcome::MentionQueryChanged => AppAction::RefreshMentionOptions,
                ModalOutcome::MentionSelected(idx) => AppAction::SelectMention(idx),
            }
        }
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
        Mode::FixVersionPicker => handle_fix_version_picker_key(app, code),
        Mode::SprintPicker => handle_sprint_picker_key(app, code),
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
        KeyCode::Char('n') => AppAction::OpenNotifications,
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
            app.column_picker_filter.clear();
            app.column_picker_state.select(Some(0));
            AppAction::None
        }
        KeyCode::Char('p') => {
            app.jql_picker_filter.clear();
            app.saved_jql_state.select(Some(0));
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
        KeyCode::Char('b') => app
            .selected_issue_key()
            .map(AppAction::AddBulkWorklog)
            .unwrap_or(AppAction::None),
        KeyCode::Char('l') => app
            .selected_issue_key()
            .map(AppAction::EditLabels)
            .unwrap_or(AppAction::None),
        KeyCode::Char('m') => app
            .selected_issue_key()
            .map(AppAction::OpenComponentPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('v') => app
            .selected_issue_key()
            .map(AppAction::OpenFixVersionPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('s') => app
            .selected_issue_key()
            .map(AppAction::OpenSprintPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('u') => app
            .selected_issue_key()
            .map(AppAction::UploadAttachment)
            .unwrap_or(AppAction::None),
        KeyCode::Char('y') => app
            .selected_issue_key()
            .map(AppAction::OpenChangeTypeModal)
            .unwrap_or(AppAction::None),
        KeyCode::Char('M') => app
            .selected_issue_key()
            .map(AppAction::OpenMoveIssueModal)
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
        KeyCode::Char('b') => app
            .selected_issue_key()
            .map(AppAction::AddBulkWorklog)
            .unwrap_or(AppAction::None),
        KeyCode::Char('m') => app
            .selected_issue_key()
            .map(AppAction::OpenComponentPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('v') => app
            .selected_issue_key()
            .map(AppAction::OpenFixVersionPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('s') => app
            .selected_issue_key()
            .map(AppAction::OpenSprintPicker)
            .unwrap_or(AppAction::None),
        KeyCode::Char('u') => app
            .selected_issue_key()
            .map(AppAction::UploadAttachment)
            .unwrap_or(AppAction::None),
        KeyCode::Char('y') => app
            .selected_issue_key()
            .map(AppAction::OpenChangeTypeModal)
            .unwrap_or(AppAction::None),
        KeyCode::Char('M') => app
            .selected_issue_key()
            .map(AppAction::OpenMoveIssueModal)
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
            let query = app.search_input.trim().to_string();
            app.mode = Mode::Browse;
            if query.is_empty() {
                AppAction::None
            } else {
                AppAction::ExecuteSearch(super::app::build_search_jql(app, &query))
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
        KeyCode::Esc => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down => {
            let total = app.filtered_picker_fields().len();
            if total == 0 {
                return AppAction::None;
            }
            let i = app
                .column_picker_state
                .selected()
                .map(|i| (i + 1).min(total - 1))
                .unwrap_or(0);
            app.column_picker_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
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
                let filtered = app.filtered_picker_fields();
                if let Some(spec) = filtered.get(idx) {
                    let id = spec.id.clone();
                    if app.visible_columns.contains(&id) {
                        if app.visible_columns.len() > 1 {
                            app.visible_columns.retain(|c| c != &id);
                        } else {
                            app.set_status("Keep at least one visible column", true);
                        }
                    } else {
                        app.visible_columns.push(id);
                    }
                }
            }
            AppAction::None
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
            AppAction::SaveColumnPreferences
        }
        KeyCode::Backspace => {
            app.column_picker_filter.pop();
            app.column_picker_state.select(Some(0));
            AppAction::None
        }
        KeyCode::Char(c)
            if c.is_ascii() && (c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') =>
        {
            // Treat alphanumeric input as filter text. Preserve special hotkeys via Ctrl/Alt elsewhere.
            // Note: KeyCode::Char(' ') is handled above, so we won't reach here for ' '.
            app.column_picker_filter.push(c);
            app.column_picker_state.select(Some(0));
            AppAction::None
        }
        KeyCode::Tab => {
            // Reset filter
            app.column_picker_filter.clear();
            app.column_picker_state.select(Some(0));
            AppAction::None
        }
        KeyCode::F(2) => {
            app.visible_columns = default_column_ids();
            app.set_status("Selected all built-in columns", false);
            AppAction::None
        }
        KeyCode::F(5) => {
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
        KeyCode::Down => {
            let i = app
                .assignee_state
                .selected()
                .map(|i| (i + 1).min(app.assignee_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.assignee_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
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
        KeyCode::Esc => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down => {
            let total = app.filtered_saved_jqls().len();
            if total == 0 {
                return AppAction::None;
            }
            let i = app
                .saved_jql_state
                .selected()
                .map(|i| (i + 1).min(total - 1))
                .unwrap_or(0);
            app.saved_jql_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
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
        KeyCode::Tab => {
            app.jql_picker_filter.clear();
            app.saved_jql_state.select(Some(0));
            AppAction::None
        }
        KeyCode::Backspace => {
            app.jql_picker_filter.pop();
            app.saved_jql_state.select(Some(0));
            AppAction::None
        }
        KeyCode::Char('c') if app.jql_picker_filter.is_empty() => AppAction::CreateSavedJql,
        KeyCode::Char('e') if app.jql_picker_filter.is_empty() => app
            .selected_saved_jql_index()
            .map(AppAction::EditSavedJql)
            .unwrap_or(AppAction::None),
        KeyCode::Char('d') if app.jql_picker_filter.is_empty() => app
            .selected_saved_jql_index()
            .map(AppAction::DeleteSavedJql)
            .unwrap_or(AppAction::None),
        KeyCode::Char(c) => {
            app.jql_picker_filter.push(c);
            app.saved_jql_state.select(Some(0));
            AppAction::None
        }
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
        KeyCode::Down => {
            picker_nav_down(&mut app.component_state, app.component_options.len());
            AppAction::None
        }
        KeyCode::Up => {
            picker_nav_up(&mut app.component_state);
            AppAction::None
        }
        KeyCode::Left => {
            picker_cursor_left(&mut app.component_cursor);
            AppAction::None
        }
        KeyCode::Right => {
            picker_cursor_right(&mut app.component_cursor, &app.component_query);
            AppAction::None
        }
        KeyCode::Backspace => {
            if picker_backspace(&mut app.component_query, &mut app.component_cursor) {
                AppAction::RefreshComponentOptions
            } else {
                AppAction::None
            }
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
            picker_type_char(&mut app.component_query, &mut app.component_cursor, c);
            AppAction::RefreshComponentOptions
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
            AppAction::EditComponents(app.component_issue_key.clone())
        }
        _ => AppAction::None,
    }
}

fn handle_fix_version_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down => {
            picker_nav_down(&mut app.fix_version_state, app.fix_version_options.len());
            AppAction::None
        }
        KeyCode::Up => {
            picker_nav_up(&mut app.fix_version_state);
            AppAction::None
        }
        KeyCode::Left => {
            picker_cursor_left(&mut app.fix_version_cursor);
            AppAction::None
        }
        KeyCode::Right => {
            picker_cursor_right(&mut app.fix_version_cursor, &app.fix_version_query);
            AppAction::None
        }
        KeyCode::Backspace => {
            if picker_backspace(&mut app.fix_version_query, &mut app.fix_version_cursor) {
                AppAction::RefreshFixVersionOptions
            } else {
                AppAction::None
            }
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = app.fix_version_state.selected() {
                if let Some(option) = app.fix_version_options.get(idx) {
                    if app.fix_version_selected.contains(&option.value) {
                        app.fix_version_selected.remove(&option.value);
                    } else {
                        app.fix_version_selected.insert(option.value.clone());
                    }
                }
            }
            AppAction::None
        }
        KeyCode::Char(c) => {
            picker_type_char(&mut app.fix_version_query, &mut app.fix_version_cursor, c);
            AppAction::RefreshFixVersionOptions
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
            AppAction::EditFixVersions(app.fix_version_issue_key.clone())
        }
        _ => AppAction::None,
    }
}

fn handle_sprint_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down => {
            picker_nav_down(&mut app.sprint_state, app.sprint_options.len());
            AppAction::None
        }
        KeyCode::Up => {
            picker_nav_up(&mut app.sprint_state);
            AppAction::None
        }
        KeyCode::Left => {
            picker_cursor_left(&mut app.sprint_cursor);
            AppAction::None
        }
        KeyCode::Right => {
            picker_cursor_right(&mut app.sprint_cursor, &app.sprint_query);
            AppAction::None
        }
        KeyCode::Backspace => {
            if picker_backspace(&mut app.sprint_query, &mut app.sprint_cursor) {
                AppAction::RefreshSprintOptions
            } else {
                AppAction::None
            }
        }
        KeyCode::Char(c) => {
            picker_type_char(&mut app.sprint_query, &mut app.sprint_cursor, c);
            AppAction::RefreshSprintOptions
        }
        KeyCode::Enter => AppAction::ApplySprintSelection(app.sprint_issue_key.clone()),
        _ => AppAction::None,
    }
}

fn picker_nav_down(state: &mut ListState, len: usize) {
    let i = state
        .selected()
        .map(|i| (i + 1).min(len.saturating_sub(1)))
        .unwrap_or(0);
    state.select(Some(i));
}

fn picker_nav_up(state: &mut ListState) {
    let i = state.selected().map(|i| i.saturating_sub(1)).unwrap_or(0);
    state.select(Some(i));
}

fn picker_cursor_left(cursor: &mut usize) {
    if *cursor > 0 {
        *cursor -= 1;
    }
}

fn picker_cursor_right(cursor: &mut usize, query: &str) {
    if *cursor < query.chars().count() {
        *cursor += 1;
    }
}

fn picker_backspace(query: &mut String, cursor: &mut usize) -> bool {
    if *cursor > 0 {
        *cursor -= 1;
        let byte_pos = query
            .char_indices()
            .nth(*cursor)
            .map(|(i, _)| i)
            .unwrap_or(query.len());
        let char_len = query[byte_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        query.drain(byte_pos..byte_pos + char_len);
        true
    } else {
        false
    }
}

fn picker_type_char(query: &mut String, cursor: &mut usize, c: char) {
    let byte_pos = query
        .char_indices()
        .nth(*cursor)
        .map(|(i, _)| i)
        .unwrap_or(query.len());
    query.insert(byte_pos, c);
    *cursor += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_backspace_removes_previous_character_at_cursor() {
        let mut query = "abcd".to_string();
        let mut cursor = 2;

        let changed = picker_backspace(&mut query, &mut cursor);

        assert!(changed);
        assert_eq!(query, "acd");
        assert_eq!(cursor, 1);
    }

    #[test]
    fn picker_backspace_is_noop_at_start() {
        let mut query = "abcd".to_string();
        let mut cursor = 0;

        let changed = picker_backspace(&mut query, &mut cursor);

        assert!(!changed);
        assert_eq!(query, "abcd");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn picker_type_char_inserts_at_cursor_position() {
        let mut query = "acd".to_string();
        let mut cursor = 1;

        picker_type_char(&mut query, &mut cursor, 'b');

        assert_eq!(query, "abcd");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn picker_helpers_handle_unicode_boundaries() {
        let mut query = "aéz".to_string();
        let mut cursor = 2;

        let changed = picker_backspace(&mut query, &mut cursor);
        assert!(changed);
        assert_eq!(query, "az");
        assert_eq!(cursor, 1);

        picker_type_char(&mut query, &mut cursor, 'é');
        assert_eq!(query, "aéz");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn picker_navigation_and_cursor_helpers_clamp_safely() {
        let mut state = ListState::default();
        picker_nav_down(&mut state, 3);
        assert_eq!(state.selected(), Some(0));
        picker_nav_down(&mut state, 3);
        assert_eq!(state.selected(), Some(1));
        picker_nav_up(&mut state);
        assert_eq!(state.selected(), Some(0));
        picker_nav_up(&mut state);
        assert_eq!(state.selected(), Some(0));

        let mut cursor = 0;
        picker_cursor_left(&mut cursor);
        assert_eq!(cursor, 0);
        picker_cursor_right(&mut cursor, "ab");
        picker_cursor_right(&mut cursor, "ab");
        picker_cursor_right(&mut cursor, "ab");
        assert_eq!(cursor, 2);
        picker_cursor_left(&mut cursor);
        assert_eq!(cursor, 1);
    }
}
