use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::blueprint;
use crate::export::bundle::{self, Bundle};
use crate::export::compiler::{Compiled, RulesCompiler};
use crate::export::r_source::{self, ExportError, RuleSet};
use crate::file_filter::{BLUEPRINT, BUNDLE, RPP_SOURCE, RULES, TILESET_IMAGE};
use crate::file_picker::{FilePicker, PickedFile};
use crate::file_saver::FileSaver;
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::{self, ChanceMode, Pool};
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Chance, MASK_TILE, TILESET_SIDE, TileRule};
use crate::preview::{self, Automapped, Sample};
use crate::tileset::{self, Tileset};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::group_panel::{GroupPanel, mode_label};
use crate::ui::preview as preview_view;
use crate::ui::status::StatusLine;
use crate::ui::tile_panel::{TileEdit, TilePanel, TilePools};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const MIN_SIDE_PANEL_WIDTH: f32 = 220.0;
const MAX_SIDE_PANEL_WIDTH: f32 = 520.0;
const HELP_WIDTH: f32 = 360.0;
const MERGE_WIDTH: f32 = 360.0;
const EXPORT_WIDTH: f32 = 320.0;
const GROUP_SWATCH: Vec2 = Vec2::new(12.0, 18.0);
const GROUP_SWATCH_CORNER_RADIUS: f32 = 2.0;
const GROUP_ROW_SPACING: f32 = 2.0;
const GROUP_SWATCH_SPACING: f32 = 4.0;
const HEADING_SPACING: f32 = 4.0;
const DROP_LINE_WIDTH: f32 = 2.0;
const FIRST_PREVIEW_SEED: u32 = 42;
const DROP_OVERLAY_ALPHA: u8 = 160;
const DROP_HINT_SIZE: f32 = 24.0;

const REMOVE_ICON: &str = "✖";
const DUPLICATE_ICON: &str = "⎘";
const GITHUB_ICON: char = egui::special_emojis::GITHUB;
const CREDIT: &str = env!("CARGO_PKG_NAME");
const SHORT_CREDIT: &str = "GitHub";
const AUTHOR_CREDIT: &str = "by iMilchshake";
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const ORIGINAL_PROJECT: &str = "https://github.com/AssassinTee/SimpleDDNetAutomapper";
const RPP: &str = "https://github.com/Aerll/rpp";
const DM1_SOURCE: &str = "https://github.com/teeworlds/teeworlds-maps";
const DM1_LICENSE: &str = "http://creativecommons.org/licenses/by-sa/3.0/";

const SELECTION_TOOLTIP: &str = "What a click on the tileset acts on. With groups selected, drag \
                                 across the tileset to add one, click one to edit it, right-click \
                                 to remove it.";
const SHARED_NAME_HINT: &str = "Another rule set has this name";
const EMPTY_INSPECTOR: &str = "Pick a tile or a group to edit it here.";
const PREVIEW_TOOLTIP: &str = "Tileset edits the rules, Preview shows them applied to a sample \
                               map the way DDNet's automapper would.";
const NORMALIZE_TOOLTIP: &str = "ON: chances are re-normalized to always add up to 100%\n\
                                 OFF: allows tiles to remain unchanged if summed chance is below \
                                 100%";
const MASK_WARNING: &str = "is reserved: rpp uses it as a mask to roll tiles that share a \
                            neighborhood.";
const AIR_WARNING: &str = "is reserved for air: DDNet treats it as empty.";
const GENERATE_TOOLTIP: &str = "Roll the chances again with the next seed.";
const PREVIEW_WAITING: &str = "Compiling the rules for the preview…";
const PREVIEW_NO_RULES: &str = "No preview possible as this rule set has no rules yet. Go configure some in the Tileset tab :)";
const GROUP_ORDER_NOTE: &str = "applied top to bottom";
const DROP_HINT: &str = "Drop a tileset image or a blueprint";
const NO_IMAGE_FOR_BLUEPRINT: &str = "Open a tileset image before loading a blueprint";
const MERGE_HEADING: &str = "Merge blueprint";
const MERGE_WARNING: &str = "Its tile ids are merged as they are.";
const MERGE_CONFIRM: &str = "Merge anyway";
const MERGE_CANCELLED: &str = "Merge cancelled";
const LOAD_HEADING: &str = "Load blueprint";
const LOAD_WARNING: &str = "Its tile ids are loaded as they are.";
const LOAD_CONFIRM: &str = "Load anyway";
const LOAD_CANCELLED: &str = "Load cancelled";
const BLUEPRINT_CANCEL: &str = "Cancel";

