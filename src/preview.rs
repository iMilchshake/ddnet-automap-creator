use thiserror::Error;
use twmap::automapper::Automapper;
use twmap::ndarray::Array2;
use twmap::{Tile, TileFlags};

use crate::model::transform::Transform;

const SOLID: char = '#';
const SOLID_TILE: u8 = 1;

/// rpp rounds a chance slightly above 100% to `Random 1`, which is equivalent to having no Random.
/// Its technically not wrong syntax, but also nonsensical as it could just be dropped.
/// DDNet reads this without any problems, but twmap refuses it. So as a hotfix we drop it before parsing.
const ALWAYS: &str = "Random 1";

/// Any non-zero seed keeps the preview stable; zero makes twmap roll its own.
pub const FIRST_SEED: u32 = 1;

// TODO: add Assa's patterns
pub const SAMPLE: [&str; 17] = [
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

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("could not read the compiled rules: {0}")]
    Syntax(String),

    #[error("the compiled rules hold no rule set")]
    NoRuleSet,
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

/// Runs the first rule set of a compiled `.rules` file over [`SAMPLE`], the
/// way DDNet's editor would.
pub fn automap(rules: &str, seed: u32) -> Result<Automapped, PreviewError> {
    let rules: String = rules
        .lines()
        .filter(|line| line.trim() != ALWAYS)
        .map(|line| format!("{line}\n"))
        .collect();
    let automapper = Automapper::parse(String::new(), &rules)
        .map_err(|error| PreviewError::Syntax(error.to_string()))?;
    let rule_set = automapper.configs.first().ok_or(PreviewError::NoRuleSet)?;

    let mut tiles = sample_tiles();
    rule_set.run(seed, &mut tiles);

    Ok(collect(&tiles))
}

pub fn next_seed(seed: u32) -> u32 {
    match seed.wrapping_add(1) {
        0 => FIRST_SEED,
        next => next,
    }
}

fn sample_tiles() -> Array2<Tile> {
    let height = SAMPLE.len();
    let width = SAMPLE[0].len();

    Array2::from_shape_fn((height, width), |(row, column)| {
        let solid = SAMPLE[row].as_bytes()[column] == SOLID as u8;
        match solid {
            true => Tile::new(SOLID_TILE, TileFlags::empty()),
            false => Tile::default(),
        }
    })
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
