use std::collections::BTreeMap;

use egui::ecolor::Hsva;
use egui::emath::GuiRounding as _;
use egui::{
    Color32, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle,
    Vec2, pos2,
};

use crate::model::group::{GroupPreset, TileGroup};
use crate::model::project::Project;
use crate::model::tile::{Chance, TILESET_SIDE};
use crate::tileset::{TileSlice, Tileset};
use crate::ui::allocate_centered;
use crate::ui::hover_card;
use crate::ui::tile_state::{TileState, tile_state};

const MIN_CELL_SIZE: f32 = 8.0;
const CUSTOM_LABEL: &str = "Custom";

const MIN_BADGE_CELL_SIZE: f32 = 16.0;

const CHECKER_LIGHT: Color32 = Color32::from_gray(64);
const CHECKER_DARK: Color32 = Color32::from_gray(48);

const CONFIGURED_OUTLINE: Color32 = Color32::from_gray(205);
const DEACTIVATED_OUTLINE: Color32 = Color32::from_gray(100);
const SELECTED_OUTLINE: Color32 = Color32::from_rgb(255, 214, 102);
const POOL_MATE_OUTLINE: Color32 = Color32::from_rgb(102, 204, 255);

const DEACTIVATED_DARKEN_ALPHA: u8 = 50;

const OUTLINE_WIDTH: f32 = 2.0;
pub(crate) const SELECTED_OUTLINE_WIDTH: f32 = 3.0;

/// Hues a golden angle apart, so neighbouring groups never share a shade.
const HUE_STEP: f32 = 137.5 / 360.0;
const GROUP_FILL_ALPHA: f32 = 0.35;
const DRAG_FILL_ALPHA: f32 = 0.25;

const DIMMED_GROUP_FILL_ALPHA: f32 = 0.12;
const DIMMED_GROUP_STROKE_ALPHA: f32 = 0.4;
const SELECTED_GROUP_OUTLINE_WIDTH: f32 = 4.0;

const NEW_GROUP_FILL: Color32 = Color32::from_rgba_premultiplied(30, 30, 30, 30);
const NEW_GROUP_DASH_LENGTH: f32 = 6.0;
const NEW_GROUP_GAP_LENGTH: f32 = 4.0;

const ACTION_LABEL_OFFSET: Vec2 = Vec2::new(16.0, 16.0);
const ACTION_LABEL_ID: &str = "group_action_label";

struct GroupLook {
    fill_alpha: f32,
    stroke_alpha: f32,
    width: f32,
}

const NORMAL_GROUP_LOOK: GroupLook = GroupLook {
    fill_alpha: GROUP_FILL_ALPHA,
    stroke_alpha: 1.0,
    width: OUTLINE_WIDTH,
};
const DIMMED_GROUP_LOOK: GroupLook = GroupLook {
    fill_alpha: DIMMED_GROUP_FILL_ALPHA,
    stroke_alpha: DIMMED_GROUP_STROKE_ALPHA,
    width: OUTLINE_WIDTH,
};
const SELECTED_GROUP_LOOK: GroupLook = GroupLook {
    fill_alpha: GROUP_FILL_ALPHA,
    stroke_alpha: 1.0,
    width: SELECTED_GROUP_OUTLINE_WIDTH,
};

const GROUP_NAME_INSET: f32 = 0.1;
const GROUP_NAME_FONT: f32 = 0.28;
const BADGE_INSET: f32 = 0.12;
const BADGE_MARK: f32 = 0.3;
const CHANCE_FONT: f32 = 0.26;
const CROSS_WIDTH: f32 = 0.18;
const MIN_CROSS_WIDTH: f32 = 1.5;

const PERCENT_ROUNDING: f32 = 10.0;

