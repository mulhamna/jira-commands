//! Mouse event → AppAction dispatcher.
//!
//! Render code records click targets into `App.hit_zones` each frame.
//! This module hit-tests pointer coordinates against those zones and
//! returns the appropriate `AppAction` (or `None` if the click missed
//! every registered target).

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::app::{App, AppAction};
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

            // List row click
            if let Some(list_rect) = app.hit_zones.list {
                if contains(&list_rect, col, row) {
                    if let Some(idx) = list_row_index(app, &list_rect, row) {
                        app.table_state.select(Some(idx));
                        if is_double_click {
                            app.focus = Focus::Detail;
                        }
                    }
                    return AppAction::None;
                }
            }

            AppAction::None
        }
        MouseEventKind::ScrollUp => {
            // Wheel up over the list scrolls selection up.
            if let Some(list_rect) = app.hit_zones.list {
                if contains(&list_rect, col, row) {
                    app.prev_issue();
                    return AppAction::None;
                }
            }
            AppAction::None
        }
        MouseEventKind::ScrollDown => {
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
