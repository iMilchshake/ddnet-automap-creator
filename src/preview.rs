use std::ops::RangeInclusive;

use thiserror::Error;
use twmap::automapper::Automapper;
use twmap::ndarray::Array2;
use twmap::{GameLayer, Tile, TileFlags, TwMap};

use crate::model::project::Project;
use crate::model::tile::{AIR_TILE, Centre, TILE_COUNT, referenced_tiles};
use crate::model::transform::Transform;

const DM1: &[u8] = include_bytes!("../assets/dm1.map");
const GAME_LAYER_HOOKABLE: u8 = 1;
const GAME_LAYER_UNHOOKABLE: u8 = 3;
pub(crate) const FALLBACK_UNMATCHED_MARKER: u8 = 1;
const SOLID: char = '#';
const CUSTOM_DEFAULT_WIDTH: usize = 32;
const CUSTOM_DEFAULT_HEIGHT: usize = 16;
pub const CUSTOM_SIZE_RANGE: RangeInclusive<usize> = 4..=128;

const SHAPES: [&str; 17] = [
    "................................",
    "..########.........#.....#.#.#..",
    "..########.........#............",
    "..##....##.........#.....#...#..",
    "..##....##.........#............",
    "..########...............#.#.#..",
    "..########......................",
    "...............#####........#...",
    "....##.............#.......##...",
    "....##..............#.....###...",
    "....##..#########...#....####...",
    "....##..............#...#####...",
    "................................",
    "#########.......#####.....######",
    "##########.....#######...#######",
    "################################",
    "################################",
];

// source: https://github.com/AssassinTee/SimpleDDNetAutomapper/blob/d0258220c6f71bc6f7747f6d7fff0a444cf11e10/data/debroijn_torus.txt
const DE_BRUIJN_TORUS: [&str; 18] = [
    "......#.###..#.###..#.###..#.###..",
    "...#..######.#..##.##.#.#....##...",
    "......#.###..#.###..#.###..#.###..",
    "###.##......#.##..#..#.#.####..###",
    "......#.###..#.###..#.###..#.###..",
    "...#..######.#..##.##.#.#....##...",
    "..##...###.#.##.#####...#.#..#....",
    "#...#.#..##.##.#.#....##...######.",
    "######.#...##.#...##.#...##.#...##",
    "###.##......#.##..#..#.#.####..###",
    "#.#.#....#..####.##....#..####.##.",
    ".#...##.#.#....##...######.#..##.#",
    "######.#...##.#...##.#...##.#...##",
    "###.##......#.##..#..#.#.####..###",
    ".##..#..#.....###.#.##.#####...#.#",
    "##.#####..###......#.##..#..#.#.##",
    "......#.###..#.###..#.###..#.###..",
    "...#..######.#..##.##.#.#....##...",
];

// source: https://github.com/AssassinTee/SimpleDDNetAutomapper/blob/03ce23995c6c3e744d4c6d8aad0e0b0005f57858/data/minimal.txt
const MINIMAL: [&str; 9] = [
    "##..##.##",
    "##.###.##",
    "..#####..",
    "########.",
    "####.####",
    ".########",
    "..#####..",
    "##.###.##",
    "##.##..##",
];

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("could not read the compiled rules: {0}")]
    Syntax(String),

    #[error("the compiled rules hold no rule set")]
    NoRuleSet,

    #[error("could not read the sample map: {0}")]
    SampleMap(String),

    #[error("the sample map has no game layer")]
    NoGameLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sample {
    #[default]
    Shapes,
    DeBruijnTorus,
    Minimal,
    Dm1,
    Custom,
}

