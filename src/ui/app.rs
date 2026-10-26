use egui::containers::panel::PanelState;
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Rect, TextureHandle, TextureOptions, Ui,
    Vec2,
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
use crate::model::tile::{AIR_TILE, Chance, TILESET_SIDE, TileRule};
use crate::preview::{self, Automapped, CUSTOM_SIZE_RANGE, CustomShape, PlacedTile, Sample};
use crate::tileset::{self, Tileset};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::group_panel::{GroupPanel, mode_label};
use crate::ui::picking;
use crate::ui::preview as preview_view;
use crate::ui::status::StatusLine;
use crate::ui::tile_panel::{CopiedRule, FlashKind, GridFlash, TileContext, TileEdit, TilePanel};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};
use crate::ui::{ADD_ICON, DRAW_ICON, DUPLICATE_ICON, REMOVE_ICON, icon_button};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const MIN_SIDE_PANEL_WIDTH: f32 = 220.0;
const MAX_SIDE_PANEL_WIDTH: f32 = 520.0;
const MIN_SPLIT_PANEL_WIDTH: f32 = 200.0;
const DEFAULT_SPLIT_RATIO: f32 = 0.5;
const SPLIT_PANEL_ID: &str = "tileset";
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
const FIRST_RULE: usize = 0;
const DROP_OVERLAY_ALPHA: u8 = 160;
const DROP_HINT_SIZE: f32 = 24.0;
const FLASH_SECONDS: f64 = 0.4;

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
                               map the way DDNet's automapper would, Split shows both.";
const NORMALIZE_TOOLTIP: &str = "ON: chances are re-normalized to always add up to 100%\n\
                                 OFF: allows tiles to remain unchanged if summed chance is below \
                                 100%";
const ZOOM_TOOLTIP: &str = "Shortcuts: Ctrl + and Ctrl - to zoom, Ctrl 0 to reset.";
const PERCENT: f32 = 100.0;
const KEYBINDS: [(&str, &str); 8] = [
    ("Ctrl O", "Open image"),
    ("Space (hold)", "Show the other view"),
    ("1 / 2 / 3", "Set hovered neighbor to Empty / Full / Any"),
    ("C", "Copy the selected rule"),
    ("V", "Paste over the selected rule"),
    ("Ctrl click", "Pick a specific tile for a neighbor"),
    ("Ctrl + / Ctrl -", "Zoom"),
    ("Ctrl 0", "Reset zoom"),
];
const AIR_WARNING: &str = "is reserved for air: DDNet treats it as empty.";
const GENERATE_TOOLTIP: &str = "Roll the chances again with the next seed.";
const DRAW_TOOLTIP: &str = "Left click paints blocks, right click erases them.";
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

