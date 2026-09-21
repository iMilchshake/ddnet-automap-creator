use egui::{Context, TextureHandle, TextureOptions, Ui, Vec2};

use crate::export::r_source::{self, RuleSet};
use crate::file_picker::{FilePicker, PickedFile};
use crate::file_saver::FileSaver;
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, TILESET_SIDE};
use crate::tileset::{self, Tileset};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::group_dialog::{GroupAction, GroupDialog, mode_label};
use crate::ui::status::StatusLine;
use crate::ui::tile_dialog::{DialogAction, TileDialog};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const SWATCH_SIZE: f32 = 12.0;

enum GroupCommand {
    Configure(usize),
    Raise(usize),
    Lower(usize),
    Remove(usize),
}

enum Workspace {
    NoImage,
    Ready(LoadedTileset),
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
    saver: FileSaver,
    status: StatusLine,
    dialog: Option<TileDialog>,
    group_dialog: Option<GroupDialog>,
    define_groups: bool,
    drag_anchor: Option<usize>,
    hovered_tile: Option<usize>,
}

impl Default for AutomapperApp {
    fn default() -> Self {
        Self {
            workspace: Workspace::NoImage,
            project: Project::default(),
            rule_set_name: String::new(),
            picker: FilePicker::new(),
            saver: FileSaver::new(),
            status: StatusLine::default(),
            dialog: None,
            group_dialog: None,
            define_groups: false,
            drag_anchor: None,
            hovered_tile: None,
        }
    }
}