impl Sample {
    pub const ALL: [Sample; 5] = [
        Sample::Shapes,
        Sample::DeBruijnTorus,
        Sample::Minimal,
        Sample::Dm1,
        Sample::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sample::Shapes => "Shapes",
            Sample::DeBruijnTorus => "de Bruijn torus",
            Sample::Minimal => "Minimal",
            Sample::Dm1 => "dm1",
            Sample::Custom => "Custom",
        }
    }

    pub fn tiles(
        self,
        custom: &CustomShape,
        solid_id: u8,
    ) -> Result<Array2<Tile>, PreviewError> {
        match self {
            Sample::Shapes => Ok(drawn_tiles(&SHAPES, solid_id)),
            Sample::DeBruijnTorus => Ok(drawn_tiles(&DE_BRUIJN_TORUS, solid_id)),
            Sample::Minimal => Ok(drawn_tiles(&MINIMAL, solid_id)),
            Sample::Dm1 => game_layer_tiles(DM1, solid_id),
            Sample::Custom => Ok(custom.tiles(solid_id)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomShape {
    width: usize,
    height: usize,
    solid: Vec<bool>,
}

impl Default for CustomShape {
    fn default() -> Self {
        Self::empty(CUSTOM_DEFAULT_WIDTH, CUSTOM_DEFAULT_HEIGHT)
    }
}

impl CustomShape {
    pub fn empty(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            solid: vec![false; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set(&mut self, column: usize, row: usize, solid: bool) -> bool {
        let cell = &mut self.solid[row * self.width + column];
        let changed = *cell != solid;
        *cell = solid;
        changed
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        let mut resized = Self::empty(width, height);
        for row in 0..height.min(self.height) {
            for column in 0..width.min(self.width) {
                resized.set(column, row, self.solid[row * self.width + column]);
            }
        }
        *self = resized;
    }

    pub fn clear(&mut self) {
        self.solid.fill(false);
    }

    fn tiles(&self, solid_id: u8) -> Array2<Tile> {
        Array2::from_shape_fn((self.height, self.width), |(row, column)| {
            match self.solid[row * self.width + column] {
                true => solid_tile(solid_id),
                false => Tile::default(),
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedTile {
    pub index: usize,
    pub transform: Transform,
    pub centre: Centre,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewCell {
    Air,
    Placed(PlacedTile),
    Unmatched,
}

/// if this marker remains, no rule was applied. We use this to visualize tiles with neighborhoods with no matching rules.
pub fn unmatched_marker(project: &Project) -> u8 {
    let rules = project.rules();
    let active: Vec<usize> = rules.iter().map(|(tile, _)| *tile).collect();
    let referenced = referenced_tiles(&rules);
    let mask = project.mask_tile();

    (AIR_TILE + 1..TILE_COUNT)
        .find(|tile| {
            !active.contains(tile)
                && !referenced.contains(tile)
                && project.group_at(*tile).is_none()
                && Some(*tile) != mask
        })
        .and_then(|tile| u8::try_from(tile).ok())
        .unwrap_or(FALLBACK_UNMATCHED_MARKER)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Automapped {
    pub width: usize,
    pub height: usize,
    cells: Vec<PreviewCell>,
}

impl Automapped {
    pub fn cell(&self, column: usize, row: usize) -> PreviewCell {
        self.cells[row * self.width + column]
    }
}

pub fn automap(
    rules: &str,
    sample: Sample,
    custom: &CustomShape,
    seed: u32,
    unmatched_marker: u8,
) -> Result<Automapped, PreviewError> {
    // rpp rounds a chance slightly above 100% to `Random 1`, which is equivalent to having no Random.
    // Its technically not wrong syntax, but also nonsensical as it could just be dropped.
    // DDNet reads this without any problems, but twmap refuses it. So as a hotfix we drop it before parsing.
    let rules: String = rules
        .lines()
        .filter(|line| line.trim() != "Random 1")
        .map(|line| format!("{line}\n"))
        .collect();
    let automapper = Automapper::parse(String::new(), &rules)
        .map_err(|error| PreviewError::Syntax(error.to_string()))?;
    let rule_set = automapper.configs.first().ok_or(PreviewError::NoRuleSet)?;

    let mut tiles = sample.tiles(custom, unmatched_marker)?;
    let was_solid = tiles.map(|tile| tile.id != 0);
    rule_set.run(seed, &mut tiles);

    Ok(collect(&tiles, &was_solid, unmatched_marker))
}

fn drawn_tiles(rows: &[&str], solid_id: u8) -> Array2<Tile> {
    let height = rows.len();
    let width = rows[0].len();

    Array2::from_shape_fn((height, width), |(row, column)| {
        match rows[row].as_bytes()[column] == SOLID as u8 {
            true => solid_tile(solid_id),
            false => Tile::default(),
        }
    })
}

fn game_layer_tiles(map: &[u8], solid_id: u8) -> Result<Array2<Tile>, PreviewError> {
    let mut map = TwMap::parse(map).map_err(|error| PreviewError::SampleMap(error.to_string()))?;
    map.load()
        .map_err(|error| PreviewError::SampleMap(error.to_string()))?;
    let game = map
        .find_physics_layer::<GameLayer>()
        .ok_or(PreviewError::NoGameLayer)?;

    Ok(game.tiles.unwrap_ref().map(|tile| match tile.id {
        GAME_LAYER_HOOKABLE | GAME_LAYER_UNHOOKABLE => solid_tile(solid_id),
        _ => Tile::default(),
    }))
}

fn solid_tile(id: u8) -> Tile {
    Tile::new(id, TileFlags::empty())
}

fn collect(tiles: &Array2<Tile>, was_solid: &Array2<bool>, unmatched_marker: u8) -> Automapped {
    let (height, width) = tiles.dim();

    Automapped {
        width,
        height,
        cells: tiles
            .iter()
            .zip(was_solid)
            .map(|(tile, was_solid)| cell(*tile, *was_solid, unmatched_marker))
            .collect(),
    }
}

fn cell(tile: Tile, was_solid: bool, unmatched_marker: u8) -> PreviewCell {
    if tile.id == 0 {
        return PreviewCell::Air;
    }
    if tile.id == unmatched_marker {
        return PreviewCell::Unmatched;
    }

    PreviewCell::Placed(PlacedTile {
        index: usize::from(tile.id),
        centre: match was_solid {
            true => Centre::Full,
            false => Centre::Empty,
        },
        transform: Transform {
            x_flip: tile.flags.contains(TileFlags::FLIP_X),
            y_flip: tile.flags.contains(TileFlags::FLIP_Y),
            rot: tile.flags.contains(TileFlags::ROTATE),
        },
    })
}