pub struct GridView<'a> {
    pub tileset: &'a Tileset,
    pub project: &'a Project,
    pub shares: &'a BTreeMap<usize, f32>,
    pub texture: &'a TextureHandle,
    pub group_editing: bool,
    pub drag_anchor: Option<usize>,
    pub selected: Option<usize>,
    pub selected_group: Option<usize>,
    pub pool_mates: &'a [usize],
    pub picked: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEdit {
    Create,
    Shape(ShapeEdit),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeEdit {
    Add,
    Remove,
}

impl DragEdit {
    pub fn of(modifiers: Modifiers) -> Self {
        match (modifiers.ctrl, modifiers.shift) {
            (true, _) => Self::Shape(ShapeEdit::Remove),
            (false, true) => Self::Shape(ShapeEdit::Add),
            (false, false) => Self::Create,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GridResponse {
    pub hovered: Option<usize>,
    pub clicked: Option<usize>,
    pub secondary_clicked: Option<usize>,
    pub drag_started: Option<usize>,
    pub drag_released: Option<usize>,
}

/// One decimal at most, and none for whole numbers.
pub fn format_percent(percent: f32) -> String {
    let rounded = (percent * PERCENT_ROUNDING).round() / PERCENT_ROUNDING;

    format!("{rounded}%")
}

pub fn group_color(index: usize) -> Color32 {
    Hsva::new((index as f32 * HUE_STEP).fract(), 0.6, 0.95, 1.0).into()
}

pub fn show(ui: &mut egui::Ui, view: GridView<'_>) -> GridResponse {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let cell_size = whole_pixel_cell_size(ui.available_size(), pixels_per_point);
    let grid_size = cell_size * TILESET_SIDE as f32;

    let (allocated, response) =
        allocate_centered(ui, Vec2::splat(grid_size), Sense::click_and_drag());
    let rect = Rect::from_min_size(
        allocated.min.round_to_pixels(pixels_per_point),
        allocated.size(),
    );
    let geometry = GridGeometry { rect, cell_size };
    let painter = ui.painter_at(rect);

    let hovered = response
        .hover_pos()
        .and_then(|pos| geometry.tile_index_at(pos));
    let pointed = response
        .interact_pointer_pos()
        .map(|pos| geometry.tile_index_clamped(pos));

    for (index, tile) in view.tileset.tiles.iter().enumerate() {
        let cell = geometry.cell_rect(index);
        let state = tile_state(view.tileset, view.project, index);

        paint_checkerboard(&painter, cell, index);

        if !matches!(state, TileState::Locked) {
            paint_tile(&painter, cell, view.texture, tile);
        }

        if let Some(darkening) = tile_darkening(state) {
            painter.rect_filled(cell, 0.0, darkening);
        }

        if let Some(color) = outline_color(state) {
            paint_outline(&painter, &view, cell, index, color, pixels_per_point);
        }

        let share = view.shares.get(&index).copied();
        paint_badge(&painter, cell, cell_size, state, share, ui.visuals());
    }

    for (index, group) in view.project.groups().iter().enumerate() {
        let look = group_look(&view, index);
        paint_group(&painter, geometry, group, group_color(index), look);
    }

    let edit = DragEdit::of(ui.input(|input| input.modifiers));
    if view.group_editing {
        let corner = pointed.or(hovered);
        let visuals = ui.visuals();
        paint_group_edit_preview(&painter, &view, edit, geometry, corner, hovered, visuals);
    }

    let outliner = BorderOutliner {
        painter: ui.painter_at(rect.expand(SELECTED_OUTLINE_WIDTH)),
        geometry,
        pixels_per_point,
    };
    for &index in view.pool_mates {
        outliner.paint_centred_on_border(index, SELECTED_OUTLINE_WIDTH, POOL_MATE_OUTLINE);
    }
    if let Some(index) = view.picked {
        let color = ui.visuals().selection.bg_fill;
        outliner.paint_centred_on_border(index, SELECTED_OUTLINE_WIDTH, color);
    }
    if let Some(index) = view.selected {
        outliner.paint_centred_on_border(index, SELECTED_OUTLINE_WIDTH, SELECTED_OUTLINE);
    }
    if let Some(index) = hovered {
        outliner.paint_centred_on_border(index, OUTLINE_WIDTH, ui.visuals().selection.stroke.color);
    }

    let label_pos = response
        .hover_pos()
        .filter(|_| shows_action_label(&view, edit, hovered));
    if let Some(pos) = label_pos {
        show_action_label(ui.ctx(), pos, action_label(edit, &view));
    }

    let response = attach_tooltip(response, &view, hovered.filter(|_| label_pos.is_none()));

    if view.group_editing {
        return GridResponse {
            hovered,
            clicked: hovered.filter(|_| response.clicked()),
            secondary_clicked: hovered.filter(|_| response.secondary_clicked()),
            drag_started: pointed.filter(|_| response.drag_started()),
            drag_released: pointed.filter(|_| response.drag_stopped()),
        };
    }

    GridResponse {
        hovered,
        clicked: hovered.filter(|_| released_by(&response, PointerButton::Primary)),
        secondary_clicked: hovered.filter(|_| released_by(&response, PointerButton::Secondary)),
        ..GridResponse::default()
    }
}

fn selected_group<'a>(view: &GridView<'a>) -> Option<(usize, &'a TileGroup)> {
    let index = view.selected_group?;

    Some((index, view.project.groups().get(index)?))
}

fn action_label(edit: DragEdit, view: &GridView<'_>) -> String {
    match (edit, selected_group(view)) {
        (DragEdit::Create, _) => "New group".to_owned(),
        (DragEdit::Shape(_), None) => "Select a group first".to_owned(),
        (DragEdit::Shape(ShapeEdit::Add), Some((_, group))) => format!("Add to {}", group.name),
        (DragEdit::Shape(ShapeEdit::Remove), Some((_, group))) => {
            format!("Remove from {}", group.name)
        }
    }
}

fn show_action_label(ctx: &egui::Context, pointer: Pos2, text: String) {
    egui::Area::new(egui::Id::new(ACTION_LABEL_ID))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointer + ACTION_LABEL_OFFSET)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.add(egui::Label::new(text).extend());
            });
        });
}

