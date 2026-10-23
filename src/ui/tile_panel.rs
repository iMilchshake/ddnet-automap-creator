use std::f32::consts::PI;

use egui::{
    Align2, Color32, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, TextureHandle, Ui, Vec2,
};

use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood, neighbor_index_at};
use crate::model::pool::{ChanceMode, Pool};
use crate::model::tile::{Chance, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tileset::Tileset;
use crate::ui::{ADD_ICON, DUPLICATE_ICON, REMOVE_ICON, grid, icon_button, preview};

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
const ORIENTATION_SIZE: f32 = 32.0;
const ORIENTATION_COLUMNS: usize = 4;
const CROSS_WIDTH: f32 = 1.5;
const CROSS_INSET_FRACTION: f32 = 0.2;
const CROSS_ALPHA: u8 = 70;
const OUTLINE_WIDTH: f32 = 2.0;
const EMPTY_CROSS_INSET_FRACTION: f32 = 0.28;
const MARK_WIDTH_FRACTION: f32 = 0.05;
const MIN_MARK_WIDTH: f32 = 1.0;
const BUTTON_FRACTION: f32 = 0.27;
const MIN_BUTTON_SIZE: f32 = 8.0;
const BUTTON_MARGIN: f32 = 1.0;
const BUTTON_GAP: f32 = 2.0;
const ICON_INSET_FRACTION: f32 = 0.22;
const ICON_STROKE_FRACTION: f32 = 0.12;
const MIN_ICON_STROKE: f32 = 1.0;
const ARC_SEGMENTS: usize = 12;
const ARC_START: f32 = -PI;
const ARC_SWEEP: f32 = 1.5 * PI;
const ARROW_HEAD_FRACTION: f32 = 0.35;
const ARROW_HEAD_WIDTH: f32 = 0.7;
const FLIP_GAP_FRACTION: f32 = 0.25;
const FLIP_BASE_FRACTION: f32 = 0.7;

const MARK_COLOR: Color32 = Color32::from_gray(150);
const FULL_FILL: Color32 = Color32::from_gray(185);
const ANY_LIGHT: Color32 = Color32::from_gray(96);
const ANY_DARK: Color32 = Color32::from_gray(64);
const PREVIEW_LIGHT: Color32 = Color32::from_gray(64);
const PREVIEW_DARK: Color32 = Color32::from_gray(48);
const DISABLED_DARKEN: Color32 = Color32::from_black_alpha(72);
const CROSS_COLOR: Color32 =
    Color32::from_rgba_premultiplied(CROSS_ALPHA, CROSS_ALPHA, CROSS_ALPHA, CROSS_ALPHA);

const CELL_TOOLTIP: &str = "Left-click toggles full and any, right-click sets empty. \
                            Or press 1 (empty), 2 (full), 3 (any) while hovering. \
                            Ctrl-click picks a specific tile from the tileset.";
const CHANCE_TOOLTIP: &str = "Tiles declaring the same neighborhood split it between them by \
                              their chances. With Normalize chances off, a total below 100 % \
                              can leave tiles unchanged.";
const POOL_TOOLTIP: &str = "A pool holds all tiles with the exact neighborhood, considering \
                            rotations and flips.";
const ROTATE_TOOLTIP: &str = "Rotate 90° clockwise";
const FLIP_HORIZONTALLY_TOOLTIP: &str = "Flip horizontally";
const FLIP_VERTICALLY_TOOLTIP: &str = "Flip vertically";
const ADD_RULE_TOOLTIP: &str = "Add rule";
const DUPLICATE_RULE_TOOLTIP: &str = "Duplicate rule";
const REMOVE_RULE_TOOLTIP: &str = "Remove rule";
const RESET_TOOLTIP: &str = "Reset orientation";
const ORIENTATION_HINT: &str = "Use the buttons to rotate or flip the whole rule.";
const UNCHANGED: &str = "unchanged";
const LAST_ORIENTATION: &str = "At least one orientation is required";

pub struct TileContext<'a> {
    pub pools: Vec<&'a Pool>,
    pub mode: ChanceMode,
    pub rule_count: usize,
    pub always_show_turn_buttons: bool,
}

#[derive(Clone, Copy)]
enum Icon {
    Rotate,
    FlipHorizontally,
    FlipVertically,
}

