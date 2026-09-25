use std::collections::BTreeMap;

use egui::ecolor::Hsva;
use egui::emath::GuiRounding as _;
use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle, Vec2, pos2};

use crate::model::group::TileGroup;
use crate::model::project::Project;
use crate::model::tile::{Chance, TILESET_SIDE};
use crate::tileset::{TileSlice, Tileset};
use crate::ui::group_panel::mode_label;
use crate::ui::tile_state::{TileState, tile_state};

const MIN_CELL_SIZE: f32 = 8.0;

const MIN_BADGE_CELL_SIZE: f32 = 16.0;

const CHECKER_LIGHT: Color32 = Color32::from_gray(64);
const CHECKER_DARK: Color32 = Color32::from_gray(48);

const REMOVED_COLOR: Color32 = Color32::from_rgb(220, 110, 110);
const CONFIGURED_OUTLINE: Color32 = Color32::from_gray(205);
const SELECTED_OUTLINE: Color32 = Color32::from_rgb(255, 214, 102);
const POOL_MATE_OUTLINE: Color32 = Color32::from_rgb(102, 204, 255);

const UNUSED_SCRIM: u8 = 150;
const REMOVED_SCRIM: u8 = 215;

const OUTLINE_WIDTH: f32 = 2.0;
const SELECTED_OUTLINE_WIDTH: f32 = 3.0;

/// Hues a golden angle apart, so neighbouring groups never share a shade.
const HUE_STEP: f32 = 137.5 / 360.0;
const GROUP_FILL_ALPHA: f32 = 0.35;
const DRAG_FILL_ALPHA: f32 = 0.25;

const GROUP_NAME_INSET: f32 = 0.1;
const GROUP_NAME_FONT: f32 = 0.28;
const BADGE_INSET: f32 = 0.12;
const BADGE_MARK: f32 = 0.3;
const CHANCE_FONT: f32 = 0.26;
const CROSS_WIDTH: f32 = 0.18;
const MIN_CROSS_WIDTH: f32 = 1.5;

const PERCENT_ROUNDING: f32 = 10.0;

pub struct GridView<'a> {
    pub tileset: &'a Tileset,
    pub project: &'a Project,
    pub shares: &'a BTreeMap<usize, f32>,
    pub texture: &'a TextureHandle,
    pub group_editing: bool,
    pub drag_anchor: Option<usize>,
    pub selected: Option<usize>,
    pub pool_mates: &'a [usize],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GridResponse {
    pub hovered: Option<usize>,
    pub clicked: Option<usize>,
    pub secondary_clicked: Option<usize>,
    pub drag_started: Option<usize>,
    pub drag_released: Option<usize>,
}

/// One decimal at most, and none for whole numbers.
pub fn format_percent(percent: f32) -> String {
    let rounded = (percent * PERCENT_ROUNDING).round() / PERCENT_ROUNDING;

    format!("{rounded}%")
}

pub fn group_color(index: usize) -> Color32 {
    Hsva::new((index as f32 * HUE_STEP).fract(), 0.6, 0.95, 1.0).into()
}

pub fn show(ui: &mut egui::Ui, view: GridView<'_>) -> GridResponse {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let cell_size = whole_pixel_cell_size(ui.available_size(), pixels_per_point);
    let grid_size = cell_size * TILESET_SIDE as f32;

    let sense = match view.group_editing {
        true => Sense::click_and_drag(),
        false => Sense::click(),
    };
    let (allocated, response) = ui.allocate_exact_size(Vec2::splat(grid_size), sense);
    let rect = Rect::from_min_size(
        allocated.min.round_to_pixels(pixels_per_point),
        allocated.size(),
    );
    let painter = ui.painter_at(rect);

    let hovered = response
        .hover_pos()
        .and_then(|pos| tile_index_at(rect, cell_size, pos));
    let pointed = response
        .interact_pointer_pos()
        .map(|pos| tile_index_clamped(rect, cell_size, pos));

    for (index, tile) in view.tileset.tiles.iter().enumerate() {
        let cell = cell_rect(rect, cell_size, index);
        let state = tile_state(view.tileset, view.project, index);

        paint_checkerboard(&painter, cell, index);

        if !matches!(state, TileState::Locked) {
            paint_tile(&painter, cell, view.texture, tile);
        }

        if let Some(scrim) = tile_scrim(state) {
            painter.rect_filled(cell, 0.0, scrim);
        }

        if matches!(state, TileState::Configured(_)) {
            paint_configured_outline(&painter, &view, cell, index, pixels_per_point);
        }

        let share = view.shares.get(&index).copied();
        paint_badge(&painter, cell, cell_size, state, share, ui.visuals());
    }

    for (index, group) in view.project.groups().iter().enumerate() {
        paint_group(&painter, rect, cell_size, group, group_color(index));
    }

    if view.group_editing
        && let (Some(anchor), Some(corner)) = (view.drag_anchor, pointed.or(hovered))
    {
        paint_selection(
            &painter,
            rect,
            cell_size,
            anchor,
            corner,
            ui.visuals().selection.stroke.color,
        );
    }

    let outliner = BorderOutliner {
        painter: ui.painter_at(rect.expand(SELECTED_OUTLINE_WIDTH)),
        grid: rect,
        cell_size,
        pixels_per_point,
    };
    for &index in view.pool_mates {
        outliner.paint_centred_on_border(index, SELECTED_OUTLINE_WIDTH, POOL_MATE_OUTLINE);
    }
    if let Some(index) = view.selected {
        outliner.paint_centred_on_border(index, SELECTED_OUTLINE_WIDTH, SELECTED_OUTLINE);
    }
    if let Some(index) = hovered {
        outliner.paint_centred_on_border(index, OUTLINE_WIDTH, ui.visuals().selection.stroke.color);
    }

    let response = match hovered.and_then(|index| describe_group(view.project, index)) {
        Some(text) => response.on_hover_text(text),
        None => response,
    };

    GridResponse {
        hovered,
        clicked: hovered.filter(|_| response.clicked()),
        secondary_clicked: hovered.filter(|_| response.secondary_clicked()),
        drag_started: pointed.filter(|_| response.drag_started()),
        drag_released: pointed.filter(|_| response.drag_stopped()),
    }
}