struct Flash {
    started: f64,
    kind: FlashKind,
}

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

    fn is_picking(&self) -> bool {
        matches!(self, Self::Tile(panel) if panel.is_picking())
    }

    fn picked(&self) -> Option<usize> {
        match self {
            Self::Tile(panel) => panel.picked(),
            _ => None,
        }
    }

    fn picking_label(&self) -> Option<&'static str> {
        match self {
            Self::Tile(panel) => panel.picking_label(),
            _ => None,
        }
    }

    fn stop_picking(&mut self) {
        if let Self::Tile(panel) = self {
            panel.stop_picking();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Tileset,
    Preview,
    Split,
}

enum CompileTarget {
    Rules,
    Bundle(PendingBundle),
    Preview(u8),
}

enum PreviewResult {
    Waiting,
    Ready {
        rules: String,
        unmatched_marker: u8,
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
    view_before_picking: Option<View>,
    was_picking: bool,
    split_ratio: f32,
    preview: PreviewResult,
    preview_source: Option<String>,
    preview_seed: u32,
    preview_sample: Sample,
    custom_shape: CustomShape,
    draw_custom: bool,
    inspector: Inspector,
    clipboard: Option<CopiedRule>,
    flash: Option<Flash>,
    define_groups: bool,
    always_show_turn_buttons: bool,
    drag_anchor: Option<usize>,
    peeking: bool,
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
            view_before_picking: None,
            was_picking: false,
            split_ratio: DEFAULT_SPLIT_RATIO,
            preview: PreviewResult::Waiting,
            preview_source: None,
            preview_seed: FIRST_PREVIEW_SEED,
            preview_sample: Sample::default(),
            custom_shape: CustomShape::default(),
            draw_custom: true,
            inspector: Inspector::Empty,
            clipboard: None,
            flash: None,
            define_groups: false,
            always_show_turn_buttons: false,
            drag_anchor: None,
            peeking: false,
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

        if self.drag_anchor.is_none() {
            self.peeking = self.is_peeking(&ctx);
        }
        let displayed_view = peeked_view(self.view, self.peeking);

        self.show_menu_bar(ui, &ctx, displayed_view);
        self.show_status_bar(ui);
        self.show_side_panel(ui);
        self.show_view(ui, &ctx, displayed_view);
        self.show_export(&ctx);
        self.show_help(&ctx);
        self.show_blueprint_confirmation(&ctx);
        show_drop_hint(&ctx);
        self.follow_picking(&ctx);

        if self.compiler.is_running() {
            ctx.request_repaint();
        }
    }
}

impl AutomapperApp {
    fn is_peeking(&self, ctx: &Context) -> bool {
        matches!(self.workspace, Workspace::Ready(_))
            && !self.inspector.is_picking()
            && !ctx.egui_wants_keyboard_input()
            && ctx.memory(|memory| memory.top_modal_layer().is_none())
            && ctx.input(|input| input.modifiers.is_none() && input.key_down(egui::Key::Space))
    }

    fn show_view(&mut self, ui: &mut Ui, ctx: &Context, view: View) {
        match view {
            View::Tileset => {
                let panel = egui::CentralPanel::default().show(ui, |ui| self.show_tileset(ui, ctx));
                self.show_picking(ctx, panel.response.rect);
            }
            View::Preview => {
                self.hovered_tile = None;
                egui::CentralPanel::default().show(ui, |ui| self.show_preview(ui));
            }
            View::Split => {
                let tileset_rect = self.show_tileset_panel(ui, ctx);
                self.show_picking(ctx, tileset_rect);
                egui::CentralPanel::default().show(ui, |ui| self.show_preview(ui));
            }
        }
    }

    fn show_picking(&mut self, ctx: &Context, view: Rect) {
        let Some(label) = self.inspector.picking_label() else {
            return;
        };
        if picking::show(ctx, view, label) {
            self.inspector.stop_picking();
        }
    }

    fn follow_picking(&mut self, ctx: &Context) {
        let picking = self.inspector.is_picking();
        let started = picking && !self.was_picking;
        self.was_picking = picking;

        let target = match (started, picking, self.view_before_picking) {
            (true, _, _) if self.view == View::Preview => {
                self.view_before_picking = Some(View::Preview);
                View::Tileset
            }
            (_, false, Some(view)) => {
                self.view_before_picking = None;
                view
            }
            _ => return,
        };

        self.view = target;
        ctx.request_repaint();
    }

    fn show_tileset_panel(&mut self, ui: &mut Ui, ctx: &Context) -> Rect {
        let available = ui.available_rect_before_wrap();
        let panel_id = Id::new(SPLIT_PANEL_ID);
        let wanted_width = self.split_ratio * available.width();
        let max_width = available.width() - MIN_SPLIT_PANEL_WIDTH;
        let state = PanelState {
            outer_rect: Rect::from_min_max(
                available.right_top() - Vec2::X * wanted_width,
                available.max,
            ),
        };
        ctx.data_mut(|data| data.insert_persisted(panel_id, state));

        let panel = egui::Panel::right(panel_id)
            .resizable(true)
            .min_size(MIN_SPLIT_PANEL_WIDTH)
            .max_size(max_width)
            .show(ui, |ui| self.show_tileset(ui, ctx));
        let width = panel.response.rect.width();
        let clamped_by_window = (width <= MIN_SPLIT_PANEL_WIDTH
            && wanted_width < MIN_SPLIT_PANEL_WIDTH)
            || (width >= max_width && wanted_width > max_width);
        if available.width() > 0.0 && !clamped_by_window {
            self.split_ratio = width / available.width();
        }

        panel.response.rect
    }

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

        if parsed.image != stem {
            self.pending_blueprint = Some(PendingBlueprint {
                file_name: picked.name,
                wanted_image: parsed.image,
                rule_sets: parsed.rule_sets,
                action,
            });
            return;
        }

        self.apply_blueprint(ctx, action, &picked.name, parsed.rule_sets);
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
            (CompileTarget::Preview(unmatched_marker), Ok(Compiled { rules, .. })) => {
                self.preview = preview_result(
                    rules,
                    self.preview_sample,
                    &self.custom_shape,
                    self.preview_seed,
                    unmatched_marker,
                );
            }
            (CompileTarget::Preview(_), Err(error)) => {
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
        self.clipboard = None;
        self.drag_anchor = None;
        self.hovered_tile = None;
        self.preview = PreviewResult::Waiting;
        self.preview_source = None;
        self.view = View::Tileset;
        self.view_before_picking = None;
        self.workspace = Workspace::Ready(LoadedTileset { tileset, texture });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|input| input.consume_shortcut(&open_image_shortcut())) {
            self.picker.open(TILESET_IMAGE);
        }
    }

    fn show_menu_bar(&mut self, ui: &mut Ui, ctx: &Context, displayed_view: View) {
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

                let options_config = egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
                egui::containers::menu::MenuButton::new("Options")
                    .config(options_config)
                    .ui(ui, |ui| self.show_options(ui));

                if ui.button("Help").clicked() {
                    self.help_open = true;
                }

                ui.separator();
                self.show_view_picker(ui);
                ui.separator();
                match displayed_view {
                    View::Tileset => self.show_selection_picker(ui),
                    View::Preview => self.show_preview_controls(ui),
                    View::Split => {
                        self.show_selection_picker(ui);
                        ui.separator();
                        self.show_preview_controls(ui);
                    }
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
        let previous = self.view;

        ui.add_enabled_ui(has_image, |ui| {
            ui.selectable_value(&mut self.view, View::Tileset, "Tileset");
            ui.selectable_value(&mut self.view, View::Preview, "Preview");
            ui.selectable_value(&mut self.view, View::Split, "Split");
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

        ui.checkbox(
            &mut self.always_show_turn_buttons,
            "Always show orientation buttons",
        );

        if ui.ctx().options(|options| options.zoom_with_keyboard) {
            show_zoom(ui);
        }
    }

    fn show_preview_controls(&mut self, ui: &mut Ui) {
        let ready = matches!(self.preview, PreviewResult::Ready { .. });

        let mut chosen = self.preview_sample;
        ui.label("Map:");
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
            .add_enabled(ready, egui::Button::new("Re-generate"))
            .on_hover_text(GENERATE_TOOLTIP)
            .clicked()
        {
            self.regenerate_preview();
        }
        ui.weak(format!("seed {}", self.preview_seed));

        if self.preview_sample == Sample::Custom {
            ui.separator();
            self.show_custom_controls(ui);
        }
    }

    fn show_custom_controls(&mut self, ui: &mut Ui) {
        let mut width = self.custom_shape.width();
        let mut height = self.custom_shape.height();

        ui.label("Size:");
        ui.add(egui::DragValue::new(&mut width).range(CUSTOM_SIZE_RANGE));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut height).range(CUSTOM_SIZE_RANGE));
        if (width, height) != (self.custom_shape.width(), self.custom_shape.height()) {
            self.custom_shape.resize(width, height);
            self.reautomap();
        }

        if ui.button("Clear").clicked() {
            self.custom_shape.clear();
            self.reautomap();
        }
        ui.toggle_value(&mut self.draw_custom, format!("{DRAW_ICON} Draw"))
            .on_hover_text(DRAW_TOOLTIP);
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
        let flash = self.active_flash(ui.ctx());
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
                                .add_enabled(can_remove, icon_button(REMOVE_ICON))
                                .on_hover_text("Remove this rule set")
                                .clicked()
                            {
                                rule_set_command = Some(RuleSetCommand::Remove);
                            }
                            if ui
                                .add(icon_button(DUPLICATE_ICON))
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
                                let context = TileContext {
                                    pools: pool::pools_of(&pools, panel.tile()),
                                    mode: self.rule_sets.active().chance_mode(),
                                    rule_count: self
                                        .rule_sets
                                        .active()
                                        .rules_of(panel.tile())
                                        .len(),
                                    always_show_turn_buttons: self.always_show_turn_buttons,
                                    copied: self.clipboard.as_ref(),
                                    flash,
                                };
                                let deactivated =
                                    self.rule_sets.active().is_deactivated(panel.tile());
                                tile_edit = panel
                                    .show(
                                        ui,
                                        &loaded.tileset,
                                        &loaded.texture,
                                        &context,
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
            self.apply_tile_edit(ui.ctx(), tile, edit);
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

    fn active_flash(&mut self, ctx: &Context) -> Option<GridFlash> {
        let flash = self.flash.as_ref()?;

        let elapsed = ctx.input(|input| input.time) - flash.started;
        if elapsed >= FLASH_SECONDS {
            self.flash = None;
            return None;
        }

        ctx.request_repaint();
        Some(GridFlash {
            kind: flash.kind,
            remaining: (1.0 - elapsed / FLASH_SECONDS) as f32,
        })
    }

    fn start_flash(&mut self, ctx: &Context, kind: FlashKind) {
        let started = ctx.input(|input| input.time);
        self.flash = Some(Flash { started, kind });
        ctx.request_repaint();
    }

    fn apply_tile_edit(&mut self, ctx: &Context, tile: usize, edit: TileEdit) {
        match edit {
            TileEdit::Apply { index, rule } => self.store_rule(tile, index, rule),
            TileEdit::Select(index) => self.select_tile(tile, index),
            TileEdit::Add => {
                if let Some(rule) = self.seeded_rule(tile) {
                    self.append_rule(tile, rule);
                }
            }
            TileEdit::Duplicate(index) => {
                if let Some(&rule) = self.rule_sets.active().rules_of(tile).get(index) {
                    self.append_rule(tile, rule);
                }
            }
            TileEdit::Remove(index) => {
                self.rule_sets.active_mut().remove_rule(tile, index);
                self.select_tile(tile, index.saturating_sub(1));
            }
            TileEdit::Copy(index) => self.copy_rule(ctx, tile, index),
            TileEdit::Paste(index) => self.paste_rule(ctx, tile, index),
        }
    }

    fn copy_rule(&mut self, ctx: &Context, tile: usize, index: usize) {
        if let Some(&rule) = self.rule_sets.active().rules_of(tile).get(index) {
            self.clipboard = Some(CopiedRule { rule, tile, index });
            self.start_flash(ctx, FlashKind::Copy);
            self.status.info(ctx, format!("Copied tile {tile}"));
        }
    }

    fn paste_rule(&mut self, ctx: &Context, tile: usize, index: usize) {
        let Some(copied) = &self.clipboard else {
            return;
        };

        let existing = self.rule_sets.active().rules_of(tile);
        let target = index.min(existing.len());
        let rule = match existing.get(target) {
            Some(replaced) => TileRule {
                drawn: replaced.drawn,
                ..copied.rule
            },
            None => copied.rule,
        };
        let source = copied.tile;
        self.store_rule(tile, target, rule);
        self.select_tile(tile, target);
        self.start_flash(ctx, FlashKind::Paste);
        self.status
            .info(ctx, format!("Pasted tile {source} onto tile {tile}"));
    }

    fn store_rule(&mut self, tile: usize, index: usize, rule: TileRule) {
        let project = self.rule_sets.active_mut();
        match index < project.rules_of(tile).len() {
            true => project.replace_rule(tile, index, rule),
            false => {
                project.add_rule(tile, rule);
            }
        }
    }

    fn append_rule(&mut self, tile: usize, rule: TileRule) {
        let index = self.rule_sets.active_mut().add_rule(tile, rule);
        self.select_tile(tile, index);
    }

    fn seeded_rule(&self, tile: usize) -> Option<TileRule> {
        match &self.workspace {
            Workspace::Ready(loaded) => Some(seed_rule(&loaded.tileset, tile)),
            Workspace::NoImage => None,
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
        let Some(primary) = pool::pools_of(pools, tile).first().copied() else {
            return Vec::new();
        };

        primary
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

        let response = match &self.workspace {
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
                            picked: self.inspector.picked(),
                        },
                    )
                })
                .inner
            }
        };

        self.handle_grid(ctx, response);
    }

    fn regenerate_preview(&mut self) {
        self.preview_seed += 1;
        self.reautomap();
    }

    fn switch_sample(&mut self, sample: Sample) {
        self.preview_sample = sample;
        if !matches!(self.preview, PreviewResult::Ready { .. }) {
            self.preview_source = None;
        }
        self.reautomap();
    }

    fn reautomap(&mut self) {
        let PreviewResult::Ready {
            rules,
            unmatched_marker,
            ..
        } = &self.preview
        else {
            return;
        };

        self.preview = preview_result(
            rules.clone(),
            self.preview_sample,
            &self.custom_shape,
            self.preview_seed,
            *unmatched_marker,
        );
    }

    fn show_preview(&mut self, ui: &mut Ui) {
        self.refresh_preview();

        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let drawing = (self.preview_sample == Sample::Custom && self.draw_custom)
            .then_some(&mut self.custom_shape);
        let mut interaction = preview_view::Interaction::default();
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
                    interaction = preview_view::show(
                        ui,
                        &loaded.tileset,
                        &loaded.texture,
                        automapped,
                        drawing,
                    );
                });
            }
        }

        if interaction.edited {
            self.reautomap();
            ui.ctx().request_repaint();
        }
        if let Some(placed) = interaction.clicked {
            self.select_rule_of(placed);
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
        let unmatched_marker = preview::unmatched_marker(self.rule_sets.active());
        self.start_compile(source, &stem, CompileTarget::Preview(unmatched_marker));
    }

    fn handle_grid(&mut self, ctx: &Context, response: GridResponse) {
        self.hovered_tile = response.hovered;

        if !matches!(self.workspace, Workspace::Ready(_)) {
            return;
        }

        if self.define_groups {
            self.handle_group_grid(ctx, response);
            return;
        }

        if self.inspector.is_picking() {
            self.pick_tile(ctx, response);
            return;
        }

        if response.clicked == Some(AIR_TILE) {
            self.status
                .warning(ctx, format!("Tile {AIR_TILE} {AIR_WARNING}"));
        }

        if let Some(tile) = response.clicked {
            self.select_tile(tile, FIRST_RULE);
        }

        if let Some(tile) = response.secondary_clicked {
            self.toggle_deactivated(ctx, tile);
        }
    }

    fn pick_tile(&mut self, ctx: &Context, response: GridResponse) {
        let Inspector::Tile(panel) = &mut self.inspector else {
            return;
        };
        if response.secondary_clicked.is_some() {
            panel.stop_picking();
            return;
        }

        let Some(tile) = response.clicked else {
            return;
        };
        if tile == AIR_TILE {
            self.status
                .warning(ctx, format!("Tile {AIR_TILE} {AIR_WARNING}"));
            return;
        }
        if let Some(edit) = panel.pick(tile) {
            let owner = panel.tile();
            self.apply_tile_edit(ctx, owner, edit);
        }
    }

    fn select_tile(&mut self, tile: usize, index: usize) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
        if !tile_state(&loaded.tileset, self.rule_sets.active(), tile).is_editable() {
            return;
        }

        let (rule, has_rule) = match self.rule_sets.active().rules_of(tile).get(index) {
            Some(&rule) => (rule, true),
            None => (seed_rule(&loaded.tileset, tile), false),
        };
        self.inspector = Inspector::Tile(TilePanel::new(tile, index, rule, has_rule));
    }

    fn select_rule_of(&mut self, placed: PlacedTile) {
        self.drag_anchor = None;
        match self.rule_sets.active().group_at(placed.index) {
            Some(index) => {
                self.define_groups = true;
                self.select_group(index);
            }
            None => {
                self.define_groups = false;
                let rule = self.rule_drawing(placed);
                self.select_tile(placed.index, rule);
            }
        }
    }

    fn rule_drawing(&self, placed: PlacedTile) -> usize {
        self.rule_sets
            .active()
            .rules_of(placed.index)
            .iter()
            .position(|rule| {
                rule.centre == placed.centre
                    && rule
                        .orientations()
                        .transforms()
                        .into_iter()
                        .any(|relative| relative.after(rule.drawn) == placed.transform)
            })
            .unwrap_or(FIRST_RULE)
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
            show_keybinds(ui);

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

fn peeked_view(selected: View, peeking: bool) -> View {
    match (selected, peeking) {
        (view, false) => view,
        (View::Tileset | View::Split, true) => View::Preview,
        (View::Preview, true) => View::Tileset,
    }
}

fn show_zoom(ui: &mut Ui) {
    let ctx = ui.ctx().clone();

    egui::containers::Sides::new().show(
        ui,
        |ui| {
            ui.label("Zoom").on_hover_text(ZOOM_TOOLTIP);
        },
        |ui| {
            if zoom_button(ui, "+") {
                egui::gui_zoom::zoom_in(&ctx);
            }
            ui.label(format!("{:.0}%", ctx.zoom_factor() * PERCENT))
                .on_hover_text(ZOOM_TOOLTIP);
            if zoom_button(ui, "-") {
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

fn show_keybinds(ui: &mut Ui) {
    ui.strong("Keybinds");
    egui::Grid::new("keybinds").show(ui, |ui| {
        for (keys, action) in KEYBINDS {
            ui.label(keys);
            ui.label(action);
            ui.end_row();
        }
    });
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

        if ui
            .small_button(ADD_ICON)
            .on_hover_text("Add rule set")
            .clicked()
        {
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
    if tile == AIR_TILE {
        return format!("Tile {tile} · reserved for air");
    }

    let text = match tile_state(tileset, project, tile) {
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
    };

    match project.rules_of(tile).len() {
        0 | 1 => text,
        count => format!("{text} · {count} rules"),
    }
}

fn describe_counts(project: &Project) -> String {
    let rules = match project.deactivated_rule_count() {
        0 => format!("{} rules", project.rule_count()),
        deactivated => format!("{} rules ({deactivated} deactivated)", project.rule_count()),
    };

    format!("{rules}, {} groups configured", project.groups().len())
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
        NeighborState::Is(_) => '=',
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

fn preview_result(
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
