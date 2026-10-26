use std::f32::consts::PI;

use egui::{
    Align2, Atom, Color32, FontId, Id, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, TextureHandle, Ui, Vec2,
};

use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood, neighbor_index_at};
use crate::model::pool::{ChanceMode, Pool};
use crate::model::tile::{Centre, Chance, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tileset::Tileset;
use crate::ui::{
    ADD_ICON, BUTTON_GAP, COPY_ICON, DUPLICATE_ICON, PASTE_ICON, REMOVE_ICON, compact_button, grid,
    preview,
};

const MIN_CELL_SIZE: f32 = 32.0;
const CELL_GAP: f32 = 3.0;
const GRID_SIDE: f32 = 3.0;
const SWATCH_SIZE: f32 = 14.0;
const MIN_CHANCE_PERCENT: f32 = 0.1;
const MAX_CHANCE_PERCENT: f32 = 100.0;
const CHANCE_DRAG_SPEED: f64 = 0.5;
const FLASH_PEAK_OPACITY: f32 = 0.5;

const HEADING_SPACING: f32 = 4.0;
const LEGEND_SPACING: f32 = 4.0;
const LEGEND_GAP: f32 = 4.0;
const SECTION_SPACING: f32 = 6.0;
const CORNER_RADIUS: f32 = 2.0;
const MEMBER_SIZE: f32 = 20.0;
const ORIENTATION_SIZE: f32 = 32.0;
const ORIENTATION_COLUMNS: usize = 4;
const FLAG_ICON_SIZE: f32 = 12.0;
const FLAG_ICON_GAP: f32 = 4.0;
const FLAG_GAP: f32 = 3.0;
const FLAG_BOX_GAP: f32 = 4.0;
const ROTATING_FLAG_COUNT: f32 = 2.0;
const PLAIN_FLAG_COUNT: f32 = 3.0;
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
const HEADING_ROW_SPACING: f32 = 2.0 * BUTTON_GAP;
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
const EMPTY_CENTRE_ALPHA: u8 = 150;
const EMPTY_CENTRE_TINT: Color32 = Color32::from_rgba_premultiplied(
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
    EMPTY_CENTRE_ALPHA,
);
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
const ROTATE_NEIGHBORS_TOOLTIP: &str = "Rotate neighbors 90° clockwise";
const FLIP_NEIGHBORS_HORIZONTALLY_TOOLTIP: &str = "Flip neighbors horizontally";
const FLIP_NEIGHBORS_VERTICALLY_TOOLTIP: &str = "Flip neighbors vertically";
const ROTATE_TILE_TOOLTIP: &str = "Rotate tile 90° clockwise";
const FLIP_TILE_HORIZONTALLY_TOOLTIP: &str = "Flip tile horizontally";
const FLIP_TILE_VERTICALLY_TOOLTIP: &str = "Flip tile vertically";
const ADD_RULE_TOOLTIP: &str = "Add rule";
const DUPLICATE_RULE_TOOLTIP: &str = "Duplicate rule";
const REMOVE_RULE_TOOLTIP: &str = "Remove rule";
const COPY_RULE_TOOLTIP: &str = "Copy rule (C)";
const NOTHING_COPIED_TOOLTIP: &str = "Nothing copied yet";
const RESET_TOOLTIP: &str = "Reset tile orientation";
const ORIENTATION_HINT: &str = "Use the buttons to rotate or flip the tile only.";
const CENTRE_HINT: &str = "Left-click: full, right-click: empty (places into empty cells).";
const RESET_GLYPH: &str = "↺";
const UNCHANGED: &str = "unchanged";
const SHARED_POOLS_HEADING: &str = "Shared with other tiles";
const NO_MODS: TileMods = TileMods::new(false, false, false);
const LAST_ORIENTATION: &str = "At least one orientation is required";
const ORIENTATIONS_LABEL: &str = "Rotated & flipped copies";
const ROTATE_FLAG_TOOLTIP: &str = "Also place it rotated by 90°, 180° and 270°";
const X_FLIP_FLAG_TOOLTIP: &str = "Also place it mirrored left-right";
const Y_FLIP_FLAG_TOOLTIP: &str = "Also place it mirrored top-bottom";
const FLIP_FLAG_TOOLTIP: &str = "Also place the mirrored copies";
const ORIENTATIONS_TOOLTIP: &str = "Each lit copy is placed too, wherever its neighbors \
                                    match the same way rotated or flipped.";

pub struct TileContext<'a> {
    pub pools: Vec<&'a Pool>,
    pub mode: ChanceMode,
    pub rule_count: usize,
    pub always_show_turn_buttons: bool,
    pub copied: Option<&'a CopiedRule>,
    pub flash: Option<GridFlash>,
}

