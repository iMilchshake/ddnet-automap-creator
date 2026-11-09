use egui::{Context, Id, RichText, ScrollArea, Ui};

use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, paragraph_gap, show_section};

const MODAL_ID: &str = "save_failure";
const WIDTH: f32 = 420.0;
const ERROR_MAX_HEIGHT: f32 = 160.0;

const HEADING: &str = "Could not save automapper";
const PROBLEM_HEADING: &str = "What went wrong";
const PROBLEM: &str = "The rules could not be built.";
const FALLBACK_HEADING: &str = "Keep your work";
const FALLBACK: &str = "You can save the raw blueprint instead, so no work is lost.";
const SAVE_BLUEPRINT: &str = "Save blueprint";

pub enum SaveFailureDecision {
    SaveBlueprint,
    Close,
}

pub fn show(ctx: &Context, error: &str) -> Option<SaveFailureDecision> {
    let modal = egui::Modal::new(Id::new(MODAL_ID))
        .frame(modal_section::modal_frame(ctx))
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            show_content(ui, error)
        });

    let dismissed = modal.should_close().then_some(SaveFailureDecision::Close);
    modal.inner.or(dismissed)
}

fn show_content(ui: &mut Ui, error: &str) -> Option<SaveFailureDecision> {
    let header = ModalHeader {
        title: HEADING,
        align: TitleAlign::Start,
        tone: Tone::Warning,
    };
    if modal_header::show(ui, &header) {
        return Some(SaveFailureDecision::Close);
    }

    show_section(ui, PROBLEM_HEADING, |ui| {
        ui.label(PROBLEM);
        paragraph_gap(ui);
        ScrollArea::vertical()
            .max_height(ERROR_MAX_HEIGHT)
            .show(ui, |ui| {
                ui.label(RichText::new(error).monospace());
            });
    });

    let mut decision = None;
    show_section(ui, FALLBACK_HEADING, |ui| {
        ui.label(FALLBACK);
        paragraph_gap(ui);
        if ui.button(SAVE_BLUEPRINT).clicked() {
            decision = Some(SaveFailureDecision::SaveBlueprint);
        }
    });

    decision
}
