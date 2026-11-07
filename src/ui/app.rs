use std::mem;

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
use crate::model::pool::{self, Pool};
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Chance, TILESET_SIDE, TileRule};
use crate::preview::{CustomShape, Sample, placing_rule};
use crate::tileset::{self, Tileset};
use crate::ui::discard::{self, DiscardDecision, Leaving};
use crate::ui::export::{self, ExportContext, ExportRequest};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::group_panel::GroupPanel;
use crate::ui::help;
use crate::ui::history::History;
use crate::ui::overview::{self, Overview};
use crate::ui::picking;
use crate::ui::preview::ClickedTile;
use crate::ui::status::StatusLine;
use crate::ui::tile_panel::{CopiedRule, FlashKind, GridFlash, TileContext, TileEdit, TilePanel};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};
use crate::ui::welcome::{self, WelcomeAction};
use crate::ui::{
    ADD_ICON, APP_NAME, BUTTON_GAP, DUPLICATE_ICON, REMOVE_ICON, SWITCH_ICON, compact_button,
    icon_button_size, section_header, section_header_with,
};

mod menu_bar;
mod preview_panel;
mod toolbar;

use preview_panel::{PreviewResult, preview_result};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const MIN_SIDE_PANEL_WIDTH: f32 = 240.0;
const MAX_SIDE_PANEL_WIDTH: f32 = 360.0;
const MIN_SPLIT_PANEL_WIDTH: f32 = 200.0;
const DEFAULT_SPLIT_RATIO: f32 = 0.5;
const SPLIT_PANEL_ID: &str = "tileset";
const MERGE_WIDTH: f32 = 360.0;
const GROUP_SWATCH: Vec2 = Vec2::new(12.0, 18.0);
const GROUP_SWATCH_CORNER_RADIUS: f32 = 2.0;
const GROUP_ROW_SPACING: f32 = 2.0;
const GROUP_SWATCH_SPACING: f32 = 4.0;
const DROP_LINE_WIDTH: f32 = 2.0;
const FIRST_PREVIEW_SEED: u32 = 42;
const FIRST_RULE: usize = 0;
const DROP_OVERLAY_ALPHA: u8 = 160;
const DROP_HINT_SIZE: f32 = 24.0;
const FLASH_SECONDS: f64 = 0.4;

