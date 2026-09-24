use egui::{Context, TextureHandle, TextureOptions, Ui, Vec2};

use crate::blueprint;
use crate::export::bundle::{self, Bundle};
use crate::export::compiler::{Compiled, RulesCompiler};
use crate::export::r_source::{self, ExportError, RuleSet};
use crate::file_filter::{BLUEPRINT, BUNDLE, RPP_SOURCE, RULES, TILESET_IMAGE};
use crate::file_picker::{FilePicker, PickedFile};
use crate::file_saver::FileSaver;
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, TILESET_SIDE};
use crate::preview::{self, Automapped};
use crate::tileset::{self, Tileset};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::group_panel::{GroupPanel, mode_label};
use crate::ui::preview as preview_view;
use crate::ui::status::StatusLine;
use crate::ui::tile_panel::{TileEdit, TilePanel};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const MIN_SIDE_PANEL_WIDTH: f32 = 220.0;
const MAX_SIDE_PANEL_WIDTH: f32 = 520.0;
const HELP_WIDTH: f32 = 360.0;
const EXPORT_WIDTH: f32 = 320.0;
const GROUP_SWATCH: Vec2 = Vec2::new(12.0, 18.0);
const GROUP_SWATCH_CORNER_RADIUS: f32 = 2.0;
const GROUP_ROW_SPACING: f32 = 2.0;
const GROUP_SWATCH_SPACING: f32 = 4.0;
const HEADING_SPACING: f32 = 4.0;
const DROP_LINE_WIDTH: f32 = 2.0;

const REMOVE_ICON: &str = "✖";
const CREDIT: &str = env!("CARGO_PKG_NAME");

const SELECTION_TOOLTIP: &str = "What a click on the tileset acts on. With groups selected, drag \
                                 across the tileset to add one, click one to edit it, right-click \
                                 to remove it.";
const EMPTY_INSPECTOR: &str = "Pick a tile or a group to edit it here.";
const PREVIEW_TOOLTIP: &str = "Tileset edits the rules, Preview shows them applied to a sample \
                               map the way DDNet's automapper would.";
const PREVIEW_WAITING: &str = "Compiling the rules for the preview…";
const GROUP_ORDER_NOTE: &str = "applied top to bottom";

enum GroupCommand {
    Select(usize),
    Move { from: usize, before: usize },
    Remove(usize),
}

#[derive(Default)]
enum Inspector {
    #[default]
    Empty,
    Tile(TilePanel),
    Group(GroupPanel),
}

impl Inspector {
    fn tile(&self) -> Option<usize> {
        match self {
            Self::Tile(panel) => Some(panel.tile()),
            _ => None,
        }
    }