struct TurnButton {
    icon: Icon,
    tooltip: &'static str,
    turn: Transform,
}

const TURN_BUTTONS: [TurnButton; 3] = [
    TurnButton {
        icon: Icon::Rotate,
        tooltip: ROTATE_TOOLTIP,
        turn: Transform::ROTATE_CLOCKWISE,
    },
    TurnButton {
        icon: Icon::FlipHorizontally,
        tooltip: FLIP_HORIZONTALLY_TOOLTIP,
        turn: Transform::FLIP_HORIZONTALLY,
    },
    TurnButton {
        icon: Icon::FlipVertically,
        tooltip: FLIP_VERTICALLY_TOOLTIP,
        turn: Transform::FLIP_VERTICALLY,
    },
];

struct PoolView<'a> {
    tileset: &'a Tileset,
    texture: &'a TextureHandle,
    mode: ChanceMode,
    own_share: String,
}

pub enum TileEdit {
    Apply { index: usize, rule: TileRule },
    Select(usize),
    Add,
    Duplicate(usize),
    Remove(usize),
}

pub struct TilePanel {
    tile: usize,
    index: usize,
    neighborhood: Neighborhood,
    mods: TileMods,
    custom: Option<Orientations>,
    drawn: Transform,
    /// A plain percentage while editing; only a valid one becomes a `Chance`.
    chance_percent: f32,
    has_rule: bool,
    error: Option<String>,
    picking: Option<usize>,
}

impl TilePanel {
    pub fn new(tile: usize, index: usize, rule: TileRule, has_rule: bool) -> Self {
        Self {
            tile,
            index,
            neighborhood: rule.neighborhood,
            mods: rule.mods,
            custom: rule.custom,
            drawn: rule.drawn,
            chance_percent: rule.chance.percent(),
            has_rule,
            error: None,
            picking: None,
        }
    }

    pub fn tile(&self) -> usize {
        self.tile
    }

    pub fn is_picking(&self) -> bool {
        self.picking.is_some()
    }

    pub fn picked(&self) -> Option<usize> {
        self.neighborhood.state(self.picking?).tile()
    }

    pub fn stop_picking(&mut self) {
        self.picking = None;
    }

    pub fn pick(&mut self, tile: usize) -> Option<TileEdit> {
        let index = self.picking.take()?;
        self.neighborhood.set_state(index, NeighborState::Is(tile));
        self.has_rule = true;

        self.apply()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
        deactivated: bool,
    ) -> Option<TileEdit> {
        let heading_edit = self.show_heading(ui, deactivated);
        ui.add_space(HEADING_SPACING);

        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.stop_picking();
        }

        let tab_edit = self.show_rule_tabs(ui, context.rule_count);
        let changed =
            self.show_neighborhood(ui, tileset, texture, context.always_show_turn_buttons);
        ui.add_space(SECTION_SPACING);
        show_legend(ui);
        if let Some(index) = self.picking {
            ui.weak(format!(
                "Picking the {} neighbor — click a tile, Esc: cancel",
                NEIGHBORS[index].label
            ));
        }

        ui.add_space(SECTION_SPACING);
        let edit = self.show_options(ui, tileset, texture, changed);

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        if !context.pools.is_empty() {
            ui.add_space(SECTION_SPACING);
            ui.strong("Pools");
            self.show_pools(ui, tileset, texture, context);
        }

