//! Mouse event → AppAction dispatcher.
//!
//! Render code records click targets into `App.hit_zones` each frame.
//! This module hit-tests pointer coordinates against those zones and
//! returns the appropriate `AppAction` (or `None` if the click missed
//! every registered target).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use super::app::{App, AppAction};

fn contains(rect: &Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

pub(super) fn handle_mouse(_app: &mut App, event: MouseEvent) -> AppAction {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // hit-test wired in follow-up commits
            AppAction::None
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            // wheel scroll wired in follow-up commits
            AppAction::None
        }
        _ => AppAction::None,
    }
}

#[allow(dead_code)]
pub(super) fn point_in(rect: Option<&Rect>, col: u16, row: u16) -> bool {
    rect.is_some_and(|r| contains(r, col, row))
}