#[derive(Clone, Copy)]
pub enum FlashKind {
    Copy,
    Paste,
}

#[derive(Clone, Copy)]
pub struct GridFlash {
    pub kind: FlashKind,
    pub remaining: f32,
}

pub struct CopiedRule {
    pub rule: TileRule,
    pub tile: usize,
    pub index: usize,
}

impl CopiedRule {
    fn paste_tooltip(&self) -> String {
        format!("Paste rule {} of tile {} (V)", self.index + 1, self.tile)
    }
}

#[derive(Clone, Copy)]
enum Icon {
    Rotate,
    FlipHorizontally,
    FlipVertically,
    Reset,
}

struct TurnButton {
    icon: Icon,
    neighbors_tooltip: &'static str,
    tile_tooltip: &'static str,
    turn: Transform,
}

const TURN_BUTTONS: [TurnButton; 3] = [
    TurnButton {
        icon: Icon::Rotate,
        neighbors_tooltip: ROTATE_NEIGHBORS_TOOLTIP,
        tile_tooltip: ROTATE_TILE_TOOLTIP,
        turn: Transform::ROTATE_CLOCKWISE,
    },
    TurnButton {
        icon: Icon::FlipHorizontally,
        neighbors_tooltip: FLIP_NEIGHBORS_HORIZONTALLY_TOOLTIP,
        tile_tooltip: FLIP_TILE_HORIZONTALLY_TOOLTIP,
        turn: Transform::FLIP_HORIZONTALLY,
    },
    TurnButton {
        icon: Icon::FlipVertically,
        neighbors_tooltip: FLIP_NEIGHBORS_VERTICALLY_TOOLTIP,
        tile_tooltip: FLIP_TILE_VERTICALLY_TOOLTIP,
        turn: Transform::FLIP_VERTICALLY,
    },
];

struct Flag {
    label: &'static str,
    icon: Icon,
    tooltip: &'static str,
}

const ROTATE_FLAG: Flag = Flag {
    label: "Rotate",
    icon: Icon::Rotate,
    tooltip: ROTATE_FLAG_TOOLTIP,
};
const FLIP_FLAG: Flag = Flag {
    label: "Flip",
    icon: Icon::FlipHorizontally,
    tooltip: FLIP_FLAG_TOOLTIP,
};
const X_FLIP_FLAG: Flag = Flag {
    label: "X-Flip",
    icon: Icon::FlipHorizontally,
    tooltip: X_FLIP_FLAG_TOOLTIP,
};
const Y_FLIP_FLAG: Flag = Flag {
    label: "Y-Flip",
    icon: Icon::FlipVertically,
    tooltip: Y_FLIP_FLAG_TOOLTIP,
};
const FLAGS: [Flag; 4] = [ROTATE_FLAG, FLIP_FLAG, X_FLIP_FLAG, Y_FLIP_FLAG];

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
    Copy(usize),
    Paste(usize),
}

pub struct TilePanel {
    tile: usize,
    index: usize,
    neighborhood: Neighborhood,
    centre: Centre,
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
            centre: rule.centre,
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