        heading_edit.or(tab_edit).or(edit)
    }

    fn show_rule_tabs(&self, ui: &mut Ui, rule_count: usize) -> Option<TileEdit> {
        if rule_count == 0 {
            return None;
        }

        let mut edit = None;
        ui.horizontal_wrapped(|ui| {
            for index in 0..rule_count {
                let label = format!("Rule {}", index + 1);
                if ui.selectable_label(index == self.index, label).clicked() {
                    edit = Some(TileEdit::Select(index));
                }
            }
        });
        ui.add_space(SECTION_SPACING);

        edit
    }

    fn show_pools(
        &self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) {
        let own_pool = context
            .pools
            .iter()
            .copied()
            .find(|pool| {
                pool.member_of(self.tile)
                    .is_some_and(|member| member.rule == self.index)
            })
            .unwrap_or(context.pools[0]);
        let view = PoolView {
            tileset,
            texture,
            mode: context.mode,
            own_share: self.share_label(own_pool, context.mode),
        };
        for (index, pool) in context.pools.iter().enumerate() {
            self.show_pool(ui, &view, pool, index == 0);
        }
    }

    fn show_pool(&self, ui: &mut Ui, view: &PoolView, pool: &Pool, open: bool) {
        let transform = pool
            .member_of(self.tile)
            .map_or(Transform::IDENTITY, |member| member.transform);
        let share = self.share_label(pool, view.mode);
        let title = format!("{}: {share}", drawn_name(transform));
        let title = match share == view.own_share {
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

    fn apply_if_changed(&mut self, changed: bool) -> Option<TileEdit> {
        if !changed {
            return None;
        }

        self.has_rule = true;
        self.apply()
    }

    fn show_heading(&mut self, ui: &mut Ui, deactivated: bool) -> Option<TileEdit> {
        let title = match deactivated {
            true => format!("Tile {} (deactivated)", self.tile),
            false => format!("Tile {}", self.tile),
        };

        ui.horizontal(|ui| {
            ui.heading(title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.show_rule_buttons(ui)
            })
            .inner
        })
        .inner
    }

    fn show_rule_buttons(&mut self, ui: &mut Ui) -> Option<TileEdit> {
        let mut edit = None;

        let remove = ui.add_enabled(self.has_rule, icon_button(ui, REMOVE_ICON));
        if remove.on_hover_text(REMOVE_RULE_TOOLTIP).clicked() {
            self.error = None;
            edit = Some(TileEdit::Remove(self.index));
        }

        let duplicate = ui.add_enabled(self.has_rule, icon_button(ui, DUPLICATE_ICON));
        if duplicate.on_hover_text(DUPLICATE_RULE_TOOLTIP).clicked() {
            edit = Some(TileEdit::Duplicate(self.index));
        }

        if ui
            .add(icon_button(ui, ADD_ICON))
            .on_hover_text(ADD_RULE_TOOLTIP)
            .clicked()
        {
            edit = self.add_rule();
        }

        edit
    }

    fn add_rule(&mut self) -> Option<TileEdit> {
        if self.has_rule {
            return Some(TileEdit::Add);
        }

        self.has_rule = true;
        self.apply()
    }

    fn rule(&self, chance: Chance) -> TileRule {
        TileRule {
            neighborhood: self.neighborhood,
            mods: self.mods,
            custom: self.custom,
            drawn: self.drawn,
            chance,
        }
    }

    fn turn(&mut self, by: Transform) {
        let chance = Chance::new(self.chance_percent).unwrap_or_default();
        let turned = self.rule(chance).turned(by);
        self.neighborhood = turned.neighborhood;
        self.drawn = turned.drawn;
        self.picking = None;
    }

    fn apply(&mut self) -> Option<TileEdit> {
        match Chance::new(self.chance_percent) {
            Ok(chance) => {
                self.error = None;
                Some(TileEdit::Apply {
                    index: self.index,
                    rule: self.rule(chance),
                })
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
        always_show_turn_buttons: bool,
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
                            Some(index) => {
                                changed |= self.show_state_cell(ui, tileset, texture, index, size)
                            }
                            None => {
                                changed |= self.show_centre_cell(
                                    ui,
                                    tileset,
                                    texture,
                                    size,
                                    always_show_turn_buttons,
                                )
                            }
                        }
                    }
                });
            }
        });

        changed | self.show_orientation_note(ui)
    }

    fn show_orientation_note(&mut self, ui: &mut Ui) -> bool {
        if self.drawn == Transform::IDENTITY {
            return false;
        }

        let reset = ui
            .horizontal(|ui| {
                ui.label(format!("{} ·", capitalized(drawn_name(self.drawn))));
                ui.link("reset").on_hover_text(RESET_TOOLTIP).clicked()
            })
            .inner;
        if reset {
            self.turn(self.drawn.inverse());
        }

        reset
    }

    fn show_centre_cell(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        size: f32,
        always_show_turn_buttons: bool,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
        self.paint_turned_tile(ui, rect, tileset, texture, self.drawn);

        let button_size = (size * BUTTON_FRACTION).max(MIN_BUTTON_SIZE);
        let mut turn = None;
        if always_show_turn_buttons || ui.rect_contains_pointer(rect) {
            for (index, button) in TURN_BUTTONS.into_iter().enumerate() {
                let area = button_rect(rect, index, button_size);
                if turn_button(ui, area, button.icon, button.tooltip) {
                    turn = Some(button.turn);
                }
            }
        }
        response.on_hover_text(format!("{}\n{ORIENTATION_HINT}", drawn_name(self.drawn)));

        let Some(by) = turn else {
            return false;
        };
        self.turn(by);
        true
    }

    fn paint_turned_tile(
        &self,
        ui: &Ui,
        rect: Rect,
        tileset: &Tileset,
        texture: &TextureHandle,
        transform: Transform,
    ) {
        paint_checker(ui.painter(), rect, PREVIEW_LIGHT, PREVIEW_DARK);
        preview::paint_turned(
            ui.painter(),
            rect,
            texture,
            &tileset.tiles[self.tile],
            transform,
        );
    }

    fn show_state_cell(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        index: usize,
        size: f32,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let current = self.neighborhood.state(index);
        paint_state(ui, rect, current);
        if let Some(tile) = current.tile() {
            grid::paint_tile(ui.painter(), rect, texture, &tileset.tiles[tile]);
        }

        let mut state = current;
        if response.hovered() || self.picking == Some(index) {
            ui.painter().rect_stroke(
                rect,
                CORNER_RADIUS,
                Stroke::new(OUTLINE_WIDTH, ui.visuals().selection.stroke.color),
                StrokeKind::Inside,
            );
        }
        if response.hovered()
            && let Some(pressed) = pressed_state(ui)
        {
            state = pressed;
        }

        let primary = released_on(ui, &response, rect, PointerButton::Primary);
        let secondary = released_on(ui, &response, rect, PointerButton::Secondary);
        response.on_hover_text(cell_tooltip(current));

        if primary && ui.input(|input| input.modifiers.command) {
            self.picking = (self.picking != Some(index)).then_some(index);
            return false;
        }
        if primary {
            state = state.toggled_solid();
        }
        if secondary {
            state = NeighborState::Empty;
        }

        if (primary || secondary || state != current) && self.picking == Some(index) {
            self.picking = None;
        }
        if state == current {
            return false;
        }

        self.neighborhood.set_state(index, state);
        true
    }

    fn orientations(&self) -> Orientations {
        Orientations::resolved(self.custom, self.mods)
    }

    fn show_transform_flags(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        ui.add_enabled_ui(self.custom.is_none(), |ui| {
            ui.horizontal(|ui| {
                changed |= show_preset_flags(ui, &mut self.mods);
            });
        });
        changed |= self.show_custom_toggle(ui);

        changed
    }

    fn show_custom_toggle(&mut self, ui: &mut Ui) -> bool {
        let mut custom = self.custom.is_some();
        if !ui.checkbox(&mut custom, "Custom").changed() {
            return false;
        }

        self.custom = custom.then(|| Orientations::generated(self.mods));
        true
    }

    fn show_orientations(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
    ) -> bool {
        let mut changed = false;

        let frame = orientation_frame(ui, self.custom.is_some());
        egui::CollapsingHeader::new("Orientations")
            .default_open(false)
            .show(ui, |ui| {
                frame.show(ui, |ui| {
                    egui::Grid::new("orientations").show(ui, |ui| {
                        for row in Transform::ALL.chunks(ORIENTATION_COLUMNS) {
                            for &transform in row {
                                changed |= self.show_orientation(ui, tileset, texture, transform);
                            }
                            ui.end_row();
                        }
                    });
                });
            });

        changed
    }

    fn show_orientation(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        transform: Transform,
    ) -> bool {
        let orientations = self.orientations();
        let enabled = orientations.contains(transform);
        let locked = self.custom.is_some() && enabled && orientations.len() == 1;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::splat(ORIENTATION_SIZE), Sense::click());
        self.paint_turned_tile(ui, rect, tileset, texture, transform.after(self.drawn));
        if !enabled {
            ui.painter().rect_filled(rect, 0.0, DISABLED_DARKEN);
            paint_cross(ui.painter(), rect);
        }
        if self.custom.is_some() && response.hovered() {
            paint_hover_outline(ui, rect);
        }

        let enable = response.clicked_by(PointerButton::Primary);
        let disable = response.clicked_by(PointerButton::Secondary) && !locked;
        let response = match self.custom {
            Some(_) => response.on_hover_cursor(egui::CursorIcon::PointingHand),
            None => response,
        };
        response.on_hover_text(match locked {
            true => LAST_ORIENTATION,
            false => transform_name(transform),
        });

        let Some(custom) = self.custom else {
            return false;
        };
        let updated = match (enable, disable) {
            (true, _) => custom.with(transform),
            (_, true) => custom.without(transform),
            _ => return false,
        };
        self.custom = Some(updated);

        updated != custom
    }

    fn show_options(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        neighborhood_changed: bool,
    ) -> Option<TileEdit> {
        let mut changed = neighborhood_changed;

        ui.label("Place these transforms:");
        changed |= self.show_transform_flags(ui);
        changed |= self.show_orientations(ui, tileset, texture);

        ui.add_space(HEADING_SPACING);

        ui.horizontal(|ui| {
            ui.label("Chance").on_hover_text(CHANCE_TOOLTIP);
            let chance = egui::DragValue::new(&mut self.chance_percent)
                .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
                .speed(CHANCE_DRAG_SPEED)
                .suffix(" %");
            changed |= ui.add(chance).on_hover_text(CHANCE_TOOLTIP).changed();
        });

        self.apply_if_changed(changed)
    }
}

