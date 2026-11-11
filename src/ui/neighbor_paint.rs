use egui::{Color32, Painter, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::model::neighbor::{CENTRE, NeighborState, Offset, Reach};
use crate::model::tile::Centre;
use crate::model::transform::Transform;
use crate::tileset::{TileSlice, Tileset};
use crate::ui::{grid, preview};

pub const CORNER_RADIUS: f32 = 2.0;
const SWATCH_SIZE: f32 = 14.0;
const LEGEND_SPACING: f32 = 4.0;
const LEGEND_GAP: f32 = 4.0;
const PATTERN_CELL_GAP: f32 = 2.0;
const EMPTY_CROSS_INSET_FRACTION: f32 = 0.28;
const MARK_WIDTH_FRACTION: f32 = 0.05;
const MIN_MARK_WIDTH: f32 = 1.0;

const MARK_COLOR: Color32 = Color32::from_gray(150);
const FULL_FILL: Color32 = Color32::from_gray(185);
const FULL_OVERLAY: Color32 = Color32::from_rgba_premultiplied(100, 100, 100, 110);
const ANY_LIGHT: Color32 = Color32::from_gray(96);
const ANY_DARK: Color32 = Color32::from_gray(64);
const PREVIEW_LIGHT: Color32 = Color32::from_gray(64);
const PREVIEW_DARK: Color32 = Color32::from_gray(48);
const EMPTY_CENTRE_ALPHA: u8 = 150;
const EMPTY_CENTRE_TINT: Color32 = Color32::from_rgba_premultiplied(
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
);

const LEGEND: [(NeighborState, &str); 3] = [
    (NeighborState::Full, "Solid"),
    (NeighborState::Empty, "Empty"),
    (NeighborState::Any, "Any"),
];

pub fn paint_state(ui: &Ui, rect: Rect, state: NeighborState) {
    let painter = ui.painter();
    let default_border = ui.visuals().widgets.inactive.bg_stroke;
    let mark = Stroke::new(
        (rect.width() * MARK_WIDTH_FRACTION).max(MIN_MARK_WIDTH),
        MARK_COLOR,
    );
    let border = match state {
        NeighborState::Empty => {
            paint_cross(painter, rect, mark);
            mark
        }
        NeighborState::Full => {
            painter.rect_filled(rect, CORNER_RADIUS, FULL_FILL);
            default_border
        }
        NeighborState::Any => {
            paint_checker(painter, rect, ANY_LIGHT, ANY_DARK);
            mark
        }
        NeighborState::Is(_) => {
            paint_checker(painter, rect, PREVIEW_LIGHT, PREVIEW_DARK);
            default_border
        }
    };

    painter.rect_stroke(rect, CORNER_RADIUS, border, StrokeKind::Inside);
}

fn paint_cross(painter: &Painter, rect: Rect, mark: Stroke) {
    let cross = rect.shrink(rect.width() * EMPTY_CROSS_INSET_FRACTION);
    painter.line_segment([cross.left_top(), cross.right_bottom()], mark);
    painter.line_segment([cross.right_top(), cross.left_bottom()], mark);
}

pub fn paint_marked_tile(
    ui: &Ui,
    rect: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    state: NeighborState,
) {
    let painter = ui.painter();
    paint_turned_tile(painter, rect, texture, slice, Transform::IDENTITY);

    let mark = Stroke::new(
        (rect.width() * MARK_WIDTH_FRACTION).max(MIN_MARK_WIDTH),
        MARK_COLOR,
    );
    match state {
        NeighborState::Full => {
            painter.rect_filled(rect, CORNER_RADIUS, FULL_OVERLAY);
        }
        NeighborState::Empty => paint_cross(painter, rect, mark),
        NeighborState::Any | NeighborState::Is(_) => {}
    }
    painter.rect_stroke(rect, CORNER_RADIUS, mark, StrokeKind::Inside);
}

pub fn paint_neighbor(
    ui: &Ui,
    rect: Rect,
    state: NeighborState,
    texture: &TextureHandle,
    tileset: &Tileset,
) {
    paint_state(ui, rect, state);
    if let Some(tile) = state.tile() {
        grid::paint_tile(ui.painter(), rect, texture, &tileset.tiles[tile]);
    }
}

pub fn paint_turned_tile(
    painter: &Painter,
    rect: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    transform: Transform,
) {
    paint_checker(painter, rect, PREVIEW_LIGHT, PREVIEW_DARK);
    preview::paint_turned(painter, rect, texture, slice, transform);
}

pub fn paint_centre(
    ui: &Ui,
    rect: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    centre: Centre,
    drawn: Transform,
) {
    match centre {
        Centre::Full => paint_turned_tile(ui.painter(), rect, texture, slice, drawn),
        Centre::Empty => {
            paint_state(ui, rect, NeighborState::Empty);
            preview::paint_tinted(ui.painter(), rect, texture, slice, drawn, EMPTY_CENTRE_TINT);
        }
    }
}

pub fn show_pattern(
    ui: &mut Ui,
    reach: Reach,
    cell_size: f32,
    paint_cell: impl Fn(&Ui, Rect, Option<Offset>),
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(PATTERN_CELL_GAP);
        for dy in -reach.up..=reach.down {
            ui.horizontal(|ui| {
                for dx in -reach.left..=reach.right {
                    let (rect, _response) =
                        ui.allocate_exact_size(Vec2::splat(cell_size), Sense::hover());
                    paint_cell(ui, rect, ((dx, dy) != CENTRE).then_some((dx, dy)));
                }
            });
        }
    });
}

pub fn show_legend(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LEGEND_SPACING;

        for (state, label) in LEGEND {
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), Sense::hover());
            paint_state(ui, rect, state);
            ui.label(label);
            ui.add_space(LEGEND_GAP);
        }
    });
}

fn paint_checker(painter: &Painter, rect: Rect, light: Color32, dark: Color32) {
    painter.rect_filled(rect, CORNER_RADIUS, dark);

    let half = rect.size() / 2.0;
    painter.rect_filled(Rect::from_min_size(rect.min, half), 0.0, light);
    painter.rect_filled(Rect::from_min_size(rect.center(), half), 0.0, light);
}
