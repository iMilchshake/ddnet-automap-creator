use image::{GenericImageView, RgbaImage};
use thiserror::Error;

use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};
use crate::model::tile::{TILE_COUNT, TILESET_SIDE};

#[derive(Debug, Error)]
pub enum TilesetError {
    #[error("could not decode the image: {0}")]
    Decode(#[from] image::ImageError),

    #[error(
        "image is {width}×{height}, too small to slice into {TILESET_SIDE}×{TILESET_SIDE} tiles"
    )]
    TooSmall { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Locked,
    Guess(Neighborhood),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileSlice {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub kind: TileKind,
}

#[derive(Debug, Clone)]
pub struct Tileset {
    pub stem: String,
    pub image_size: [u32; 2],
    pub tile_size: [u32; 2],
    pub tiles: Vec<TileSlice>,
    pub rgba: RgbaImage,
}

pub fn decode_tileset(bytes: &[u8], stem: &str) -> Result<Tileset, TilesetError> {
    let rgba = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = rgba.dimensions();

    let side = TILESET_SIDE as u32;
    let tile_size = [width / side, height / side];
    if tile_size[0] == 0 || tile_size[1] == 0 {
        return Err(TilesetError::TooSmall { width, height });
    }

    let stride = [
        width as f32 / TILESET_SIDE as f32,
        height as f32 / TILESET_SIDE as f32,
    ];

    let tiles = (0..TILE_COUNT)
        .map(|index| slice_tile(&rgba, index, tile_size, stride))
        .collect();

    Ok(Tileset {
        stem: stem.to_owned(),
        image_size: [width, height],
        tile_size,
        tiles,
        rgba,
    })
}

pub fn file_stem(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((stem, _extension)) => stem.to_owned(),
        None => file_name.to_owned(),
    }
}

/// Origins follow the fractional stride, so images whose size is not a
/// multiple of 16 do not drift off the art by the end of a row.
fn slice_tile(rgba: &RgbaImage, index: usize, tile_size: [u32; 2], stride: [f32; 2]) -> TileSlice {
    let column = (index % TILESET_SIDE) as u32;
    let row = (index / TILESET_SIDE) as u32;
    let origin = [
        (column as f32 * stride[0]) as u32,
        (row as f32 * stride[1]) as u32,
    ];

    let (width, height) = rgba.dimensions();
    let uv_min = [
        origin[0] as f32 / width as f32,
        origin[1] as f32 / height as f32,
    ];
    let uv_max = [
        (origin[0] + tile_size[0]) as f32 / width as f32,
        (origin[1] + tile_size[1]) as f32 / height as f32,
    ];

    TileSlice {
        uv_min,
        uv_max,
        kind: classify_tile(rgba, origin, tile_size),
    }
}

fn classify_tile(rgba: &RgbaImage, origin: [u32; 2], tile_size: [u32; 2]) -> TileKind {
    if is_fully_transparent(rgba, origin, tile_size) {
        return TileKind::Locked;
    }

    TileKind::Guess(guess_neighborhood(rgba, origin, tile_size))
}

fn is_fully_transparent(rgba: &RgbaImage, origin: [u32; 2], tile_size: [u32; 2]) -> bool {
    rgba.view(origin[0], origin[1], tile_size[0], tile_size[1])
        .pixels()
        .all(|(_x, _y, pixel)| pixel.0[3] == 0)
}

fn guess_neighborhood(rgba: &RgbaImage, origin: [u32; 2], tile_size: [u32; 2]) -> Neighborhood {
    let sample_x = [0, tile_size[0] / 2, tile_size[0] - 1];
    let sample_y = [0, tile_size[1] / 2, tile_size[1] - 1];

    let mut neighborhood = Neighborhood::default();
    for (index, neighbor) in NEIGHBORS.iter().enumerate() {
        let x = origin[0] + sample_x[(neighbor.dx + 1) as usize];
        let y = origin[1] + sample_y[(neighbor.dy + 1) as usize];

        let state = match rgba.get_pixel(x, y).0[3] {
            0 => NeighborState::Empty,
            _ => NeighborState::Full,
        };
        neighborhood.set_state(index, state);
    }

    neighborhood
}
