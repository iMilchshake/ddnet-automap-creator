use egui::{Frame, Margin, RichText, Ui};

const BLOCK_MARGIN: Margin = Margin::symmetric(6, 2);
const CHIP_MARGIN: Margin = Margin::symmetric(4, 1);
const CORNER_RADIUS: u8 = 4;
const BG_BLEND: f32 = 0.4;

pub fn code_block(ui: &mut Ui, text: &str) {
    show_code(ui, BLOCK_MARGIN, text);
}

pub fn code_chip(ui: &mut Ui, text: &str) {
    show_code(ui, CHIP_MARGIN, text);
}

fn show_code(ui: &mut Ui, margin: Margin, text: &str) {
    code_frame(ui, margin).show(ui, |ui| {
        ui.label(RichText::new(text).monospace());
    });
}

fn code_frame(ui: &Ui, margin: Margin) -> Frame {
    let visuals = ui.visuals();
    Frame::new()
        .fill(
            visuals
                .window_fill
                .lerp_to_gamma(visuals.extreme_bg_color, BG_BLEND),
        )
        .corner_radius(CORNER_RADIUS)
        .inner_margin(margin)
}
