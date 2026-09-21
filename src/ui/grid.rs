use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle, Vec2, pos2};

use crate::model::project::Project;
use crate::tileset::{TILESET_SIDE, Tileset};
use crate::ui::tile_state::{TileState, tile_state};

const MIN_CELL_SIZE: f32 = 8.0;

const MIN_BADGE_CELL_SIZE: f32 = 16.0;

const CHECKER_LIGHT: Color32 = Color32::from_gray(64);
const CHECKER_DARK: Color32 = Color32::from_gray(48);

const CONFIGURED_COLOR: Color32 = Color32::from_rgb(120, 200, 120);
const REMOVED_COLOR: Color32 = Color32::from_rgb(220, 110, 110);
const REMOVED_TINT: Color32 = Color32::from_gray(85);

pub struct GridView<'a> {
    pub tileset: &'a Tileset,
    pub project: &'a Project,
    pub texture: &'a TextureHandle,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GridResponse {
    pub hovered: Option<usize>,
    pub clicked: Option<usize>,
    pub secondary_clicked: Option<usize>,
}

pub fn show(ui: &mut egui::Ui, view: GridView<'_>) -> GridResponse {
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

    for (index, tile) in view.tileset.tiles.iter().enumerate() {
        let cell = cell_rect(rect, cell_size, index);
        let state = tile_state(view.tileset, view.project, index);

        if matches!(state, TileState::Locked) {
            painter.rect_filled(cell, 0.0, ui.visuals().extreme_bg_color);
            continue;
        }

        let tint = match state {
            TileState::Removed => REMOVED_TINT,
            _ => Color32::WHITE,
        };
        paint_checkerboard(&painter, cell, index);
        let uv = Rect::from_min_max(tile.uv_min.into(), tile.uv_max.into());
        painter.image(view.texture.id(), cell, uv, tint);

        paint_badge(&painter, cell, cell_size, state, ui.visuals());
    }

    if let Some(index) = hovered {
        painter.rect_stroke(
            cell_rect(rect, cell_size, index),
            0.0,
            Stroke::new(2.0, ui.visuals().selection.stroke.color),
            StrokeKind::Inside,
        );
    }

    GridResponse {
        hovered,
        clicked: hovered.filter(|_| response.clicked()),
        secondary_clicked: hovered.filter(|_| response.secondary_clicked()),
    }
}

fn cell_rect(grid: Rect, cell_size: f32, index: usize) -> Rect {
    let column = (index % TILESET_SIDE as usize) as f32;
    let row = (index / TILESET_SIDE as usize) as f32;
    let min = grid.min + Vec2::new(column * cell_size, row * cell_size);

    Rect::from_min_size(min, Vec2::splat(cell_size))
}

fn tile_index_at(grid: Rect, cell_size: f32, pos: Pos2) -> Option<usize> {
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

/// Shapes rather than glyphs, so badges do not depend on font coverage.
fn paint_badge(
    painter: &egui::Painter,
    cell: Rect,
    cell_size: f32,
    state: TileState,
    visuals: &egui::Visuals,
) {
    if cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }

    let inset = cell_size * 0.12;
    let mark = Rect::from_min_size(
        pos2(cell.right() - inset - cell_size * 0.3, cell.top() + inset),
        Vec2::splat(cell_size * 0.3),
    );

    match state {
        TileState::Locked => {}
        TileState::Guessed(_) => {
            painter.text(
                mark.right_top(),
                egui::Align2::RIGHT_TOP,
                "?",
                egui::FontId::proportional(cell_size * 0.35),
                visuals.warn_fg_color,
            );
        }
        TileState::Removed => paint_cross(painter, mark),
        TileState::Configured(rule) => {
            paint_check(painter, mark);
            if !rule.chance.is_full() {
                painter.text(
                    pos2(cell.left() + inset, cell.bottom() - inset),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{}%", rule.chance.percent()),
                    egui::FontId::proportional(cell_size * 0.26),
                    visuals.strong_text_color(),
                );
            }
        }
    }
}

fn paint_check(painter: &egui::Painter, mark: Rect) {
    let stroke = Stroke::new((mark.width() * 0.18).max(1.5), CONFIGURED_COLOR);
    let left = pos2(mark.left(), mark.center().y);
    let bottom = pos2(mark.center().x - mark.width() * 0.1, mark.bottom());

    painter.line_segment([left, bottom], stroke);
    painter.line_segment([bottom, mark.right_top()], stroke);
}

fn paint_cross(painter: &egui::Painter, mark: Rect) {
    let stroke = Stroke::new((mark.width() * 0.18).max(1.5), REMOVED_COLOR);

    painter.line_segment([mark.left_top(), mark.right_bottom()], stroke);
    painter.line_segment([mark.right_top(), mark.left_bottom()], stroke);
}