fn edit_color(edit: DragEdit, view: &GridView<'_>, visuals: &egui::Visuals) -> Option<Color32> {
    let (selected, _) = selected_group(view)?;

    match edit {
        DragEdit::Create => None,
        DragEdit::Shape(ShapeEdit::Add) => Some(group_color(selected)),
        DragEdit::Shape(ShapeEdit::Remove) => Some(visuals.error_fg_color),
    }
}

fn group_look(view: &GridView<'_>, index: usize) -> &'static GroupLook {
    match selected_group(view) {
        Some((selected, _)) if view.group_editing && selected == index => &SELECTED_GROUP_LOOK,
        Some(_) if view.group_editing => &DIMMED_GROUP_LOOK,
        _ => &NORMAL_GROUP_LOOK,
    }
}

fn paint_group_edit_preview(
    painter: &egui::Painter,
    view: &GridView<'_>,
    edit: DragEdit,
    geometry: GridGeometry,
    corner: Option<usize>,
    hovered: Option<usize>,
    visuals: &egui::Visuals,
) {
    if edit == DragEdit::Create
        && let (Some(anchor), Some(corner)) = (view.drag_anchor, corner)
    {
        paint_new_group_preview(painter, geometry, anchor, corner);
        return;
    }

    let Some(color) = edit_color(edit, view, visuals) else {
        return;
    };

    match (view.drag_anchor, corner, hovered) {
        (Some(anchor), Some(corner), _) => {
            paint_selection(painter, geometry, anchor, corner, color)
        }
        (None, _, Some(tile)) => {
            let tint = color.gamma_multiply(DRAG_FILL_ALPHA);
            painter.rect_filled(geometry.cell_rect(tile), 0.0, tint);
        }
        _ => {}
    }
}

fn shows_action_label(view: &GridView<'_>, edit: DragEdit, hovered: Option<usize>) -> bool {
    if !view.group_editing || hovered.is_none() {
        return false;
    }

    match (edit, view.drag_anchor) {
        (DragEdit::Create, None) => false,
        (DragEdit::Create, Some(anchor)) => view.project.group_at(anchor).is_none(),
        (DragEdit::Shape(_), _) => true,
    }
}

fn released_by(response: &egui::Response, button: PointerButton) -> bool {
    response.clicked_by(button) || response.drag_stopped_by(button)
}

fn attach_tooltip(
    response: egui::Response,
    view: &GridView<'_>,
    hovered: Option<usize>,
) -> egui::Response {
    match hovered {
        Some(tile) if view.group_editing => match group_of(view.project, tile) {
            Some(group) => response.on_hover_text(describe_group(group)),
            None => response,
        },
        Some(tile) if tile_state(view.tileset, view.project, tile) == TileState::Locked => response,
        Some(tile) => response.on_hover_ui_at_pointer(|ui| hover_card::show(ui, view, tile)),
        None => response,
    }
}

pub fn group_of(project: &Project, tile: usize) -> Option<&TileGroup> {
    project.groups().get(project.group_at(tile)?)
}

pub fn describe_group(group: &TileGroup) -> String {
    let mut text = format!(
        "{}: {}, {}",
        group.name,
        group.size_label(),
        group.preset().map_or(CUSTOM_LABEL, GroupPreset::label)
    );
    if !group.chance.is_full() {
        text.push_str(&format!(", {}%", group.chance.percent()));
    }

    text
}

fn paint_group(
    painter: &egui::Painter,
    geometry: GridGeometry,
    group: &TileGroup,
    color: Color32,
    look: &GroupLook,
) {
    let cell_size = geometry.cell_size;
    let stroke = color.gamma_multiply(look.stroke_alpha);
    for &tile in &group.tiles {
        let cell = geometry.cell_rect(tile);
        painter.rect_filled(cell, 0.0, color.gamma_multiply(look.fill_alpha));
        for edge in outer_edges(group, tile, cell, look.width) {
            painter.rect_filled(edge, 0.0, stroke);
        }
    }

    if let Some(anchor) = group.anchor() {
        painter.text(
            geometry.cell_rect(anchor).left_top() + Vec2::splat(cell_size * GROUP_NAME_INSET),
            egui::Align2::LEFT_TOP,
            &group.name,
            egui::FontId::proportional(cell_size * GROUP_NAME_FONT),
            Color32::WHITE,
        );
    }

    if group.chance.is_full() || cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }
    let Some(badge_tile) = group.bottom_left_tile() else {
        return;
    };

    let inset = Vec2::new(cell_size * BADGE_INSET, -cell_size * BADGE_INSET);
    painter.text(
        geometry.cell_rect(badge_tile).left_bottom() + inset,
        egui::Align2::LEFT_BOTTOM,
        format!("{}%", group.chance.percent()),
        egui::FontId::proportional(cell_size * CHANCE_FONT),
        Color32::WHITE,
    );
}