fn show_preset_flags(ui: &mut Ui, mods: &mut TileMods) -> bool {
    let mut changed = ui.checkbox(&mut mods.can_rotate, "Rotate").changed();

    if mods.can_rotate {
        let mut flip = mods.can_x_flip || mods.can_y_flip;
        if ui.checkbox(&mut flip, "Flip").changed() {
            mods.can_x_flip = flip;
            mods.can_y_flip = false;
            changed = true;
        }
    } else {
        changed |= ui.checkbox(&mut mods.can_x_flip, "X-Flip").changed();
        changed |= ui.checkbox(&mut mods.can_y_flip, "Y-Flip").changed();
    }

    changed
}

fn orientation_frame(ui: &Ui, editable: bool) -> egui::Frame {
    let frame = egui::Frame::group(ui.style());
    match editable {
        true => frame.fill(ui.visuals().extreme_bg_color),
        false => frame.stroke(Stroke::new(frame.stroke.width, Color32::TRANSPARENT)),
    }
}

fn button_rect(cell: Rect, index: usize, size: f32) -> Rect {
    let count = TURN_BUTTONS.len() as f32;
    let row = Vec2::new(count * size + (count - 1.0) * BUTTON_GAP, size);
    let area = Align2::CENTER_BOTTOM.align_size_within_rect(row, cell.shrink(BUTTON_MARGIN));

    Rect::from_min_size(
        area.min + Vec2::X * index as f32 * (size + BUTTON_GAP),
        Vec2::splat(size),
    )
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn turn_button(ui: &Ui, rect: Rect, icon: Icon, tooltip: &str) -> bool {
    let response = ui.interact(rect, ui.id().with(tooltip), Sense::click());
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        CORNER_RADIUS,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );

    let icon_area = rect.shrink(rect.width() * ICON_INSET_FRACTION);
    let width = (icon_area.width() * ICON_STROKE_FRACTION).max(MIN_ICON_STROKE);
    paint_icon(
        ui.painter(),
        icon_area,
        Stroke::new(width, visuals.fg_stroke.color),
        icon,
    );

    let clicked = response.clicked();
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip);

    clicked
}

