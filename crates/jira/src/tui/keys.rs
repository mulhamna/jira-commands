use crossterm::event::{KeyCode, KeyEvent};

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
            let i = app
                .component_state
                .selected()
                .map(|i| (i + 1).min(app.component_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.component_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
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

fn handle_fix_version_picker_key(app: &mut App, code: KeyCode) -> AppAction {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Browse;
            AppAction::None
        }
        KeyCode::Down => {
            let i = app
                .fix_version_state
                .selected()
                .map(|i| (i + 1).min(app.fix_version_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.fix_version_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
            let i = app
                .fix_version_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.fix_version_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Left => {
            if app.fix_version_cursor > 0 {
                app.fix_version_cursor -= 1;
            }
            AppAction::None
        }
        KeyCode::Right => {
            if app.fix_version_cursor < app.fix_version_query.chars().count() {
                app.fix_version_cursor += 1;
            }
            AppAction::None
        }
        KeyCode::Backspace => {
            if app.fix_version_cursor > 0 {
                app.fix_version_cursor -= 1;
                let byte_pos = app
                    .fix_version_query
                    .char_indices()
                    .nth(app.fix_version_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.fix_version_query.len());
                let char_len = app.fix_version_query[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.fix_version_query.drain(byte_pos..byte_pos + char_len);
                return AppAction::RefreshFixVersionOptions;
            }
            AppAction::None
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
            let byte_pos = app
                .fix_version_query
                .char_indices()
                .nth(app.fix_version_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.fix_version_query.len());
            app.fix_version_query.insert(byte_pos, c);
            app.fix_version_cursor += 1;
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
            let i = app
                .sprint_state
                .selected()
                .map(|i| (i + 1).min(app.sprint_options.len().saturating_sub(1)))
                .unwrap_or(0);
            app.sprint_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Up => {
            let i = app
                .sprint_state
                .selected()
                .map(|i| i.saturating_sub(1))
                .unwrap_or(0);
            app.sprint_state.select(Some(i));
            AppAction::None
        }
        KeyCode::Left => {
            if app.sprint_cursor > 0 {
                app.sprint_cursor -= 1;
            }
            AppAction::None
        }
        KeyCode::Right => {
            if app.sprint_cursor < app.sprint_query.chars().count() {
                app.sprint_cursor += 1;
            }
            AppAction::None
        }
        KeyCode::Backspace => {
            if app.sprint_cursor > 0 {
                app.sprint_cursor -= 1;
                let byte_pos = app
                    .sprint_query
                    .char_indices()
                    .nth(app.sprint_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.sprint_query.len());
                let char_len = app.sprint_query[byte_pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                app.sprint_query.drain(byte_pos..byte_pos + char_len);
                return AppAction::RefreshSprintOptions;
            }
            AppAction::None
        }
        KeyCode::Char(c) => {
            let byte_pos = app
                .sprint_query
                .char_indices()
                .nth(app.sprint_cursor)
                .map(|(i, _)| i)
                .unwrap_or(app.sprint_query.len());
            app.sprint_query.insert(byte_pos, c);
            app.sprint_cursor += 1;
            AppAction::RefreshSprintOptions
        }
        KeyCode::Enter => AppAction::ApplySprintSelection(app.sprint_issue_key.clone()),
        _ => AppAction::None,
    }
}