    pub fn picking_label(&self) -> Option<&'static str> {
        self.picking.map(|index| NEIGHBORS[index].label)
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
    ) -> Option<TileEdit> {
        let heading_edit = self.show_heading(ui, context);
        ui.add_space(HEADING_SPACING);

        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.stop_picking();
        }

        let changed = self.show_neighborhood(ui, tileset, texture, context);
        ui.add_space(SECTION_SPACING);
        let edit = self.show_options(ui, tileset, texture, changed);

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        self.show_pools(ui, tileset, texture, context);

        heading_edit.or(edit).or(self.key_edit(ui, context))
    }

    fn key_edit(&self, ui: &Ui, context: &TileContext) -> Option<TileEdit> {
        let ctx = ui.ctx();
        if ctx.egui_wants_keyboard_input()
            || ctx.memory(|memory| memory.top_modal_layer().is_some())
        {
            return None;
        }

        let (copy, paste) = ui.input(|input| {
            let plain = input.modifiers.is_none();
            (
                plain && input.key_pressed(egui::Key::C),
                plain && input.key_pressed(egui::Key::V),
            )
        });
        match (copy && self.has_rule, paste && context.copied.is_some()) {
            (true, _) => Some(TileEdit::Copy(self.index)),
            (_, true) => Some(TileEdit::Paste(self.index)),
            _ => None,
        }
    }

    fn show_rule_tabs(&self, ui: &mut Ui, rule_count: usize) -> Option<TileEdit> {
        let mut edit = None;
        ui.spacing_mut().item_spacing.x = tab_spacing(ui);
        for index in 0..rule_count {
            if ui
                .selectable_label(index == self.index, rule_label(index))
                .clicked()
            {
                edit = Some(TileEdit::Select(index));
            }
        }

        edit
    }

    fn show_pools(
        &self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        context: &TileContext,
    ) {
        let shared: Vec<&Pool> = context
            .pools
            .iter()
            .copied()
            .filter(|pool| pool.members.len() > 1)
            .collect();
        if shared.is_empty() {
            return;
        }

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
        ui.add_space(SECTION_SPACING);
        ui.strong(SHARED_POOLS_HEADING);
        for (index, pool) in shared.into_iter().enumerate() {
            self.show_pool(ui, &view, pool, index == 0);
        }
    }

    fn show_pool(&self, ui: &mut Ui, view: &PoolView, pool: &Pool, open: bool) {
        let transform = pool
            .member_of(self.tile)
            .map_or(Transform::IDENTITY, |member| member.transform);
        let share = self.share_label(pool, view.mode);
        let title = match pool.centre {
            Centre::Full => format!("{}: {share}", drawn_name(transform)),
            Centre::Empty => format!("{}, empty center: {share}", drawn_name(transform)),
        };
        let title = match share == view.own_share {
            true => egui::RichText::new(title),
            false => egui::RichText::new(title).color(ui.visuals().warn_fg_color),
        };

        let header = egui::CollapsingHeader::new(title)
            .id_salt(("pool", self.tile, pool.neighborhood, pool.centre))
            .default_open(open)
            .show(ui, |ui| self.show_members(ui, view, pool));
        header.header_response.on_hover_text(POOL_TOOLTIP);
    }

    fn show_members(&self, ui: &mut Ui, view: &PoolView, pool: &Pool) {
        egui::Grid::new(("pool_members", pool.neighborhood, pool.centre)).show(ui, |ui| {
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

    fn show_heading(&mut self, ui: &mut Ui, context: &TileContext) -> Option<TileEdit> {
        let mut tab_edit = None;
        let size = ui.spacing().interact_size.y;
        let (button_edit, tabs_fit) = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = HEADING_ROW_SPACING;
                ui.heading(format!("Tile {}", self.tile));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_edit = ui
                        .scope(|ui| self.show_rule_buttons(ui, context, size))
                        .inner;
                    let gap = ui.available_rect_before_wrap();
                    let tabs_fit = rule_tabs_width(ui, context.rule_count) <= gap.width();
                    if tabs_fit {
                        let builder = egui::UiBuilder::new()
                            .max_rect(gap)
                            .layout(egui::Layout::left_to_right(egui::Align::Center));
                        tab_edit = ui
                            .scope_builder(builder, |ui| {
                                self.show_rule_tabs(ui, context.rule_count)
                            })
                            .inner;
                    }
                    (button_edit, tabs_fit)
                })
                .inner
            })
            .inner;

        if !tabs_fit {
            ui.horizontal_wrapped(|ui| {
                tab_edit = self.show_rule_tabs(ui, context.rule_count);
            });
        }

        button_edit.or(tab_edit)
    }

    fn show_rule_buttons(
        &mut self,
        ui: &mut Ui,
        context: &TileContext,
        size: f32,
    ) -> Option<TileEdit> {
        let mut edit = None;
        ui.spacing_mut().item_spacing.x = BUTTON_GAP;

        let paste = ui.add_enabled(context.copied.is_some(), compact_button(PASTE_ICON, size));
        let paste = match context.copied {
            Some(copied) => paste.on_hover_text(copied.paste_tooltip()),
            None => paste.on_disabled_hover_text(NOTHING_COPIED_TOOLTIP),
        };
        if paste.clicked() {
            edit = Some(TileEdit::Paste(self.index));
        }

        let copy = ui.add_enabled(self.has_rule, compact_button(COPY_ICON, size));
        if copy.on_hover_text(COPY_RULE_TOOLTIP).clicked() {
            edit = Some(TileEdit::Copy(self.index));
        }

        ui.add(egui::Separator::default().spacing(HEADING_ROW_SPACING));

        let remove = ui.add_enabled(self.has_rule, compact_button(REMOVE_ICON, size));
        if remove.on_hover_text(REMOVE_RULE_TOOLTIP).clicked() {
            self.error = None;
            edit = Some(TileEdit::Remove(self.index));
        }

        let duplicate = ui.add_enabled(self.has_rule, compact_button(DUPLICATE_ICON, size));
        if duplicate.on_hover_text(DUPLICATE_RULE_TOOLTIP).clicked() {
            edit = Some(TileEdit::Duplicate(self.index));
        }

        if ui
            .add(compact_button(ADD_ICON, size))
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
            centre: self.centre,
            mods: self.mods,
            custom: self.custom,
            drawn: self.drawn,
            chance,
        }
    }

    fn turn(&mut self, turn: fn(TileRule, Transform) -> TileRule, by: Transform) {
        let turned = turn(self.rule(Chance::FULL), by);
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
        context: &TileContext,
    ) -> bool {
        let mut changed = false;
        let size = cell_size(ui);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(CELL_GAP);

            let grid_area = Rect::from_min_size(ui.cursor().min, Vec2::splat(grid_extent(size)));
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
                                    context.always_show_turn_buttons,
                                )
                            }
                        }
                    }
                });
            }

            paint_flash(ui, grid_area, context.flash);
            changed |= self.show_neighbor_turn_row(ui, size);
        });

        changed
    }

    fn reset_drawn(&mut self) {
        self.drawn = Transform::IDENTITY;
        self.picking = None;
    }

    fn show_neighbor_turn_row(&mut self, ui: &mut Ui, size: f32) -> bool {
        let (turn, chance_changed) = egui::containers::Sides::new().show(
            ui,
            |ui| show_neighbor_turn_buttons(ui, button_size(size)),
            |ui| show_chance(ui, &mut self.chance_percent),
        );

        let Some(by) = turn else {
            return chance_changed;
        };
        self.turn(TileRule::turned_neighbors, by);
        true
    }

    fn show_centre_cell(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        size: f32,
        always_show_turn_buttons: bool,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
        self.paint_centre(ui, rect, tileset, texture);

        let button_size = button_size(size);
        let bounds = rect.shrink(BUTTON_MARGIN);
        let mut turn = None;
        let mut reset = false;
        if always_show_turn_buttons || ui.rect_contains_pointer(rect) {
            for (index, button) in TURN_BUTTONS.into_iter().enumerate() {
                let area = button_rect(bounds, Align2::CENTER_BOTTOM, index, button_size);
                if turn_button(ui, area, button.icon, button.tile_tooltip) {
                    turn = Some(button.turn);
                }
            }
            if self.drawn != Transform::IDENTITY {
                let area =
                    Align2::RIGHT_TOP.align_size_within_rect(Vec2::splat(button_size), bounds);
                reset = turn_button(ui, area, Icon::Reset, RESET_TOOLTIP);
            }
        }
        let clicked = clicked_centre(&response).filter(|centre| *centre != self.centre);
        response.on_hover_text(format!(
            "{}\n{ORIENTATION_HINT}\n{CENTRE_HINT}",
            drawn_name(self.drawn)
        ));

        if reset {
            self.reset_drawn();
        }
        if let Some(by) = turn {
            self.turn(TileRule::turned_tile, by);
        }
        if let Some(centre) = clicked {
            self.centre = centre;
            self.picking = None;
        }
        reset || turn.is_some() || clicked.is_some()
    }

    fn paint_centre(&self, ui: &Ui, rect: Rect, tileset: &Tileset, texture: &TextureHandle) {
        match self.centre {
            Centre::Full => self.paint_turned_tile(ui, rect, tileset, texture, self.drawn),
            Centre::Empty => {
                paint_state(ui, rect, NeighborState::Empty);
                preview::paint_tinted(
                    ui.painter(),
                    rect,
                    texture,
                    &tileset.tiles[self.tile],
                    self.drawn,
                    EMPTY_CENTRE_TINT,
                );
            }
        }
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
        let is_picked = self.picking == Some(index);
        let outline = match is_picked {
            true => Stroke::new(grid::SELECTED_OUTLINE_WIDTH, ui.visuals().selection.bg_fill),
            false => Stroke::new(OUTLINE_WIDTH, ui.visuals().selection.stroke.color),
        };
        if response.hovered() || is_picked {
            ui.painter()
                .rect_stroke(rect, CORNER_RADIUS, outline, StrokeKind::Inside);
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

    fn show_transform_flags(&mut self, ui: &mut Ui, orientation_box: Rect, width: f32) -> bool {
        let mut shown = match self.custom {
            Some(_) => NO_MODS,
            None => self.mods,
        };
        let column = Rect::from_min_size(
            orientation_box.right_top() + Vec2::X * FLAG_BOX_GAP,
            Vec2::new(width, orientation_box.height()),
        );
        let changed = ui
            .scope_builder(egui::UiBuilder::new().max_rect(column), |ui| {
                show_preset_flags(ui, &mut shown, column.height(), width)
            })
            .inner;
        if changed {
            self.mods = shown;
            self.custom = None;
        }

        changed
    }

    fn show_orientations(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
    ) -> bool {
        let mut chosen = None;
        let mut flags_changed = false;

        let frame = egui::Frame::group(ui.style()).fill(ui.visuals().extreme_bg_color);
        let frame_width = orientation_grid_width() + frame.total_margin().sum().x;
        let flag_width = flag_width(ui);
        let row_width = frame_width + FLAG_BOX_GAP + flag_width;
        let area = ui.available_rect_before_wrap();
        let left = ((area.width() - row_width) / 2.0).max(0.0);
        let centered = egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(area.min + Vec2::X * left, area.max));

        ui.scope_builder(centered, |ui| {
            let orientation_box = frame.show(ui, |ui| {
                egui::Grid::new("orientations")
                    .spacing(Vec2::splat(CELL_GAP))
                    .min_col_width(ORIENTATION_SIZE)
                    .show(ui, |ui| {
                        for row in Transform::ALL.chunks(ORIENTATION_COLUMNS) {
                            for &transform in row {
                                let clicked =
                                    self.show_orientation(ui, tileset, texture, transform);
                                chosen = clicked.or(chosen);
                            }
                            ui.end_row();
                        }
                    });
            });
            flags_changed =
                self.show_transform_flags(ui, orientation_box.response.rect, flag_width);
        });

        let Some(orientations) = chosen else {
            return flags_changed;
        };
        (self.custom, self.mods) = orientations.choice(self.mods);
        true
    }

    fn show_orientation(
        &self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        transform: Transform,
    ) -> Option<Orientations> {
        let orientations = self.orientations();
        let enabled = orientations.contains(transform);
        let locked = enabled && orientations.len() == 1;

        let (rect, response) =
            ui.allocate_exact_size(Vec2::splat(ORIENTATION_SIZE), Sense::click());
        self.paint_turned_tile(ui, rect, tileset, texture, transform.after(self.drawn));
        if !enabled {
            ui.painter().rect_filled(rect, 0.0, DISABLED_DARKEN);
            paint_cross(ui.painter(), rect);
        }
        if response.hovered() {
            paint_hover_outline(ui, rect);
        }

        let enable = response.clicked_by(PointerButton::Primary);
        let disable = response.clicked_by(PointerButton::Secondary) && !locked;
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(match locked {
                true => LAST_ORIENTATION,
                false => transform_name(transform),
            });

        let updated = match (enable, disable) {
            (true, _) => orientations.with(transform),
            (_, true) => orientations.without(transform),
            _ => return None,
        };

        (updated != orientations).then_some(updated)
    }

    fn show_options(
        &mut self,
        ui: &mut Ui,
        tileset: &Tileset,
        texture: &TextureHandle,
        neighborhood_changed: bool,
    ) -> Option<TileEdit> {
        let mut changed = neighborhood_changed;

        ui.label(ORIENTATIONS_LABEL)
            .on_hover_text(ORIENTATIONS_TOOLTIP);
        changed |= self.show_orientations(ui, tileset, texture);

        self.apply_if_changed(changed)
    }
}

