use egui::{Align, Button, Layout, TextStyle, TextWrapMode, Ui, UiBuilder, Vec2, WidgetText};

use super::icons::power_button;
use super::{TileContext, TileEdit, TilePanel};
use crate::ui::{ADD_ICON, BUTTON_GAP, MENU_ICON, compact_button, icon_button_size};

const PADDED_SIDES: f32 = 2.0;
const USE_TILE_TOOLTIP: &str = "This tile is used. Click to turn it off.";
const TURN_ON_TOOLTIP: &str = "This tile is turned off. Click to use it again.";
const ADD_RULE_TOOLTIP: &str = "Add another rule for this tile";
const MORE_TOOLTIP: &str = "More rule actions";
const DUPLICATE_LABEL: &str = "Duplicate rule";
const DELETE_LABEL: &str = "Delete rule";
const COPY_LABEL: &str = "Copy rule";
const COPY_KEY: &str = "C";
const PASTE_LABEL: &str = "Paste rule";
const PASTE_KEY: &str = "V";
const NOTHING_COPIED_TOOLTIP: &str = "Nothing copied yet";

impl TilePanel {
    pub(super) fn show_heading(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let title = format!("Tile {}", self.tile);
        let mut tab_edit = None;
        let row_size = Vec2::new(
            ui.available_width(),
            ui.text_style_height(&TextStyle::Heading),
        );
        let (icon_edit, tabs_fit) = ui
            .allocate_ui_with_layout(row_size, Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = BUTTON_GAP;
                let menu_edit = self.show_more_menu(ui, context);
                let toggle_edit = self.show_active_toggle(ui, context);
                let gap = ui.available_rect_before_wrap();
                let wanted = heading_width(ui, &title)
                    + tab_spacing(ui)
                    + rule_tabs_width(ui, context.rule_count);
                let tabs_fit = wanted <= gap.width();
                let builder = UiBuilder::new()
                    .max_rect(gap)
                    .layout(Layout::left_to_right(Align::Center));
                tab_edit = ui
                    .scope_builder(builder, |ui| {
                        ui.heading(&title);
                        tabs_fit
                            .then(|| self.show_rule_tabs(ui, context.rule_count))
                            .flatten()
                    })
                    .inner;
                (menu_edit.or(toggle_edit), tabs_fit)
            })
            .inner;

        if !tabs_fit {
            ui.horizontal_wrapped(|ui| {
                tab_edit = self.show_rule_tabs(ui, context.rule_count);
            });
        }

        icon_edit.or(tab_edit)
    }

    fn show_active_toggle(&self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let enabled = self.has_rule || context.deactivated;
        let tooltip = match context.deactivated {
            true => TURN_ON_TOOLTIP,
            false => USE_TILE_TOOLTIP,
        };
        let toggle = ui
            .add_enabled(enabled, |ui: &mut Ui| power_button(ui, context.deactivated))
            .on_hover_text(tooltip)
            .on_disabled_hover_text(tooltip);

        toggle.clicked().then_some(TileEdit::ToggleActive)
    }

    fn show_rule_tabs(&mut self, ui: &mut Ui, rule_count: usize) -> Option<TileEdit> {
        let mut edit = None;
        ui.spacing_mut().item_spacing.x = tab_spacing(ui);
        for index in 0..rule_count {
            if ui
                .selectable_label(index == self.index, rule_label(index))
                .clicked()
            {
                edit = Some(TileEdit::Select(index));
            }
        }

        if ui
            .button(ADD_ICON)
            .on_hover_text(ADD_RULE_TOOLTIP)
            .clicked()
        {
            edit = self.add_rule();
        }

        edit
    }

    fn show_more_menu(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let button = ui
            .add(compact_button(MENU_ICON, icon_button_size(ui)))
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
        if ui
            .add_enabled(self.has_rule, Button::new(DELETE_LABEL))
            .clicked()
        {
            self.error = None;
            edit = Some(TileEdit::Remove(self.index));
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
    let labels: Vec<String> = (0..rule_count)
        .map(rule_label)
        .chain([ADD_ICON.to_owned()])
        .collect();
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

    widths + (labels.len() - 1) as f32 * tab_spacing(ui)
}

fn heading_width(ui: &Ui, title: &str) -> f32 {
    WidgetText::from(title)
        .into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Heading,
        )
        .size()
        .x
}

fn tab_spacing(ui: &Ui) -> f32 {
    ui.ctx().global_style().spacing.item_spacing.x
}
