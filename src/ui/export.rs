use egui::{Button, Context, Id, Ui, vec2};

use crate::model::rule_sets::RuleSets;
use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, show_section};

const EXPORT_WIDTH: f32 = 480.0;
const MODAL_ID: &str = "export";
const OPTIONS_GRID_ID: &str = "export_options";
const OPTIONS_COLUMN_GAP: f32 = 12.0;
const OPTIONS_ROW_GAP: f32 = 8.0;

const HEADING: &str = "Advanced export";
const RULES_LABEL: &str = "Export .rules";
const RULES_EMBEDDED_NOTE: &str = "Automapper with embedded blueprint";
const RULES_PLAIN_NOTE: &str = "Automapper without blueprint";
const SOURCE_LABEL: &str = "Export .r";
const SOURCE_NOTE: &str = "r++ source of the rules";
const BLUEPRINT_LABEL: &str = "Export .json";
const BLUEPRINT_NOTE: &str = "Blueprint without rules";
const EMBED_LABEL: &str = "Embed blueprint";
const RULE_SETS_HEADING: &str = "Rulesets";
const FILES_HEADING: &str = "Files";

pub enum ExportRequest {
    Rules,
    Source,
    Blueprint,
    Close,
}

struct ExportOption {
    label: &'static str,
    note: &'static str,
    enabled: bool,
    request: ExportRequest,
}

pub struct ExportContext<'a> {
    pub rule_sets: &'a RuleSets,
    pub selection: &'a mut [bool],
    pub compiling: bool,
    pub embed_blueprint: &'a mut bool,
}

pub fn show(ctx: &Context, export: &mut ExportContext) -> Option<ExportRequest> {
    let mut request = None;

    let modal = egui::Modal::new(Id::new(MODAL_ID))
        .frame(modal_section::modal_frame(ctx))
        .show(ctx, |ui| {
            ui.set_max_width(EXPORT_WIDTH);
            let header = ModalHeader {
                title: HEADING,
                align: TitleAlign::Start,
                tone: Tone::Plain,
            };
            if modal_header::show(ui, &header) {
                request = Some(ExportRequest::Close);
            }
            if export.rule_sets.len() > 1 {
                show_section(ui, RULE_SETS_HEADING, |ui| {
                    show_rule_set_choices(ui, export.rule_sets, export.selection);
                });
            }

            let exportable = export.selection.contains(&true);
            let options = [
                ExportOption {
                    label: RULES_LABEL,
                    note: match *export.embed_blueprint {
                        true => RULES_EMBEDDED_NOTE,
                        false => RULES_PLAIN_NOTE,
                    },
                    enabled: exportable && !export.compiling,
                    request: ExportRequest::Rules,
                },
                ExportOption {
                    label: SOURCE_LABEL,
                    note: SOURCE_NOTE,
                    enabled: exportable,
                    request: ExportRequest::Source,
                },
                ExportOption {
                    label: BLUEPRINT_LABEL,
                    note: BLUEPRINT_NOTE,
                    enabled: exportable,
                    request: ExportRequest::Blueprint,
                },
            ];
            show_section(ui, FILES_HEADING, |ui| {
                if let Some(chosen) = show_options(ui, options, export.embed_blueprint) {
                    request = Some(chosen);
                }
            });
        });

    request.or(modal.should_close().then_some(ExportRequest::Close))
}

fn show_rule_set_choices(ui: &mut Ui, rule_sets: &RuleSets, selection: &mut [bool]) {
    for ((name, _), chosen) in rule_sets.sets().iter().zip(selection) {
        ui.checkbox(chosen, name);
    }
}

fn show_options(
    ui: &mut Ui,
    options: [ExportOption; 3],
    embed_blueprint: &mut bool,
) -> Option<ExportRequest> {
    let mut chosen = None;
    egui::Grid::new(OPTIONS_GRID_ID)
        .spacing(vec2(OPTIONS_COLUMN_GAP, OPTIONS_ROW_GAP))
        .show(ui, |ui| {
            for option in options {
                let is_rules_row = matches!(option.request, ExportRequest::Rules);
                if ui
                    .add_enabled(option.enabled, Button::new(option.label))
                    .clicked()
                {
                    chosen = Some(option.request);
                }
                ui.weak(option.note);
                ui.end_row();
                if is_rules_row {
                    ui.label("");
                    ui.checkbox(embed_blueprint, EMBED_LABEL);
                    ui.end_row();
                }
            }
        });

    chosen
}
