use egui::Ui;

use super::{
    AutomapperApp, View, Workspace, export_shortcut, open_image_shortcut, redo_shortcut,
    save_automapper_shortcut, undo_shortcut,
};
use crate::file_filter::{AUTOMAPPER, TILESET_IMAGE};
use crate::model::pool::ChanceMode;
use crate::ui::{APP_NAME, GITHUB_ICON, REPOSITORY, external_link, text_width};

const SHORT_CREDIT: &str = "GitHub";
const AUTHOR_CREDIT: &str = "by iMilchshake";

const UNDO_LABEL: &str = "Undo";
const REDO_LABEL: &str = "Redo";
const COPY_RULE_LABEL: &str = "Copy rule";
const PASTE_RULE_LABEL: &str = "Paste rule";
const FILE_MENU: &str = "File";
const OPEN_IMAGE_LABEL: &str = "Open image…";
const OPEN_AUTOMAPPER_LABEL: &str = "Open automapper…";
const SAVE_AUTOMAPPER_LABEL: &str = "Save automapper…";
const MERGE_AUTOMAPPER_LABEL: &str = "Merge automapper…";
const EXPORT_LABEL: &str = "Export…";
const EDIT_MENU: &str = "Edit";
const OPTIONS_MENU: &str = "Options";
const HELP_LABEL: &str = "Help";
const TILESET_LABEL: &str = "Tileset";
const PREVIEW_LABEL: &str = "Preview";
const SPLIT_LABEL: &str = "Split";
const NORMALIZE_LABEL: &str = "Normalize chances";
const ZOOM_LABEL: &str = "Zoom";
const ZOOM_IN_LABEL: &str = "+";
const ZOOM_OUT_LABEL: &str = "-";

const PREVIEW_TOOLTIP: &str = "Tileset is where you edit the rules. Preview shows them on a test \
                               map, the way DDNet would. Split shows both.";
const NORMALIZE_TOOLTIP: &str = "On: chances get scaled so they always add up to 100%.\n\
                                 Off: if they add up to less, some spots can stay as they are.";
const ZOOM_TOOLTIP: &str = "Ctrl + and Ctrl - zoom, Ctrl 0 resets.";
const PERCENT: f32 = 100.0;

impl AutomapperApp {
    fn show_edit_menu(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let selected = self.inspector.rule();
        let has_clipboard = self.clipboard.is_some();

        let undo =
            egui::Button::new(UNDO_LABEL).shortcut_text(ctx.format_shortcut(&undo_shortcut()));
        let can_undo = self.history.can_undo(&self.rule_sets);
        if ui.add_enabled(can_undo, undo).clicked() {
            self.undo(&ctx);
            ui.close();
        }

        let redo =
            egui::Button::new(REDO_LABEL).shortcut_text(ctx.format_shortcut(&redo_shortcut()));
        let can_redo = self.history.can_redo(&self.rule_sets);
        if ui.add_enabled(can_redo, redo).clicked() {
            self.redo(&ctx);
            ui.close();
        }

        ui.separator();

        let copyable = selected
            .filter(|&(tile, index)| self.rule_sets.active().rules_of(tile).get(index).is_some());
        let copy = ui.add_enabled(copyable.is_some(), egui::Button::new(COPY_RULE_LABEL));
        if let Some((tile, index)) = copyable
            && copy.clicked()
        {
            self.copy_rule(&ctx, tile, index);
            ui.close();
        }

        let pasteable = selected.filter(|_| has_clipboard);
        let paste = ui.add_enabled(pasteable.is_some(), egui::Button::new(PASTE_RULE_LABEL));
        if let Some((tile, index)) = pasteable
            && paste.clicked()
        {
            self.paste_rule(&ctx, tile, index);
            ui.close();
        }
    }

