use std::f32::consts::PI;

use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::model::transform::Transform;
use crate::ui::BUTTON_GAP;
use crate::ui::neighbor_paint::CORNER_RADIUS;

pub(super) const TURN_BUTTON_SIZE: f32 = 20.0;
const ICON_INSET_FRACTION: f32 = 0.22;
const ICON_STROKE_FRACTION: f32 = 0.12;
const MIN_ICON_STROKE: f32 = 1.0;
const ARC_SEGMENTS: usize = 12;
const ARC_START: f32 = -PI;
const ARC_SWEEP: f32 = 1.5 * PI;
const ARROW_HEAD_FRACTION: f32 = 0.35;
const ARROW_HEAD_WIDTH: f32 = 0.7;
const FLIP_GAP_FRACTION: f32 = 0.25;
const FLIP_BASE_FRACTION: f32 = 0.7;
const RESIZE_HEAD_FRACTION: f32 = 0.4;

const ROTATE_NEIGHBORS_TOOLTIP: &str = "Rotate neighbors 90° clockwise";
const FLIP_NEIGHBORS_HORIZONTALLY_TOOLTIP: &str = "Flip neighbors horizontally";
const FLIP_NEIGHBORS_VERTICALLY_TOOLTIP: &str = "Flip neighbors vertically";
const ROTATE_TILE_TOOLTIP: &str = "Rotate tile 90° clockwise";
const FLIP_TILE_HORIZONTALLY_TOOLTIP: &str = "Flip tile horizontally";
const FLIP_TILE_VERTICALLY_TOOLTIP: &str = "Flip tile vertically";
const RESET_GLYPH: &str = "↺";

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Rotate,
    FlipHorizontally,
    FlipVertically,
    Reset,
    Add,
    Remove,
    Resize,
}

pub(super) struct TurnButton {
    pub icon: Icon,
    pub neighbors_tooltip: &'static str,
    pub tile_tooltip: &'static str,
    pub turn: Transform,
}

pub(super) const TURN_BUTTONS: [TurnButton; 3] = [
    TurnButton {
        icon: Icon::Rotate,
        neighbors_tooltip: ROTATE_NEIGHBORS_TOOLTIP,
        tile_tooltip: ROTATE_TILE_TOOLTIP,
        turn: Transform::ROTATE_CLOCKWISE,
    },
    TurnButton {
        icon: Icon::FlipHorizontally,
        neighbors_tooltip: FLIP_NEIGHBORS_HORIZONTALLY_TOOLTIP,
        tile_tooltip: FLIP_TILE_HORIZONTALLY_TOOLTIP,
        turn: Transform::FLIP_HORIZONTALLY,
    },
    TurnButton {
        icon: Icon::FlipVertically,
        neighbors_tooltip: FLIP_NEIGHBORS_VERTICALLY_TOOLTIP,
        tile_tooltip: FLIP_TILE_VERTICALLY_TOOLTIP,
        turn: Transform::FLIP_VERTICALLY,
    },
];

pub(super) fn turn_buttons_width() -> f32 {
    let count = TURN_BUTTONS.len() as f32;
    count * TURN_BUTTON_SIZE + (count - 1.0) * BUTTON_GAP
}

pub(super) fn button_rect(bounds: Rect, align: Align2, index: usize) -> Rect {
    let row = Vec2::new(turn_buttons_width(), TURN_BUTTON_SIZE);
    let area = align.align_size_within_rect(row, bounds);

    Rect::from_min_size(
        area.min + Vec2::X * index as f32 * (TURN_BUTTON_SIZE + BUTTON_GAP),
        Vec2::splat(TURN_BUTTON_SIZE),
    )
}

pub(super) fn turn_button(ui: &Ui, rect: Rect, icon: Icon, tooltip: &str) -> bool {
    icon_button(ui, rect, icon, tooltip, false)
}

