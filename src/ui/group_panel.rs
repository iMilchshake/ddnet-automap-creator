use egui::{TextureHandle, Ui, Vec2};

use crate::model::group::{GroupPreset, TileGroup};
use crate::model::tile::{Chance, TileMods};
use crate::tileset::Tileset;
use crate::ui::group_neighbors;
use crate::ui::tile_panel::orientations::{ORIENTATIONS_LABEL, flag_width, show_preset_flags};

const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;
const CHANCE_DRAG_SPEED: f64 = 0.5;

const SURROUNDINGS_LABEL: &str = "Surroundings";
const ORIENTATIONS_HINT: &str = "Also adds rotated and flipped copies of the group.";
const CHANCE_HINT: &str = "Chance of the group, shared with its rotated and flipped copies.";

fn preset_hint(preset: GroupPreset) -> &'static str {
    match preset {
        GroupPreset::Fill => "Only needs its own area to be solid.",
        GroupPreset::Decorate => "Also needs the blocks around it to be solid.",
        GroupPreset::OnFloor => {
            "Needs its own area to be empty and the blocks below it to be solid."
        }
    }
}

pub struct GroupPanel {
    index: usize,
    name: String,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    mods: TileMods,
    surroundings: TileGroup,
    error: Option<String>,
}

impl GroupPanel {
    pub fn new(index: usize, group: &TileGroup) -> Self {
        Self {
            index,
            name: group.name.clone(),
            chance_percent: group.chance.percent(),
            mods: group.mods,
            surroundings: group.clone(),
            error: None,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        groups: &[TileGroup],
    ) -> Option<TileGroup> {
        ui.heading("Group");
        ui.add_space(4.0);

        let edit = match self.show_options(ui, tileset, texture) {
            true => self.validated(groups),
            false => None,
        };

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        edit
    }

    fn show_options(&mut self, ui: &mut Ui, tileset: &Tileset, texture: &TextureHandle) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Name");
            changed |= ui
                .add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY))
                .changed();
        });

        ui.add_space(4.0);
        ui.label(SURROUNDINGS_LABEL);
        ui.horizontal(|ui| {
            for preset in GroupPreset::ALL {
                let active = self.surroundings.preset() == Some(preset);
                let button = ui
                    .selectable_label(active, preset.label())
                    .on_hover_text(preset_hint(preset));
                if button.clicked() {
                    self.surroundings.apply_preset(preset);
                    changed = true;
                }
            }
        });
        changed |= group_neighbors::show(ui, tileset, texture, &mut self.surroundings);

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
            chance,
            mods: self.mods,
            ..self.surroundings.clone()
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