fn show_preset_flags(ui: &mut Ui, mods: &mut TileMods, height: f32, width: f32) -> bool {
    let count = match mods.can_rotate {
        true => ROTATING_FLAG_COUNT,
        false => PLAIN_FLAG_COUNT,
    };
    let button_height = (height - (count - 1.0) * FLAG_GAP) / count;
    let size = Vec2::new(width, button_height);
    ui.spacing_mut().item_spacing.y = FLAG_GAP;
    ui.spacing_mut().icon_spacing = FLAG_ICON_GAP;

    let mut changed = toggle_button(ui, size, &ROTATE_FLAG, &mut mods.can_rotate);

    if mods.can_rotate {
        let mut flip = mods.can_x_flip || mods.can_y_flip;
        if toggle_button(ui, size, &FLIP_FLAG, &mut flip) {
            mods.can_x_flip = flip;
            mods.can_y_flip = false;
            changed = true;
        }
    } else {
        changed |= toggle_button(ui, size, &X_FLIP_FLAG, &mut mods.can_x_flip);
        changed |= toggle_button(ui, size, &Y_FLIP_FLAG, &mut mods.can_y_flip);
    }

    changed
}

fn toggle_button(ui: &mut Ui, size: Vec2, flag: &Flag, active: &mut bool) -> bool {
    let icon_id = Id::new(flag.label);
    let icon_atom = Atom::custom(icon_id, Vec2::splat(FLAG_ICON_SIZE));
    let layout = egui::Button::new((icon_atom, flag.label))
        .selected(*active)
        .min_size(size)
        .atom_ui(ui);

    if let Some(rect) = layout.rect(icon_id) {
        let color = ui
            .style()
            .interact_selectable(&layout.response, *active)
            .fg_stroke
            .color;
        paint_stroked_icon(ui.painter(), rect, color, flag.icon);
    }

    let clicked = layout.response.on_hover_text(flag.tooltip).clicked();
    if clicked {
        *active = !*active;
    }

    clicked
}