fn describe_group(project: &Project, tile: usize) -> Option<String> {
    let group = project.groups().get(project.group_at(tile)?)?;
    let mut text = format!(
        "{} — {}×{}, {}",
        group.name,
        group.width,
        group.height,
        mode_label(group.mode)
    );
    if !group.chance.is_full() {
        text.push_str(&format!(", {}%", group.chance.percent()));
    }

    Some(text)
}

fn paint_group(
    painter: &egui::Painter,
    grid: Rect,
    cell_size: f32,
    group: &TileGroup,
    color: Color32,
) {
    let area = tile_span(grid, cell_size, group.top_left, group.bottom_right());
    painter.rect_filled(area, 0.0, color.gamma_multiply(GROUP_FILL_ALPHA));
    painter.rect_stroke(
        area,
        0.0,
        Stroke::new(OUTLINE_WIDTH, color),
        StrokeKind::Inside,
    );

    let anchor = cell_rect(grid, cell_size, group.top_left);
    painter.text(
        anchor.left_top() + Vec2::splat(cell_size * GROUP_NAME_INSET),
        egui::Align2::LEFT_TOP,
        &group.name,
        egui::FontId::proportional(cell_size * GROUP_NAME_FONT),
        Color32::WHITE,
    );

    if group.chance.is_full() || cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }

    let inset = Vec2::new(cell_size * BADGE_INSET, -cell_size * BADGE_INSET);
    painter.text(
        area.left_bottom() + inset,
        egui::Align2::LEFT_BOTTOM,
        format!("{}%", group.chance.percent()),
        egui::FontId::proportional(cell_size * CHANCE_FONT),
        Color32::WHITE,
    );
}

fn paint_selection(
    painter: &egui::Painter,
    grid: Rect,
    cell_size: f32,
    anchor: usize,
    corner: usize,
    color: Color32,
) {
    let (top_left, bottom_right) = corners(anchor, corner);
    let area = tile_span(grid, cell_size, top_left, bottom_right);

    painter.rect_filled(area, 0.0, color.gamma_multiply(DRAG_FILL_ALPHA));
    painter.rect_stroke(
        area,
        0.0,
        Stroke::new(OUTLINE_WIDTH, color),
        StrokeKind::Inside,
    );
}

/// The smallest rectangle holding both tiles, as top-left and bottom-right ids.
pub fn corners(first: usize, second: usize) -> (usize, usize) {
    let columns = [first % TILESET_SIDE, second % TILESET_SIDE];
    let rows = [first / TILESET_SIDE, second / TILESET_SIDE];

    let top_left = rows.iter().min().unwrap() * TILESET_SIDE + columns.iter().min().unwrap();
    let bottom_right = rows.iter().max().unwrap() * TILESET_SIDE + columns.iter().max().unwrap();

    (top_left, bottom_right)
}

fn tile_span(grid: Rect, cell_size: f32, top_left: usize, bottom_right: usize) -> Rect {
    cell_rect(grid, cell_size, top_left).union(cell_rect(grid, cell_size, bottom_right))
}

struct BorderOutliner {
    painter: egui::Painter,
    grid: Rect,
    cell_size: f32,
    pixels_per_point: f32,
}

impl BorderOutliner {
    fn paint_centred_on_border(&self, index: usize, width: f32, color: Color32) {
        let half = (width / 2.0 * self.pixels_per_point).round().max(1.0) / self.pixels_per_point;
        let cell = cell_rect(self.grid, self.cell_size, index);
        let outer = cell.expand(half);

        for edge in [
            Rect::from_min_max(outer.min, pos2(outer.max.x, cell.min.y + half)),
            Rect::from_min_max(pos2(outer.min.x, cell.max.y - half), outer.max),
            Rect::from_min_max(outer.min, pos2(cell.min.x + half, outer.max.y)),
            Rect::from_min_max(pos2(cell.max.x - half, outer.min.y), outer.max),
        ] {
            self.painter.rect_filled(edge, 0.0, color);
        }
    }
}

