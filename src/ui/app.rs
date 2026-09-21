//! Application shell: layout, menu, and the path from picked file to grid.

use egui::{Context, TextureHandle, TextureOptions, Ui};

use crate::file_picker::{FilePicker, PickedFile};
use crate::model::neighbor::NeighborState;
use crate::tileset::{self, TileKind, Tileset};
use crate::ui::{grid, status::StatusLine};

const SIDE_PANEL_WIDTH: f32 = 260.0;

/// The tools are dead until an image is loaded, so the two states are modelled
/// directly rather than as an absent tileset.
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
    picker: FilePicker,
    status: StatusLine,
    hovered_tile: Option<usize>,
}

impl Default for AutomapperApp {
    fn default() -> Self {
        Self {
            workspace: Workspace::NoImage,
            picker: FilePicker::new(),
            status: StatusLine::default(),
            hovered_tile: None,
        }
    }
}

impl eframe::App for AutomapperApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.receive_picked_file(&ctx);
        self.handle_shortcuts(&ctx);

        self.show_menu_bar(ui);
        self.show_status_bar(ui);
        self.show_side_panel(ui);
        self.show_tileset(ui);
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

    /// Replaces the whole workspace, so loading an image resets all state.
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
                        ui.weak(describe_tile(&loaded.tileset, index));
                    }
                });
            });
        });
    }

    fn show_side_panel(&mut self, ui: &mut Ui) {
        let has_image = matches!(self.workspace, Workspace::Ready(_));

        egui::Panel::right("tools")
            .resizable(false)
            .exact_size(SIDE_PANEL_WIDTH)
            .show(ui, |ui| {
                ui.add_enabled_ui(has_image, |ui| {
                    ui.heading("Generator");
                    // TODO: editable rule-set name, generate, export.
                    if let Workspace::Ready(loaded) = &self.workspace {
                        ui.label(format!("Rule set: {}", loaded.tileset.stem));
                    }
                    ui.label("Not implemented yet.");

                    ui.separator();

                    ui.heading("Group editor");
                    // TODO: tile groups.
                    ui.label("Not implemented yet.");
                });
            });
    }

    fn show_tileset(&mut self, ui: &mut Ui) {
        egui::CentralPanel::default().show(ui, |ui| match &self.workspace {
            Workspace::NoImage => {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.4);
                    if ui.button("Select Image").clicked() {
                        self.picker.open_image();
                    }
                });
            }
            Workspace::Ready(loaded) => {
                ui.vertical_centered(|ui| {
                    let response = grid::show(ui, &loaded.tileset, &loaded.texture);
                    self.hovered_tile = response.hovered;

                    // TODO: open the per-tile config dialog, plus undo/redo
                    // once it can mutate anything.
                    if let Some(index) = response.clicked {
                        log::debug!("clicked tile {index}");
                    }
                });
            }
        });
    }
}

/// Reads the alpha scan's guess back out, so it can be checked against the art.
fn describe_tile(tileset: &Tileset, index: usize) -> String {
    match tileset.tiles[index].kind {
        TileKind::Locked => format!("Tile {index} · locked"),
        TileKind::Guess(neighborhood) => {
            let glyphs: String = neighborhood
                .states()
                .iter()
                .copied()
                .map(state_glyph)
                .collect();

            let (top, rest) = glyphs.split_at(3);
            let (sides, bottom) = rest.split_at(2);
            format!("Tile {index} · guess {top}/{sides}/{bottom}")
        }
    }
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

    // Tilesets are pixel art and the grid scales them up.
    ctx.load_texture("tileset_atlas", image, TextureOptions::NEAREST)
}