enum GroupCommand {
    Select(usize),
    Move { from: usize, before: usize },
    Remove(usize),
}

enum RuleSetCommand {
    Select(usize),
    Add,
    Duplicate,
    Remove,
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
    Ready {
        rules: String,
        automapped: Automapped,
    },
    Failed(String),
    NoRules,
}

enum BlueprintDecision {
    Confirm,
    Cancel,
}

#[derive(Clone, Copy)]
enum BlueprintAction {
    Load,
    Merge,
}

impl BlueprintAction {
    fn heading(self) -> &'static str {
        match self {
            Self::Load => LOAD_HEADING,
            Self::Merge => MERGE_HEADING,
        }
    }

    fn warning(self) -> &'static str {
        match self {
            Self::Load => LOAD_WARNING,
            Self::Merge => MERGE_WARNING,
        }
    }

    fn confirm(self) -> &'static str {
        match self {
            Self::Load => LOAD_CONFIRM,
            Self::Merge => MERGE_CONFIRM,
        }
    }

    fn cancelled(self) -> &'static str {
        match self {
            Self::Load => LOAD_CANCELLED,
            Self::Merge => MERGE_CANCELLED,
        }
    }
}

struct PendingBlueprint {
    file_name: String,
    wanted_image: String,
    rule_sets: RuleSets,
    action: BlueprintAction,
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
    rule_sets: RuleSets,
    picker: FilePicker,
    merge_picker: FilePicker,
    pending_blueprint: Option<PendingBlueprint>,
    saver: FileSaver,
    compiler: RulesCompiler,
    compile_target: Option<CompileTarget>,
    status: StatusLine,
    view: View,
    preview: PreviewResult,
    preview_source: Option<String>,
    preview_seed: u32,
    preview_sample: Sample,
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
            rule_sets: RuleSets::default(),
            picker: FilePicker::new(),
            merge_picker: FilePicker::new(),
            pending_blueprint: None,
            saver: FileSaver::new(),
            compiler: RulesCompiler::new(),
            compile_target: None,
            status: StatusLine::default(),
            view: View::Tileset,
            preview: PreviewResult::Waiting,
            preview_source: None,
            preview_seed: FIRST_PREVIEW_SEED,
            preview_sample: Sample::default(),
            inspector: Inspector::Empty,
            define_groups: false,
            drag_anchor: None,
            hovered_tile: None,
            export_open: false,
            help_open: false,
        }
    }
}

impl AutomapperApp {
    pub fn new(ctx: &Context, image: Option<PickedFile>, blueprint: Option<PickedFile>) -> Self {
        let mut app = Self::default();
        if let Some(image) = image {
            app.load_tileset(ctx, image);
        }
        if let Some(blueprint) = blueprint {
            app.load_blueprint(ctx, blueprint);
        }
        app
    }
}

impl eframe::App for AutomapperApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.receive_files(&ctx);
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
        self.show_blueprint_confirmation(&ctx);
        show_drop_hint(&ctx);

        if self.compiler.is_running() {
            ctx.request_repaint();
        }
    }
}

impl AutomapperApp {
    fn receive_files(&mut self, ctx: &Context) {
        let mut dropped = ctx.input(|input| input.raw.dropped_files.clone());
        dropped.sort_by_key(|file| BLUEPRINT.matches(&file.path().to_string_lossy()));
        self.picker.read_dropped(ctx, dropped);

        while let Some(result) = self.picker.poll() {
            match result {
                Ok(picked) => self.load_file(ctx, picked),
                Err(error) => self.status.warning(ctx, error.to_string()),
            }
        }

        while let Some(result) = self.merge_picker.poll() {
            match result {
                Ok(picked) => self.merge_blueprint(ctx, picked),
                Err(error) => self.status.warning(ctx, error.to_string()),
            }
        }
    }

