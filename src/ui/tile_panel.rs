use egui::{Color32, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};

use crate::model::neighbor::{NeighborState, Neighborhood, neighbor_index_at};
use crate::model::pool::{ChanceMode, Pool};
use crate::model::tile::{Chance, TileMods, TileRule};
use crate::model::transform::Transform;
use crate::tileset::Tileset;
use crate::ui::{grid, preview};

const MIN_CELL_SIZE: f32 = 32.0;
const MAX_CELL_SIZE: f32 = 80.0;
const CELL_GAP: f32 = 3.0;
const SWATCH_SIZE: f32 = 14.0;
const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;
const CHANCE_DRAG_SPEED: f64 = 0.5;

const HEADING_SPACING: f32 = 4.0;
const LEGEND_SPACING: f32 = 4.0;
const LEGEND_GAP: f32 = 4.0;
const SECTION_SPACING: f32 = 6.0;
const CORNER_RADIUS: f32 = 2.0;
const MEMBER_SIZE: f32 = 20.0;
const OUTLINE_WIDTH: f32 = 2.0;

const EMPTY_FILL: Color32 = Color32::from_gray(28);
const FULL_FILL: Color32 = Color32::from_gray(185);
const ANY_LIGHT: Color32 = Color32::from_gray(96);
const ANY_DARK: Color32 = Color32::from_gray(64);
const PREVIEW_LIGHT: Color32 = Color32::from_gray(64);
const PREVIEW_DARK: Color32 = Color32::from_gray(48);

const CELL_TOOLTIP: &str = "Left-click for the next state, right-click for the previous one. \
                            Or press 1 (empty), 2 (full), 3 (any) while hovering.";
const CHANCE_TOOLTIP: &str = "Tiles declaring the same neighborhood split it between them by \
                              their chances. With Normalize chances off, a total below 100 % \
                              can leave tiles unchanged.";
const POOL_TOOLTIP: &str = "A pool holds all tiles with the exact neighborhood, considering \
                            rotations and flips.";
const UNCHANGED: &str = "unchanged";

pub struct TilePools<'a> {
    pub pools: Vec<&'a Pool>,
    pub mode: ChanceMode,
}

struct PoolView<'a> {
    tileset: &'a Tileset,
    texture: &'a TextureHandle,
    mode: ChanceMode,
    as_drawn_share: String,
}

pub enum TileEdit {
    Apply(TileRule),
    Remove,
}

pub struct TilePanel {
    tile: usize,
    neighborhood: Neighborhood,
    mods: TileMods,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    has_rule: bool,
    error: Option<String>,
}

impl TilePanel {
    pub fn new(tile: usize, rule: TileRule, has_rule: bool) -> Self {
        Self {
            tile,
            neighborhood: rule.neighborhood,
            mods: rule.mods,
            chance_percent: rule.chance.percent(),
            has_rule,
            error: None,
        }
    }

    pub fn tile(&self) -> usize {
        self.tile
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        pools: &TilePools,
    ) -> Option<TileEdit> {
        ui.heading(match self.has_rule {
            true => format!("Tile {}", self.tile),
            false => format!("Tile {} (no rule yet)", self.tile),
        });
        ui.add_space(HEADING_SPACING);

        let changed = self.show_neighborhood(ui, tileset, texture);
        ui.add_space(SECTION_SPACING);
        show_legend(ui);

        ui.add_space(SECTION_SPACING);
        let edit = self.show_options(ui, changed);

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        if !pools.pools.is_empty() {
            ui.add_space(SECTION_SPACING);
            self.show_pools(ui, tileset, texture, pools);
        }

        edit
    }

    fn show_pools(
        &self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        pools: &TilePools,
    ) {
        let view = PoolView {
            tileset,
            texture,
            mode: pools.mode,
            as_drawn_share: self.share_label(pools.pools[0], pools.mode),
        };
        for (index, pool) in pools.pools.iter().enumerate() {
            self.show_pool(ui, &view, pool, index == 0);
        }
    }