pub(super) fn icon_button(ui: &Ui, rect: Rect, icon: Icon, tooltip: &str, active: bool) -> bool {
    let response = ui.interact(rect, ui.id().with(tooltip), Sense::click());
    let visuals = ui.style().interact(&response);
    let fill = match active {
        true => ui.visuals().selection.bg_fill,
        false => visuals.weak_bg_fill,
    };
    ui.painter().rect(
        rect,
        CORNER_RADIUS,
        fill,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );

    let icon_area = rect.shrink(rect.width() * ICON_INSET_FRACTION);
    paint_stroked_icon(ui.painter(), icon_area, visuals.fg_stroke.color, icon);

    let clicked = response.clicked();
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip);

    clicked
}

pub(super) fn paint_stroked_icon(painter: &Painter, rect: Rect, color: Color32, icon: Icon) {
    let width = (rect.width() * ICON_STROKE_FRACTION).max(MIN_ICON_STROKE);
    paint_icon(painter, rect, Stroke::new(width, color), icon);
}

fn paint_icon(painter: &Painter, rect: Rect, stroke: Stroke, icon: Icon) {
    match icon {
        Icon::Rotate => paint_curl(painter, rect, stroke),
        Icon::FlipHorizontally => paint_flip(painter, rect, stroke, true),
        Icon::FlipVertically => paint_flip(painter, rect, stroke, false),
        Icon::Remove => paint_minus(painter, rect, stroke),
        Icon::Add => {
            paint_minus(painter, rect, stroke);
            painter.line_segment([rect.center_top(), rect.center_bottom()], stroke);
        }
        Icon::Resize => paint_resize(painter, rect, stroke),
        Icon::Reset => {
            let font = FontId::proportional(rect.height());
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                RESET_GLYPH,
                font,
                stroke.color,
            );
        }
    }
}

fn paint_curl(painter: &Painter, rect: Rect, stroke: Stroke) {
    let radius = rect.width() / 2.0;
    let point = |angle: f32| rect.center() + radius * Vec2::new(angle.cos(), angle.sin());
    let arc: Vec<Pos2> = (0..=ARC_SEGMENTS)
        .map(|step| point(ARC_START + ARC_SWEEP * step as f32 / ARC_SEGMENTS as f32))
        .collect();

    let end = arc[arc.len() - 1];
    let before_end = arc[arc.len() - 2];
    let tangent = (end - before_end).normalized();
    let head = rect.width() * ARROW_HEAD_FRACTION;
    let wing = tangent.rot90() * head * ARROW_HEAD_WIDTH;
    let arrow = vec![end + tangent * head, end + wing, end - wing];

    painter.add(Shape::line(arc, stroke));
    painter.add(Shape::convex_polygon(arrow, stroke.color, Stroke::NONE));
}

fn paint_minus(painter: &Painter, rect: Rect, stroke: Stroke) {
    painter.line_segment([rect.left_center(), rect.right_center()], stroke);
}

fn paint_resize(painter: &Painter, rect: Rect, stroke: Stroke) {
    let corner = rect.right_bottom();
    let head = rect.width() * RESIZE_HEAD_FRACTION;
    let wedge = vec![
        corner,
        corner - Vec2::new(head, 0.0),
        corner - Vec2::new(0.0, head),
    ];

    painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
    painter.line_segment([rect.center(), corner], stroke);
    painter.add(Shape::convex_polygon(wedge, stroke.color, Stroke::NONE));
}

fn paint_flip(painter: &Painter, rect: Rect, stroke: Stroke, horizontal: bool) {
    let half = rect.width() / 2.0;
    let place = |along: f32, across: f32| {
        rect.center()
            + match horizontal {
                true => Vec2::new(along, across),
                false => Vec2::new(across, along),
            }
    };

    painter.line_segment([place(0.0, -half), place(0.0, half)], stroke);
    for side in [-1.0, 1.0] {
        let base = half * FLIP_BASE_FRACTION;
        let gap = side * half * FLIP_GAP_FRACTION;
        let wedge = vec![place(gap, -base), place(gap, base), place(side * half, 0.0)];
        painter.add(Shape::convex_polygon(wedge, stroke.color, Stroke::NONE));
    }
}
