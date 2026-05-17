//! Mouse event → AppAction dispatcher.
//!
//! Render code records click targets into `App.hit_zones` each frame.
//! This module hit-tests pointer coordinates against those zones and
//! returns the appropriate `AppAction` (or `None` if the click missed
//! every registered target).

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

use super::app::{App, AppAction};
use super::keys;
use super::mode::Mode;
use super::panel::Focus;

/// Maximum gap between two left-clicks on the same cell to count as a double-click.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(400);

/// Vertical offset from a list/table's top-left corner to the first data row.
/// Layout is: top border (1) + header row (1) + header bottom_margin (1) = 3.
const LIST_HEADER_OFFSET: u16 = 3;

fn contains(rect: &Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

/// Translate a pointer row inside the list rect to an issue index, accounting
/// for table scroll offset. Returns `None` if the click landed on the header,
/// border, or below the last visible row.
fn list_row_index(app: &App, rect: &Rect, row: u16) -> Option<usize> {
    let first_data_row = rect.y.saturating_add(LIST_HEADER_OFFSET);
    let last_data_row = rect.y.saturating_add(rect.height).saturating_sub(2); // bottom border
    if row < first_data_row || row >= last_data_row {
        return None;
    }
    let visible_offset = (row - first_data_row) as usize;
    let scroll_offset = app.table_state.offset();
    let idx = scroll_offset + visible_offset;
    (idx < app.issues.len()).then_some(idx)
}

fn synth_press(app: &mut App, code: KeyCode) -> AppAction {
    keys::handle_key(app, KeyEvent::new(code, KeyModifiers::NONE))
}

/// Select an option in a picker list state and then synthesize the keypress
/// (`Enter` for single-select pickers, `Space` for multi-select) so the
/// existing keyboard handler runs the apply/toggle logic.
fn select_and_press(
    app: &mut App,
    state_field: fn(&mut App) -> &mut ListState,
    idx: usize,
    code: KeyCode,
) -> AppAction {
    state_field(app).select(Some(idx));
    synth_press(app, code)
}

pub(super) fn handle_mouse(app: &mut App, event: MouseEvent) -> AppAction {
    let col = event.column;
    let row = event.row;

    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let now = Instant::now();
            let is_double_click = app
                .last_click
                .map(|(t, c, r)| {
                    now.duration_since(t) <= DOUBLE_CLICK_WINDOW && c == col && r == row
                })
                .unwrap_or(false);
            app.last_click = Some((now, col, row));

            // Picker option click — dispatch per-mode.
            if let Some(picker) = app.hit_zones.picker {
                if let Some(idx) = picker.row_to_index(row) {
                    if col >= picker.area.x && col < picker.area.x.saturating_add(picker.area.width)
                    {
                        match app.mode {
                            Mode::Transition => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.transition_list_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::AssigneePicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.assignee_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::SprintPicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.sprint_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::SavedJqlPicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.saved_jql_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::ThemePicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.theme_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::ProjectVersionBrowser => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.project_version_state,
                                    idx,
                                    KeyCode::Enter,
                                );
                            }
                            Mode::ComponentPicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.component_state,
                                    idx,
                                    KeyCode::Char(' '),
                                );
                            }
                            Mode::FixVersionPicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.fix_version_state,
                                    idx,
                                    KeyCode::Char(' '),
                                );
                            }
                            Mode::ColumnPicker => {
                                return select_and_press(
                                    app,
                                    |a| &mut a.column_picker_state,
                                    idx,
                                    KeyCode::Char(' '),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Detail tab header click → switch tab + warm async.
            for (tab_rect, tab) in app.hit_zones.detail_tabs.clone() {
                if contains(&tab_rect, col, row) {
                    app.focus = Focus::Detail;
                    app.set_active_tab(tab);
                    return AppAction::WarmActiveTab;
                }
            }

            // List row click → select issue. In list-only view a double-click
            // promotes focus to Detail (matches Enter key behavior).
            if let Some(list_rect) = app.hit_zones.list {
                if contains(&list_rect, col, row) {
                    if let Some(idx) = list_row_index(app, &list_rect, row) {
                        app.table_state.select(Some(idx));
                        if is_double_click && app.focus == Focus::List {
                            app.focus = Focus::Detail;
                            return AppAction::WarmActiveTab;
                        }
                    }
                    return AppAction::None;
                }
            }

            // Click anywhere inside the detail pane but not on a tab → focus
            // the detail (no-op if focus is already Detail).
            if let Some(pane) = app.hit_zones.detail_pane {
                if contains(&pane, col, row) {
                    if app.focus != Focus::Detail {
                        app.focus = Focus::Detail;
                        return AppAction::WarmActiveTab;
                    }
                    return AppAction::None;
                }
            }

            // Click outside an active popup → close (synthesize Esc, which every
            // popup-mode handler treats as cancel).
            if is_popup_mode(&app.mode) {
                if let Some(popup) = app.hit_zones.popup {
                    if !contains(&popup, col, row) {
                        return synth_press(app, KeyCode::Esc);
                    }
                }
            }

            AppAction::None
        }
        MouseEventKind::ScrollUp => {
            // Wheel over a picker → move selection up.
            if let Some(picker) = app.hit_zones.picker {
                if contains(&picker.area, col, row) {
                    return synth_press(app, KeyCode::Up);
                }
            }
            // Wheel over the issue list → previous issue.
            if let Some(list_rect) = app.hit_zones.list {
                if contains(&list_rect, col, row) {
                    app.prev_issue();
                    return AppAction::None;
                }
            }
            AppAction::None
        }
        MouseEventKind::ScrollDown => {
            if let Some(picker) = app.hit_zones.picker {
                if contains(&picker.area, col, row) {
                    return synth_press(app, KeyCode::Down);
                }
            }
            if let Some(list_rect) = app.hit_zones.list {
                if contains(&list_rect, col, row) {
                    app.next_issue();
                    return AppAction::None;
                }
            }
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn is_popup_mode(mode: &Mode) -> bool {
    matches!(
        mode,
        Mode::Transition
            | Mode::Help
            | Mode::ProjectVersionBrowser
            | Mode::ColumnPicker
            | Mode::AssigneePicker
            | Mode::ComponentPicker
            | Mode::FixVersionPicker
            | Mode::SprintPicker
            | Mode::SavedJqlPicker
            | Mode::ThemePicker
            | Mode::ServerInfo
            | Mode::ConfigView
    )
}
