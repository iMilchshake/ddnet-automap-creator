use egui::containers::Sides;
use egui::{
    Align2, PointerButton, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, TextureHandle,
    Ui, Vec2, pos2,
};

use super::icons::{
    Icon, TURN_BUTTON_SIZE, TURN_BUTTONS, button_rect, icon_button, turn_button, turn_buttons_width,
};
use super::orientations::drawn_name;
use super::{CELL_GAP, FlashKind, GridFlash, TileContext, TilePanel};
use crate::model::neighbor::{CENTRE, Extent, NeighborState, Neighborhood, Offset, Side};
use crate::model::tile::{Centre, TileRule};
use crate::model::transform::Transform;
use crate::tileset::Tileset;
use crate::ui::BUTTON_GAP;
use crate::ui::grid;
use crate::ui::neighbor_input::{pressed_state, released_on};
use crate::ui::neighbor_paint::{CORNER_RADIUS, paint_centre, paint_neighbor};

const MIN_CELL_SIZE: f32 = 16.0;
const EDGE_MARGIN: f32 = TURN_BUTTON_SIZE + CELL_GAP;
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
const RESIZE_TOOLTIP: &str = "Change neighborhood size";
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

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

            changed |= self.show_grid(ui, tileset, texture, context);

            ui.add_space(ROW_TOP_SPACING);
            changed |= self.show_turn_and_chance_row(ui);
        });

        changed
    }

    fn show_grid(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) -> bool {
        let margin = self.edge_margin();
        let size = cell_size(ui, self.neighborhood.extent().width(), margin);

        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                self.show_cells(ui, tileset, texture, context, size)
            })
            .inner
    }

    fn show_cells(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
        size: f32,
    ) -> bool {
        let extent = self.neighborhood.extent();
        let margin = self.edge_margin();
        let grid_size = Vec2::new(
            grid_extent(size, extent.width()),
            grid_extent(size, extent.height()),
        );
        let (outer, _response) =
            ui.allocate_exact_size(grid_size + Vec2::splat(2.0 * margin), Sense::hover());
        let grid = Rect::from_min_size(outer.min + Vec2::splat(margin), grid_size);

        let mut changed = false;
        for offset in extent.offsets() {
            let rect = cell_rect(grid, size, extent, offset);
            changed |= match offset {
                CENTRE => self.show_centre_cell(ui, tileset, texture, rect),
                _ => self.show_state_cell(ui, tileset, texture, offset, rect),
            };
        }

        paint_flash(ui, grid, context.flash);
        changed | self.show_edge_controls(ui, grid)
    }

    fn edge_margin(&self) -> f32 {
        match self.resizing {
            true => EDGE_MARGIN,
            false => 0.0,
        }
    }

    fn show_edge_controls(&mut self, ui: &Ui, grid: Rect) -> bool {
        if !self.resizing {
            return false;
        }

        let resized = Side::ALL
            .map(|side| edge_click(ui, grid, side, &self.neighborhood))
            .into_iter()
            .flatten()
            .next();
        let Some(resized) = resized else {
            return false;
        };

        self.neighborhood = resized;
        self.picking = self
            .picking
            .filter(|offset| self.neighborhood.extent().contains(*offset));
        true
    }

    fn reset_drawn(&mut self) {
        self.drawn = Transform::IDENTITY;
        self.picking = None;
    }

    fn show_turn_and_chance_row(&mut self, ui: &mut Ui) -> bool {
        let (turn, chance_changed) = Sides::new().show(
            ui,
            |ui| {
                let turn = show_neighbor_turn_buttons(ui);
                show_resize_toggle(ui, &mut self.resizing);
                turn
            },
            |ui| show_chance(ui, &mut self.chance_percent),
        );

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
        rect: Rect,
    ) -> bool {
        let response = ui.interact(rect, ui.id().with(CENTRE), Sense::click());
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
        offset: Offset,
        rect: Rect,
    ) -> bool {
        let response = ui.interact(rect, ui.id().with(offset), Sense::click_and_drag());
        let current = self.neighborhood.state(offset);
        paint_neighbor(ui, rect, current, texture, tileset);

        let mut state = current;
        let is_picked = self.picking == Some(offset);
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
            self.picking = (self.picking != Some(offset)).then_some(offset);
            return false;
        }
        if primary {
            state = state.toggled_solid();
        }
        if secondary {
            state = NeighborState::Empty;
        }

        if (primary || secondary || state != current) && self.picking == Some(offset) {
            self.picking = None;
        }
        if state == current {
            return false;
        }

        self.neighborhood.set_state(offset, state);
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

