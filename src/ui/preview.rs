use egui::emath::GuiRounding as _;
use egui::epaint::Vertex;
use egui::{
    Align2, Color32, CursorIcon, FontId, Mesh, PointerButton, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextureHandle, Vec2, pos2,
};

use crate::model::transform::Transform;
use crate::preview::{Automapped, CustomShape, PlacedTile, PreviewCell};
use crate::tileset::{TileSlice, Tileset};

const MIN_CELL_SIZE: f32 = 4.0;
const BACKGROUND: Color32 = Color32::from_gray(40);
const UNMATCHED_COLOR: Color32 = Color32::RED;
const UNMATCHED_STROKE_WIDTH: f32 = 1.0;
const HOVER_STROKE_WIDTH: f32 = 2.0;
const UNMATCHED_MARK: &str = "?";
const UNMATCHED_MARK_SCALE: f32 = 0.7;
const MIN_MARK_CELL_SIZE: f32 = 10.0;
const PATH_SAMPLES_PER_CELL: f32 = 2.0;

#[derive(Default)]
pub struct Interaction {
    pub clicked: Option<PlacedTile>,
    pub edited: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    tileset: &Tileset,
    texture: &TextureHandle,
    automapped: &Automapped,
    drawing: Option<&mut CustomShape>,
) -> Interaction {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let cell_size = whole_pixel_cell_size(ui.available_size(), automapped, pixels_per_point);
    let size = Vec2::new(
        cell_size * automapped.width as f32,
        cell_size * automapped.height as f32,
    );

    let sense = match drawing {
        Some(_) => Sense::click_and_drag(),
        None => Sense::click(),
    };
    let (allocated, response) = ui.allocate_exact_size(size, sense);
    let rect = Rect::from_min_size(
        allocated.min.round_to_pixels(pixels_per_point),
        allocated.size(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, BACKGROUND);

    let mut mesh = Mesh::with_texture(texture.id());
    let mut unmatched = Vec::new();
    for row in 0..automapped.height {
        for column in 0..automapped.width {
            let cell = cell_rect(rect, cell_size, column, row);

            match automapped.cell(column, row) {
                PreviewCell::Air => {}
                PreviewCell::Unmatched => unmatched.push(cell),
                PreviewCell::Placed(placed) => {
                    if let Some(slice) = tileset.tiles.get(placed.index) {
                        add_tile(&mut mesh, cell, slice, placed.transform, Color32::WHITE);
                    }
                }
            }
        }
    }

    painter.add(egui::Shape::mesh(mesh));
    for cell in unmatched {
        paint_unmatched(&painter, cell);
    }

    match drawing {
        Some(shape) => Interaction {
            clicked: None,
            edited: draw_cells(ui, &response, &painter, rect, cell_size, shape),
        },
        None => Interaction {
            clicked: clicked_tile(ui, &response, &painter, rect, cell_size, automapped),
            edited: false,
        },
    }
}

fn clicked_tile(
    ui: &egui::Ui,
    response: &egui::Response,
    painter: &egui::Painter,
    rect: Rect,
    cell_size: f32,
    automapped: &Automapped,
) -> Option<PlacedTile> {
    let (column, row) = response
        .hover_pos()
        .and_then(|pos| cell_at(pos, rect, cell_size, automapped.width, automapped.height))?;
    let PreviewCell::Placed(placed) = automapped.cell(column, row) else {
        return None;
    };

    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    outline_cell(ui, painter, cell_rect(rect, cell_size, column, row));

    response
        .clicked_by(PointerButton::Primary)
        .then_some(placed)
}

fn outline_cell(ui: &egui::Ui, painter: &egui::Painter, cell: Rect) {
    painter.rect_stroke(
        cell,
        0.0,
        Stroke::new(HOVER_STROKE_WIDTH, ui.visuals().selection.stroke.color),
        StrokeKind::Inside,
    );
}

fn draw_cells(
    ui: &egui::Ui,
    response: &egui::Response,
    painter: &egui::Painter,
    rect: Rect,
    cell_size: f32,
    shape: &mut CustomShape,
) -> bool {
    if let Some((column, row)) = response
        .hover_pos()
        .and_then(|pos| cell_at(pos, rect, cell_size, shape.width(), shape.height()))
    {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        outline_cell(ui, painter, cell_rect(rect, cell_size, column, row));
    }

    let Some(to) = response.interact_pointer_pos() else {
        return false;
    };
    let (primary_down, secondary_down) = ui.input(|input| {
        (
            input.pointer.primary_down(),
            input.pointer.secondary_down(),
        )
    });
    let solid = if primary_down {
        true
    } else if secondary_down {
        false
    } else {
        return false;
    };

    let mut edited = false;
    for pos in pointer_path(to - response.drag_delta(), to, cell_size) {
        if let Some((column, row)) =
            cell_at(pos, rect, cell_size, shape.width(), shape.height())
        {
            edited |= shape.set(column, row, solid);
        }
    }

    edited
}

fn pointer_path(from: Pos2, to: Pos2, cell_size: f32) -> impl Iterator<Item = Pos2> {
    let steps = ((to - from).length() / cell_size * PATH_SAMPLES_PER_CELL)
        .ceil()
        .max(1.0) as usize;

    (0..=steps).map(move |step| from.lerp(to, step as f32 / steps as f32))
}

fn cell_rect(rect: Rect, cell_size: f32, column: usize, row: usize) -> Rect {
    let min = rect.min + Vec2::new(column as f32, row as f32) * cell_size;
    Rect::from_min_size(min, Vec2::splat(cell_size))
}

fn cell_at(
    pos: Pos2,
    rect: Rect,
    cell_size: f32,
    width: usize,
    height: usize,
) -> Option<(usize, usize)> {
    let offset = pos - rect.min;
    let column = (offset.x / cell_size) as usize;
    let row = (offset.y / cell_size) as usize;

    (offset.x >= 0.0 && offset.y >= 0.0 && column < width && row < height)
        .then_some((column, row))
}

fn paint_unmatched(painter: &egui::Painter, cell: Rect) {
    let stroke = Stroke::new(UNMATCHED_STROKE_WIDTH, UNMATCHED_COLOR);
    painter.rect_stroke(cell, 0.0, stroke, StrokeKind::Inside);

    if cell.width() >= MIN_MARK_CELL_SIZE {
        painter.text(
            cell.center(),
            Align2::CENTER_CENTER,
            UNMATCHED_MARK,
            FontId::proportional(cell.width() * UNMATCHED_MARK_SCALE),
            UNMATCHED_COLOR,
        );
    }
}

fn whole_pixel_cell_size(available: Vec2, automapped: &Automapped, pixels_per_point: f32) -> f32 {
    let fit = Vec2::new(
        available.x / automapped.width as f32,
        available.y / automapped.height as f32,
    );
    let points = fit.min_elem().max(MIN_CELL_SIZE);

    (points * pixels_per_point).floor() / pixels_per_point
}

pub fn paint_turned(
    painter: &egui::Painter,
    cell: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    transform: Transform,
) {
    paint_tinted(painter, cell, texture, slice, transform, Color32::WHITE);
}

pub fn paint_tinted(
    painter: &egui::Painter,
    cell: Rect,
    texture: &TextureHandle,
    slice: &TileSlice,
    transform: Transform,
    tint: Color32,
) {
    let mut mesh = Mesh::with_texture(texture.id());
    add_tile(&mut mesh, cell, slice, transform, tint);
    painter.add(egui::Shape::mesh(mesh));
}

fn add_tile(mesh: &mut Mesh, cell: Rect, slice: &TileSlice, transform: Transform, tint: Color32) {
    let corners = [
        cell.left_top(),
        cell.right_top(),
        cell.right_bottom(),
        cell.left_bottom(),
    ];
    let uvs = corner_uvs(slice, transform);

    let first = mesh.vertices.len() as u32;
    for (pos, uv) in corners.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            pos,
            uv,
            color: tint,
        });
    }
    mesh.add_triangle(first, first + 1, first + 2);
    mesh.add_triangle(first, first + 2, first + 3);
}

/// Texture corners for the screen corners top-left, top-right, bottom-right,
/// bottom-left, in DDNet's order: flips first, then a clockwise quarter turn.
fn corner_uvs(slice: &TileSlice, transform: Transform) -> [Pos2; 4] {
    let [left, top] = slice.uv_min;
    let [right, bottom] = slice.uv_max;
    let mut uvs = [
        pos2(left, top),
        pos2(right, top),
        pos2(right, bottom),
        pos2(left, bottom),
    ];

    if transform.x_flip {
        uvs.swap(0, 1);
        uvs.swap(2, 3);
    }
    if transform.y_flip {
        uvs.swap(0, 3);
        uvs.swap(1, 2);
    }
    if transform.rot {
        uvs.rotate_right(1);
    }

    uvs
}
