use std::sync::Arc;

use egui::{FontSelection, Frame, Galley, Label, Margin, RichText, TextWrapMode, Ui, WidgetText};

const BLOCK_MARGIN: Margin = Margin::symmetric(6, 2);
const CHIP_MARGIN: Margin = Margin::symmetric(4, 1);
const CORNER_RADIUS: u8 = 4;
const BG_BLEND: f32 = 0.4;

pub fn code_block(ui: &mut Ui, text: &str) {
    show_code(ui, BLOCK_MARGIN, Label::new(code_text(text)));
}

pub fn code_chip(ui: &mut Ui, text: &str) {
    let width = code_galley(ui, text).size().x + CHIP_MARGIN.sum().x;
    if width > ui.available_size_before_wrap().x {
        ui.end_row();
    }
    show_code(ui, CHIP_MARGIN, Label::new(code_text(text)).extend());
}

fn show_code(ui: &mut Ui, margin: Margin, label: Label) {
    code_frame(ui, margin).show(ui, |ui| {
        ui.add(label);
    });
}

fn code_text(text: &str) -> RichText {
    RichText::new(text).monospace()
}

fn code_galley(ui: &Ui, text: &str) -> Arc<Galley> {
    WidgetText::from(code_text(text)).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        FontSelection::Default,
    )
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
