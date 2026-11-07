use egui::{Id, Ui};

use super::{AutomapperApp, CompileTarget, Workspace, render_active_source};
use crate::export::r_source::ExportError;
use crate::preview::{self, Automapped, CustomShape, Sample};
use crate::ui::preview as preview_view;
use crate::ui::show_vertically_centered;

const PREVIEW_STATE_ID: &str = "preview_state";
const PREVIEW_WAITING: &str = "Updating the preview…";

pub(super) enum PreviewResult {
    Waiting,
    Ready {
        rules: String,
        unmatched_marker: u8,
        automapped: Automapped,
    },
    Failed(String),
    NoRules(Automapped),
}

impl AutomapperApp {
    pub(super) fn regenerate_preview(&mut self) {
        self.preview_seed += 1;
        self.reautomap();
    }

    pub(super) fn switch_sample(&mut self, sample: Sample) {
        self.preview_sample = sample;
        if !matches!(self.preview, PreviewResult::Ready { .. }) {
            self.preview_source = None;
        }
        self.reautomap();
    }

    pub(super) fn reautomap(&mut self) {
        match &self.preview {
            PreviewResult::Ready {
                rules,
                unmatched_marker,
                ..
            } => {
                self.preview = preview_result(
                    rules.clone(),
                    self.preview_sample,
                    &self.custom_shape,
                    self.preview_seed,
                    *unmatched_marker,
                );
            }
            PreviewResult::NoRules(_) => {
                self.preview = unmapped_result(self.preview_sample, &self.custom_shape);
            }
            PreviewResult::Waiting | PreviewResult::Failed(_) => {}
        }
    }

    pub(super) fn show_preview(&mut self, ui: &mut Ui) {
        self.refresh_preview();
        self.show_preview_header(ui);

        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let drawing = (self.preview_sample == Sample::Custom && self.draw_custom)
            .then_some(&mut self.custom_shape);
        let mut interaction = preview_view::Interaction::default();
        let state_id = Id::new(PREVIEW_STATE_ID);
        match &self.preview {
            PreviewResult::Waiting => show_vertically_centered(ui, state_id, |ui| {
                ui.spinner();
                ui.weak(PREVIEW_WAITING);
            }),
            PreviewResult::Failed(error) => show_vertically_centered(ui, state_id, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }),
            PreviewResult::NoRules(automapped) | PreviewResult::Ready { automapped, .. } => {
                interaction =
                    preview_view::show(ui, &loaded.tileset, &loaded.texture, automapped, drawing);
            }
        }

        if interaction.edited {
            self.reautomap();
            ui.ctx().request_repaint();
        }
        if let Some(clicked) = interaction.clicked {
            self.select_rule_of(clicked);
        }
    }

    fn refresh_preview(&mut self) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
        if self.compiler.is_running() {
            return;
        }

        let stem = loaded.tileset.stem.clone();
        let source = match render_active_source(&self.rule_sets, &stem) {
            Ok(source) => source,
            Err(error) => {
                match error {
                    ExportError::EmptyRuleSet(_) => {
                        if !matches!(self.preview, PreviewResult::NoRules(_)) {
                            self.preview = unmapped_result(self.preview_sample, &self.custom_shape);
                        }
                    }
                    _ => self.preview = PreviewResult::Failed(error.to_string()),
                }
                self.preview_source = None;
                return;
            }
        };
        if self.preview_source.as_ref() == Some(&source) {
            return;
        }

        self.preview_source = Some(source.clone());
        let unmatched_marker = preview::unmatched_marker(self.rule_sets.active());
        self.start_compile(source, &stem, CompileTarget::Preview(unmatched_marker));
    }
}

pub(super) fn preview_result(
    rules: String,
    sample: Sample,
    custom: &CustomShape,
    seed: u32,
    unmatched_marker: u8,
) -> PreviewResult {
    match preview::automap(&rules, sample, custom, seed, unmatched_marker) {
        Ok(automapped) => PreviewResult::Ready {
            rules,
            unmatched_marker,
            automapped,
        },
        Err(error) => PreviewResult::Failed(error.to_string()),
    }
}

fn unmapped_result(sample: Sample, custom: &CustomShape) -> PreviewResult {
    match Automapped::unmapped(sample, custom) {
        Ok(automapped) => PreviewResult::NoRules(automapped),
        Err(error) => PreviewResult::Failed(error.to_string()),
    }
}
