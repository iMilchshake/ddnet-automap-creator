//! The 16×16 tileset grid.
//!
//! Drawn from a single atlas texture with one uv rectangle per tile.

use egui::{Color32, Rect, Sense, Stroke, StrokeKind, TextureHandle, Vec2, pos2};

use crate::tileset::{TILESET_SIDE, TileKind, Tileset};

/// The grid stops shrinking below this cell size.
const MIN_CELL_SIZE: f32 = 8.0;

/// Below this the badge would be unreadable anyway.
const MIN_BADGE_CELL_SIZE: f32 = 16.0;

/// Backdrop that makes transparent parts of a tile visible.
const CHECKER_LIGHT: Color32 = Color32::from_gray(64);
const CHECKER_DARK: Color32 = Color32::from_gray(48);

pub struct GridResponse {
    pub hovered: Option<usize>,
    pub clicked: Option<usize>,
}

/// Draws the grid and reports what the pointer did over it.
pub fn show(ui: &mut egui::Ui, tileset: &Tileset, texture: &TextureHandle) -> GridResponse {
    let available = ui.available_size();
    let cell_size = (available.min_elem() / TILESET_SIDE as f32)
        .floor()
        .max(MIN_CELL_SIZE);
    let grid_size = cell_size * TILESET_SIDE as f32;

    let (rect, response) = ui.allocate_exact_size(Vec2::splat(grid_size), Sense::click());
    let painter = ui.painter_at(rect);

    let hovered = response
        .hover_pos()
        .and_then(|pos| tile_index_at(rect, cell_size, pos));

    for (index, tile) in tileset.tiles.iter().enumerate() {
        let cell = cell_rect(rect, cell_size, index);

        match tile.kind {
            TileKind::Locked => {
                painter.rect_filled(cell, 0.0, ui.visuals().extreme_bg_color);
            }
            TileKind::Guess(_) => {
                paint_checkerboard(&painter, cell, index);
                let uv = Rect::from_min_max(tile.uv_min.into(), tile.uv_max.into());
                painter.image(texture.id(), cell, uv, Color32::WHITE);
                paint_guess_badge(&painter, cell, cell_size, ui.visuals().warn_fg_color);
            }
        }
    }

    if let Some(index) = hovered {
        painter.rect_stroke(
            cell_rect(rect, cell_size, index),
            0.0,
            Stroke::new(2.0, ui.visuals().selection.stroke.color),
            StrokeKind::Inside,
        );
    }

    let clicked = hovered.filter(|_| response.clicked());
    GridResponse { hovered, clicked }
}

fn cell_rect(grid: Rect, cell_size: f32, index: usize) -> Rect {
    let column = (index % TILESET_SIDE as usize) as f32;
    let row = (index / TILESET_SIDE as usize) as f32;
    let min = grid.min + Vec2::new(column * cell_size, row * cell_size);

    Rect::from_min_size(min, Vec2::splat(cell_size))
}

fn tile_index_at(grid: Rect, cell_size: f32, pos: egui::Pos2) -> Option<usize> {
    let local = pos - grid.min;
    let column = (local.x / cell_size).floor() as i32;
    let row = (local.y / cell_size).floor() as i32;

    let side = TILESET_SIDE as i32;
    if !(0..side).contains(&column) || !(0..side).contains(&row) {
        return None;
    }

    Some((row * side + column) as usize)
}

fn paint_checkerboard(painter: &egui::Painter, cell: Rect, index: usize) {
    let column = index % TILESET_SIDE as usize;
    let row = index / TILESET_SIDE as usize;
    let color = if (column + row).is_multiple_of(2) {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    };

    painter.rect_filled(cell, 0.0, color);
}

/// Marks a tile whose neighborhood was guessed but not confirmed.
fn paint_guess_badge(painter: &egui::Painter, cell: Rect, cell_size: f32, color: Color32) {
    if cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }

    let inset = cell_size * 0.12;
    painter.text(
        pos2(cell.right() - inset, cell.top() + inset),
        egui::Align2::RIGHT_TOP,
        "?",
        egui::FontId::proportional(cell_size * 0.35),
        color,
    );
}