fn paint_icon(painter: &Painter, rect: Rect, stroke: Stroke, icon: Icon) {
    match icon {
        Icon::Rotate => paint_curl(painter, rect, stroke),
        Icon::FlipHorizontally => paint_flip(painter, rect, stroke, true),
        Icon::FlipVertically => paint_flip(painter, rect, stroke, false),
    }
}

fn paint_curl(painter: &Painter, rect: Rect, stroke: Stroke) {
    let radius = rect.width() / 2.0;
    let point = |angle: f32| rect.center() + radius * Vec2::new(angle.cos(), angle.sin());
    let arc: Vec<Pos2> = (0..=ARC_SEGMENTS)
        .map(|step| point(ARC_START + ARC_SWEEP * step as f32 / ARC_SEGMENTS as f32))
        .collect();

    let end = arc[arc.len() - 1];
    let before_end = arc[arc.len() - 2];
    let tangent = (end - before_end).normalized();
    let head = rect.width() * ARROW_HEAD_FRACTION;
    let wing = tangent.rot90() * head * ARROW_HEAD_WIDTH;
    let arrow = vec![end + tangent * head, end + wing, end - wing];

    painter.add(Shape::line(arc, stroke));
    painter.add(Shape::convex_polygon(arrow, stroke.color, Stroke::NONE));
}