fn is_configured(view: &GridView<'_>, index: usize) -> bool {
    matches!(
        tile_state(view.tileset, view.project, index),
        TileState::Configured(_)
    )
}

fn paint_configured_outline(
    painter: &egui::Painter,
    view: &GridView<'_>,
    cell: Rect,
    index: usize,
    pixels_per_point: f32,
) {
    let pixel = 1.0 / pixels_per_point;
    let column = index % TILESET_SIDE;
    let row = index / TILESET_SIDE;
    let right_is_configured = column + 1 < TILESET_SIDE && is_configured(view, index + 1);
    let below_is_configured = row + 1 < TILESET_SIDE && is_configured(view, index + TILESET_SIDE);

    let mut edges = vec![
        Rect::from_min_max(cell.min, pos2(cell.max.x, cell.min.y + pixel)),
        Rect::from_min_max(cell.min, pos2(cell.min.x + pixel, cell.max.y)),
    ];
    if !right_is_configured {
        edges.push(Rect::from_min_max(
            pos2(cell.max.x - pixel, cell.min.y),
            cell.max,
        ));
    }
    if !below_is_configured {
        edges.push(Rect::from_min_max(
            pos2(cell.min.x, cell.max.y - pixel),
            cell.max,
        ));
    }

    for edge in edges {
        painter.rect_filled(edge, 0.0, CONFIGURED_OUTLINE);
    }
}

fn cell_rect(grid: Rect, cell_size: f32, index: usize) -> Rect {
    let column = (index % TILESET_SIDE) as f32;
    let row = (index / TILESET_SIDE) as f32;
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

fn tile_index_clamped(grid: Rect, cell_size: f32, pos: Pos2) -> usize {
    let local = pos - grid.min;
    let last = TILESET_SIDE as f32 - 1.0;
    let column = (local.x / cell_size).floor().clamp(0.0, last) as usize;
    let row = (local.y / cell_size).floor().clamp(0.0, last) as usize;

    row * TILESET_SIDE + column
}

fn whole_pixel_cell_size(available: Vec2, pixels_per_point: f32) -> f32 {
    let points = (available.min_elem() / TILESET_SIDE as f32).max(MIN_CELL_SIZE);

    (points * pixels_per_point).floor() / pixels_per_point
}

pub fn paint_tile(painter: &egui::Painter, rect: Rect, texture: &TextureHandle, slice: &TileSlice) {
    let uv = Rect::from_min_max(slice.uv_min.into(), slice.uv_max.into());
    let mut mesh = egui::Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);

    painter.add(egui::Shape::mesh(mesh));
}

fn paint_checkerboard(painter: &egui::Painter, cell: Rect, index: usize) {
    let column = index % TILESET_SIDE;
    let row = index / TILESET_SIDE;
    let color = if (column + row).is_multiple_of(2) {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    };

    painter.rect_filled(cell, 0.0, color);
}

fn tile_scrim(state: TileState) -> Option<Color32> {
    match state {
        TileState::Configured(_) | TileState::Grouped => None,
        TileState::Locked | TileState::Guessed(_) => Some(Color32::from_black_alpha(UNUSED_SCRIM)),
        TileState::Removed => Some(Color32::from_black_alpha(REMOVED_SCRIM)),
    }
}

/// Shapes rather than glyphs, so badges do not depend on font coverage.
fn paint_badge(
    painter: &egui::Painter,
    cell: Rect,
    cell_size: f32,
    state: TileState,
    share: Option<f32>,
    visuals: &egui::Visuals,
) {
    if cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }

    let inset = cell_size * BADGE_INSET;
    let mark_size = cell_size * BADGE_MARK;
    let mark = Rect::from_min_size(
        pos2(cell.right() - inset - mark_size, cell.top() + inset),
        Vec2::splat(mark_size),
    );

    match state {
        TileState::Locked | TileState::Grouped | TileState::Guessed(_) => {}
        TileState::Removed => paint_cross(painter, mark),
        TileState::Configured(_) => {
            if let Some(share) = share
                && share < Chance::FULL.percent()
            {
                painter.text(
                    pos2(cell.left() + inset, cell.bottom() - inset),
                    egui::Align2::LEFT_BOTTOM,
                    format_percent(share),
                    egui::FontId::proportional(cell_size * CHANCE_FONT),
                    visuals.strong_text_color(),
                );
            }
        }
    }
}

fn paint_cross(painter: &egui::Painter, mark: Rect) {
    let width = (mark.width() * CROSS_WIDTH).max(MIN_CROSS_WIDTH);
    let stroke = Stroke::new(width, REMOVED_COLOR);

    painter.line_segment([mark.left_top(), mark.right_bottom()], stroke);
    painter.line_segment([mark.right_top(), mark.left_bottom()], stroke);
}