    pub(super) fn show_menu_bar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let mut save_automapper = false;

        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(FILE_MENU, |ui| {
                    let open = egui::Button::new(OPEN_IMAGE_LABEL)
                        .shortcut_text(ui.ctx().format_shortcut(&open_image_shortcut()));
                    if ui.add(open).clicked() {
                        self.picker.open(TILESET_IMAGE);
                        ui.close();
                    }

                    if ui
                        .add_enabled(has_image, egui::Button::new(OPEN_AUTOMAPPER_LABEL))
                        .clicked()
                    {
                        self.picker.open(AUTOMAPPER);
                        ui.close();
                    }

                    let save = egui::Button::new(SAVE_AUTOMAPPER_LABEL)
                        .shortcut_text(ui.ctx().format_shortcut(&save_automapper_shortcut()));
                    if ui.add_enabled(has_image, save).clicked() {
                        save_automapper = true;
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(has_image, egui::Button::new(MERGE_AUTOMAPPER_LABEL))
                        .clicked()
                    {
                        self.merge_picker.open(AUTOMAPPER);
                        ui.close();
                    }

                    let export = egui::Button::new(EXPORT_LABEL)
                        .shortcut_text(ui.ctx().format_shortcut(&export_shortcut()));
                    if ui.add_enabled(has_image, export).clicked() {
                        self.open_export();
                        ui.close();
                    }
                });

                ui.menu_button(EDIT_MENU, |ui| self.show_edit_menu(ui));

                let options_config = egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
                egui::containers::menu::MenuButton::new(OPTIONS_MENU)
                    .config(options_config)
                    .ui(ui, |ui| self.show_options(ui));

                if ui.button(HELP_LABEL).clicked() {
                    self.help_open = true;
                }

                if has_image {
                    ui.separator();
                    self.show_view_picker(ui);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    show_credit(ui);
                });
            });
        });

        if save_automapper {
            self.save_automapper(&ctx, None);
        }
    }

    fn show_view_picker(&mut self, ui: &mut Ui) {
        let previous = self.view;

        ui.scope(|ui| {
            ui.selectable_value(&mut self.view, View::Tileset, TILESET_LABEL);
            ui.selectable_value(&mut self.view, View::Preview, PREVIEW_LABEL);
            ui.selectable_value(&mut self.view, View::Split, SPLIT_LABEL);
        })
        .response
        .on_hover_text(PREVIEW_TOOLTIP);

        if self.view != previous {
            self.view_before_picking = None;
        }
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut normalize = self.rule_sets.active().chance_mode() == ChanceMode::Normalize;
        if ui
            .checkbox(&mut normalize, NORMALIZE_LABEL)
            .on_hover_text(NORMALIZE_TOOLTIP)
            .changed()
        {
            self.rule_sets
                .active_mut()
                .set_chance_mode(match normalize {
                    true => ChanceMode::Normalize,
                    false => ChanceMode::Exact,
                });
        }

        if ui.ctx().options(|options| options.zoom_with_keyboard) {
            show_zoom(ui);
        }
    }
}

fn show_zoom(ui: &mut Ui) {
    let ctx = ui.ctx().clone();

    egui::containers::Sides::new().show(
        ui,
        |ui| {
            ui.label(ZOOM_LABEL).on_hover_text(ZOOM_TOOLTIP);
        },
        |ui| {
            if zoom_button(ui, ZOOM_IN_LABEL) {
                egui::gui_zoom::zoom_in(&ctx);
            }
            ui.label(format!("{:.0}%", ctx.zoom_factor() * PERCENT))
                .on_hover_text(ZOOM_TOOLTIP);
            if zoom_button(ui, ZOOM_OUT_LABEL) {
                egui::gui_zoom::zoom_out(&ctx);
            }
        },
    );
}

fn zoom_button(ui: &mut Ui, text: &str) -> bool {
    let button = egui::Button::new(text)
        .frame(true)
        .frame_when_inactive(true);

    ui.add(button).on_hover_text(ZOOM_TOOLTIP).clicked()
}

#[derive(Clone, Copy)]
enum CreditForm {
    Signed,
    Full,
    Short,
    Icon,
}

impl CreditForm {
    const WIDEST_FIRST: [Self; 3] = [Self::Signed, Self::Full, Self::Short];

    fn link_text(self) -> String {
        match self {
            Self::Signed | Self::Full => format!("{GITHUB_ICON} {APP_NAME}"),
            Self::Short => format!("{GITHUB_ICON} {SHORT_CREDIT}"),
            Self::Icon => GITHUB_ICON.to_string(),
        }
    }

    fn signature(self) -> Option<&'static str> {
        matches!(self, Self::Signed).then_some(AUTHOR_CREDIT)
    }
}

fn credit_width(ui: &Ui, form: CreditForm) -> f32 {
    let link_width = text_width(ui, &form.link_text());
    match form.signature() {
        Some(signature) => link_width + text_width(ui, signature),
        None => link_width,
    }
}

fn choose_credit_form(ui: &Ui) -> CreditForm {
    CreditForm::WIDEST_FIRST
        .into_iter()
        .find(|form| credit_width(ui, *form) <= ui.available_width())
        .unwrap_or(CreditForm::Icon)
}

fn show_credit(ui: &mut Ui) {
    let form = choose_credit_form(ui);
    if let Some(signature) = form.signature() {
        ui.label(egui::RichText::new(signature).weak());
    }
    let link = external_link(ui, form.link_text(), REPOSITORY);
    if matches!(form, CreditForm::Icon) {
        link.on_hover_text(APP_NAME);
    }
}