fn flag_width(ui: &Ui) -> f32 {
    let widest_label = FLAGS
        .iter()
        .map(|flag| {
            egui::WidgetText::from(flag.label)
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    egui::TextStyle::Button,
                )
                .size()
                .x
        })
        .fold(0.0, f32::max);

    FLAG_ICON_SIZE + FLAG_ICON_GAP + widest_label + 2.0 * ui.spacing().button_padding.x
}

fn show_neighbor_turn_buttons(ui: &mut Ui, button_size: f32) -> Option<Transform> {
    let (row, _response) = ui.allocate_exact_size(
        Vec2::new(turn_buttons_width(button_size), button_size),
        Sense::hover(),
    );

    let mut turn = None;
    for (index, button) in TURN_BUTTONS.into_iter().enumerate() {
        let area = button_rect(row, Align2::LEFT_CENTER, index, button_size);
        if turn_button(ui, area, button.icon, button.neighbors_tooltip) {
            turn = Some(button.turn);
        }
    }

    turn
}

fn show_chance(ui: &mut Ui, chance_percent: &mut f32) -> bool {
    ui.spacing_mut().item_spacing = ui.ctx().global_style().spacing.item_spacing;
    let chance = egui::DragValue::new(chance_percent)
        .range(MIN_CHANCE_PERCENT..=MAX_CHANCE_PERCENT)
        .speed(CHANCE_DRAG_SPEED)
        .suffix(" %");
    let changed = ui.add(chance).on_hover_text(CHANCE_TOOLTIP).changed();
    ui.label("Chance").on_hover_text(CHANCE_TOOLTIP);

    changed
}

