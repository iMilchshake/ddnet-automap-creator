use thiserror::Error;
use twmap::automapper::Automapper;
use twmap::ndarray::Array2;
use twmap::{GameLayer, Tile, TileFlags, TwMap};

use crate::model::transform::Transform;

const DM1: &[u8] = include_bytes!("../assets/dm1.map");
const GAME_LAYER_HOOKABLE: u8 = 1;
const GAME_LAYER_UNHOOKABLE: u8 = 3;
const PAINTED_TILE: u8 = 1;
const SOLID: char = '#';

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
}

impl Sample {
    pub const ALL: [Sample; 4] = [
        Sample::Shapes,
        Sample::DeBruijnTorus,
        Sample::Minimal,
        Sample::Dm1,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sample::Shapes => "Shapes",
            Sample::DeBruijnTorus => "de Bruijn torus",
            Sample::Minimal => "Minimal",
            Sample::Dm1 => "dm1",
        }
    }

    pub fn tiles(self) -> Result<Array2<Tile>, PreviewError> {
        match self {
            Sample::Shapes => Ok(drawn_tiles(&SHAPES)),
            Sample::DeBruijnTorus => Ok(drawn_tiles(&DE_BRUIJN_TORUS)),
            Sample::Minimal => Ok(drawn_tiles(&MINIMAL)),
            Sample::Dm1 => game_layer_tiles(DM1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedTile {
    pub index: usize,
    pub transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Automapped {
    pub width: usize,
    pub height: usize,
    tiles: Vec<Option<PlacedTile>>,
}

impl Automapped {
    pub fn tile(&self, column: usize, row: usize) -> Option<PlacedTile> {
        self.tiles[row * self.width + column]
    }
}

pub fn automap(rules: &str, sample: Sample, seed: u32) -> Result<Automapped, PreviewError> {
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

    let mut tiles = sample.tiles()?;
    rule_set.run(seed, &mut tiles);

    Ok(collect(&tiles))
}

fn drawn_tiles(rows: &[&str]) -> Array2<Tile> {
    let height = rows.len();
    let width = rows[0].len();

    Array2::from_shape_fn((height, width), |(row, column)| {
        match rows[row].as_bytes()[column] == SOLID as u8 {
            true => solid_tile(),
            false => Tile::default(),
        }
    })
}

fn game_layer_tiles(map: &[u8]) -> Result<Array2<Tile>, PreviewError> {
    let mut map = TwMap::parse(map).map_err(|error| PreviewError::SampleMap(error.to_string()))?;
    map.load()
        .map_err(|error| PreviewError::SampleMap(error.to_string()))?;
    let game = map
        .find_physics_layer::<GameLayer>()
        .ok_or(PreviewError::NoGameLayer)?;

    Ok(game.tiles.unwrap_ref().map(|tile| match tile.id {
        GAME_LAYER_HOOKABLE | GAME_LAYER_UNHOOKABLE => solid_tile(),
        _ => Tile::default(),
    }))
}

fn solid_tile() -> Tile {
    Tile::new(PAINTED_TILE, TileFlags::empty())
}

fn collect(tiles: &Array2<Tile>) -> Automapped {
    let (height, width) = tiles.dim();

    Automapped {
        width,
        height,
        tiles: tiles.iter().map(|tile| placed(*tile)).collect(),
    }
}

fn placed(tile: Tile) -> Option<PlacedTile> {
    if tile.id == 0 {
        return None;
    }

    Some(PlacedTile {
        index: usize::from(tile.id),
        transform: Transform {
            x_flip: tile.flags.contains(TileFlags::FLIP_X),
            y_flip: tile.flags.contains(TileFlags::FLIP_Y),
            rot: tile.flags.contains(TileFlags::ROTATE),
        },
    })
}
