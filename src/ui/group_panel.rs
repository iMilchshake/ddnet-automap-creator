use std::collections::BTreeSet;

use egui::{Ui, Vec2};

use crate::model::group::{GroupMode, TileGroup};
use crate::model::tile::{Chance, TileMods};
use crate::ui::tile_panel::orientations::{ORIENTATIONS_LABEL, flag_width, show_preset_flags};

const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;
const CHANCE_DRAG_SPEED: f64 = 0.5;

const FILL_HINT: &str = "Placed wherever its own tiles are solid.";
const DECORATE_HINT: &str = "Also needs a solid ring of tiles around it.";
const ORIENTATIONS_HINT: &str = "Also places the group turned or mirrored where it fits.";
const CHANCE_HINT: &str = "Shared across the group and its rotated and flipped copies.";

pub fn mode_label(mode: GroupMode) -> &'static str {
    match mode {
        GroupMode::Fill => "Fill",
        GroupMode::Decorate => "Decorate",
    }
}

pub struct GroupPanel {
    index: usize,
    name: String,
    mode: GroupMode,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    mods: TileMods,
    tiles: BTreeSet<usize>,
    subtitle: String,
    error: Option<String>,
}

impl GroupPanel {
    pub fn new(index: usize, group: &TileGroup) -> Self {
        let subtitle = match group.as_rect() {
            Some((top_left, _)) => format!("Tile {top_left}, {}", group.size_label()),
            None => group.size_label(),
        };

        Self {
            index,
            name: group.name.clone(),
            mode: group.mode,
            chance_percent: group.chance.percent(),
            mods: group.mods,
            tiles: group.tiles.clone(),
            subtitle,
            error: None,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn show(&mut self, ui: &mut Ui, groups: &[TileGroup]) -> Option<TileGroup> {
        ui.heading("Group");
        ui.weak(&self.subtitle);
        ui.add_space(4.0);

        let edit = match self.show_options(ui) {
            true => self.validated(groups),
            false => None,
        };

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        edit
    }

    fn show_options(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Name");
            changed |= ui
                .add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Mode");
            changed |= ui
                .selectable_value(&mut self.mode, GroupMode::Fill, "Fill")
                .on_hover_text(FILL_HINT)
                .changed();
            changed |= ui
                .selectable_value(&mut self.mode, GroupMode::Decorate, "Decorate")
                .on_hover_text(DECORATE_HINT)
                .changed();
        });

        ui.add_space(4.0);
        ui.label(ORIENTATIONS_LABEL)
            .on_hover_text(ORIENTATIONS_HINT);
        ui.horizontal(|ui| {
            let size = Vec2::new(flag_width(ui), ui.spacing().interact_size.y);
            changed |= show_preset_flags(ui, &mut self.mods, size);
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Chance");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut self.chance_percent)
                        .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
                        .speed(CHANCE_DRAG_SPEED)
                        .suffix(" %"),
                )
                .on_hover_text(CHANCE_HINT)
                .changed();
        });

        changed
    }

    fn validated(&mut self, groups: &[TileGroup]) -> Option<TileGroup> {
        let chance = match Chance::new(self.chance_percent) {
            Ok(chance) => chance,
            Err(error) => {
                self.error = Some(error.to_string());
                return None;
            }
        };

        let group = TileGroup {
            name: self.name.trim().to_owned(),
            tiles: self.tiles.clone(),
            mode: self.mode,
            chance,
            mods: self.mods,
        };

        if let Err(error) = group.validate() {
            self.error = Some(error.to_string());
            return None;
        }

        let taken = groups
            .iter()
            .enumerate()
            .any(|(index, other)| index != self.index && other.name == group.name);
        if taken {
            self.error = Some(format!("`{}` already names another group", group.name));
            return None;
        }

        self.error = None;
        Some(group)
    }
}
