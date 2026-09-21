use egui::{Context, Ui};

use crate::model::group::{GroupMode, TileGroup};
use crate::model::tile::Chance;

const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;

const FILL_HINT: &str = "Places the group wherever its own tiles are solid.";
const DECORATE_HINT: &str = "Also needs the ring of tiles around the group to be solid.";

pub fn mode_label(mode: GroupMode) -> &'static str {
    match mode {
        GroupMode::Fill => "Fill",
        GroupMode::Decorate => "Decorate",
    }
}

pub enum GroupAction {
    Pending,
    Commit { index: usize, group: TileGroup },
    Cancel,
}

pub struct GroupDialog {
    index: usize,
    name: String,
    mode: GroupMode,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    top_left: usize,
    width: usize,
    height: usize,
    error: Option<String>,
}

impl GroupDialog {
    pub fn new(index: usize, group: &TileGroup) -> Self {
        Self {
            index,
            name: group.name.clone(),
            mode: group.mode,
            chance_percent: group.chance.percent(),
            top_left: group.top_left,
            width: group.width,
            height: group.height,
            error: None,
        }
    }

    pub fn show(&mut self, ctx: &Context, groups: &[TileGroup]) -> GroupAction {
        let mut action = GroupAction::Pending;

        let modal = egui::Modal::new(egui::Id::new("group_dialog")).show(ctx, |ui| {
            ui.heading("Group");
            ui.weak(format!(
                "Tile {}, {}×{}",
                self.top_left, self.width, self.height
            ));
            ui.separator();

            self.show_options(ui);

            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = GroupAction::Cancel;
                }
                if ui.button("OK").clicked() {
                    action = self.commit(groups);
                }
            });
        });

        if modal.should_close() {
            return GroupAction::Cancel;
        }

        action
    }

    fn show_options(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.name);
        });

        ui.horizontal(|ui| {
            ui.label("Mode");
            ui.selectable_value(&mut self.mode, GroupMode::Fill, "Fill");
            ui.selectable_value(&mut self.mode, GroupMode::Decorate, "Decorate");
        });
        ui.weak(match self.mode {
            GroupMode::Fill => FILL_HINT,
            GroupMode::Decorate => DECORATE_HINT,
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Chance");
            ui.add(
                egui::DragValue::new(&mut self.chance_percent)
                    .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
                    .speed(0.5)
                    .suffix(" %"),
            );
        });
    }

    fn commit(&mut self, groups: &[TileGroup]) -> GroupAction {
        let chance = match Chance::new(self.chance_percent) {
            Ok(chance) => chance,
            Err(error) => {
                self.error = Some(error.to_string());
                return GroupAction::Pending;
            }
        };

        let group = TileGroup {
            name: self.name.trim().to_owned(),
            top_left: self.top_left,
            width: self.width,
            height: self.height,
            mode: self.mode,
            chance,
        };

        if let Err(error) = group.validate() {
            self.error = Some(error.to_string());
            return GroupAction::Pending;
        }

        let taken = groups
            .iter()
            .enumerate()
            .any(|(index, other)| index != self.index && other.name == group.name);
        if taken {
            self.error = Some(format!("`{}` already names another group", group.name));
            return GroupAction::Pending;
        }

        GroupAction::Commit {
            index: self.index,
            group,
        }
    }
}