const SHARED_NAME_HINT: &str = "Another rule set has this name";
const DEFAULT_RULE_SET_NAME: &str = "default";
const RULE_SET_SECTION: &str = "Rule set";
const OVERVIEW_SECTION: &str = "Overview";
const TILE_SECTION: &str = "Tile";
const GROUP_SECTION: &str = "Group";
const GROUPS_SECTION: &str = "Groups";
const GROUP_ORDER_NOTE: &str = "top one is applied first";
const DROP_HINT: &str = "Drop a tileset image or a blueprint";
const NO_IMAGE_FOR_BLUEPRINT: &str = "Open a tileset image first, then load the blueprint.";
const MERGE_HEADING: &str = "Merge blueprint";
const MERGE_WARNING: &str = "Tile numbers are taken over as they are.";
const MERGE_CONFIRM: &str = "Merge anyway";
const MERGE_CANCELLED: &str = "Merge cancelled";
const LOAD_HEADING: &str = "Load blueprint";
const LOAD_WARNING: &str = "Tile numbers are taken over as they are.";
const LOAD_CONFIRM: &str = "Load anyway";
const LOAD_CANCELLED: &str = "Load cancelled";
const BLUEPRINT_CANCEL: &str = "Cancel";
const UNDONE: &str = "Undone";
const REDONE: &str = "Redone";
const NOTHING_TO_UNDO: &str = "Nothing to undo";
const NOTHING_TO_REDO: &str = "Nothing to redo";
const CLOSE_PENDING: &str = "Please answer the open question before opening more files.";
const OPEN_PENDING: &str = "Please answer the open question before closing.";
const UNSAVED_MARK: char = '*';

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
    fn section(&self) -> &'static str {
        match self {
            Self::Empty => OVERVIEW_SECTION,
            Self::Tile(_) => TILE_SECTION,
            Self::Group(_) => GROUP_SECTION,
        }
    }

    fn tile(&self) -> Option<usize> {
        match self {
            Self::Tile(panel) => Some(panel.tile()),
            _ => None,
        }
    }

    fn rule(&self) -> Option<(usize, usize)> {
        match self {
            Self::Tile(panel) => Some((panel.tile(), panel.index())),
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

#[derive(Default)]
struct SaveTag {
    snapshot: Option<RuleSets>,
    not_opened: Option<String>,
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

enum Replacement {
    Files(Vec<PickedFile>),
    Close,
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
    rule_sets: RuleSets,
}

struct LoadedTileset {
    tileset: Tileset,
    texture: TextureHandle,
}

pub struct AutomapperApp {
    workspace: Workspace,
    rule_sets: RuleSets,
    history: History,
    saved: RuleSets,
    title: String,
    picker: FilePicker,
    merge_picker: FilePicker,
    pending_blueprint: Option<PendingBlueprint>,
    pending_discard: Option<Replacement>,
    allow_close: bool,
    saver: FileSaver<SaveTag>,
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
    drag_anchor: Option<usize>,
    peeking: bool,
    hovered_tile: Option<usize>,
    export_open: bool,
    export_selection: Vec<bool>,
    help_open: bool,
}

impl Default for AutomapperApp {
    fn default() -> Self {
        Self {
            workspace: Workspace::NoImage,
            rule_sets: RuleSets::default(),
            history: History::new(RuleSets::default()),
            saved: RuleSets::default(),
            title: String::new(),
            picker: FilePicker::new(),
            merge_picker: FilePicker::new(),
            pending_blueprint: None,
            pending_discard: None,
            allow_close: false,
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
            drag_anchor: None,
            peeking: false,
            hovered_tile: None,
            export_open: false,
            export_selection: Vec::new(),
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
        self.intercept_close(&ctx);

        if self.drag_anchor.is_none() {
            self.peeking = self.is_peeking(&ctx);
        }
        let displayed_view = peeked_view(self.view, self.peeking);

        self.show_menu_bar(ui);
        self.show_status_bar(ui);
        self.show_side_panel(ui);
        self.show_view(ui, &ctx, displayed_view);
        self.show_export(&ctx);
        if self.help_open && help::show_until_closed(&ctx) {
            self.help_open = false;
        }
        self.show_blueprint_confirmation(&ctx);
        self.show_discard_confirmation(&ctx);
        show_drop_hint(&ctx);
        self.follow_picking(&ctx);
        self.commit_history(&ctx);
        self.update_title(&ctx);

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
            && !has_modal(ctx)
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
        if picking::show(ctx, view, &picking::neighbor_heading(label)) {
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
            .frame(egui::Frame::central_panel(ui.style()))
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
        let replaces = TILESET_IMAGE.matches(&picked.name) || BLUEPRINT.matches(&picked.name);
        if !replaces || !(self.is_dirty() || self.pending_discard.is_some()) {
            return self.open_file(ctx, picked);
        }

        match &mut self.pending_discard {
            Some(Replacement::Files(files)) => files.push(picked),
            Some(Replacement::Close) => self.status.warning(ctx, CLOSE_PENDING),
            None => self.pending_discard = Some(Replacement::Files(vec![picked])),
        }
    }

    fn open_file(&mut self, ctx: &Context, picked: PickedFile) {
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
        self.track_from_here();
        self.inspector = Inspector::Empty;
        self.drag_anchor = None;
        self.status.info(ctx, format!("Loaded {file_name}"));
    }

    fn track_from_here(&mut self) {
        self.history.reset(self.rule_sets.clone());
        self.saved = self.rule_sets.clone();
    }

    fn is_dirty(&self) -> bool {
        self.rule_sets.sets() != self.saved.sets()
    }

    fn commit_history(&mut self, ctx: &Context) {
        if !is_editing(ctx) {
            self.history.commit(&self.rule_sets);
        }
    }

    fn update_title(&mut self, ctx: &Context) {
        let title = window_title(&self.workspace, self.is_dirty());
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn intercept_close(&mut self, ctx: &Context) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && self.is_dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            match self.pending_discard {
                Some(Replacement::Files(_)) => self.status.warning(ctx, OPEN_PENDING),
                _ => self.pending_discard = Some(Replacement::Close),
            }
        }
    }

    fn show_discard_confirmation(&mut self, ctx: &Context) {
        let leaving = match self.pending_discard {
            None => return,
            Some(Replacement::Close) => Leaving::Quit,
            Some(Replacement::Files(_)) => Leaving::OpenFiles,
        };
        let Some(decision) = discard::show(ctx, leaving) else {
            return;
        };

        match (decision, self.pending_discard.take()) {
            (DiscardDecision::Save, Some(Replacement::Files(files))) => {
                let names: Vec<&str> = files.iter().map(|picked| picked.name.as_str()).collect();
                self.save_blueprint(ctx, Some(names.join(", ")));
            }
            (DiscardDecision::Save, _) => self.save_blueprint(ctx, None),
            (DiscardDecision::Discard, Some(Replacement::Files(files))) => {
                for picked in files {
                    self.open_file(ctx, picked);
                }
            }
            (DiscardDecision::Discard, Some(Replacement::Close)) => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }

    fn undo(&mut self, ctx: &Context) {
        match self.history.undo(&self.rule_sets) {
            Some(restored) => self.restore(ctx, restored, UNDONE),
            None => self.status.info(ctx, NOTHING_TO_UNDO),
        }
    }

    fn redo(&mut self, ctx: &Context) {
        match self.history.redo(&self.rule_sets) {
            Some(restored) => self.restore(ctx, restored, REDONE),
            None => self.status.info(ctx, NOTHING_TO_REDO),
        }
    }

    fn restore(&mut self, ctx: &Context, restored: RuleSets, message: &str) {
        let active = self.rule_sets.active_index();
        self.rule_sets = RuleSets::from_parts(restored.sets().to_vec(), active);
        self.drag_anchor = None;
        let previous = mem::take(&mut self.inspector);
        self.reselect(previous);
        self.status.info(ctx, message);
    }

    fn reselect(&mut self, previous: Inspector) {
        match previous {
            Inspector::Tile(panel) => {
                let last = self
                    .rule_sets
                    .active()
                    .rules_of(panel.tile())
                    .len()
                    .saturating_sub(1);
                self.select_tile(panel.tile(), panel.index().min(last));
            }
            Inspector::Group(panel) => self.select_group(panel.index()),
            Inspector::Empty => {}
        }
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

    fn save_blueprint(&mut self, ctx: &Context, not_opened: Option<String>) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = &loaded.tileset.stem;
        let tag = SaveTag {
            snapshot: Some(self.rule_sets.clone()),
            not_opened,
        };
        match blueprint::to_json(&self.rule_sets, stem) {
            Ok(text) => self
                .saver
                .save_text(BLUEPRINT, &format!("{stem}.json"), text, tag),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn receive_saved_file(&mut self, ctx: &Context) {
        let Some(result) = self.saver.poll() else {
            return;
        };

        match result {
            Ok((name, tag)) => {
                if let Some(snapshot) = tag.snapshot {
                    self.saved = snapshot;
                }
                match tag.not_opened {
                    Some(names) => self
                        .status
                        .warning(ctx, format!("Saved {name}, but didn't open {names}.")),
                    None => self.status.info(ctx, format!("Saved {name}")),
                }
            }
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn export_source(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = loaded.tileset.stem.clone();
        match render_selected_source(&self.rule_sets, &self.export_selection, &stem) {
            Ok(source) => {
                self.saver
                    .save_text(RPP_SOURCE, &format!("{stem}.r"), source, SaveTag::default())
            }
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn compile_rules(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let stem = loaded.tileset.stem.clone();
        match render_selected_source(&self.rule_sets, &self.export_selection, &stem) {
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
        let source = match render_selected_source(&self.rule_sets, &self.export_selection, &stem) {
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
            rule_sets: self.rule_sets.clone(),
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
                self.saver
                    .save_text(RULES, &file_name, rules, SaveTag::default());
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
                let tag = SaveTag {
                    snapshot: Some(pending.rule_sets),
                    ..SaveTag::default()
                };
                self.saver.save_bytes(BUNDLE, &name, archive, tag);
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
            "Loaded {} ({width}×{height}, {tile_width}×{tile_height} per tile).",
            picked.name
        );
        match width == height {
            true => self.status.info(ctx, message),
            false => self.status.warning(
                ctx,
                format!("{message} The image isn't square, so tiles will look stretched."),
            ),
        }

        self.rule_sets = RuleSets::new(DEFAULT_RULE_SET_NAME.to_owned());
        self.track_from_here();
        self.inspector = Inspector::Empty;
        self.clipboard = None;
        self.drag_anchor = None;
        self.hovered_tile = None;
        self.preview = PreviewResult::Waiting;
        self.preview_source = None;
        self.view = View::Split;
        self.view_before_picking = None;
        self.workspace = Workspace::Ready(LoadedTileset { tileset, texture });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if pressed(ctx, &open_image_shortcut()) {
            self.picker.open(TILESET_IMAGE);
        }
        if pressed(ctx, &save_blueprint_shortcut()) {
            self.save_blueprint(ctx, None);
        }
        if pressed(ctx, &export_shortcut())
            && matches!(self.workspace, Workspace::Ready(_))
            && !self.export_open
        {
            self.open_export();
        }
        if is_editing(ctx) || has_modal(ctx) {
            return;
        }

        if self.can_deselect(ctx) && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.inspector = Inspector::Empty;
        }

        if pressed(ctx, &redo_shortcut()) || pressed(ctx, &redo_alt_shortcut()) {
            self.redo(ctx);
        } else if pressed(ctx, &undo_shortcut()) {
            self.undo(ctx);
        }
    }

    fn show_status_bar(&mut self, ui: &mut Ui) {
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                self.status.show(ui);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(index) = self.hovered_tile {
                        ui.weak(format!("Tile {index}"));
                    }
                });
            });
        });
    }

    fn show_side_panel(&mut self, ui: &mut Ui) {
        let selected_group = self.inspector.group();
        let flash = self.active_flash(ui.ctx());
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
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
                section_header(ui, RULE_SET_SECTION);
                show_rule_set_row(ui, &mut self.rule_sets, &mut rule_set_command);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    section_header(ui, self.inspector.section());
                    match &mut self.inspector {
                        Inspector::Tile(panel) => {
                            let context = TileContext {
                                pools: pool::pools_of(&pools, panel.tile()),
                                mode: self.rule_sets.active().chance_mode(),
                                rule_count: self.rule_sets.active().rules_of(panel.tile()).len(),
                                deactivated: self.rule_sets.active().is_deactivated(panel.tile()),
                                copied: self.clipboard.as_ref(),
                                flash,
                            };
                            tile_edit = panel
                                .show(ui, &loaded.tileset, &loaded.texture, &context)
                                .map(|edit| (panel.tile(), edit));
                        }
                        Inspector::Group(panel) => {
                            group_edit = panel
                                .show(ui, self.rule_sets.active().groups())
                                .map(|group| (panel.index(), group));
                        }
                        Inspector::Empty => {
                            let project = self.rule_sets.active();
                            overview::show(ui, &Overview::new(project));
                        }
                    }

                    section_header_with(ui, GROUPS_SECTION, |ui| {
                        let note = egui::RichText::new(GROUP_ORDER_NOTE).small().weak();
                        ui.add(egui::Label::new(note).truncate());
                    });
                    show_group_list(
                        ui,
                        self.rule_sets.active().groups(),
                        selected_group,
                        &mut pending,
                    );
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
            TileEdit::ToggleActive => self.toggle_deactivated(ctx, tile),
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

    fn open_export(&mut self) {
        self.export_selection = vec![true; self.rule_sets.len()];
        self.export_open = true;
    }

    fn show_export(&mut self, ctx: &Context) {
        if !self.export_open {
            return;
        }

        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
        self.export_selection.resize(self.rule_sets.len(), true);
        let mut export = ExportContext {
            rule_sets: &self.rule_sets,
            selection: &mut self.export_selection,
            image_stem: &loaded.tileset.stem,
            compiling: self.compiler.is_running(),
        };
        let Some(request) = export::show(ctx, &mut export) else {
            return;
        };

        self.export_open = false;
        match request {
            ExportRequest::Rules => self.compile_rules(ctx),
            ExportRequest::Source => self.export_source(ctx),
            ExportRequest::Bundle => self.compile_bundle(ctx),
            ExportRequest::Close => {}
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
        if matches!(self.workspace, Workspace::Ready(_)) {
            self.show_tileset_header(ui);
        }

        let pools = pool::pools(&self.rule_sets.active().rules());
        let shares = pool::base_shares(&pools, self.rule_sets.active().chance_mode());
        let pool_mates = self.pool_mates(&pools);

        let response = match &self.workspace {
            Workspace::NoImage => {
                self.show_welcome(ui);
                GridResponse::default()
            }
            Workspace::Ready(loaded) => grid::show(
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
            ),
        };

        self.handle_grid(ctx, response);
    }

    fn show_welcome(&mut self, ui: &mut Ui) {
        let shortcut = ui.ctx().format_shortcut(&open_image_shortcut());
        match welcome::show(ui, &shortcut) {
            Some(WelcomeAction::OpenImage) => self.picker.open(TILESET_IMAGE),
            Some(WelcomeAction::OpenFirstSteps) => {
                help::select_getting_started(ui.ctx());
                self.help_open = true;
            }
            None => {}
        }
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
            self.status.warning(ctx, air_warning());
        }

        if let Some(tile) = response.clicked {
            self.toggle_tile_selection(tile);
        }

        if let Some(tile) = response.secondary_clicked {
            self.toggle_deactivated(ctx, tile);
        }
    }

    fn toggle_tile_selection(&mut self, tile: usize) {
        match self.inspector.tile() == Some(tile) {
            true => self.inspector = Inspector::Empty,
            false => self.select_tile(tile, FIRST_RULE),
        }
    }

    fn toggle_group_selection(&mut self, index: usize) {
        match self.inspector.group() == Some(index) {
            true => self.inspector = Inspector::Empty,
            false => self.select_group(index),
        }
    }

    fn can_deselect(&self, ctx: &Context) -> bool {
        !self.inspector.is_picking() && !egui::Popup::is_any_open(ctx)
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
            self.status.warning(ctx, air_warning());
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

    fn select_rule_of(&mut self, clicked: ClickedTile) {
        let ClickedTile { placed, around } = clicked;
        self.drag_anchor = None;
        match self.rule_sets.active().group_at(placed.index) {
            Some(index) => {
                self.define_groups = true;
                self.select_group(index);
            }
            None => {
                self.define_groups = false;
                let rule =
                    placing_rule(self.rule_sets.active(), placed, around).unwrap_or(FIRST_RULE);
                self.select_tile(placed.index, rule);
            }
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
            self.toggle_group_selection(index);
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
            Some(index) if anchor == corner => self.toggle_group_selection(index),
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
                self.status.info(ctx, format!("Tile {tile} is used again"));
            }
            _ => {
                self.rule_sets.active_mut().deactivate(tile);
                self.status.info(ctx, format!("Tile {tile} is turned off"));
            }
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

fn is_editing(ctx: &Context) -> bool {
    ctx.input(|input| input.pointer.any_down()) || ctx.egui_wants_keyboard_input()
}

fn has_modal(ctx: &Context) -> bool {
    ctx.memory(|memory| memory.top_modal_layer().is_some())
}

fn show_rule_set_row(ui: &mut Ui, rule_sets: &mut RuleSets, command: &mut Option<RuleSetCommand>) {
    let size = icon_button_size(ui);
    let can_remove = rule_sets.len() > 1;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = BUTTON_GAP;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(can_remove, compact_button(REMOVE_ICON, size))
                .on_hover_text("Remove this rule set")
                .clicked()
            {
                *command = Some(RuleSetCommand::Remove);
            }
            if ui
                .add(compact_button(DUPLICATE_ICON, size))
                .on_hover_text("Duplicate this rule set")
                .clicked()
            {
                *command = Some(RuleSetCommand::Duplicate);
            }
            if ui
                .add(compact_button(ADD_ICON, size))
                .on_hover_text("Add rule set")
                .clicked()
            {
                *command = Some(RuleSetCommand::Add);
            }

            ui.add_space(size);
            let switch = ui
                .add(compact_button(SWITCH_ICON, size))
                .on_hover_text("Switch rule set");
            egui::Popup::menu(&switch).show(|ui| show_rule_set_menu(ui, rule_sets, command));

            let shared = rule_sets.is_name_shared(rule_sets.active_index());
            let mut field = egui::TextEdit::singleline(rule_sets.active_name_mut());
            if shared {
                field = field.text_color(ui.visuals().error_fg_color);
            }
            let response = ui.add_sized(ui.available_size(), field);
            if shared {
                response.on_hover_text(SHARED_NAME_HINT);
            }
        });
    });
}

fn show_rule_set_menu(ui: &mut Ui, rule_sets: &RuleSets, command: &mut Option<RuleSetCommand>) {
    for index in 0..rule_sets.len() {
        let shared = rule_sets.is_name_shared(index);
        let label = egui::RichText::new(rule_sets.name(index).unwrap_or_default());
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
                        .on_hover_text(grid::describe_group(group));
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

fn air_warning() -> String {
    format!("Tile {AIR_TILE} is always air in DDNet, so it can't have a rule.")
}

pub(super) fn open_image_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O)
}

pub(super) fn save_blueprint_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S)
}

pub(super) fn export_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::E)
}

pub(super) fn undo_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z)
}

pub(super) fn redo_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::Z,
    )
}

pub(super) fn redo_alt_shortcut() -> egui::KeyboardShortcut {
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y)
}

fn pressed(ctx: &Context, shortcut: &egui::KeyboardShortcut) -> bool {
    ctx.input_mut(|input| input.consume_shortcut(shortcut))
}

fn window_title(workspace: &Workspace, dirty: bool) -> String {
    match (workspace, dirty) {
        (Workspace::NoImage, _) => APP_NAME.to_owned(),
        (Workspace::Ready(loaded), true) => {
            format!("{}{UNSAVED_MARK} - {APP_NAME}", loaded.tileset.stem)
        }
        (Workspace::Ready(loaded), false) => format!("{} - {APP_NAME}", loaded.tileset.stem),
    }
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

fn render_selected_source(
    rule_sets: &RuleSets,
    selection: &[bool],
    image_stem: &str,
) -> Result<String, ExportError> {
    r_source::render_rule_sets(&rule_sets.selected(selection), image_stem)
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
