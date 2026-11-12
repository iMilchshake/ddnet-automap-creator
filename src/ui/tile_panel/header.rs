use egui::{
    Align, Button, Layout, RichText, TextStyle, TextWrapMode, Ui, UiBuilder, Vec2, WidgetText,
};

use super::{TileContext, TileEdit, TilePanel};
use crate::ui::{ADD_ICON, BUTTON_GAP, REMOVE_ICON, SWITCH_ICON, compact_button, icon_button_size};

const PADDED_SIDES: f32 = 2.0;
const TURNED_OFF_TOOLTIP: &str =
    "This tile is turned off. Right-click it in the tileset to use it again.";
const NO_RULE_LABEL: &str = "No rule";
const NO_RULE_TOOLTIP: &str = "Edit the neighborhood below to create a rule";
const ADD_RULE_TOOLTIP: &str = "Add another rule for this tile";
const MORE_TOOLTIP: &str = "More rule actions";
const DELETE_TOOLTIP: &str = "Delete this rule";
const DUPLICATE_LABEL: &str = "Duplicate rule";
const COPY_LABEL: &str = "Copy rule";
const COPY_KEY: &str = "C";
const PASTE_LABEL: &str = "Paste rule";
const PASTE_KEY: &str = "V";
const NOTHING_COPIED_TOOLTIP: &str = "Nothing copied yet";

impl TilePanel {
    pub(super) fn show_header_row(
        &mut self,
        ui: &mut Ui,
        context: &TileContext,
    ) -> Option<TileEdit> {
        let mut tab_edit = None;
        let row_size = Vec2::new(ui.available_width(), icon_button_size(ui));
        let (icon_edit, tabs_fit) = ui
            .allocate_ui_with_layout(row_size, Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = BUTTON_GAP;
                let menu_edit = self.show_more_menu(ui, context);
                let delete_edit = self.show_delete_button(ui);
                let add_edit = self.show_add_button(ui);
                let gap = ui.available_rect_before_wrap();
                let tabs_fit = rule_tabs_width(ui, context.rule_count) <= gap.width();
                let builder = UiBuilder::new()
                    .max_rect(gap)
                    .layout(Layout::left_to_right(Align::Center));
                tab_edit = ui
                    .scope_builder(builder, |ui| {
                        tabs_fit.then(|| self.show_rule_tabs(ui, context)).flatten()
                    })
                    .inner;
                (menu_edit.or(delete_edit).or(add_edit), tabs_fit)
            })
            .inner;

        if !tabs_fit {
            ui.horizontal_wrapped(|ui| {
                tab_edit = self.show_rule_tabs(ui, context);
            });
        }

        icon_edit.or(tab_edit)
    }

    fn show_delete_button(&mut self, ui: &mut Ui) -> Option<TileEdit> {
        let button = ui
            .add_enabled(
                self.has_rule,
                compact_button(REMOVE_ICON, icon_button_size(ui)),
            )
            .on_hover_text(DELETE_TOOLTIP);
        if !button.clicked() {
            return None;
        }

        self.error = None;
        Some(TileEdit::Remove(self.index))
    }

    fn show_rule_tabs(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        if !self.has_rule {
            ui.weak(NO_RULE_LABEL).on_hover_text(NO_RULE_TOOLTIP);
            return None;
        }

        let mut edit = None;
        ui.spacing_mut().item_spacing.x = tab_spacing(ui);
        for index in 0..context.rule_count {
            let mut label = RichText::new(rule_label(index));
            if context.deactivated {
                label = label.color(ui.visuals().warn_fg_color);
            }
            let mut tab = ui.selectable_label(index == self.index, label);
            if context.deactivated {
                tab = tab.on_hover_text(TURNED_OFF_TOOLTIP);
            }
            if tab.clicked() {
                edit = Some(TileEdit::Select(index));
            }
        }

        edit
    }

    fn show_add_button(&mut self, ui: &mut Ui) -> Option<TileEdit> {
        let button = ui
            .add(compact_button(ADD_ICON, icon_button_size(ui)))
            .on_hover_text(ADD_RULE_TOOLTIP);

        button.clicked().then(|| self.add_rule()).flatten()
    }

    fn show_more_menu(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let button = ui
            .add(compact_button(SWITCH_ICON, icon_button_size(ui)))
            .on_hover_text(MORE_TOOLTIP);

        egui::Popup::menu(&button)
            .show(|ui| self.show_more_items(ui, context))
            .and_then(|shown| shown.inner)
    }

    fn show_more_items(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let mut edit = None;

        if ui
            .add_enabled(self.has_rule, Button::new(DUPLICATE_LABEL))
            .clicked()
        {
            edit = Some(TileEdit::Duplicate(self.index));
        }

        ui.separator();

        let copy = Button::new(COPY_LABEL).shortcut_text(COPY_KEY);
        if ui.add_enabled(self.has_rule, copy).clicked() {
            edit = Some(TileEdit::Copy(self.index));
        }

        let paste = Button::new(PASTE_LABEL).shortcut_text(PASTE_KEY);
        let paste = ui.add_enabled(context.copied.is_some(), paste);
        let paste = match context.copied {
            Some(copied) => paste.on_hover_text(copied.paste_tooltip()),
            None => paste.on_disabled_hover_text(NOTHING_COPIED_TOOLTIP),
        };
        if paste.clicked() {
            edit = Some(TileEdit::Paste(self.index));
        }

        edit
    }

    fn add_rule(&mut self) -> Option<TileEdit> {
        if self.has_rule {
            return Some(TileEdit::Add);
        }

        self.has_rule = true;
        self.apply()
    }
}

fn rule_label(index: usize) -> String {
    format!("Rule {}", index + 1)
}

fn rule_tabs_width(ui: &Ui, rule_count: usize) -> f32 {
    let labels: Vec<String> = (0..rule_count).map(rule_label).collect();
    let button_padding = PADDED_SIDES * ui.spacing().button_padding.x;
    let widths: f32 = labels
        .iter()
        .map(|label| {
            let text = WidgetText::from(label.as_str()).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextStyle::Button,
            );
            text.size().x + button_padding
        })
        .sum();

    widths + labels.len().saturating_sub(1) as f32 * tab_spacing(ui)
}

fn tab_spacing(ui: &Ui) -> f32 {
    ui.ctx().global_style().spacing.item_spacing.x
}