    fn load_file(&mut self, ctx: &Context, picked: PickedFile) {
        self.pending_blueprint = None;
        if TILESET_IMAGE.matches(&picked.name) {
            return self.load_tileset(ctx, picked);
        }
        if BLUEPRINT.matches(&picked.name) {
            return self.load_blueprint(ctx, picked);
        }
        self.status.warning(
            ctx,
            format!("{} is neither a tileset image nor a blueprint", picked.name),
        );
    }

    fn load_blueprint(&mut self, ctx: &Context, picked: PickedFile) {
        self.submit_blueprint(ctx, picked, BlueprintAction::Load);
    }

    fn merge_blueprint(&mut self, ctx: &Context, picked: PickedFile) {
        self.submit_blueprint(ctx, picked, BlueprintAction::Merge);
    }

    fn submit_blueprint(&mut self, ctx: &Context, picked: PickedFile, action: BlueprintAction) {
        let (text, stem) = match blueprint_text(&self.workspace, &picked) {
            Ok(found) => found,
            Err(warning) => return self.status.warning(ctx, warning),
        };

        let parsed = match blueprint::parse(text) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.status
                    .warning(ctx, format!("{}: {error}", picked.name));
                return;
            }
        };

        let other_image = parsed.image.filter(|wanted| wanted != stem);
        match other_image {
            Some(wanted_image) => {
                self.pending_blueprint = Some(PendingBlueprint {
                    file_name: picked.name,
                    wanted_image,
                    rule_sets: parsed.rule_sets,
                    action,
                });
            }
            None => self.apply_blueprint(ctx, action, &picked.name, parsed.rule_sets),
        }
    }

    fn apply_blueprint(
        &mut self,
        ctx: &Context,
        action: BlueprintAction,
        file_name: &str,
        rule_sets: RuleSets,
    ) {
        match action {
            BlueprintAction::Load => self.apply_load(ctx, file_name, rule_sets),
            BlueprintAction::Merge => self.apply_merge(ctx, file_name, rule_sets),
        }
    }

    fn apply_load(&mut self, ctx: &Context, file_name: &str, rule_sets: RuleSets) {
        self.rule_sets = rule_sets;
        self.inspector = Inspector::Empty;
        self.drag_anchor = None;
        self.status.info(ctx, format!("Loaded {file_name}"));
    }

    fn apply_merge(&mut self, ctx: &Context, file_name: &str, rule_sets: RuleSets) {
        let merged = self.rule_sets.merge(rule_sets);
        self.status
            .info(ctx, format!("Merged {merged} rule sets from {file_name}"));
    }

    fn show_blueprint_confirmation(&mut self, ctx: &Context) {
        let (Some(pending), Workspace::Ready(loaded)) = (&self.pending_blueprint, &self.workspace)
        else {
            return;
        };

        let mut decision = None;
        let modal = egui::Modal::new(egui::Id::new("blueprint")).show(ctx, |ui| {
            ui.set_max_width(MERGE_WIDTH);
            ui.heading(pending.action.heading());
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "{} was made for {}, but {} is open. {}",
                    pending.file_name,
                    pending.wanted_image,
                    loaded.tileset.stem,
                    pending.action.warning()
                ),
            );

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(pending.action.confirm()).clicked() {
                    decision = Some(BlueprintDecision::Confirm);
                }
                if ui.button(BLUEPRINT_CANCEL).clicked() {
                    decision = Some(BlueprintDecision::Cancel);
                }
            });
        });

        if modal.should_close() {
            decision = decision.or(Some(BlueprintDecision::Cancel));
        }

        let Some(decision) = decision else {
            return;
        };
        let Some(pending) = self.pending_blueprint.take() else {
            return;
        };
        match decision {
            BlueprintDecision::Confirm => {
                self.apply_blueprint(ctx, pending.action, &pending.file_name, pending.rule_sets)
            }
            BlueprintDecision::Cancel => self.status.info(ctx, pending.action.cancelled()),
        }
    }

    fn save_blueprint(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = &loaded.tileset.stem;
        match blueprint::to_json(&self.rule_sets, stem) {
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
        match render_source(&self.rule_sets, &stem) {
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
        match render_source(&self.rule_sets, &stem) {
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
        let source = match render_source(&self.rule_sets, &stem) {
            Ok(source) => source,
            Err(error) => {
                self.status.warning(ctx, error.to_string());
                return;
            }
        };
        let blueprint = match blueprint::to_json(&self.rule_sets, &stem) {
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
                self.preview = preview_result(rules, self.preview_sample, self.preview_seed);
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
        let message = format!(
            "Loaded {} ({width}×{height}, {tile_width}×{tile_height} per tile)",
            picked.name
        );
        match width == height {
            true => self.status.info(ctx, message),
            false => self.status.warning(
                ctx,
                format!("{message}, not square, tiles will be stretched, might be unintentional"),
            ),
        }

        self.rule_sets = RuleSets::new(stem.clone());
        self.inspector = Inspector::Empty;
        self.drag_anchor = None;
        self.hovered_tile = None;
        self.preview = PreviewResult::Waiting;
        self.preview_source = None;
        self.view = View::Tileset;
        self.workspace = Workspace::Ready(LoadedTileset { tileset, texture });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|input| input.consume_shortcut(&open_image_shortcut())) {
            self.picker.open(TILESET_IMAGE);
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
                        self.picker.open(TILESET_IMAGE);
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(has_image, egui::Button::new("Load blueprint…"))
                        .clicked()
                    {
                        self.picker.open(BLUEPRINT);
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_image, egui::Button::new("Save blueprint…"))
                        .clicked()
                    {
                        save_blueprint = true;
                        ui.close();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(has_image, egui::Button::new("Merge blueprint…"))
                        .clicked()
                    {
                        self.merge_picker.open(BLUEPRINT);
                        ui.close();
                    }
                });

                if ui
                    .add_enabled(has_image, egui::Button::new("Export"))
                    .clicked()
                {
                    self.export_open = true;
                }

                ui.menu_button("Options", |ui| self.show_options(ui));

                if ui.button("Help").clicked() {
                    self.help_open = true;
                }

                ui.separator();
                self.show_view_picker(ui);
                ui.separator();
                match self.view {
                    View::Tileset => self.show_selection_picker(ui),
                    View::Preview => self.show_preview_controls(ui),
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    show_credit(ui);
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

    fn show_options(&mut self, ui: &mut Ui) {
        let mut normalize = self.rule_sets.active().chance_mode() == ChanceMode::Normalize;
        if ui
            .checkbox(&mut normalize, "Normalize chances")
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
    }

    fn show_preview_controls(&mut self, ui: &mut Ui) {
        let ready = matches!(self.preview, PreviewResult::Ready { .. });

        ui.label("Map:");
        for sample in Sample::ALL {
            if ui
                .selectable_label(self.preview_sample == sample, sample.name())
                .clicked()
            {
                self.switch_sample(sample);
            }
        }
        ui.separator();

        if ui
            .add_enabled(ready, egui::Button::new("Re-generate"))
            .on_hover_text(GENERATE_TOOLTIP)
            .clicked()
        {
            self.regenerate_preview();
        }
        ui.weak(format!("seed {}", self.preview_seed));
    }

    fn show_selection_picker(&mut self, ui: &mut Ui) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let was_defining = self.define_groups;

        ui.add_enabled_ui(has_image, |ui| {
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
                        ui.weak(describe_tile(
                            &loaded.tileset,
                            self.rule_sets.active(),
                            index,
                        ));
                    }
                });
            });
        });
    }

    fn show_side_panel(&mut self, ui: &mut Ui) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let selected_group = self.inspector.group();
        let pools = pool::pools(&self.rule_sets.active().rules());
        let mut pending = None;
        let mut tile_edit = None;
        let mut group_edit = None;
        let mut rule_set_command = None;

        egui::Panel::right("tools")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(MIN_SIDE_PANEL_WIDTH)
            .max_size(MAX_SIDE_PANEL_WIDTH)
            .show(ui, |ui| {
                ui.add_enabled_ui(has_image, |ui| {
                    show_rule_set_tabs(ui, &self.rule_sets, &mut rule_set_command);
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let can_remove = self.rule_sets.len() > 1;
                            if ui
                                .add_enabled(can_remove, egui::Button::new(REMOVE_ICON))
                                .on_hover_text("Remove this rule set")
                                .clicked()
                            {
                                rule_set_command = Some(RuleSetCommand::Remove);
                            }
                            if ui
                                .button(DUPLICATE_ICON)
                                .on_hover_text("Duplicate this rule set")
                                .clicked()
                            {
                                rule_set_command = Some(RuleSetCommand::Duplicate);
                            }
                            ui.add_sized(
                                ui.available_size(),
                                egui::TextEdit::singleline(self.rule_sets.active_name_mut()),
                            );
                        });
                    });
                    ui.label(describe_counts(self.rule_sets.active()));

                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        match (&self.workspace, &mut self.inspector) {
                            (Workspace::Ready(loaded), Inspector::Tile(panel)) => {
                                let tile_pools = TilePools {
                                    pools: pool::pools_of(&pools, panel.tile()),
                                    mode: self.rule_sets.active().chance_mode(),
                                };
                                let deactivated =
                                    self.rule_sets.active().is_deactivated(panel.tile());
                                tile_edit = panel
                                    .show(
                                        ui,
                                        &loaded.tileset,
                                        &loaded.texture,
                                        &tile_pools,
                                        deactivated,
                                    )
                                    .map(|edit| (panel.tile(), edit));
                            }
                            (_, Inspector::Group(panel)) => {
                                group_edit = panel
                                    .show(ui, self.rule_sets.active().groups())
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
                        show_group_list(
                            ui,
                            self.rule_sets.active().groups(),
                            selected_group,
                            &mut pending,
                        );
                    });
                });
            });

        if let Some((tile, edit)) = tile_edit {
            match edit {
                TileEdit::Apply(rule) => self.rule_sets.active_mut().set_rule(tile, rule),
                TileEdit::Remove => self.rule_sets.active_mut().clear_rule(tile),
            }
        }
        if let Some((index, group)) = group_edit {
            self.rule_sets.active_mut().replace_group(index, group);
        }

        match pending {
            Some(GroupCommand::Select(index)) => self.select_group(index),
            Some(GroupCommand::Move { from, before }) => {
                self.forget_group_selection();
                if let Some(index) = self.rule_sets.active_mut().move_group(from, before) {
                    self.select_group(index);
                }
            }
            Some(GroupCommand::Remove(index)) => {
                self.rule_sets.active_mut().remove_group(index);
                self.forget_group_selection();
            }
            None => {}
        }

        match rule_set_command {
            Some(RuleSetCommand::Select(index)) => self.select_rule_set(index),
            Some(RuleSetCommand::Add) => {
                let index = self.rule_sets.add();
                self.select_rule_set(index);
            }
            Some(RuleSetCommand::Duplicate) => {
                let index = self.rule_sets.active_index();
                if let Some(index) = self.rule_sets.duplicate(index) {
                    self.select_rule_set(index);
                }
            }
            Some(RuleSetCommand::Remove) => self.remove_active_rule_set(),
            None => {}
        }
    }

    fn select_rule_set(&mut self, index: usize) {
        self.rule_sets.select(index);
        self.inspector = Inspector::Empty;
        self.drag_anchor = None;
        self.hovered_tile = None;
    }

    fn remove_active_rule_set(&mut self) {
        let index = self.rule_sets.active_index();
        if self.rule_sets.remove(index) {
            self.inspector = Inspector::Empty;
            self.drag_anchor = None;
            self.hovered_tile = None;
        }
    }

    fn show_export(&mut self, ctx: &Context) {
        if !self.export_open {
            return;
        }

        let exportable = self
            .rule_sets
            .sets()
            .iter()
            .any(|(_, project)| !project.rules().is_empty() || !project.groups().is_empty());
        let can_compile = exportable && !self.compiler.is_running();
        let mut export = false;
        let mut compile = false;
        let mut pack = false;

        let modal = egui::Modal::new(egui::Id::new("export")).show(ctx, |ui| {
            ui.set_max_width(EXPORT_WIDTH);
            ui.heading("Export");
            ui.label(format!("{} rule set(s) configured", self.rule_sets.len()));
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
        if let Some(group) = self.rule_sets.active().groups().get(index) {
            self.inspector = Inspector::Group(GroupPanel::new(index, group));
        }
    }

    fn forget_group_selection(&mut self) {
        if self.inspector.group().is_some() {
            self.inspector = Inspector::Empty;
        }
    }

    fn pool_mates(&self, pools: &[Pool]) -> Vec<usize> {
        let Some(tile) = self.inspector.tile() else {
            return Vec::new();
        };
        let Some(as_drawn) = pool::pools_of(pools, tile).first().copied() else {
            return Vec::new();
        };

        as_drawn
            .members
            .iter()
            .map(|member| member.tile)
            .filter(|mate| *mate != tile)
            .collect()
    }

    fn show_tileset(&mut self, ui: &mut Ui, ctx: &Context) {
        let pools = pool::pools(&self.rule_sets.active().rules());
        let shares = pool::base_shares(&pools, self.rule_sets.active().chance_mode());
        let pool_mates = self.pool_mates(&pools);

        let response = egui::CentralPanel::default()
            .show(ui, |ui| match &self.workspace {
                Workspace::NoImage => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        if ui.button("Select Image").clicked() {
                            self.picker.open(TILESET_IMAGE);
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
                                project: self.rule_sets.active(),
                                shares: &shares,
                                texture: &loaded.texture,
                                group_editing: self.define_groups,
                                drag_anchor: self.drag_anchor,
                                selected: self.inspector.tile(),
                                pool_mates: &pool_mates,
                            },
                        )
                    })
                    .inner
                }
            })
            .inner;

        self.handle_grid(ctx, response);
    }

    fn regenerate_preview(&mut self) {
        let PreviewResult::Ready { rules, .. } = &self.preview else {
            return;
        };

        self.preview_seed += 1;
        self.preview = preview_result(rules.clone(), self.preview_sample, self.preview_seed);
    }

    fn switch_sample(&mut self, sample: Sample) {
        self.preview_sample = sample;

        let PreviewResult::Ready { rules, .. } = &self.preview else {
            self.preview_source = None;
            return;
        };
        self.preview = preview_result(rules.clone(), sample, self.preview_seed);
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
                PreviewResult::NoRules => {
                    ui.weak(PREVIEW_NO_RULES);
                }
                PreviewResult::Ready { automapped, .. } => {
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
        let source = match render_active_source(&self.rule_sets, &stem) {
            Ok(source) => source,
            Err(error) => {
                self.preview = match error {
                    ExportError::EmptyRuleSet(_) => PreviewResult::NoRules,
                    _ => PreviewResult::Failed(error.to_string()),
                };
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

        if response.clicked == Some(MASK_TILE) {
            self.status
                .warning(ctx, format!("Tile {MASK_TILE} {MASK_WARNING}"));
        }
        if response.clicked == Some(AIR_TILE) {
            self.status
                .warning(ctx, format!("Tile {AIR_TILE} {AIR_WARNING}"));
        }

        if let Some(tile) = response.clicked
            && tile_state(&loaded.tileset, self.rule_sets.active(), tile).is_editable()
        {
            let rule = seed_rule(&loaded.tileset, self.rule_sets.active(), tile);
            let has_rule = self.rule_sets.active().rule(tile).is_some();
            self.inspector = Inspector::Tile(TilePanel::new(tile, rule, has_rule));
        }

        if let Some(tile) = response.secondary_clicked {
            self.toggle_deactivated(ctx, tile);
        }
    }

    fn handle_group_grid(&mut self, ctx: &Context, response: GridResponse) {
        if let Some(tile) = response.drag_started {
            self.drag_anchor = Some(tile);
        }

        if let Some(index) = response
            .clicked
            .and_then(|tile| self.rule_sets.active().group_at(tile))
        {
            self.select_group(index);
        }

        if let Some(tile) = response.secondary_clicked
            && let Some(index) = self.rule_sets.active().group_at(tile)
        {
            let name = self.rule_sets.active().groups()[index].name.clone();
            self.rule_sets.active_mut().remove_group(index);
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

        match self.rule_sets.active().group_at(anchor) {
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
            name: self.rule_sets.active().unused_group_name(),
            top_left,
            width: bottom_right % TILESET_SIDE - top_left % TILESET_SIDE + 1,
            height: bottom_right / TILESET_SIDE - top_left / TILESET_SIDE + 1,
            mode: GroupMode::Fill,
            chance: Chance::FULL,
        };

        if let Err(error) = group.validate() {
            self.status.warning(ctx, error.to_string());
            return;
        }
        if !group
            .footprint()
            .iter()
            .all(|tile| self.rule_sets.active().is_free(*tile))
        {
            self.status
                .warning(ctx, "Those tiles already belong to a group or a rule");
            return;
        }

        self.status.info(ctx, format!("Added group {}", group.name));
        self.rule_sets.active_mut().add_group(group);
        self.select_group(0);
    }

    fn toggle_deactivated(&mut self, ctx: &Context, tile: usize) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let state = tile_state(&loaded.tileset, self.rule_sets.active(), tile);
        if !state.is_editable() {
            return;
        }

        match state {
            TileState::Deactivated(_) => {
                self.rule_sets.active_mut().reactivate(tile);
                self.status.info(ctx, format!("Tile {tile} reactivated"));
            }
            _ => {
                self.rule_sets.active_mut().deactivate(tile);
                self.status.info(ctx, format!("Tile {tile} deactivated"));
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

            ui.add_space(8.0);
            show_acknowledgements(ui);

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

fn blueprint_text<'a>(
    workspace: &'a Workspace,
    picked: &'a PickedFile,
) -> Result<(&'a str, &'a str), String> {
    let Workspace::Ready(loaded) = workspace else {
        return Err(NO_IMAGE_FOR_BLUEPRINT.to_owned());
    };

    std::str::from_utf8(&picked.bytes)
        .map(|text| (text, loaded.tileset.stem.as_str()))
        .map_err(|_| format!("{} is not text", picked.name))
}

fn show_acknowledgements(ui: &mut Ui) {
    ui.strong("Acknowledgements");
    show_sentence(ui, |ui| {
        ui.label("A Rust rewrite and extension of a PyQt6 ");
        ui.hyperlink_to("project", ORIGINAL_PROJECT);
        ui.label(" by Assa.");
    });
    show_sentence(ui, |ui| {
        ui.label("Rules are exported via ");
        ui.hyperlink_to("r++", RPP);
        ui.label(" by HiPulsar, which enables features such as grouped tiles.");
    });
    show_sentence(ui, |ui| {
        ui.label("The preview map dm1 is from ");
        ui.hyperlink_to("teeworlds-maps", DM1_SOURCE);
        ui.label(", under ");
        ui.hyperlink_to("CC-BY-SA 3.0", DM1_LICENSE);
        ui.label(".");
    });
}

fn show_drop_hint(ctx: &Context) {
    if ctx.input(|input| input.raw.hovered_files.is_empty()) {
        return;
    }

    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("drop_hint")));
    let screen = ctx.content_rect();
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(DROP_OVERLAY_ALPHA));
    painter.text(
        screen.center(),
        Align2::CENTER_CENTER,
        DROP_HINT,
        FontId::proportional(DROP_HINT_SIZE),
        Color32::WHITE,
    );
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
            Self::Signed | Self::Full => format!("{GITHUB_ICON} {CREDIT}"),
            Self::Short => format!("{GITHUB_ICON} {SHORT_CREDIT}"),
            Self::Icon => GITHUB_ICON.to_string(),
        }
    }

    fn signature(self) -> Option<&'static str> {
        matches!(self, Self::Signed).then_some(AUTHOR_CREDIT)
    }
}

