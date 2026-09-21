use std::cmp::Ordering;

use egui::{Color32, Context, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::tile::{Chance, TileMods, TileRule};
use crate::tileset::Tileset;

const CELL_SIZE: f32 = 54.0;
const CELL_GAP: f32 = 3.0;
const SWATCH_SIZE: f32 = 14.0;
const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;

const EMPTY_FILL: Color32 = Color32::from_gray(28);
const FULL_FILL: Color32 = Color32::from_gray(185);
const ANY_LIGHT: Color32 = Color32::from_gray(96);
const ANY_DARK: Color32 = Color32::from_gray(64);
const PREVIEW_LIGHT: Color32 = Color32::from_gray(64);
const PREVIEW_DARK: Color32 = Color32::from_gray(48);

const CELL_TOOLTIP: &str = "Left-click for the next state, right-click for the previous one, \
                            or press 1 (empty), 2 (full), 3 (any) while hovering.";
const CHANCE_TOOLTIP: &str = "Tiles declaring the same neighborhood split it between them, \
                              anything left over falls through to a less specific rule.";

pub enum DialogAction {
    Pending,
    Commit { tile: usize, rule: TileRule },
    Cancel,
}

pub struct TileDialog {
    tile: usize,
    neighborhood: Neighborhood,
    mods: TileMods,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    error: Option<String>,
}

impl TileDialog {
    pub fn new(tile: usize, rule: TileRule) -> Self {
        Self {
            tile,
            neighborhood: rule.neighborhood,
            mods: rule.mods,
            chance_percent: rule.chance.percent(),
            error: None,
        }
    }

    pub fn show(
        &mut self,
        ctx: &Context,
        tileset: &Tileset,
        texture: &TextureHandle,
    ) -> DialogAction {
        let mut action = DialogAction::Pending;

        let modal = egui::Modal::new(egui::Id::new("tile_dialog")).show(ctx, |ui| {
            ui.heading(format!("Tile {}", self.tile));
            ui.separator();

            self.show_neighborhood(ui, tileset, texture);
            ui.add_space(6.0);
            show_legend(ui);

            ui.separator();
            self.show_options(ui);

            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = DialogAction::Cancel;
                }
                if ui.button("OK").clicked() {
                    action = self.commit();
                }
            });
        });

        if modal.should_close() {
            return DialogAction::Cancel;
        }

        action
    }

    fn commit(&mut self) -> DialogAction {
        match Chance::new(self.chance_percent) {
            Ok(chance) => DialogAction::Commit {
                tile: self.tile,
                rule: TileRule {
                    neighborhood: self.neighborhood,
                    mods: self.mods,
                    chance,
                },
            },
            Err(error) => {
                self.error = Some(error.to_string());
                DialogAction::Pending
            }
        }
    }

    fn show_neighborhood(&mut self, ui: &mut Ui, tileset: &Tileset, texture: &TextureHandle) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

            for row in 0..3 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

                    for column in 0..3 {
                        match neighbor_index(row, column) {
                            Some(index) => self.show_state_cell(ui, index),
                            None => show_tile_preview(ui, tileset, texture, self.tile),
                        }
                    }
                });
            }
        });
    }

    fn show_state_cell(&mut self, ui: &mut Ui, index: usize) {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(CELL_SIZE), Sense::click());
        paint_state(ui, rect, self.neighborhood.state(index));

        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                2.0,
                Stroke::new(2.0, ui.visuals().selection.stroke.color),
                StrokeKind::Inside,
            );

            if let Some(state) = pressed_state(ui) {
                self.neighborhood.set_state(index, state);
            }
        }

        if response.clicked() {
            let next = self.neighborhood.state(index).next();
            self.neighborhood.set_state(index, next);
        }
        if response.secondary_clicked() {
            let previous = self.neighborhood.state(index).previous();
            self.neighborhood.set_state(index, previous);
        }

        response.on_hover_text(CELL_TOOLTIP);
    }

    fn show_options(&mut self, ui: &mut Ui) {
        ui.label("Also place these transforms:");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.mods.can_x_flip, "X-Flip");
            ui.checkbox(&mut self.mods.can_y_flip, "Y-Flip");
            ui.checkbox(&mut self.mods.can_rotate, "Rotate");
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Chance");
            let chance = egui::DragValue::new(&mut self.chance_percent)
                .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
                .speed(0.5)
                .suffix(" %");
            ui.add(chance).on_hover_text(CHANCE_TOOLTIP);
        })
        .response
        .on_hover_text(CHANCE_TOOLTIP);
    }
}

fn neighbor_index(row: usize, column: usize) -> Option<usize> {
    const CENTER: usize = 4;

    let position = row * 3 + column;
    match position.cmp(&CENTER) {
        Ordering::Less => Some(position),
        Ordering::Equal => None,
        Ordering::Greater => Some(position - 1),
    }
}

fn pressed_state(ui: &Ui) -> Option<NeighborState> {
    ui.input(|input| {
        if input.key_pressed(egui::Key::Num1) {
            Some(NeighborState::Empty)
        } else if input.key_pressed(egui::Key::Num2) {
            Some(NeighborState::Full)
        } else if input.key_pressed(egui::Key::Num3) {
            Some(NeighborState::Any)
        } else {
            None
        }
    })
}

fn paint_state(ui: &Ui, rect: Rect, state: NeighborState) {
    let painter = ui.painter();
    match state {
        NeighborState::Empty => {
            painter.rect_filled(rect, 2.0, EMPTY_FILL);
        }
        NeighborState::Full => {
            painter.rect_filled(rect, 2.0, FULL_FILL);
        }
        NeighborState::Any => paint_checker(painter, rect, ANY_LIGHT, ANY_DARK),
    }

    painter.rect_stroke(
        rect,
        2.0,
        ui.visuals().widgets.inactive.bg_stroke,
        StrokeKind::Inside,
    );
}

fn show_tile_preview(ui: &mut Ui, tileset: &Tileset, texture: &TextureHandle, tile: usize) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(CELL_SIZE), Sense::hover());
    let slice = &tileset.tiles[tile];

    paint_checker(ui.painter(), rect, PREVIEW_LIGHT, PREVIEW_DARK);
    let uv = Rect::from_min_max(slice.uv_min.into(), slice.uv_max.into());
    ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
}

fn show_legend(ui: &mut Ui) {
    ui.horizontal(|ui| {
        for (state, label) in [
            (NeighborState::Empty, "empty"),
            (NeighborState::Full, "full"),
            (NeighborState::Any, "any"),
        ] {
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), Sense::hover());
            paint_state(ui, rect, state);
            ui.label(label);
            ui.add_space(6.0);
        }
    });
}

fn paint_checker(painter: &egui::Painter, rect: Rect, light: Color32, dark: Color32) {
    painter.rect_filled(rect, 2.0, dark);

    let half = rect.size() / 2.0;
    painter.rect_filled(Rect::from_min_size(rect.min, half), 0.0, light);
    painter.rect_filled(Rect::from_min_size(rect.center(), half), 0.0, light);
}

#[cfg(test)]
mod tests {
    use super::neighbor_index;

    #[test]
    fn neighbor_indices_skip_the_center_in_reading_order() {
        let indices: Vec<Option<usize>> = (0..3)
            .flat_map(|row| (0..3).map(move |column| neighbor_index(row, column)))
            .collect();

        let expected = [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
        ];
        assert_eq!(indices, expected);
    }
}
