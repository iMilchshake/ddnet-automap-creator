use egui::containers::Sides;
use egui::{
    Align2, PointerButton, Rect, Response, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2,
};

use super::icons::{
    Icon, TURN_BUTTON_SIZE, TURN_BUTTONS, button_rect, turn_button, turn_buttons_width,
};
use super::orientations::drawn_name;
use super::{CELL_GAP, FlashKind, GridFlash, TileContext, TilePanel};
use crate::model::neighbor::{NeighborState, neighbor_index_at};
use crate::model::tile::{Centre, TileRule};
use crate::model::transform::Transform;
use crate::tileset::Tileset;
use crate::ui::grid;
use crate::ui::neighbor_input::{pressed_state, released_on};
use crate::ui::neighbor_paint::{CORNER_RADIUS, paint_centre, paint_neighbor};

const MIN_CELL_SIZE: f32 = 32.0;
const GRID_SIDE: f32 = 3.0;
const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;
const CHANCE_DRAG_SPEED: f64 = 0.5;
const FLASH_PEAK_OPACITY: f32 = 0.5;
const ROW_TOP_SPACING: f32 = 4.0;
const OUTLINE_WIDTH: f32 = 2.0;
const BUTTON_MARGIN: f32 = 1.0;

const CELL_TOOLTIP: &str = "Left-click switches between solid and any. Right-click makes it empty. \
                            You can also press 1, 2 or 3 while hovering. \
                            Ctrl+click picks a specific tile.";
const CHANCE_TOOLTIP: &str = "Tiles with the same neighbors share the spot by their chances. \
                              With Normalize chances off, a total below 100% can leave some \
                              spots as they are.";
const CHANCE_LABEL: &str = "Chance";
const RESET_TOOLTIP: &str = "Reset tile orientation";
const ORIENTATION_HINT: &str = "The buttons turn or flip only this tile.";
const CENTRE_HINT: &str = "Left-click for a solid center, right-click for an empty one. \
                           With an empty center the tile goes into empty spots.";

impl TilePanel {
    pub(super) fn show_neighborhood(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) -> bool {
        let mut changed = false;
        let size = cell_size(ui);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

            let grid_area = Rect::from_min_size(ui.cursor().min, Vec2::splat(grid_extent(size)));
            for dy in -1..=1 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

                    for dx in -1..=1 {
                        match neighbor_index_at(dx, dy) {
                            Some(index) => {
                                changed |= self.show_state_cell(ui, tileset, texture, index, size)
                            }
                            None => changed |= self.show_centre_cell(ui, tileset, texture, size),
                        }
                    }
                });
            }

            paint_flash(ui, grid_area, context.flash);

            ui.add_space(ROW_TOP_SPACING);
            changed |= self.show_turn_and_chance_row(ui);
        });

        changed
    }

    fn reset_drawn(&mut self) {
        self.drawn = Transform::IDENTITY;
        self.picking = None;
    }

    fn show_turn_and_chance_row(&mut self, ui: &mut Ui) -> bool {
        let (turn, chance_changed) = Sides::new().show(ui, show_neighbor_turn_buttons, |ui| {
            show_chance(ui, &mut self.chance_percent)
        });

        let Some(by) = turn else {
            return chance_changed;
        };
        self.turn(TileRule::turned_neighbors, by);
        true
    }

    fn show_centre_cell(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        size: f32,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        let slice = &tileset.tiles[self.tile];
        paint_centre(ui, rect, texture, slice, self.centre, self.drawn);

        let bounds = rect.shrink(BUTTON_MARGIN);
        let mut turn = None;
        let mut reset = false;
        if ui.rect_contains_pointer(rect) {
            for (index, button) in TURN_BUTTONS.into_iter().enumerate() {
                let area = button_rect(bounds, Align2::CENTER_BOTTOM, index);
                if turn_button(ui, area, button.icon, button.tile_tooltip) {
                    turn = Some(button.turn);
                }
            }
            if self.drawn != Transform::IDENTITY {
                let area =
                    Align2::RIGHT_TOP.align_size_within_rect(Vec2::splat(TURN_BUTTON_SIZE), bounds);
                reset = turn_button(ui, area, Icon::Reset, RESET_TOOLTIP);
            }
        }
        let clicked = clicked_centre(&response).filter(|centre| *centre != self.centre);
        response.on_hover_text(format!(
            "{}\n{ORIENTATION_HINT}\n{CENTRE_HINT}",
            drawn_name(self.drawn)
        ));

        if reset {
            self.reset_drawn();
        }
        if let Some(by) = turn {
            self.turn(TileRule::turned_tile, by);
        }
        if let Some(centre) = clicked {
            self.centre = centre;
            self.picking = None;
        }
        reset || turn.is_some() || clicked.is_some()
    }

    fn show_state_cell(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        index: usize,
        size: f32,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let current = self.neighborhood.state(index);
        paint_neighbor(ui, rect, current, texture, tileset);

        let mut state = current;
        let is_picked = self.picking == Some(index);
        let outline = match is_picked {
            true => Stroke::new(grid::SELECTED_OUTLINE_WIDTH, ui.visuals().selection.bg_fill),
            false => Stroke::new(OUTLINE_WIDTH, ui.visuals().selection.stroke.color),
        };
        if response.hovered() || is_picked {
            ui.painter()
                .rect_stroke(rect, CORNER_RADIUS, outline, StrokeKind::Inside);
        }
        if response.hovered()
            && let Some(pressed) = pressed_state(ui)
        {
            state = pressed;
        }

        let primary = released_on(ui, &response, rect, PointerButton::Primary);
        let secondary = released_on(ui, &response, rect, PointerButton::Secondary);
        response.on_hover_text(cell_tooltip(current));

        if primary && ui.input(|input| input.modifiers.command) {
            self.picking = (self.picking != Some(index)).then_some(index);
            return false;
        }
        if primary {
            state = state.toggled_solid();
        }
        if secondary {
            state = NeighborState::Empty;
        }

        if (primary || secondary || state != current) && self.picking == Some(index) {
            self.picking = None;
        }
        if state == current {
            return false;
        }

        self.neighborhood.set_state(index, state);
        true
    }
}

