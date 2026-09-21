use egui::{Context, TextureHandle, TextureOptions, Ui};

use crate::export::r_source::{self, RuleSet};
use crate::file_picker::{FilePicker, PickedFile};
use crate::file_saver::FileSaver;
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::tileset::{self, Tileset};
use crate::ui::grid::{self, GridResponse, GridView};
use crate::ui::status::StatusLine;
use crate::ui::tile_dialog::{DialogAction, TileDialog};
use crate::ui::tile_state::{TileState, seed_rule, tile_state};

const SIDE_PANEL_WIDTH: f32 = 260.0;

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
                    ui.label(format!("{} tiles configured", self.project.rule_count()));

                    ui.add_space(4.0);
                    let exportable = self.project.rule_count() > 0;
                    if ui
                        .add_enabled(exportable, egui::Button::new("Export .r…"))
                        .clicked()
                    {
                        export = true;
                    }

                    ui.separator();

                    ui.heading("Group editor");
                    // TODO: tile groups.
                    ui.label("Not implemented yet.");
                });
            });

        if export {
            self.export_rules(ctx);
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
