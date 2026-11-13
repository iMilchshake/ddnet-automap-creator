use egui::{Button, Context, Id, Ui};

use crate::ui::code_frame::code_chip;
use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, MODAL_BUTTON_SIZE, paragraph_gap};

const MODAL_ID: &str = "not_editable";
const WIDTH: f32 = 360.0;

const HEADING: &str = "Can't open automapper";
const CONTAINS_NO_BLUEPRINT: &str = " has no embedded blueprint.";
const REASON: &str = "Only automappers with an embedded blueprint can be opened.";
const CONFIRM: &str = "OK";

pub fn show(ctx: &Context, file_name: &str) -> bool {
    let modal = egui::Modal::new(Id::new(MODAL_ID))
        .frame(modal_section::modal_frame(ctx))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            show_content(ui, file_name)
        });

    modal.inner || modal.should_close()
}

fn show_content(ui: &mut Ui, file_name: &str) -> bool {
    let header = ModalHeader {
        title: HEADING,
        align: TitleAlign::Center,
        tone: Tone::Warning,
    };
    if modal_header::show(ui, &header) {
        return true;
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        code_chip(ui, file_name);
        ui.label(CONTAINS_NO_BLUEPRINT);
    });
    paragraph_gap(ui);
    ui.label(REASON);
    paragraph_gap(ui);

    ui.vertical_centered(|ui| {
        ui.add_sized(MODAL_BUTTON_SIZE, Button::new(CONFIRM))
            .clicked()
    })
    .inner
}