    fn group(&self) -> Option<usize> {
        match self {
            Self::Group(panel) => Some(panel.index()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Tileset,
    Preview,
}

enum CompileTarget {
    Rules,
    Bundle(PendingBundle),
    Preview,
}

enum PreviewResult {
    Waiting,
    Ready(Automapped),
    Failed(String),
}

enum Workspace {
    NoImage,
    Ready(LoadedTileset),
}

struct PendingBundle {
    image_stem: String,
    source: String,
    blueprint: String,
}

struct LoadedTileset {
    tileset: Tileset,
    texture: TextureHandle,
}

pub struct AutomapperApp {
    workspace: Workspace,
    project: Project,
    rule_set_name: String,
    picker: FilePicker,
    blueprint_picker: FilePicker,
    saver: FileSaver,
    compiler: RulesCompiler,
    compile_target: Option<CompileTarget>,
    status: StatusLine,
    view: View,
    preview: PreviewResult,
    preview_source: Option<String>,
    inspector: Inspector,
    define_groups: bool,
    drag_anchor: Option<usize>,
    hovered_tile: Option<usize>,
    export_open: bool,
    help_open: bool,
}

impl Default for AutomapperApp {
    fn default() -> Self {
        Self {
            workspace: Workspace::NoImage,
            project: Project::default(),
            rule_set_name: String::new(),
            picker: FilePicker::new(TILESET_IMAGE),
            blueprint_picker: FilePicker::new(BLUEPRINT),
            saver: FileSaver::new(),
            compiler: RulesCompiler::new(),
            compile_target: None,
            status: StatusLine::default(),
            view: View::Tileset,
            preview: PreviewResult::Waiting,
            preview_source: None,
            inspector: Inspector::Empty,
            define_groups: false,
            drag_anchor: None,
            hovered_tile: None,
            export_open: false,
            help_open: false,
        }
    }
}

impl eframe::App for AutomapperApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.receive_picked_file(&ctx);
        self.receive_blueprint_file(&ctx);
        self.receive_saved_file(&ctx);
        self.receive_compiled_rules(&ctx);
        self.handle_shortcuts(&ctx);

        self.show_menu_bar(ui, &ctx);
        self.show_status_bar(ui);
        self.show_side_panel(ui);
        match self.view {
            View::Tileset => self.show_tileset(ui, &ctx),
            View::Preview => self.show_preview(ui),
        }
        self.show_export(&ctx);
        self.show_help(&ctx);

        if self.compiler.is_running() {
            ctx.request_repaint();
        }
    }
}

impl AutomapperApp {
    fn receive_picked_file(&mut self, ctx: &Context) {
        let Some(result) = self.picker.poll() else {
            return;
        };

        match result {
            Ok(picked) => self.load_tileset(ctx, picked),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn receive_blueprint_file(&mut self, ctx: &Context) {
        let Some(result) = self.blueprint_picker.poll() else {
            return;
        };

        match result {
            Ok(picked) => self.load_blueprint(ctx, picked),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn load_blueprint(&mut self, ctx: &Context, picked: PickedFile) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let text = match std::str::from_utf8(&picked.bytes) {
            Ok(text) => text,
            Err(_) => {
                self.status
                    .warning(ctx, format!("{} is not text", picked.name));
                return;
            }
        };

        match blueprint::from_json(text, &loaded.tileset.stem) {
            Ok(blueprint) => {
                self.project = blueprint.project;
                self.rule_set_name = blueprint.rule_set;
                self.inspector = Inspector::Empty;
                self.drag_anchor = None;
                self.status.info(ctx, format!("Loaded {}", picked.name));
            }
            Err(error) => self
                .status
                .warning(ctx, format!("{}: {error}", picked.name)),
        }
    }

    fn save_blueprint(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = &loaded.tileset.stem;
        match blueprint::to_json(&self.project, stem, &self.rule_set_name) {
            Ok(text) => self
                .saver
                .save_text(BLUEPRINT, &format!("{stem}.json"), text),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn receive_saved_file(&mut self, ctx: &Context) {
        let Some(result) = self.saver.poll() else {
            return;
        };

        match result {
            Ok(name) => self.status.info(ctx, format!("Saved {name}")),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn export_source(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = loaded.tileset.stem.clone();
        match render_source(&self.project, &stem, &self.rule_set_name) {
            Ok(source) => self
                .saver
                .save_text(RPP_SOURCE, &format!("{stem}.r"), source),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn compile_rules(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = loaded.tileset.stem.clone();
        match render_source(&self.project, &stem, &self.rule_set_name) {
            Ok(source) => {
                self.start_compile(source, &stem, CompileTarget::Rules);
                self.status.info(ctx, "Compiling with rpp…");
            }
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn compile_bundle(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = loaded.tileset.stem.clone();
        let source = match render_source(&self.project, &stem, &self.rule_set_name) {
            Ok(source) => source,
            Err(error) => {
                self.status.warning(ctx, error.to_string());
                return;
            }
        };
        let blueprint = match blueprint::to_json(&self.project, &stem, &self.rule_set_name) {
            Ok(blueprint) => blueprint,
            Err(error) => {
                self.status.warning(ctx, error.to_string());
                return;
            }
        };

        let pending = PendingBundle {
            image_stem: stem.clone(),
            source: source.clone(),
            blueprint,
        };
        self.start_compile(source, &stem, CompileTarget::Bundle(pending));
        self.status.info(ctx, "Compiling with rpp…");
    }

    fn start_compile(&mut self, source: String, stem: &str, target: CompileTarget) {
        self.compiler.start(source, r_source::output_file(stem));
        self.compile_target = Some(target);
    }

    fn receive_compiled_rules(&mut self, ctx: &Context) {
        let Some(result) = self.compiler.poll() else {
            return;
        };
        let Some(target) = self.compile_target.take() else {
            return;
        };

        match (target, result) {
            (CompileTarget::Preview, Ok(Compiled { rules, .. })) => {
                self.preview = match preview::automap(&rules) {
                    Ok(automapped) => PreviewResult::Ready(automapped),
                    Err(error) => PreviewResult::Failed(error.to_string()),
                };
            }
            (CompileTarget::Preview, Err(error)) => {
                self.preview = PreviewResult::Failed(error.to_string());
            }
            (CompileTarget::Rules, Ok(Compiled { file_name, rules })) => {
                self.saver.save_text(RULES, &file_name, rules);
            }
            (CompileTarget::Bundle(pending), Ok(Compiled { rules, .. })) => {
                self.save_bundle(ctx, pending, &rules);
            }
            (CompileTarget::Rules | CompileTarget::Bundle(_), Err(error)) => {
                self.status.warning(ctx, error.to_string());
            }
        }
    }

    fn save_bundle(&mut self, ctx: &Context, pending: PendingBundle, rules: &str) {
        let bundle = Bundle {
            image_stem: &pending.image_stem,
            rules,
            source: &pending.source,
            blueprint: &pending.blueprint,
        };

        match bundle::archive(&bundle) {
            Ok(archive) => {
                let name = bundle::file_name(&pending.image_stem);
                self.saver.save_bytes(BUNDLE, &name, archive);
            }
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn load_tileset(&mut self, ctx: &Context, picked: PickedFile) {
        let stem = tileset::file_stem(&picked.name);
        let tileset = match tileset::decode_tileset(&picked.bytes, &stem) {
            Ok(tileset) => tileset,
            Err(error) => {
                self.status
                    .warning(ctx, format!("{}: {error}", picked.name));
                return;
            }
        };

        let texture = upload_atlas(ctx, &tileset);
        let [width, height] = tileset.image_size;
        let [tile_width, tile_height] = tileset.tile_size;
        self.status.info(
            ctx,
            format!(
                "Loaded {} ({width}×{height}, {tile_width}×{tile_height} per tile)",
                picked.name
            ),
        );

        self.project = Project::default();
        self.rule_set_name = stem.clone();
        self.inspector = Inspector::Empty;
        self.drag_anchor = None;
        self.hovered_tile = None;
        self.preview = PreviewResult::Waiting;
        self.preview_source = None;
        self.workspace = Workspace::Ready(LoadedTileset { tileset, texture });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|input| input.consume_shortcut(&open_image_shortcut())) {
            self.picker.open();
        }
    }

    fn show_menu_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let mut save_blueprint = false;

        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let open = egui::Button::new("Open image…")
                        .shortcut_text(ui.ctx().format_shortcut(&open_image_shortcut()));
                    if ui.add(open).clicked() {
                        self.picker.open();
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(has_image, egui::Button::new("Load blueprint…"))
                        .clicked()
                    {
                        self.blueprint_picker.open();
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_image, egui::Button::new("Save blueprint…"))
                        .clicked()
                    {
                        save_blueprint = true;
                        ui.close();
                    }
                });

                if ui
                    .add_enabled(has_image, egui::Button::new("Export"))
                    .clicked()
                {
                    self.export_open = true;
                }

                if ui.button("Help").clicked() {
                    self.help_open = true;
                }

                ui.separator();
                self.show_view_picker(ui);
                ui.separator();
                self.show_selection_picker(ui);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let credit = egui::RichText::new(CREDIT).weak();
                    ui.add(egui::Label::new(credit).truncate());
                });
            });
        });

        if save_blueprint {
            self.save_blueprint(ctx);
        }
    }

    fn show_view_picker(&mut self, ui: &mut Ui) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));

        ui.add_enabled_ui(has_image, |ui| {
            ui.selectable_value(&mut self.view, View::Tileset, "Tileset");
            ui.selectable_value(&mut self.view, View::Preview, "Preview");
        })
        .response
        .on_hover_text(PREVIEW_TOOLTIP);
    }

    fn show_selection_picker(&mut self, ui: &mut Ui) {
        let editing = matches!(self.workspace, Workspace::Ready(_)) && self.view == View::Tileset;
        let was_defining = self.define_groups;

        ui.add_enabled_ui(editing, |ui| {
            ui.label("Selecting:");
            ui.selectable_value(&mut self.define_groups, false, "Tiles");
            ui.selectable_value(&mut self.define_groups, true, "Groups");
        })
        .response
        .on_hover_text(SELECTION_TOOLTIP);

        if self.define_groups != was_defining {
            self.inspector = Inspector::Empty;
            self.drag_anchor = None;
        }
    }

    fn show_status_bar(&mut self, ui: &mut Ui) {
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                self.status.show(ui);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let (Workspace::Ready(loaded), Some(index)) =
                        (&self.workspace, self.hovered_tile)
                    {
                        ui.weak(describe_tile(&loaded.tileset, &self.project, index));
                    }
                });
            });
        });
    }

    fn show_side_panel(&mut self, ui: &mut Ui) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let selected_group = self.inspector.group();
        let mut pending = None;
        let mut tile_edit = None;
        let mut group_edit = None;

        egui::Panel::right("tools")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(MIN_SIDE_PANEL_WIDTH)
            .max_size(MAX_SIDE_PANEL_WIDTH)
            .show(ui, |ui| {
                ui.add_enabled_ui(has_image, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Rule name");
                        ui.text_edit_singleline(&mut self.rule_set_name);
                    });
                    ui.label(format!(
                        "{} tiles, {} groups configured",
                        self.project.rule_count(),
                        self.project.groups().len()
                    ));

                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        match (&self.workspace, &mut self.inspector) {
                            (Workspace::Ready(loaded), Inspector::Tile(panel)) => {
                                tile_edit = panel
                                    .show(ui, &loaded.tileset, &loaded.texture)
                                    .map(|edit| (panel.tile(), edit));
                            }
                            (_, Inspector::Group(panel)) => {
                                group_edit = panel
                                    .show(ui, self.project.groups())
                                    .map(|group| (panel.index(), group));
                            }
                            _ => {
                                ui.weak(EMPTY_INSPECTOR);
                            }
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.heading("Groups");
                            let note = egui::RichText::new(GROUP_ORDER_NOTE).small().weak();
                            ui.add(egui::Label::new(note).truncate());
                        });
                        ui.add_space(HEADING_SPACING);
                        show_group_list(ui, self.project.groups(), selected_group, &mut pending);
                    });
                });
            });

        if let Some((tile, edit)) = tile_edit {
            match edit {
                TileEdit::Apply(rule) => self.project.set_rule(tile, rule),
                TileEdit::Remove => self.project.clear_rule(tile),
            }
        }
        if let Some((index, group)) = group_edit {
            self.project.replace_group(index, group);
        }

        match pending {
            Some(GroupCommand::Select(index)) => self.select_group(index),
            Some(GroupCommand::Move { from, before }) => {
                self.forget_group_selection();
                if let Some(index) = self.project.move_group(from, before) {
                    self.select_group(index);
                }
            }
            Some(GroupCommand::Remove(index)) => {
                self.project.remove_group(index);
                self.forget_group_selection();
            }
            None => {}
        }
    }

    fn show_export(&mut self, ctx: &Context) {
        if !self.export_open {
            return;
        }

        let exportable = self.project.rule_count() > 0 || !self.project.groups().is_empty();
        let can_compile = exportable && !self.compiler.is_running();
        let mut export = false;
        let mut compile = false;
        let mut pack = false;

        let modal = egui::Modal::new(egui::Id::new("export")).show(ctx, |ui| {
            ui.set_max_width(EXPORT_WIDTH);
            ui.heading("Export");
            ui.label(format!(
                "{} tiles, {} groups configured",
                self.project.rule_count(),
                self.project.groups().len()
            ));
            ui.separator();

            if ui
                .add_enabled(can_compile, egui::Button::new("Export .rules"))
                .clicked()
            {
                compile = true;
            }
            ui.weak("Compiles the rule set with rpp, ready for DDNet.");

            ui.add_space(8.0);
            if ui
                .add_enabled(exportable, egui::Button::new("Export .r"))
                .clicked()
            {
                export = true;
            }
            ui.weak("Saves the rpp source instead of compiling it.");

            ui.add_space(8.0);
            if ui
                .add_enabled(can_compile, egui::Button::new("Export bundle"))
                .clicked()
            {
                pack = true;
            }
            ui.weak("A zip with the .rules, the .r and the blueprint.");

            ui.separator();
            if ui.button("Close").clicked() {
                self.export_open = false;
            }
        });

        if modal.should_close() {
            self.export_open = false;
        }

        if export {
            self.export_source(ctx);
            self.export_open = false;
        }
        if compile {
            self.compile_rules(ctx);
            self.export_open = false;
        }
        if pack {
            self.compile_bundle(ctx);
            self.export_open = false;
        }
    }

    fn select_group(&mut self, index: usize) {
        if let Some(group) = self.project.groups().get(index) {
            self.inspector = Inspector::Group(GroupPanel::new(index, group));
        }
    }

    fn forget_group_selection(&mut self) {
        if self.inspector.group().is_some() {
            self.inspector = Inspector::Empty;
        }
    }

    fn show_tileset(&mut self, ui: &mut Ui, ctx: &Context) {
        let response = egui::CentralPanel::default()
            .show(ui, |ui| match &self.workspace {
                Workspace::NoImage => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        if ui.button("Select Image").clicked() {
                            self.picker.open();
                        }
                    });
                    GridResponse::default()
                }
                Workspace::Ready(loaded) => {
                    ui.vertical_centered(|ui| {
                        grid::show(
                            ui,
                            GridView {
                                tileset: &loaded.tileset,
                                project: &self.project,
                                texture: &loaded.texture,
                                group_editing: self.define_groups,
                                drag_anchor: self.drag_anchor,
                                selected: self.inspector.tile(),
                            },
                        )
                    })
                    .inner
                }
            })
            .inner;

        self.handle_grid(ctx, response);
    }

    fn show_preview(&mut self, ui: &mut Ui) {
        self.hovered_tile = None;
        self.refresh_preview();

        egui::CentralPanel::default().show(ui, |ui| {
            let Workspace::Ready(loaded) = &self.workspace else {
                return;
            };

            match &self.preview {
                PreviewResult::Waiting => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.weak(PREVIEW_WAITING);
                    });
                }
                PreviewResult::Failed(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                PreviewResult::Ready(automapped) => {
                    ui.vertical_centered(|ui| {
                        preview_view::show(ui, &loaded.tileset, &loaded.texture, automapped);
                    });
                }
            }
        });
    }

    fn refresh_preview(&mut self) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
        if self.compiler.is_running() {
            return;
        }

        let stem = loaded.tileset.stem.clone();
        let source = match render_source(&self.project, &stem, &self.rule_set_name) {
            Ok(source) => source,
            Err(error) => {
                self.preview = PreviewResult::Failed(error.to_string());
                self.preview_source = None;
                return;
            }
        };
        if self.preview_source.as_ref() == Some(&source) {
            return;
        }

        self.preview_source = Some(source.clone());
        self.start_compile(source, &stem, CompileTarget::Preview);
    }

    fn handle_grid(&mut self, ctx: &Context, response: GridResponse) {
        self.hovered_tile = response.hovered;

        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        if self.define_groups {
            self.handle_group_grid(ctx, response);
            return;
        }

        if let Some(tile) = response.clicked
            && tile_state(&loaded.tileset, &self.project, tile).is_editable()
        {
            let rule = seed_rule(&loaded.tileset, &self.project, tile);
            let has_rule = self.project.rule(tile).is_some();
            self.inspector = Inspector::Tile(TilePanel::new(tile, rule, has_rule));
        }

        if let Some(tile) = response.secondary_clicked {
            self.toggle_removed(ctx, tile);
        }
    }

    fn handle_group_grid(&mut self, ctx: &Context, response: GridResponse) {
        if let Some(tile) = response.drag_started {
            self.drag_anchor = Some(tile);
        }

        if let Some(index) = response
            .clicked
            .and_then(|tile| self.project.group_at(tile))
        {
            self.select_group(index);
        }

        if let Some(tile) = response.secondary_clicked
            && let Some(index) = self.project.group_at(tile)
        {
            let name = self.project.groups()[index].name.clone();
            self.project.remove_group(index);
            self.forget_group_selection();
            self.status.info(ctx, format!("Removed group {name}"));
            self.drag_anchor = None;
            return;
        }

        let Some(anchor) = self.drag_anchor.take() else {
            return;
        };
        let Some(corner) = response.drag_released else {
            self.drag_anchor = Some(anchor);
            return;
        };

        match self.project.group_at(anchor) {
            Some(index) if anchor == corner => self.select_group(index),
            Some(_) => self
                .status
                .warning(ctx, "That rectangle starts inside another group"),
            None => self.create_group(ctx, anchor, corner),
        }
    }

    fn create_group(&mut self, ctx: &Context, anchor: usize, corner: usize) {
        let (top_left, bottom_right) = grid::corners(anchor, corner);
        let group = TileGroup {
            name: self.project.unused_group_name(),
            top_left,
            width: bottom_right % TILESET_SIDE - top_left % TILESET_SIDE + 1,
            height: bottom_right / TILESET_SIDE - top_left / TILESET_SIDE + 1,
            mode: GroupMode::Fill,
            chance: Chance::FULL,
        };

        if !group
            .footprint()
            .iter()
            .all(|tile| self.project.is_free(*tile))
        {
            self.status
                .warning(ctx, "Those tiles already belong to a group or a rule");
            return;
        }
        if let Err(error) = group.validate() {
            self.status.warning(ctx, error.to_string());
            return;
        }

        self.status.info(ctx, format!("Added group {}", group.name));
        self.project.add_group(group);
        self.select_group(0);
    }

    fn toggle_removed(&mut self, ctx: &Context, tile: usize) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let state = tile_state(&loaded.tileset, &self.project, tile);
        if !state.is_editable() {
            return;
        }

        match state {
            TileState::Removed => {
                self.project.restore(tile);
                self.status.info(ctx, format!("Tile {tile} restored"));
            }
            _ => {
                self.project.remove(tile);
                if self.inspector.tile() == Some(tile) {
                    self.inspector = Inspector::Empty;
                }
                self.status.info(ctx, format!("Tile {tile} removed"));
            }
        }
    }

    fn show_help(&mut self, ctx: &Context) {
        if !self.help_open {
            return;
        }

        let modal = egui::Modal::new(egui::Id::new("help")).show(ctx, |ui| {
            ui.set_max_width(HELP_WIDTH);
            ui.heading(concat!(
                env!("CARGO_PKG_NAME"),
                " ",
                env!("CARGO_PKG_VERSION")
            ));
            ui.label("Visual editor to easily create DDNet automappers.");

            ui.add_space(8.0);
            ui.colored_label(ui.visuals().warn_fg_color, "TODO: <add tutorial here>");

            ui.separator();
            if ui.button("Close").clicked() {
                self.help_open = false;
            }
        });

        if modal.should_close() {
            self.help_open = false;
        }
    }
}

