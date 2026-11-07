use egui::{Context, Frame, Margin, Ui};

const SPACING: i8 = 12;
const SECTION_SPACING: f32 = SPACING as f32;
const MODAL_MARGIN: Margin = Margin::same(SPACING);

pub fn modal_frame(ctx: &Context) -> Frame {
    Frame::popup(&ctx.style_of(ctx.theme())).inner_margin(MODAL_MARGIN)
}

pub fn show_section(ui: &mut Ui, heading: &str, add_body: impl FnOnce(&mut Ui)) {
    ui.add_space(SECTION_SPACING);
    ui.strong(heading);
    add_body(ui);
}

pub fn paragraph_gap(ui: &mut Ui) {
    ui.add_space(SECTION_SPACING);
}
