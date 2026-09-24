use egui::emath::GuiRounding as _;
use egui::epaint::Vertex;
use egui::{Color32, Mesh, Pos2, Rect, Sense, TextureHandle, Vec2, pos2};

use crate::model::transform::Transform;
use crate::preview::{Automapped, PlacedTile};
use crate::tileset::{TileSlice, Tileset};

const MIN_CELL_SIZE: f32 = 4.0;
const BACKGROUND: Color32 = Color32::from_gray(40);

pub fn show(
    ui: &mut egui::Ui,
    tileset: &Tileset,
    texture: &TextureHandle,
    automapped: &Automapped,
) {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let cell_size = whole_pixel_cell_size(ui.available_size(), automapped, pixels_per_point);
    let size = Vec2::new(
        cell_size * automapped.width as f32,
        cell_size * automapped.height as f32,
    );

    let (allocated, _response) = ui.allocate_exact_size(size, Sense::hover());
    let rect = Rect::from_min_size(
        allocated.min.round_to_pixels(pixels_per_point),
        allocated.size(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, BACKGROUND);

    let mut mesh = Mesh::with_texture(texture.id());
    for row in 0..automapped.height {
        for column in 0..automapped.width {
            let Some(placed) = automapped.tile(column, row) else {
                continue;
            };
            let Some(slice) = tileset.tiles.get(placed.index) else {
                continue;
            };

            let min = rect.min + Vec2::new(column as f32, row as f32) * cell_size;
            let cell = Rect::from_min_size(min, Vec2::splat(cell_size));
            add_tile(&mut mesh, cell, slice, placed);
        }
    }

    painter.add(egui::Shape::mesh(mesh));
}

fn whole_pixel_cell_size(available: Vec2, automapped: &Automapped, pixels_per_point: f32) -> f32 {
    let fit = Vec2::new(
        available.x / automapped.width as f32,
        available.y / automapped.height as f32,
    );
    let points = fit.min_elem().max(MIN_CELL_SIZE);

    (points * pixels_per_point).floor() / pixels_per_point
}

fn add_tile(mesh: &mut Mesh, cell: Rect, slice: &TileSlice, placed: PlacedTile) {
    let corners = [
        cell.left_top(),
        cell.right_top(),
        cell.right_bottom(),
        cell.left_bottom(),
    ];
    let uvs = corner_uvs(slice, placed.transform);

    let first = mesh.vertices.len() as u32;
    for (pos, uv) in corners.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            pos,
            uv,
            color: Color32::WHITE,
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
