use egui::{Button, Context, Frame, Id, Margin, RichText, Ui, vec2};

use crate::model::rule_sets::RuleSets;
use crate::ui::DDNET_AUTOMAP_PATHS;
use crate::ui::modal_header::{self, ModalHeader, TitleAlign, Tone};
use crate::ui::modal_section::{self, paragraph_gap, show_section};

const EXPORT_WIDTH: f32 = 480.0;
const MODAL_ID: &str = "export";
const OPTIONS_GRID_ID: &str = "export_options";
const CODE_BLOCK_MARGIN: Margin = Margin::symmetric(6, 2);
const CODE_CHIP_MARGIN: Margin = Margin::symmetric(4, 1);
const CODE_CORNER_RADIUS: u8 = 4;
const OPTIONS_COLUMN_GAP: f32 = 12.0;
const OPTIONS_ROW_GAP: f32 = 8.0;
const CODE_BG_BLEND: f32 = 0.4;

const HEADING: &str = "Export";
const RULES_LABEL: &str = "Export .rules";
const RULES_NOTE: &str = "DDNet compatible automapper";
const SOURCE_LABEL: &str = "Export .r";
const SOURCE_NOTE: &str = "r++ source used for rules generation";
const BUNDLE_LABEL: &str = "Export bundle";
const BUNDLE_NOTE: &str = "A zip archive with .rules, .r and blueprint";
const RULE_SETS_HEADING: &str = "Rule sets";
const FILES_HEADING: &str = "Files";
const DESTINATION_HEADING: &str = "Where to put the automapper?";
const DESTINATION: &str =
    "Put the .rules file into your local automapper folder, depending on your system.";
const NAME_NOTE_START: &str = "For DDNet to recognise the automapper, it ";
const NAME_NOTE_EMPHASIS: &str = "must";
const NAME_NOTE_MIDDLE: &str = " be named like the tileset image, here ";
const NAME_NOTE_END: &str = ".";
const RULES_EXTENSION: &str = ".rules";
const PATHS_GRID_ID: &str = "automap_paths";

pub enum ExportRequest {
    Rules,
    Source,
    Bundle,
    Close,
}

struct ExportOption {
    label: &'static str,
    note: &'static str,
    enabled: bool,
    highlighted: bool,
    request: ExportRequest,
}

pub struct ExportContext<'a> {
    pub rule_sets: &'a RuleSets,
    pub selection: &'a mut [bool],
    pub image_stem: &'a str,
    pub compiling: bool,
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
            let can_compile = exportable && !export.compiling;

            let options = [
                ExportOption {
                    label: RULES_LABEL,
                    note: RULES_NOTE,
                    enabled: can_compile,
                    highlighted: true,
                    request: ExportRequest::Rules,
                },
                ExportOption {
                    label: SOURCE_LABEL,
                    note: SOURCE_NOTE,
                    enabled: exportable,
                    highlighted: false,
                    request: ExportRequest::Source,
                },
                ExportOption {
                    label: BUNDLE_LABEL,
                    note: BUNDLE_NOTE,
                    enabled: can_compile,
                    highlighted: false,
                    request: ExportRequest::Bundle,
                },
            ];
            show_section(ui, FILES_HEADING, |ui| {
                if let Some(chosen) = show_options(ui, options) {
                    request = Some(chosen);
                }
            });
            paragraph_gap(ui);
            ui.separator();
            show_section(ui, DESTINATION_HEADING, |ui| {
                show_destination(ui, export.image_stem);
            });
        });

    request.or(modal.should_close().then_some(ExportRequest::Close))
}

fn show_rule_set_choices(ui: &mut Ui, rule_sets: &RuleSets, selection: &mut [bool]) {
    for ((name, _), chosen) in rule_sets.sets().iter().zip(selection) {
        ui.checkbox(chosen, name);
    }
}

fn show_options(ui: &mut Ui, options: [ExportOption; 3]) -> Option<ExportRequest> {
    let mut chosen = None;
    egui::Grid::new(OPTIONS_GRID_ID)
        .spacing(vec2(OPTIONS_COLUMN_GAP, OPTIONS_ROW_GAP))
        .show(ui, |ui| {
            for option in options {
                let mut button = Button::new(option.label);
                if option.highlighted {
                    button = button.fill(ui.visuals().selection.bg_fill);
                }
                if ui.add_enabled(option.enabled, button).clicked() {
                    chosen = Some(option.request);
                }
                ui.weak(option.note);
                ui.end_row();
            }
        });

    chosen
}

fn show_destination(ui: &mut Ui, image_stem: &str) {
    ui.label(DESTINATION);
    paragraph_gap(ui);
    egui::Grid::new(PATHS_GRID_ID)
        .spacing(vec2(OPTIONS_COLUMN_GAP, OPTIONS_ROW_GAP))
        .show(ui, |ui| {
            for (system, path) in DDNET_AUTOMAP_PATHS {
                ui.label(system);
                code_frame(ui, CODE_BLOCK_MARGIN).show(ui, |ui| {
                    ui.label(RichText::new(path).monospace());
                });
                ui.end_row();
            }
        });
    paragraph_gap(ui);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(NAME_NOTE_START);
        ui.strong(NAME_NOTE_EMPHASIS);
        ui.label(NAME_NOTE_MIDDLE);
        code_frame(ui, CODE_CHIP_MARGIN).show(ui, |ui| {
            ui.label(RichText::new(format!("{image_stem}{RULES_EXTENSION}")).monospace());
        });
        ui.label(NAME_NOTE_END);
    });
}

fn code_frame(ui: &Ui, margin: Margin) -> Frame {
    let visuals = ui.visuals();
    Frame::new()
        .fill(
            visuals
                .window_fill
                .lerp_to_gamma(visuals.extreme_bg_color, CODE_BG_BLEND),
        )
        .corner_radius(CODE_CORNER_RADIUS)
        .inner_margin(margin)
}