fn paint_flip(painter: &Painter, rect: Rect, stroke: Stroke, horizontal: bool) {
    let half = rect.width() / 2.0;
    let place = |along: f32, across: f32| {
        rect.center()
            + match horizontal {
                true => Vec2::new(along, across),
                false => Vec2::new(across, along),
            }
    };

    painter.line_segment([place(0.0, -half), place(0.0, half)], stroke);
    for side in [-1.0, 1.0] {
        let base = half * FLIP_BASE_FRACTION;
        let gap = side * half * FLIP_GAP_FRACTION;
        let wedge = vec![place(gap, -base), place(gap, base), place(side * half, 0.0)];
        painter.add(Shape::convex_polygon(wedge, stroke.color, Stroke::NONE));
    }
}

fn paint_hover_outline(ui: &Ui, rect: Rect) {
    let stroke = ui.visuals().widgets.hovered.bg_stroke;
    ui.painter()
        .rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
}

fn paint_cross(painter: &egui::Painter, rect: Rect) {
    let rect = rect.shrink(rect.width() * CROSS_INSET_FRACTION);
    let stroke = Stroke::new(CROSS_WIDTH, CROSS_COLOR);
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

fn drawn_name(transform: Transform) -> &'static str {
    match transform == Transform::IDENTITY {
        true => "unrotated",
        false => transform_name(transform),
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
        (true, false, true) => "mirrored anti-diagonally",
        (false, true, true) => "mirrored diagonally",
    }
}

fn cell_size(ui: &Ui) -> f32 {
    ((ui.available_width() - 2.0 * CELL_GAP) / 3.0)
        .floor()
        .clamp(MIN_CELL_SIZE, MAX_CELL_SIZE)
}

fn released_on(ui: &Ui, response: &Response, rect: Rect, button: PointerButton) -> bool {
    let released_inside = response.drag_stopped_by(button)
        && ui.input(|input| {
            input
                .pointer
                .latest_pos()
                .is_some_and(|pos| rect.contains(pos))
        });
    response.clicked_by(button) || released_inside
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
    let default_border = ui.visuals().widgets.inactive.bg_stroke;
    let mark = Stroke::new(
        (rect.width() * MARK_WIDTH_FRACTION).max(MIN_MARK_WIDTH),
        MARK_COLOR,
    );
    let border = match state {
        NeighborState::Empty => {
            let cross = rect.shrink(rect.width() * EMPTY_CROSS_INSET_FRACTION);
            painter.line_segment([cross.left_top(), cross.right_bottom()], mark);
            painter.line_segment([cross.right_top(), cross.left_bottom()], mark);
            default_border
        }
        NeighborState::Full => {
            painter.rect_filled(rect, CORNER_RADIUS, FULL_FILL);
            default_border
        }
        NeighborState::Any => {
            paint_checker(painter, rect, ANY_LIGHT, ANY_DARK);
            mark
        }
        NeighborState::Is(_) => {
            paint_checker(painter, rect, PREVIEW_LIGHT, PREVIEW_DARK);
            default_border
        }
    };

    painter.rect_stroke(rect, CORNER_RADIUS, border, StrokeKind::Inside);
}

fn cell_tooltip(state: NeighborState) -> String {
    match state {
        NeighborState::Is(tile) => format!("is tile {tile}\n{CELL_TOOLTIP}"),
        NeighborState::Empty | NeighborState::Full | NeighborState::Any => CELL_TOOLTIP.to_owned(),
    }
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