fn offset_tile(tile: usize, dx: isize, dy: isize) -> Option<usize> {
    let column = (tile % TILESET_SIDE)
        .checked_add_signed(dx)
        .filter(|column| *column < TILESET_SIDE)?;
    let row = (tile / TILESET_SIDE).checked_add_signed(dy)?;

    Some(row * TILESET_SIDE + column)
}

fn corner_square(cell: Rect, dx: isize, dy: isize, width: f32) -> Rect {
    let x = match dx < 0 {
        true => cell.min.x,
        false => cell.max.x - width,
    };
    let y = match dy < 0 {
        true => cell.min.y,
        false => cell.max.y - width,
    };

    Rect::from_min_size(pos2(x, y), Vec2::splat(width))
}

fn outer_edges(group: &TileGroup, tile: usize, cell: Rect, width: f32) -> Vec<Rect> {
    let covered = |dx, dy| offset_tile(tile, dx, dy).is_some_and(|other| group.covers(other));
    let top = Rect::from_min_max(cell.min, pos2(cell.max.x, cell.min.y + width));
    let bottom = Rect::from_min_max(pos2(cell.min.x, cell.max.y - width), cell.max);
    let left = Rect::from_min_max(cell.min, pos2(cell.min.x + width, cell.max.y));
    let right = Rect::from_min_max(pos2(cell.max.x - width, cell.min.y), cell.max);

    let sides = [(0, -1, top), (0, 1, bottom), (-1, 0, left), (1, 0, right)]
        .into_iter()
        .filter(|&(dx, dy, _)| !covered(dx, dy))
        .map(|(_, _, strip)| strip);
    let inner_corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)]
        .into_iter()
        .filter(|&(dx, dy)| covered(dx, 0) && covered(0, dy) && !covered(dx, dy))
        .map(|(dx, dy)| corner_square(cell, dx, dy, width));

    sides.chain(inner_corners).collect()
}

fn paint_selection(
    painter: &egui::Painter,
    geometry: GridGeometry,
    anchor: usize,
    corner: usize,
    color: Color32,
) {
    let (top_left, bottom_right) = corners(anchor, corner);
    let area = geometry.tile_span(top_left, bottom_right);

    painter.rect_filled(area, 0.0, color.gamma_multiply(DRAG_FILL_ALPHA));
    painter.rect_stroke(
        area,
        0.0,
        Stroke::new(OUTLINE_WIDTH, color),
        StrokeKind::Inside,
    );
}