fn show_neighbor_turn_buttons(ui: &mut Ui) -> Option<Transform> {
    let (row, _response) = ui.allocate_exact_size(
        Vec2::new(turn_buttons_width(), TURN_BUTTON_SIZE),
        Sense::hover(),
    );

    let mut turn = None;
    for (index, button) in TURN_BUTTONS.into_iter().enumerate() {
        let area = button_rect(row, Align2::LEFT_CENTER, index);
        if turn_button(ui, area, button.icon, button.neighbors_tooltip) {
            turn = Some(button.turn);
        }
    }

    turn
}

fn show_chance(ui: &mut Ui, chance_percent: &mut f32) -> bool {
    let chance = egui::DragValue::new(chance_percent)
        .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
        .speed(CHANCE_DRAG_SPEED)
        .suffix(" %");
    let changed = ui.add(chance).on_hover_text(CHANCE_TOOLTIP).changed();
    ui.label(CHANCE_LABEL).on_hover_text(CHANCE_TOOLTIP);

    changed
}

fn grid_extent(cell_size: f32) -> f32 {
    GRID_SIDE * cell_size + (GRID_SIDE - 1.0) * CELL_GAP
}

fn cell_size(ui: &Ui) -> f32 {
    ((ui.available_width() - (GRID_SIDE - 1.0) * CELL_GAP) / GRID_SIDE)
        .floor()
        .max(MIN_CELL_SIZE)
}

fn paint_flash(ui: &Ui, rect: Rect, flash: Option<GridFlash>) {
    let Some(flash) = flash else {
        return;
    };

    let base = match flash.kind {
        FlashKind::Copy => ui.visuals().selection.bg_fill,
        FlashKind::Paste => ui.visuals().warn_fg_color,
    };
    let color = base.gamma_multiply(flash.remaining * FLASH_PEAK_OPACITY);
    ui.painter().rect_filled(rect, CORNER_RADIUS, color);
}

fn clicked_centre(response: &Response) -> Option<Centre> {
    if response.clicked_by(PointerButton::Primary) {
        return Some(Centre::Full);
    }

    response
        .clicked_by(PointerButton::Secondary)
        .then_some(Centre::Empty)
}

fn cell_tooltip(state: NeighborState) -> String {
    match state {
        NeighborState::Is(tile) => format!("is tile {tile}\n{CELL_TOOLTIP}"),
        NeighborState::Empty | NeighborState::Full | NeighborState::Any => CELL_TOOLTIP.to_owned(),
    }
}