fn text_width(ui: &Ui, text: &str) -> f32 {
    egui::WidgetText::from(text)
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::FontSelection::Default,
        )
        .size()
        .x
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
    let link = ui.hyperlink_to(form.link_text(), REPOSITORY);
    if matches!(form, CreditForm::Icon) {
        link.on_hover_text(CREDIT);
    }
}

fn show_sentence(ui: &mut Ui, add_words: impl FnOnce(&mut Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        add_words(ui);
    });
}

fn show_rule_set_tabs(ui: &mut Ui, rule_sets: &RuleSets, command: &mut Option<RuleSetCommand>) {
    ui.horizontal_wrapped(|ui| {
        for index in 0..rule_sets.len() {
            let name = rule_sets.name(index).unwrap_or_default();
            let shared = rule_sets.is_name_shared(index);
            let label = egui::RichText::new(name);
            let label = if shared {
                label.color(ui.visuals().error_fg_color)
            } else {
                label
            };
            let response = ui.selectable_label(index == rule_sets.active_index(), label);
            if shared {
                response.clone().on_hover_text(SHARED_NAME_HINT);
            }
            if response.clicked() {
                *command = Some(RuleSetCommand::Select(index));
            }
        }

        if ui.small_button("+").on_hover_text("Add rule set").clicked() {
            *command = Some(RuleSetCommand::Add);
        }
    });
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
    if tile == MASK_TILE {
        return format!("Tile {tile} · reserved as mask");
    }
    if tile == AIR_TILE {
        return format!("Tile {tile} · reserved for air");
    }

    match tile_state(tileset, project, tile) {
        TileState::Locked => format!("Tile {tile} · locked"),
        TileState::Grouped => format!("Tile {tile} · in a group"),
        TileState::Deactivated(None) => format!("Tile {tile} · deactivated"),
        TileState::Deactivated(Some(rule)) => format!(
            "Tile {tile} · {} · deactivated",
            format_neighborhood(rule.neighborhood)
        ),
        TileState::Unconfigured => format!("Tile {tile}"),
        TileState::Configured(rule) => {
            let neighborhood = format_neighborhood(rule.neighborhood);
            match rule.chance.is_full() {
                true => format!("Tile {tile} · {neighborhood}"),
                false => format!("Tile {tile} · {neighborhood} · {}%", rule.chance.percent()),
            }
        }
    }
}