fn rule_label(index: usize) -> String {
    format!("Rule {}", index + 1)
}

fn rule_tabs_width(ui: &Ui, rule_count: usize) -> f32 {
    let spacing = ui.spacing();
    let tab_gap = tab_spacing(ui);
    let tabs: f32 = (0..rule_count)
        .map(|index| {
            let text = egui::WidgetText::from(rule_label(index)).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            text.size().x + 2.0 * spacing.button_padding.x
        })
        .sum();

    tabs + rule_count.saturating_sub(1) as f32 * tab_gap
}

fn tab_spacing(ui: &Ui) -> f32 {
    ui.ctx().global_style().spacing.item_spacing.x
}

fn turn_buttons_width(button_size: f32) -> f32 {
    let count = TURN_BUTTONS.len() as f32;
    count * button_size + (count - 1.0) * BUTTON_GAP
}

fn orientation_grid_width() -> f32 {
    let columns = ORIENTATION_COLUMNS as f32;
    columns * ORIENTATION_SIZE + (columns - 1.0) * CELL_GAP
}

fn grid_extent(cell_size: f32) -> f32 {
    GRID_SIDE * cell_size + (GRID_SIDE - 1.0) * CELL_GAP
}

fn paint_flash(ui: &Ui, rect: Rect, flash: Option<GridFlash>) {
    let Some(flash) = flash else {
        return;
    };

    let base = match flash.kind {
        FlashKind::Copy => ui.visuals().selection.bg_fill,
        FlashKind::Paste => ui.visuals().warn_fg_color,
    };
    let color = base.gamma_multiply(flash.remaining * FLASH_PEAK_OPACITY);
    ui.painter().rect_filled(rect, CORNER_RADIUS, color);
}

