use egui::{Color32, Painter, Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::model::neighbor::{CENTER, Extent, NeighborState, Offset};
use crate::model::tile::RuleCenter;
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
const EMPTY_CENTER_ALPHA: u8 = 150;
const ANY_CENTER_ALPHA: u8 = 140;
const ANY_CENTER_TINT: Color32 = Color32::from_rgba_premultiplied(
    ANY_CENTER_ALPHA,
    ANY_CENTER_ALPHA,
    ANY_CENTER_ALPHA,
    ANY_CENTER_ALPHA,
);
const ANY_BORDER_WIDTH: f32 = 2.0;
const ANY_BORDER_COLOR: Color32 = Color32::from_gray(220);
const DASH_LENGTH: f32 = 5.0;
const DASH_GAP: f32 = 3.0;
const EMPTY_CENTER_TINT: Color32 = Color32::from_rgba_premultiplied(
    EMPTY_CENTER_ALPHA,
    EMPTY_CENTER_ALPHA,
    EMPTY_CENTER_ALPHA,
    EMPTY_CENTER_ALPHA,
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

pub fn paint_center(
    ui: &Ui,
    rect: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    center: RuleCenter,
    drawn: Transform,
) {
    match center {
        RuleCenter::Full => paint_turned_tile(ui.painter(), rect, texture, slice, drawn),
        RuleCenter::Any => {
            let painter = ui.painter();
            paint_checker(painter, rect, PREVIEW_LIGHT, PREVIEW_DARK);
            preview::paint_tinted(painter, rect, texture, slice, drawn, ANY_CENTER_TINT);
            paint_dashed_border(painter, rect);
        }
        RuleCenter::Empty => {
            paint_state(ui, rect, NeighborState::Empty);
            preview::paint_tinted(ui.painter(), rect, texture, slice, drawn, EMPTY_CENTER_TINT);
        }
    }
}

pub fn show_pattern(
    ui: &mut Ui,
    extent: Extent,
    cell_size: f32,
    paint_cell: impl Fn(&Ui, Rect, Option<Offset>),
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(PATTERN_CELL_GAP);
        for dy in -extent.up..=extent.down {
            ui.horizontal(|ui| {
                for dx in -extent.left..=extent.right {
                    let (rect, _response) =
                        ui.allocate_exact_size(Vec2::splat(cell_size), Sense::hover());
                    paint_cell(ui, rect, ((dx, dy) != CENTER).then_some((dx, dy)));
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

fn paint_dashed_border(painter: &Painter, rect: Rect) {
    let outline = rect.shrink(ANY_BORDER_WIDTH / 2.0);
    let corners = [
        outline.left_top(),
        outline.right_top(),
        outline.right_bottom(),
        outline.left_bottom(),
        outline.left_top(),
    ];
    let stroke = Stroke::new(ANY_BORDER_WIDTH, ANY_BORDER_COLOR);
    painter.extend(Shape::dashed_line(&corners, stroke, DASH_LENGTH, DASH_GAP));
}

fn paint_checker(painter: &Painter, rect: Rect, light: Color32, dark: Color32) {
    painter.rect_filled(rect, CORNER_RADIUS, dark);

    let half = rect.size() / 2.0;
    painter.rect_filled(Rect::from_min_size(rect.min, half), 0.0, light);
    painter.rect_filled(Rect::from_min_size(rect.center(), half), 0.0, light);
}