    fn show_pool(&self, ui: &mut Ui, view: &PoolView, pool: &Pool, open: bool) {
        let transform = pool
            .member_of(self.tile)
            .map_or(Transform::IDENTITY, |member| member.transform);
        let share = self.share_label(pool, view.mode);
        let title = format!(
            "Pool {} · {} tiles · {share}",
            transform_name(transform),
            pool.members.len()
        );
        let title = match share == view.as_drawn_share {
            true => egui::RichText::new(title),
            false => egui::RichText::new(title).color(ui.visuals().warn_fg_color),
        };

        let header = egui::CollapsingHeader::new(title)
            .id_salt(("pool", self.tile, pool.neighborhood))
            .default_open(open)
            .show(ui, |ui| self.show_members(ui, view, pool));
        header.header_response.on_hover_text(POOL_TOOLTIP);
    }

    fn show_members(&self, ui: &mut Ui, view: &PoolView, pool: &Pool) {
        egui::Grid::new(("pool_members", pool.neighborhood)).show(ui, |ui| {
            for (member, share) in pool.members.iter().zip(pool.shares(view.mode)) {
                let (rect, _response) =
                    ui.allocate_exact_size(Vec2::splat(MEMBER_SIZE), Sense::hover());
                paint_checker(ui.painter(), rect, PREVIEW_LIGHT, PREVIEW_DARK);
                let slice = &view.tileset.tiles[member.tile];
                preview::paint_turned(ui.painter(), rect, view.texture, slice, member.transform);

                let name = egui::RichText::new(format!("Tile {}", member.tile));
                ui.label(match member.tile == self.tile {
                    true => name.strong(),
                    false => name,
                });
                ui.label(grid::format_percent(share));
                ui.end_row();
            }

            let unchanged = Chance::FULL.percent() - pool.coverage(view.mode);
            if unchanged > 0.0 {
                let warn = ui.visuals().warn_fg_color;
                ui.label("");
                ui.colored_label(warn, UNCHANGED);
                ui.colored_label(warn, grid::format_percent(unchanged));
                ui.end_row();
            }
        });
    }

    fn share_label(&self, pool: &Pool, mode: ChanceMode) -> String {
        pool.share_of(self.tile, mode)
            .map(grid::format_percent)
            .unwrap_or_default()
    }

    fn show_action(&mut self, ui: &mut Ui, changed: bool) -> Option<TileEdit> {
        if self.has_rule {
            if ui.button("Remove rule").clicked() {
                self.has_rule = false;
                self.error = None;
                return Some(TileEdit::Remove);
            }
        } else if ui.button("Add rule").clicked() {
            self.has_rule = true;
            return self.apply();
        }

        if !changed {
            return None;
        }

        self.has_rule = true;
        self.apply()
    }

    fn apply(&mut self) -> Option<TileEdit> {
        match Chance::new(self.chance_percent) {
            Ok(chance) => {
                self.error = None;
                Some(TileEdit::Apply(TileRule {
                    neighborhood: self.neighborhood,
                    mods: self.mods,
                    chance,
                }))
            }
            Err(error) => {
                self.error = Some(error.to_string());
                None
            }
        }
    }

