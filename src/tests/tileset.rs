use image::{Rgba, RgbaImage};

use crate::model::neighbor::NeighborState;
use crate::tileset::{TILE_COUNT, TileKind, decode_tileset, file_stem};

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
    assert!(matches!(
        error,
        crate::tileset::TilesetError::TooSmall { .. }
    ));
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