fn show_group_list(
    ui: &mut Ui,
    groups: &[TileGroup],
    selected: Option<usize>,
    pending: &mut Option<GroupCommand>,
) {
    if groups.is_empty() {
        ui.weak("No groups yet.");
        return;
    }

    for (index, group) in groups.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GROUP_ROW_SPACING;

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(REMOVE_ICON)
                    .on_hover_text("Remove group")
                    .clicked()
                {
                    *pending = Some(GroupCommand::Remove(index));
                }

                let draggable = ui.available_size();
                let layout = egui::Layout::left_to_right(egui::Align::Center);
                ui.allocate_ui_with_layout(draggable, layout, |ui| {
                    show_group_row(ui, index, group, selected, pending);
                });
            });
        });
    }
}

fn show_group_row(
    ui: &mut Ui,
    index: usize,
    group: &TileGroup,
    selected: Option<usize>,
    pending: &mut Option<GroupCommand>,
) {
    let id = egui::Id::new(("group_row", index));
    let row = ui
        .dnd_drag_source(id, index, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GROUP_SWATCH_SPACING;

                let (swatch, _response) =
                    ui.allocate_exact_size(GROUP_SWATCH, egui::Sense::hover());
                ui.painter().rect_filled(
                    swatch,
                    GROUP_SWATCH_CORNER_RADIUS,
                    grid::group_color(index),
                );

                let name_area = ui.available_size();
                let layout =
                    egui::Layout::left_to_right(egui::Align::Center).with_main_justify(true);
                ui.allocate_ui_with_layout(name_area, layout, |ui| {
                    let chosen = selected == Some(index);
                    let label = ui
                        .add(egui::Button::selectable(chosen, group.name.as_str()).truncate())
                        .on_hover_text(describe_group(group));
                    if label.clicked() {
                        *pending = Some(GroupCommand::Select(index));
                    }
                });
            });
        })
        .response
        .on_hover_cursor(egui::CursorIcon::Grab);

    if row.drag_started() {
        *pending = Some(GroupCommand::Select(index));
    }

    if let Some(target) = drop_target(ui, &row, index)
        && let Some(from) = row.dnd_release_payload::<usize>()
    {
        *pending = Some(GroupCommand::Move {
            from: *from,
            before: target,
        });
    }
}

