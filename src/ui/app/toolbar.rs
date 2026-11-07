use egui::Ui;

use super::{AutomapperApp, Inspector, PreviewResult};
use crate::preview::{CUSTOM_SIZE_RANGE, Sample};
use crate::ui::DRAW_ICON;

const PADDED_SIDES: f32 = 2.0;
const SELECTION_TOOLTIP: &str = "Choose what clicks on the tileset do. Tiles edits single tiles. \
                                 Groups lets you drag out rectangles that get placed as one piece. \
                                 Click a group to edit it, right-click to remove it.";
const GENERATE_TOOLTIP: &str = "Roll the chances again.";
const DRAW_TOOLTIP: &str = "Left-click draws blocks, right-click erases them.";
const MAP_LABEL: &str = "Map:";
const REGENERATE_LABEL: &str = "Re-generate";
const SEED_LABEL: &str = "seed";
const SIZE_LABEL: &str = "Size:";
const SIZE_SEPARATOR: &str = "×";
const CLEAR_LABEL: &str = "Clear";
const DRAW_LABEL: &str = "Draw";
const TOOL_LABEL: &str = "Tool:";
const TILES_LABEL: &str = "Tiles";
const GROUPS_LABEL: &str = "Groups";

fn show_header_row(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    let spacing = ui.spacing();
    let height = spacing.interact_size.y + PADDED_SIDES * spacing.button_padding.y;
    ui.horizontal(|ui| {
        ui.set_min_height(height);
        add_contents(ui);
    });
    ui.separator();
}

impl AutomapperApp {
    pub(super) fn show_tileset_header(&mut self, ui: &mut Ui) {
        let was_defining = self.define_groups;

        show_header_row(ui, |ui| {
            ui.scope(|ui| {
                ui.label(TOOL_LABEL);
                ui.selectable_value(&mut self.define_groups, false, TILES_LABEL);
                ui.selectable_value(&mut self.define_groups, true, GROUPS_LABEL);
            })
            .response
            .on_hover_text(SELECTION_TOOLTIP);
        });

        if self.define_groups != was_defining {
            self.inspector = Inspector::Empty;
            self.drag_anchor = None;
        }
    }

    pub(super) fn show_preview_header(&mut self, ui: &mut Ui) {
        show_header_row(ui, |ui| self.show_preview_controls(ui));
    }

    fn show_preview_controls(&mut self, ui: &mut Ui) {
        let ready = matches!(self.preview, PreviewResult::Ready { .. });

        let mut chosen = self.preview_sample;
        ui.label(MAP_LABEL);
        egui::ComboBox::from_id_salt("preview_sample")
            .selected_text(chosen.name())
            .show_ui(ui, |ui| {
                for sample in Sample::ALL {
                    ui.selectable_value(&mut chosen, sample, sample.name());
                }
            });
        if chosen != self.preview_sample {
            self.switch_sample(chosen);
        }
        ui.separator();

        if ui
            .add_enabled(ready, egui::Button::new(REGENERATE_LABEL))
            .on_hover_text(GENERATE_TOOLTIP)
            .clicked()
        {
            self.regenerate_preview();
        }
        ui.weak(format!("{SEED_LABEL} {}", self.preview_seed));

        if self.preview_sample == Sample::Custom {
            ui.separator();
            self.show_custom_controls(ui);
        }
    }

    fn show_custom_controls(&mut self, ui: &mut Ui) {
        let mut width = self.custom_shape.width();
        let mut height = self.custom_shape.height();

        ui.label(SIZE_LABEL);
        ui.add(egui::DragValue::new(&mut width).range(CUSTOM_SIZE_RANGE));
        ui.label(SIZE_SEPARATOR);
        ui.add(egui::DragValue::new(&mut height).range(CUSTOM_SIZE_RANGE));
        if (width, height) != (self.custom_shape.width(), self.custom_shape.height()) {
            self.custom_shape.resize(width, height);
            self.reautomap();
        }

        if ui.button(CLEAR_LABEL).clicked() {
            self.custom_shape.clear();
            self.reautomap();
        }
        ui.toggle_value(&mut self.draw_custom, format!("{DRAW_ICON} {DRAW_LABEL}"))
            .on_hover_text(DRAW_TOOLTIP);
    }
}
