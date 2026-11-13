use egui::{PointerButton, Rect, Response, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::model::group::{Cell, CellState, TileGroup};
use crate::tileset::Tileset;
use crate::ui::neighbor_input::{pressed_state, released_on};
use crate::ui::neighbor_paint::{CORNER_RADIUS, paint_marked_tile, paint_state};

const MIN_CELL_SIZE: f32 = 8.0;
const MAX_CELL_SIZE: f32 = 40.0;
const CELL_GAP: f32 = 2.0;
const OUTLINE_WIDTH: f32 = 2.0;

const CELL_TOOLTIP: &str = "Left-click switches between solid and any.\n\
                            Right-click sets it to empty.\n\
                            Keys 1, 2 and 3 set it to empty, solid or any.";

pub fn show(
    ui: &mut Ui,
    tileset: &Tileset,
    texture: &TextureHandle,
    group: &mut TileGroup,
) -> bool {
    let Some(((left, top), (right, bottom))) = bounds(group) else {
        return false;
    };
    let size = cell_size(ui, (right - left + 1) as f32);
    let mut changed = false;

    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);
        for row in top..=bottom {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);
                for column in left..=right {
                    changed |= show_cell(ui, tileset, texture, group, (column, row), size);
                }
            });
        }
    });

    changed
}

fn bounds(group: &TileGroup) -> Option<(Cell, Cell)> {
    let columns = || group.neighbors.keys().map(|&(column, _)| column);
    let rows = || group.neighbors.keys().map(|&(_, row)| row);

    Some((
        (columns().min()?, rows().min()?),
        (columns().max()?, rows().max()?),
    ))
}

fn cell_size(ui: &Ui, columns: f32) -> f32 {
    ((ui.available_width() - (columns - 1.0) * CELL_GAP) / columns)
        .floor()
        .clamp(MIN_CELL_SIZE, MAX_CELL_SIZE)
}

fn show_cell(
    ui: &mut Ui,
    tileset: &Tileset,
    texture: &TextureHandle,
    group: &mut TileGroup,
    cell: Cell,
    size: f32,
) -> bool {
    let Some(&current) = group.neighbors.get(&cell) else {
        ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
        return false;
    };

    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    match group.own_tile_at(cell) {
        Some(tile) => {
            paint_marked_tile(ui, rect, texture, &tileset.tiles[tile], current.into());
        }
        None => paint_state(ui, rect, current.into()),
    }
    if response.hovered() {
        let outline = Stroke::new(OUTLINE_WIDTH, ui.visuals().selection.stroke.color);
        ui.painter()
            .rect_stroke(rect, CORNER_RADIUS, outline, StrokeKind::Inside);
    }

    let state = edited_state(ui, &response, rect, current);
    response.on_hover_text(CELL_TOOLTIP);
    group.neighbors.insert(cell, state);

    state != current
}

fn edited_state(ui: &Ui, response: &Response, rect: Rect, current: CellState) -> CellState {
    let pressed = pressed_state(ui).and_then(CellState::from_neighbor);
    let mut state = pressed.filter(|_| response.hovered()).unwrap_or(current);

    if released_on(ui, response, rect, PointerButton::Primary) {
        state = state.toggled_solid();
    }
    if released_on(ui, response, rect, PointerButton::Secondary) {
        state = CellState::Empty;
    }

    state
}