fn describe_counts(project: &Project) -> String {
    let tiles = match project.deactivated_rule_count() {
        0 => format!("{} tiles", project.rule_count()),
        deactivated => format!("{} tiles ({deactivated} deactivated)", project.rule_count()),
    };

    format!("{tiles}, {} groups configured", project.groups().len())
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

fn preview_result(rules: String, sample: Sample, seed: u32) -> PreviewResult {
    match preview::automap(&rules, sample, seed) {
        Ok(automapped) => PreviewResult::Ready { rules, automapped },
        Err(error) => PreviewResult::Failed(error.to_string()),
    }
}

fn render_source(rule_sets: &RuleSets, image_stem: &str) -> Result<String, ExportError> {
    let tiles: Vec<Vec<(usize, TileRule)>> = rule_sets
        .sets()
        .iter()
        .map(|(_, project)| project.all_rules())
        .collect();
    let deactivated: Vec<Vec<usize>> = rule_sets
        .sets()
        .iter()
        .map(|(_, project)| project.deactivated_tiles())
        .collect();

    let sets: Vec<RuleSet> = rule_sets
        .sets()
        .iter()
        .zip(tiles.iter().zip(&deactivated))
        .map(|((name, project), (tiles, deactivated))| RuleSet {
            image_stem,
            name,
            tiles,
            deactivated,
            groups: project.groups(),
            chance_mode: project.chance_mode(),
        })
        .collect();

    r_source::render(&sets)
}

// We have a separate function which only considers the active rule set, so it works
// regardless of errors in other rule sets.
fn render_active_source(rule_sets: &RuleSets, image_stem: &str) -> Result<String, ExportError> {
    let project = rule_sets.active();
    let tiles = project.all_rules();
    let deactivated = project.deactivated_tiles();
    let rule_set = RuleSet {
        image_stem,
        name: rule_sets.active_name(),
        tiles: &tiles,
        deactivated: &deactivated,
        groups: project.groups(),
        chance_mode: project.chance_mode(),
    };

    r_source::render(std::slice::from_ref(&rule_set))
}
