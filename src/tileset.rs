use image::{GenericImageView, RgbaImage};
use thiserror::Error;

use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};

pub const TILESET_SIDE: u32 = 16;
pub const TILE_COUNT: usize = (TILESET_SIDE * TILESET_SIDE) as usize;

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

    let tile_size = [width / TILESET_SIDE, height / TILESET_SIDE];
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
    let column = index as u32 % TILESET_SIDE;
    let row = index as u32 / TILESET_SIDE;
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

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{TILE_COUNT, TileKind, decode_tileset, file_stem};
    use crate::model::neighbor::NeighborState;

    fn opaque_image(width: u32, height: u32) -> Vec<u8> {
        encode(RgbaImage::from_pixel(width, height, Rgba([255, 0, 0, 255])))
    }

    fn encode(image: RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("png encoding failed");
        bytes
    }

    #[test]
    fn slices_a_divisible_image_into_256_tiles() {
        let tileset = decode_tileset(&opaque_image(32, 32), "grass_main").unwrap();

        assert_eq!(tileset.tiles.len(), TILE_COUNT);
        assert_eq!(tileset.tile_size, [2, 2]);
        assert_eq!(tileset.stem, "grass_main");

        assert_eq!(tileset.tiles[0].uv_min, [0.0, 0.0]);
        assert_eq!(tileset.tiles[0].uv_max, [2.0 / 32.0, 2.0 / 32.0]);
        assert_eq!(tileset.tiles[15].uv_min, [30.0 / 32.0, 0.0]);
        assert_eq!(tileset.tiles[16].uv_min, [0.0, 2.0 / 32.0]);
        assert_eq!(tileset.tiles[255].uv_max, [1.0, 1.0]);
    }

    #[test]
    fn non_divisible_sizes_place_origins_on_the_float_stride() {
        // 40 / 16 = 2.5: tiles are 2 px wide but sit 2.5 px apart.
        let tileset = decode_tileset(&opaque_image(40, 40), "odd").unwrap();

        assert_eq!(tileset.tile_size, [2, 2]);
        assert_eq!(tileset.tiles[3].uv_min[0], 7.0 / 40.0); // floor(3 * 2.5), not 6
        assert_eq!(tileset.tiles[15].uv_min[0], 37.0 / 40.0); // floor(15 * 2.5), not 30
        assert_eq!(tileset.tiles[15].uv_max[0], 39.0 / 40.0);
    }

    #[test]
    fn rejects_images_smaller_than_the_grid() {
        let error = decode_tileset(&opaque_image(15, 64), "tiny").unwrap_err();
        assert!(matches!(error, super::TilesetError::TooSmall { .. }));
    }

    #[test]
    fn fully_transparent_tiles_are_locked() {
        let mut image = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
        // Tile 17 is row 1, column 1 of a 2 px grid.
        for y in 2..4 {
            for x in 2..4 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }

        let tileset = decode_tileset(&encode(image), "sparse").unwrap();

        assert_eq!(tileset.tiles[0].kind, TileKind::Locked);
        assert!(matches!(tileset.tiles[17].kind, TileKind::Guess(_)));
    }

    #[test]
    fn the_guess_follows_the_alpha_at_each_sample_point() {
        // 3×3 pixel tiles, so the sample points are the 8 pixels around the centre.
        let mut image = RgbaImage::from_pixel(48, 48, Rgba([255, 255, 255, 255]));
        for x in 0..3 {
            image.put_pixel(x, 0, Rgba([0, 0, 0, 0]));
        }

        let tileset = decode_tileset(&encode(image), "guess").unwrap();

        let TileKind::Guess(neighborhood) = tileset.tiles[0].kind else {
            panic!("tile 0 should not be locked");
        };
        let states = neighborhood.states();
        assert_eq!(&states[0..3], &[NeighborState::Empty; 3]);
        assert_eq!(&states[3..8], &[NeighborState::Full; 5]);
    }

    #[test]
    fn file_stem_drops_the_last_extension() {
        assert_eq!(file_stem("grass_main.png"), "grass_main");
        assert_eq!(file_stem("a.b.png"), "a.b");
        assert_eq!(file_stem("noextension"), "noextension");
    }
}