fn paint_new_group_preview(
    painter: &egui::Painter,
    geometry: GridGeometry,
    anchor: usize,
    corner: usize,
) {
    let (top_left, bottom_right) = corners(anchor, corner);
    let area = geometry.tile_span(top_left, bottom_right);
    let outline = area.shrink(OUTLINE_WIDTH / 2.0);
    let path = [
        outline.left_top(),
        outline.right_top(),
        outline.right_bottom(),
        outline.left_bottom(),
        outline.left_top(),
    ];

    painter.rect_filled(area, 0.0, NEW_GROUP_FILL);
    painter.extend(Shape::dashed_line(
        &path,
        Stroke::new(OUTLINE_WIDTH, Color32::WHITE),
        NEW_GROUP_DASH_LENGTH,
        NEW_GROUP_GAP_LENGTH,
    ));
}

/// The smallest rectangle holding both tiles, as top-left and bottom-right ids.
pub fn corners(first: usize, second: usize) -> (usize, usize) {
    let columns = [first % TILESET_SIDE, second % TILESET_SIDE];
    let rows = [first / TILESET_SIDE, second / TILESET_SIDE];

    let top_left = rows.iter().min().unwrap() * TILESET_SIDE + columns.iter().min().unwrap();
    let bottom_right = rows.iter().max().unwrap() * TILESET_SIDE + columns.iter().max().unwrap();

    (top_left, bottom_right)
}

struct BorderOutliner {
    painter: egui::Painter,
    geometry: GridGeometry,
    pixels_per_point: f32,
}

impl BorderOutliner {
    fn paint_centred_on_border(&self, index: usize, width: f32, color: Color32) {
        let half = (width / 2.0 * self.pixels_per_point).round().max(1.0) / self.pixels_per_point;
        let cell = self.geometry.cell_rect(index);
        let outer = cell.expand(half);

        for edge in [
            Rect::from_min_max(outer.min, pos2(outer.max.x, cell.min.y + half)),
            Rect::from_min_max(pos2(outer.min.x, cell.max.y - half), outer.max),
            Rect::from_min_max(outer.min, pos2(cell.min.x + half, outer.max.y)),
            Rect::from_min_max(pos2(cell.max.x - half, outer.min.y), outer.max),
        ] {
            self.painter.rect_filled(edge, 0.0, color);
        }
    }
}

fn outline_color(state: TileState) -> Option<Color32> {
    match state {
        TileState::Configured(_) => Some(CONFIGURED_OUTLINE),
        TileState::Deactivated(Some(_)) => Some(DEACTIVATED_OUTLINE),
        _ => None,
    }
}

fn has_same_outline(view: &GridView<'_>, index: usize, color: Color32) -> bool {
    outline_color(tile_state(view.tileset, view.project, index)) == Some(color)
}

fn paint_outline(
    painter: &egui::Painter,
    view: &GridView<'_>,
    cell: Rect,
    index: usize,
    color: Color32,
    pixels_per_point: f32,
) {
    let pixel = 1.0 / pixels_per_point;
    let column = index % TILESET_SIDE;
    let row = index / TILESET_SIDE;
    let right_has_outline = column + 1 < TILESET_SIDE && has_same_outline(view, index + 1, color);
    let below_has_outline =
        row + 1 < TILESET_SIDE && has_same_outline(view, index + TILESET_SIDE, color);

    let mut edges = vec![
        Rect::from_min_max(cell.min, pos2(cell.max.x, cell.min.y + pixel)),
        Rect::from_min_max(cell.min, pos2(cell.min.x + pixel, cell.max.y)),
    ];
    if !right_has_outline {
        edges.push(Rect::from_min_max(
            pos2(cell.max.x - pixel, cell.min.y),
            cell.max,
        ));
    }
    if !below_has_outline {
        edges.push(Rect::from_min_max(
            pos2(cell.min.x, cell.max.y - pixel),
            cell.max,
        ));
    }

    for edge in edges {
        painter.rect_filled(edge, 0.0, color);
    }
}

#[derive(Clone, Copy)]
struct GridGeometry {
    rect: Rect,
    cell_size: f32,
}

