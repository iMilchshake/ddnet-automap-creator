use egui::{
    Atom, Color32, Id, Painter, PointerButton, Rect, Sense, Stroke, StrokeKind, TextureHandle, Ui,
    Vec2,
};

use super::icons::{Icon, paint_stroked_icon};
use super::{CELL_GAP, TileEdit, TilePanel};
use crate::model::tile::TileMods;
use crate::model::transform::{Orientations, Transform};
use crate::tileset::Tileset;
use crate::ui::neighbor_paint::paint_turned_tile;

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

const DISABLED_DARKEN: Color32 = Color32::from_black_alpha(72);
const CROSS_COLOR: Color32 =
    Color32::from_rgba_premultiplied(CROSS_ALPHA, CROSS_ALPHA, CROSS_ALPHA, CROSS_ALPHA);

const NO_MODS: TileMods = TileMods::new(false, false, false);
const LAST_ORIENTATION: &str = "Keep at least one direction.";
const ORIENTATIONS_LABEL: &str = "Rotated and flipped copies";
const ROTATE_FLAG_TOOLTIP: &str = "Also place it rotated by 90°, 180° and 270°";
const X_FLIP_FLAG_TOOLTIP: &str = "Also place it mirrored left-right";
const Y_FLIP_FLAG_TOOLTIP: &str = "Also place it mirrored top-bottom";
const FLIP_FLAG_TOOLTIP: &str = "Also place the mirrored copies";
const ORIENTATIONS_TOOLTIP: &str = "Every highlighted copy gets placed too, wherever the \
                                    neighbors match the same way turned or mirrored.";

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

impl TilePanel {
    pub(super) fn show_options(
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
        let left = (area.width() - row_width).max(0.0);
        let right_aligned = egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(area.min + Vec2::X * left, area.max));

        ui.scope_builder(right_aligned, |ui| {
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
        let slice = &tileset.tiles[self.tile];
        paint_turned_tile(
            ui.painter(),
            rect,
            texture,
            slice,
            transform.after(self.drawn),
        );
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

fn orientation_grid_width() -> f32 {
    let columns = ORIENTATION_COLUMNS as f32;
    columns * ORIENTATION_SIZE + (columns - 1.0) * CELL_GAP
}

fn paint_hover_outline(ui: &Ui, rect: Rect) {
    let stroke = ui.visuals().widgets.hovered.bg_stroke;
    ui.painter()
        .rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
}

fn paint_cross(painter: &Painter, rect: Rect) {
    let rect = rect.shrink(rect.width() * CROSS_INSET_FRACTION);
    let stroke = Stroke::new(CROSS_WIDTH, CROSS_COLOR);
    painter.line_segment([rect.left_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_top(), rect.left_bottom()], stroke);
}

pub(super) fn drawn_name(transform: Transform) -> &'static str {
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