fn describe_tile(tileset: &Tileset, project: &Project, tile: usize) -> String {
    match tile_state(tileset, project, tile) {
        TileState::Locked => format!("Tile {tile} · locked"),
        TileState::Grouped => format!("Tile {tile} · in a group"),
        TileState::Removed => format!("Tile {tile} · removed"),
        TileState::Guessed(_) => format!("Tile {tile}"),
        TileState::Configured(rule) => {
            let neighborhood = format_neighborhood(rule.neighborhood);
            match rule.chance.is_full() {
                true => format!("Tile {tile} · {neighborhood}"),
                false => format!("Tile {tile} · {neighborhood} · {}%", rule.chance.percent()),
            }
        }
    }
}

fn format_neighborhood(neighborhood: Neighborhood) -> String {
    let glyphs: String = neighborhood
        .states()
        .iter()
        .copied()
        .map(state_glyph)
        .collect();

    let (top, rest) = glyphs.split_at(3);
    let (sides, bottom) = rest.split_at(2);
    format!("{top}/{sides}/{bottom}")
}

fn state_glyph(state: NeighborState) -> char {
    match state {
        NeighborState::Empty => '.',
        NeighborState::Full => '#',
        NeighborState::Any => '?',
    }
}

fn open_image_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O)
}

fn upload_atlas(ctx: &Context, tileset: &Tileset) -> TextureHandle {
    let [width, height] = tileset.image_size;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        tileset.rgba.as_raw(),
    );

    ctx.load_texture("tileset_atlas", image, TextureOptions::NEAREST)
}

fn drop_target(ui: &Ui, row: &egui::Response, index: usize) -> Option<usize> {
    let pointer = ui.input(|input| input.pointer.interact_pos())?;
    row.dnd_hover_payload::<usize>()?;

    let above = pointer.y < row.rect.center().y;
    let edge = match above {
        true => row.rect.top(),
        false => row.rect.bottom(),
    };
    let stroke = egui::Stroke::new(DROP_LINE_WIDTH, ui.visuals().selection.stroke.color);
    ui.painter().hline(row.rect.x_range(), edge, stroke);

    Some(index + usize::from(!above))
}

fn describe_group(group: &TileGroup) -> String {
    let mut text = format!(
        "{} — {}×{}, {}",
        group.name,
        group.width,
        group.height,
        mode_label(group.mode)
    );
    if !group.chance.is_full() {
        text.push_str(&format!(", {}%", group.chance.percent()));
    }

    text
}

fn render_source(
    project: &Project,
    image_stem: &str,
    rule_set_name: &str,
) -> Result<String, ExportError> {
    let tiles = project.rules();
    let rule_set = RuleSet {
        image_stem,
        name: rule_set_name,
        tiles: &tiles,
        groups: project.groups(),
    };

    r_source::render(&rule_set)
}