impl GridGeometry {
    fn cell_rect(&self, index: usize) -> Rect {
        let column = (index % TILESET_SIDE) as f32;
        let row = (index / TILESET_SIDE) as f32;
        let min = self.rect.min + Vec2::new(column * self.cell_size, row * self.cell_size);

        Rect::from_min_size(min, Vec2::splat(self.cell_size))
    }

    fn tile_span(&self, top_left: usize, bottom_right: usize) -> Rect {
        self.cell_rect(top_left).union(self.cell_rect(bottom_right))
    }

    fn tile_index_at(&self, pos: Pos2) -> Option<usize> {
        let local = pos - self.rect.min;
        let column = (local.x / self.cell_size).floor() as i32;
        let row = (local.y / self.cell_size).floor() as i32;

        let side = TILESET_SIDE as i32;
        if !(0..side).contains(&column) || !(0..side).contains(&row) {
            return None;
        }

        Some((row * side + column) as usize)
    }

    fn tile_index_clamped(&self, pos: Pos2) -> usize {
        let local = pos - self.rect.min;
        let last = TILESET_SIDE as f32 - 1.0;
        let column = (local.x / self.cell_size).floor().clamp(0.0, last) as usize;
        let row = (local.y / self.cell_size).floor().clamp(0.0, last) as usize;

        row * TILESET_SIDE + column
    }
}

fn whole_pixel_cell_size(available: Vec2, pixels_per_point: f32) -> f32 {
    let points = (available.min_elem() / TILESET_SIDE as f32).max(MIN_CELL_SIZE);

    (points * pixels_per_point).floor() / pixels_per_point
}

pub fn paint_tile(painter: &egui::Painter, rect: Rect, texture: &TextureHandle, slice: &TileSlice) {
    let uv = Rect::from_min_max(slice.uv_min.into(), slice.uv_max.into());
    let mut mesh = egui::Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(rect, uv, Color32::WHITE);

    painter.add(egui::Shape::mesh(mesh));
}

fn paint_checkerboard(painter: &egui::Painter, cell: Rect, index: usize) {
    let column = index % TILESET_SIDE;
    let row = index / TILESET_SIDE;
    let color = if (column + row).is_multiple_of(2) {
        CHECKER_LIGHT
    } else {
        CHECKER_DARK
    };

    painter.rect_filled(cell, 0.0, color);
}

fn tile_darkening(state: TileState) -> Option<Color32> {
    match state {
        TileState::Deactivated(_) => Some(Color32::from_black_alpha(DEACTIVATED_DARKEN_ALPHA)),
        TileState::Configured(_)
        | TileState::Grouped
        | TileState::Locked
        | TileState::Unconfigured => None,
    }
}

/// Shapes rather than glyphs, so badges do not depend on font coverage.
fn paint_badge(
    painter: &egui::Painter,
    cell: Rect,
    cell_size: f32,
    state: TileState,
    share: Option<f32>,
    visuals: &egui::Visuals,
) {
    if cell_size < MIN_BADGE_CELL_SIZE {
        return;
    }

    let inset = cell_size * BADGE_INSET;

    match state {
        TileState::Locked | TileState::Grouped | TileState::Unconfigured => {}
        TileState::Deactivated(_) => paint_cross(
            painter,
            Rect::from_center_size(cell.center(), Vec2::splat(cell_size * BADGE_MARK)),
            visuals.warn_fg_color,
        ),
        TileState::Configured(_) => {
            if let Some(share) = share
                && share < Chance::FULL.percent()
            {
                painter.text(
                    pos2(cell.left() + inset, cell.bottom() - inset),
                    egui::Align2::LEFT_BOTTOM,
                    format_percent(share),
                    egui::FontId::proportional(cell_size * CHANCE_FONT),
                    visuals.strong_text_color(),
                );
            }
        }
    }
}

fn paint_cross(painter: &egui::Painter, mark: Rect, color: Color32) {
    let width = (mark.width() * CROSS_WIDTH).max(MIN_CROSS_WIDTH);
    let stroke = Stroke::new(width, color);

    painter.line_segment([mark.left_top(), mark.right_bottom()], stroke);
    painter.line_segment([mark.right_top(), mark.left_bottom()], stroke);
}