fn show_resize_toggle(ui: &mut Ui, resizing: &mut bool) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(TURN_BUTTON_SIZE), Sense::hover());
    if icon_button(ui, rect, Icon::Resize, RESIZE_TOOLTIP, *resizing) {
        *resizing = !*resizing;
    }
}

fn grid_extent(cell_size: f32, count: i32) -> f32 {
    count as f32 * cell_size + (count - 1) as f32 * CELL_GAP
}

fn cell_size(ui: &Ui, columns: i32, margin: f32) -> f32 {
    ((ui.available_width() - 2.0 * margin - (columns - 1) as f32 * CELL_GAP) / columns as f32)
        .floor()
        .max(MIN_CELL_SIZE)
}

fn cell_rect(grid: Rect, size: f32, extent: Extent, (dx, dy): Offset) -> Rect {
    let column = (dx + extent.left) as f32;
    let row = (dy + extent.up) as f32;

    Rect::from_min_size(
        grid.min + Vec2::new(column, row) * (size + CELL_GAP),
        Vec2::splat(size),
    )
}

fn edge_band(grid: Rect, side: Side) -> Rect {
    match side {
        Side::Left => Rect::from_min_max(
            pos2(grid.min.x - EDGE_MARGIN, grid.min.y),
            pos2(grid.min.x - CELL_GAP, grid.max.y),
        ),
        Side::Right => Rect::from_min_max(
            pos2(grid.max.x + CELL_GAP, grid.min.y),
            pos2(grid.max.x + EDGE_MARGIN, grid.max.y),
        ),
        Side::Up => Rect::from_min_max(
            pos2(grid.min.x, grid.min.y - EDGE_MARGIN),
            pos2(grid.max.x, grid.min.y - CELL_GAP),
        ),
        Side::Down => Rect::from_min_max(
            pos2(grid.min.x, grid.max.y + CELL_GAP),
            pos2(grid.max.x, grid.max.y + EDGE_MARGIN),
        ),
    }
}

fn edge_tooltips(side: Side) -> (&'static str, &'static str) {
    match side {
        Side::Left => ("Add a column on the left", "Remove the left column"),
        Side::Right => ("Add a column on the right", "Remove the right column"),
        Side::Up => ("Add a row on top", "Remove the top row"),
        Side::Down => ("Add a row at the bottom", "Remove the bottom row"),
    }
}

fn edge_click(
    ui: &Ui,
    grid: Rect,
    side: Side,
    neighborhood: &Neighborhood,
) -> Option<Neighborhood> {
    let (along, pair) = match side {
        Side::Left | Side::Right => (Vec2::Y, Vec2::new(1.0, 2.0)),
        Side::Up | Side::Down => (Vec2::X, Vec2::new(2.0, 1.0)),
    };
    let pair_size = Vec2::splat(TURN_BUTTON_SIZE) * pair + along * BUTTON_GAP;
    let area = Align2::CENTER_CENTER.align_size_within_rect(pair_size, edge_band(grid, side));
    let add_rect = Rect::from_min_size(area.min, Vec2::splat(TURN_BUTTON_SIZE));
    let remove_rect = add_rect.translate(along * (TURN_BUTTON_SIZE + BUTTON_GAP));
    let (add_tooltip, remove_tooltip) = edge_tooltips(side);

    let grown =
        icon_button(ui, add_rect, Icon::Add, add_tooltip, false).then(|| neighborhood.grown(side));
    let shrunk = neighborhood.shrunk(side);
    if shrunk.is_some() && icon_button(ui, remove_rect, Icon::Remove, remove_tooltip, false) {
        return shrunk;
    }

    grown
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
