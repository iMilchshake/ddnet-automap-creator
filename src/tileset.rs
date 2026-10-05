use image::{GenericImageView, RgbaImage};
use thiserror::Error;

use crate::model::tile::{TILE_COUNT, TILESET_SIDE};

#[derive(Debug, Error)]
pub enum TilesetError {
    #[error("could not decode the image: {0}")]
    Decode(#[from] image::ImageError),

    #[error("image is {width}×{height}, must be divisible by {TILESET_SIDE}")]
    NotDivisible { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Locked,
    Open,
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
    if width % side != 0 || height % side != 0 {
        return Err(TilesetError::NotDivisible { width, height });
    }

    let tile_size = [width / side, height / side];

    let tiles = (0..TILE_COUNT)
        .map(|index| slice_tile(&rgba, index, tile_size))
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

fn slice_tile(rgba: &RgbaImage, index: usize, tile_size: [u32; 2]) -> TileSlice {
    let column = (index % TILESET_SIDE) as u32;
    let row = (index / TILESET_SIDE) as u32;
    let origin = [column * tile_size[0], row * tile_size[1]];

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

    TileKind::Open
}

fn is_fully_transparent(rgba: &RgbaImage, origin: [u32; 2], tile_size: [u32; 2]) -> bool {
    rgba.view(origin[0], origin[1], tile_size[0], tile_size[1])
        .pixels()
        .all(|(_x, _y, pixel)| pixel.0[3] == 0)
}