impl eframe::App for AutomapperApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.receive_picked_file(&ctx);
        self.receive_saved_file(&ctx);
        self.handle_shortcuts(&ctx);

        self.show_menu_bar(ui);
        self.show_status_bar(ui);
        self.show_side_panel(ui, &ctx);
        self.show_tileset(ui, &ctx);
        self.show_dialog(&ctx);
        self.show_group_dialog(&ctx);
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

    fn receive_saved_file(&mut self, ctx: &Context) {
        let Some(result) = self.saver.poll() else {
            return;
        };

        match result {
            Ok(name) => self.status.info(ctx, format!("Saved {name}")),
            Err(error) => self.status.warning(ctx, error.to_string()),
        }
    }

    fn export_rules(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        let rules = self.project.rules();
        let rule_set = RuleSet {
            image_stem: &loaded.tileset.stem,
            name: &self.rule_set_name,
            tiles: &rules,
            groups: self.project.groups(),
        };

        match r_source::render(&rule_set) {
            Ok(source) => self
                .saver
                .save_text(&format!("{}.r", loaded.tileset.stem), source),
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
        self.dialog = None;
        self.group_dialog = None;
        self.drag_anchor = None;
        self.hovered_tile = None;
        self.workspace = Workspace::Ready(LoadedTileset { tileset, texture });
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        if ctx.input_mut(|input| input.consume_shortcut(&open_image_shortcut())) {
            self.picker.open_image();
        }
    }

    fn show_menu_bar(&mut self, ui: &mut Ui) {
        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    let open = egui::Button::new("Open image…")
                        .shortcut_text(ui.ctx().format_shortcut(&open_image_shortcut()));
                    if ui.add(open).clicked() {
                        self.picker.open_image();
                        ui.close();
                    }

                    ui.separator();

                    // TODO: blueprint save/load.
                    ui.add_enabled(false, egui::Button::new("Load blueprint…"));
                    ui.add_enabled(false, egui::Button::new("Save blueprint…"));
                });

                ui.menu_button("Help", |ui| {
                    ui.label(concat!(
                        "SimpleDDNetAutomapper ",
                        env!("CARGO_PKG_VERSION"),
                        "\nGenerates rpp sources for DDNet automappers."
                    ));
                });
            });
        });
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

    fn show_side_panel(&mut self, ui: &mut Ui, ctx: &Context) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));
        let mut export = false;
        let mut pending = None;

        egui::Panel::right("tools")
            .resizable(false)
            .exact_size(SIDE_PANEL_WIDTH)
            .show(ui, |ui| {
                ui.add_enabled_ui(has_image, |ui| {
                    ui.heading("Generator");
                    ui.horizontal(|ui| {
                        ui.label("Rule set");
                        ui.text_edit_singleline(&mut self.rule_set_name);
                    });
                    ui.label(format!(
                        "{} tiles, {} groups configured",
                        self.project.rule_count(),
                        self.project.groups().len()
                    ));

                    ui.add_space(4.0);
                    let exportable =
                        self.project.rule_count() > 0 || !self.project.groups().is_empty();
                    if ui
                        .add_enabled(exportable, egui::Button::new("Export .r…"))
                        .clicked()
                    {
                        export = true;
                    }

                    ui.separator();

                    ui.heading("Group editor");
                    ui.checkbox(&mut self.define_groups, "Define groups")
                        .on_hover_text(
                            "Drag across the tileset to add a group, click one to edit it, \
                             right-click to remove it.",
                        );
                    ui.weak(
                        "Groups are placed top to bottom, and whoever comes first gets first pick.",
                    );

                    ui.add_space(4.0);
                    self.show_group_list(ui, &mut pending);
                });
            });

        match pending {
            Some(GroupCommand::Configure(index)) => self.open_group_dialog(index),
            Some(GroupCommand::Raise(index)) => self.project.raise_group(index),
            Some(GroupCommand::Lower(index)) => self.project.lower_group(index),
            Some(GroupCommand::Remove(index)) => self.project.remove_group(index),
            None => {}
        }

        if export {
            self.export_rules(ctx);
        }
    }

    fn show_group_list(&self, ui: &mut Ui, pending: &mut Option<GroupCommand>) {
        if self.project.groups().is_empty() {
            ui.weak("No groups yet.");
            return;
        }

        for (index, group) in self.project.groups().iter().enumerate() {
            ui.horizontal(|ui| {
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, grid::group_color(index));

                if ui.button("▲").clicked() {
                    *pending = Some(GroupCommand::Raise(index));
                }
                if ui.button("▼").clicked() {
                    *pending = Some(GroupCommand::Lower(index));
                }
                if ui.button("✕").clicked() {
                    *pending = Some(GroupCommand::Remove(index));
                }

                let label = ui.selectable_label(false, describe_group(group));
                if label.clicked() {
                    *pending = Some(GroupCommand::Configure(index));
                }
            });
        }
    }

    fn show_tileset(&mut self, ui: &mut Ui, ctx: &Context) {
        let response = egui::CentralPanel::default()
            .show(ui, |ui| match &self.workspace {
                Workspace::NoImage => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.4);
                        if ui.button("Select Image").clicked() {
                            self.picker.open_image();
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
                            },
                        )
                    })
                    .inner
                }
            })
            .inner;

        self.handle_grid(ctx, response);
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
            && !matches!(
                tile_state(&loaded.tileset, &self.project, tile),
                TileState::Locked
            )
        {
            let rule = seed_rule(&loaded.tileset, &self.project, tile);
            self.dialog = Some(TileDialog::new(tile, rule));
        }

        if let Some(tile) = response.secondary_clicked {
            self.toggle_removed(ctx, tile);
        }
    }

    fn handle_group_grid(&mut self, ctx: &Context, response: GridResponse) {
        if let Some(tile) = response.drag_started {
            self.drag_anchor = Some(tile);
        }

        if let Some(tile) = response.secondary_clicked
            && let Some(index) = self.project.group_at(tile)
        {
            let name = self.project.groups()[index].name.clone();
            self.project.remove_group(index);
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
            Some(index) if anchor == corner => self.open_group_dialog(index),
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

        if group
            .footprint()
            .iter()
            .any(|tile| self.project.group_at(*tile).is_some())
        {
            self.status
                .warning(ctx, "That rectangle overlaps an existing group");
            return;
        }
        if let Err(error) = group.validate() {
            self.status.warning(ctx, error.to_string());
            return;
        }

        self.status.info(ctx, format!("Added group {}", group.name));
        self.project.add_group(group);
        self.open_group_dialog(0);
    }

    fn open_group_dialog(&mut self, index: usize) {
        if let Some(group) = self.project.groups().get(index) {
            self.group_dialog = Some(GroupDialog::new(index, group));
        }
    }

    fn toggle_removed(&mut self, ctx: &Context, tile: usize) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };

        match tile_state(&loaded.tileset, &self.project, tile) {
            TileState::Locked => {}
            TileState::Removed => {
                self.project.restore(tile);
                self.status.info(ctx, format!("Tile {tile} restored"));
            }
            _ => {
                self.project.remove(tile);
                self.status.info(ctx, format!("Tile {tile} removed"));
            }
        }
    }

    fn show_group_dialog(&mut self, ctx: &Context) {
        let Some(dialog) = &mut self.group_dialog else {
            return;
        };

        match dialog.show(ctx, self.project.groups()) {
            GroupAction::Pending => {}
            GroupAction::Cancel => self.group_dialog = None,
            GroupAction::Commit { index, group } => {
                self.project.replace_group(index, group);
                self.group_dialog = None;
            }
        }
    }

    fn show_dialog(&mut self, ctx: &Context) {
        let Workspace::Ready(loaded) = &self.workspace else {
            return;
        };
        let Some(dialog) = &mut self.dialog else {
            return;
        };

        match dialog.show(ctx, &loaded.tileset, &loaded.texture) {
            DialogAction::Pending => {}
            DialogAction::Cancel => self.dialog = None,
            DialogAction::Commit { tile, rule } => {
                self.project.set_rule(tile, rule);
                self.dialog = None;
                self.status.info(ctx, format!("Tile {tile} configured"));
            }
        }
    }
}

fn describe_tile(tileset: &Tileset, project: &Project, tile: usize) -> String {
    match tile_state(tileset, project, tile) {
        TileState::Locked => format!("Tile {tile} · locked"),
        TileState::Removed => format!("Tile {tile} · removed"),
        TileState::Guessed(guess) => {
            format!("Tile {tile} · guess {}", format_neighborhood(guess))
        }
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