fn button_size(cell_size: f32) -> f32 {
    (cell_size * BUTTON_FRACTION).max(MIN_BUTTON_SIZE)
}

fn button_rect(bounds: Rect, align: Align2, index: usize, size: f32) -> Rect {
    let row = Vec2::new(turn_buttons_width(size), size);
    let area = align.align_size_within_rect(row, bounds);

    Rect::from_min_size(
        area.min + Vec2::X * index as f32 * (size + BUTTON_GAP),
        Vec2::splat(size),
    )
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
    paint_stroked_icon(ui.painter(), icon_area, visuals.fg_stroke.color, icon);

    let clicked = response.clicked();
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip);

    clicked
}

fn paint_stroked_icon(painter: &Painter, rect: Rect, color: Color32, icon: Icon) {
    let width = (rect.width() * ICON_STROKE_FRACTION).max(MIN_ICON_STROKE);
    paint_icon(painter, rect, Stroke::new(width, color), icon);
}

fn paint_icon(painter: &Painter, rect: Rect, stroke: Stroke, icon: Icon) {
    match icon {
        Icon::Rotate => paint_curl(painter, rect, stroke),
        Icon::FlipHorizontally => paint_flip(painter, rect, stroke, true),
        Icon::FlipVertically => paint_flip(painter, rect, stroke, false),
        Icon::Reset => {
            let font = FontId::proportional(rect.height());
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                RESET_GLYPH,
                font,
                stroke.color,
            );
        }
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
    ((ui.available_width() - (GRID_SIDE - 1.0) * CELL_GAP) / GRID_SIDE)
        .floor()
        .max(MIN_CELL_SIZE)
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

fn clicked_centre(response: &Response) -> Option<Centre> {
    if response.clicked_by(PointerButton::Primary) {
        return Some(Centre::Full);
    }

    response
        .clicked_by(PointerButton::Secondary)
        .then_some(Centre::Empty)
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

pub fn show_legend(ui: &mut Ui) {
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
