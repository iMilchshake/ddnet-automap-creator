use egui::{
    Align2, Area, Context, Frame, Id, LayerId, Order, Rect, RichText, Stroke, StrokeKind, Vec2,
};

const PICKING_ORDER: Order = Order::Middle;

const GLOW_WIDTH: f32 = 12.0;
const GLOW_STEP: f32 = 1.0;
const GLOW_OPACITY: f32 = 0.5;
const BANNER_MARGIN: f32 = 16.0;

const BANNER_HINT: &str = "Click a tile in the tileset. Press Esc or right-click to cancel.";
const BANNER_CANCEL: &str = "Cancel";
const NEIGHBOR_HEADING: &str = "Pick the {label} neighbor";
const LABEL_PLACEHOLDER: &str = "{label}";

pub fn neighbor_heading(label: &str) -> String {
    NEIGHBOR_HEADING.replace(LABEL_PLACEHOLDER, label)
}

pub fn show(ctx: &Context, view: Rect, heading: &str) -> bool {
    paint_glow(ctx, view);
    show_banner(ctx, view, heading)
}

fn paint_glow(ctx: &Context, view: Rect) {
    let painter = ctx
        .layer_painter(LayerId::new(PICKING_ORDER, Id::new("picking_glow")))
        .with_clip_rect(view);

    let color = ctx.global_style().visuals.selection.bg_fill;
    for layer in 0..(GLOW_WIDTH / GLOW_STEP) as u32 {
        let inset = layer as f32 * GLOW_STEP;
        let fade = 1.0 - inset / GLOW_WIDTH;
        let stroke = Stroke::new(GLOW_STEP, color.gamma_multiply(GLOW_OPACITY * fade * fade));
        painter.rect_stroke(view.shrink(inset), 0.0, stroke, StrokeKind::Inside);
    }
}

fn show_banner(ctx: &Context, view: Rect, heading: &str) -> bool {
    Area::new(Id::new("picking_banner"))
        .order(PICKING_ORDER)
        .fixed_pos(view.center_top() + Vec2::new(0.0, BANNER_MARGIN))
        .pivot(Align2::CENTER_TOP)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .show(ui, |ui| {
                    ui.label(RichText::new(heading).strong());
                    ui.weak(BANNER_HINT);
                    ui.button(BANNER_CANCEL).clicked()
                })
                .inner
        })
        .inner
}
