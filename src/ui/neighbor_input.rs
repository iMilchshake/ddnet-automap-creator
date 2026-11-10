use egui::{PointerButton, Rect, Response, Ui};

use crate::model::neighbor::NeighborState;

pub fn released_on(ui: &Ui, response: &Response, rect: Rect, button: PointerButton) -> bool {
    let released_inside = response.drag_stopped_by(button)
        && ui.input(|input| {
            input
                .pointer
                .latest_pos()
                .is_some_and(|pos| rect.contains(pos))
        });
    response.clicked_by(button) || released_inside
}

pub fn pressed_state(ui: &Ui) -> Option<NeighborState> {
    ui.input(|input| {
        if input.key_pressed(egui::Key::Num1) {
            Some(NeighborState::Empty)
        } else if input.key_pressed(egui::Key::Num2) {
            Some(NeighborState::Full)
        } else if input.key_pressed(egui::Key::Num3) {
            Some(NeighborState::Any)
        } else {
            None
        }
    })
}