    fn show_neighborhood(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
    ) -> bool {
        let mut changed = false;
        let size = cell_size(ui);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

            for dy in -1..=1 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

                    for dx in -1..=1 {
                        match neighbor_index_at(dx, dy) {
                            Some(index) => changed |= self.show_state_cell(ui, index, size),
                            None => show_tile_preview(ui, tileset, texture, self.tile, size),
                        }
                    }
                });
            }
        });

        changed
    }

    fn show_state_cell(&mut self, ui: &mut Ui, index: usize, size: f32) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        paint_state(ui, rect, self.neighborhood.state(index));

        let mut state = self.neighborhood.state(index);
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                CORNER_RADIUS,
                Stroke::new(OUTLINE_WIDTH, ui.visuals().selection.stroke.color),
                StrokeKind::Inside,
            );

            if let Some(pressed) = pressed_state(ui) {
                state = pressed;
            }
        }

        if response.clicked() {
            state = state.next();
        }
        if response.secondary_clicked() {
            state = state.previous();
        }

        response.on_hover_text(CELL_TOOLTIP);

        if state == self.neighborhood.state(index) {
            return false;
        }

        self.neighborhood.set_state(index, state);
        true
    }

    fn show_options(&mut self, ui: &mut Ui, neighborhood_changed: bool) -> Option<TileEdit> {
        let mut changed = neighborhood_changed;

        ui.label("Also place these transforms:");
        ui.horizontal_wrapped(|ui| {
            changed |= ui.checkbox(&mut self.mods.can_x_flip, "X-Flip").changed();
            changed |= ui.checkbox(&mut self.mods.can_y_flip, "Y-Flip").changed();
            changed |= ui.checkbox(&mut self.mods.can_rotate, "Rotate").changed();
        });

        ui.add_space(HEADING_SPACING);

        let mut edit = None;
        ui.horizontal(|ui| {
            ui.label("Chance").on_hover_text(CHANCE_TOOLTIP);
            let chance = egui::DragValue::new(&mut self.chance_percent)
                .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
                .speed(CHANCE_DRAG_SPEED)
                .suffix(" %");
            changed |= ui.add(chance).on_hover_text(CHANCE_TOOLTIP).changed();

            edit = self.show_action(ui, changed);
        });

        edit
    }
}

fn transform_name(transform: Transform) -> &'static str {
    match (transform.x_flip, transform.y_flip, transform.rot) {
        (false, false, false) => "as drawn",
        (false, false, true) => "rotated 90°",
        (true, true, false) => "rotated 180°",
        (true, true, true) => "rotated 270°",
        (true, false, false) => "mirrored left-right",
        (false, true, false) => "mirrored top-bottom",
        (true, false, true) | (false, true, true) => "mirrored diagonally",
    }
}

fn cell_size(ui: &Ui) -> f32 {
    ((ui.available_width() - 2.0 * CELL_GAP) / 3.0)
        .floor()
        .clamp(MIN_CELL_SIZE, MAX_CELL_SIZE)
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
            painter.rect_filled(rect, CORNER_RADIUS, EMPTY_FILL);
        }
        NeighborState::Full => {
            painter.rect_filled(rect, CORNER_RADIUS, FULL_FILL);
        }
        NeighborState::Any => paint_checker(painter, rect, ANY_LIGHT, ANY_DARK),
    }

    painter.rect_stroke(
        rect,
        CORNER_RADIUS,
        ui.visuals().widgets.inactive.bg_stroke,
        StrokeKind::Inside,
    );
}

fn show_tile_preview(
    ui: &mut Ui,
    tileset: &Tileset,
    texture: &TextureHandle,
    tile: usize,
    size: f32,
) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let slice = &tileset.tiles[tile];

    paint_checker(ui.painter(), rect, PREVIEW_LIGHT, PREVIEW_DARK);
    grid::paint_tile(ui.painter(), rect, texture, slice);
}

fn show_legend(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LEGEND_SPACING;

        for (state, label) in [
            (NeighborState::Empty, "empty"),
            (NeighborState::Full, "full"),
            (NeighborState::Any, "any"),
        ] {
            let (rect, _response) =
                ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), Sense::hover());
            paint_state(ui, rect, state);
            ui.label(label);
            ui.add_space(LEGEND_GAP);
        }
    });
}

fn paint_checker(painter: &egui::Painter, rect: Rect, light: Color32, dark: Color32) {
    painter.rect_filled(rect, CORNER_RADIUS, dark);

    let half = rect.size() / 2.0;
    painter.rect_filled(Rect::from_min_size(rect.min, half), 0.0, light);
    painter.rect_filled(Rect::from_min_size(rect.center(), half), 0.0, light);
}
