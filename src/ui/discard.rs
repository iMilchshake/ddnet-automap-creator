use egui::{Button, Color32, Context, RichText, Ui, vec2};

use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, paragraph_gap};

const DISCARD_HEADING: &str = "You have unsaved changes";
const SAVE_FIRST: &str = "Save";
const QUIT: &str = "Quit";
const DISCARD: &str = "Discard";
const DISCARD_WIDTH: f32 = 320.0;
const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 28.0;
const MODAL_ID: &str = "discard";
const DISCARD_TEXT_COLOR: Color32 = Color32::BLACK;

pub enum DiscardDecision {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Copy)]
pub enum Leaving {
    Quit,
    OpenFiles,
}

impl Leaving {
    fn discard_label(self) -> &'static str {
        match self {
            Self::Quit => QUIT,
            Self::OpenFiles => DISCARD,
        }
    }
}

pub fn show(ctx: &Context, leaving: Leaving) -> Option<DiscardDecision> {
    let modal = egui::Modal::new(egui::Id::new(MODAL_ID))
        .frame(modal_section::modal_frame(ctx))
        .show(ctx, |ui| {
            ui.set_width(DISCARD_WIDTH);
            show_content(ui, leaving)
        });

    let dismissed = modal.should_close().then_some(DiscardDecision::Cancel);
    modal.inner.or(dismissed)
}

fn show_content(ui: &mut Ui, leaving: Leaving) -> Option<DiscardDecision> {
    let header = ModalHeader {
        title: DISCARD_HEADING,
        align: TitleAlign::Center,
        tone: Tone::Warning,
    };
    if modal_header::show(ui, &header) {
        return Some(DiscardDecision::Cancel);
    }
    paragraph_gap(ui);

    show_buttons(ui, leaving)
}

fn show_buttons(ui: &mut Ui, leaving: Leaving) -> Option<DiscardDecision> {
    ui.horizontal(|ui| {
        let used = 2.0 * BUTTON_WIDTH + ui.spacing().item_spacing.x;
        ui.add_space((ui.available_width() - used) / 2.0);

        let size = vec2(BUTTON_WIDTH, BUTTON_HEIGHT);
        let save = ui.add_sized(size, Button::new(SAVE_FIRST)).clicked();

        let label = RichText::new(leaving.discard_label()).color(DISCARD_TEXT_COLOR);
        let discard = Button::new(label).fill(ui.visuals().warn_fg_color);
        let discard = ui.add_sized(size, discard).clicked();

        save.then_some(DiscardDecision::Save)
            .or(discard.then_some(DiscardDecision::Discard))
    })
    .inner
}
