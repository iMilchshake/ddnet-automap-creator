use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::model::neighbor::NeighborState;
use crate::model::tile::{AIR_TILE, Chance, TILE_COUNT, TILESET_SIDE, TileMods};
use crate::model::transform::{Orientations, Transform};

pub type Cell = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Any,
    Full,
    Empty,
}

impl CellState {
    pub fn toggled_solid(self) -> Self {
        match self {
            Self::Any | Self::Empty => Self::Full,
            Self::Full => Self::Any,
        }
    }

    pub fn from_neighbor(state: NeighborState) -> Option<Self> {
        match state {
            NeighborState::Any => Some(Self::Any),
            NeighborState::Full => Some(Self::Full),
            NeighborState::Empty => Some(Self::Empty),
            NeighborState::Is(_) => None,
        }
    }
}

impl From<CellState> for NeighborState {
    fn from(state: CellState) -> Self {
        match state {
            CellState::Any => Self::Any,
            CellState::Full => Self::Full,
            CellState::Empty => Self::Empty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPreset {
    Fill,
    Decorate,
    OnFloor,
}

impl GroupPreset {
    pub const ALL: [Self; 3] = [Self::Fill, Self::Decorate, Self::OnFloor];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill",
            Self::Decorate => "Decorate",
            Self::OnFloor => "On floor",
        }
    }

    fn own_state(self) -> CellState {
        match self {
            Self::Fill | Self::Decorate => CellState::Full,
            Self::OnFloor => CellState::Empty,
        }
    }

    fn ring_state(self, own: &BTreeSet<Cell>, (x, y): Cell) -> CellState {
        match self {
            Self::Fill => CellState::Any,
            Self::Decorate => CellState::Full,
            Self::OnFloor if own.contains(&(x, y - 1)) => CellState::Full,
            Self::OnFloor => CellState::Any,
        }
    }

    fn states(self, own: &BTreeSet<Cell>) -> BTreeMap<Cell, CellState> {
        own.iter()
            .map(|&cell| (cell, self.own_state()))
            .chain(
                ring_around(own)
                    .into_iter()
                    .map(|cell| (cell, self.ring_state(own, cell))),
            )
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileGroup {
    pub name: String,
    pub tiles: BTreeSet<usize>,
    pub neighbors: BTreeMap<Cell, CellState>,
    pub chance: Chance,
    pub mods: TileMods,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupCopy {
    pub transform: Transform,
    pub cells: Vec<Cell>,
    pub full: Vec<Cell>,
    pub empty: Vec<Cell>,
}

fn reading_order(&(x, y): &Cell) -> (i32, i32) {
    (y, x)
}

fn cell_of(tile: usize) -> Cell {
    (column_of(tile) as i32, row_of(tile) as i32)
}

fn own_cells(tiles: &BTreeSet<usize>) -> BTreeSet<Cell> {
    tiles.iter().map(|&tile| cell_of(tile)).collect()
}

fn ring_around(own: &BTreeSet<Cell>) -> BTreeSet<Cell> {
    own.iter()
        .flat_map(|&(x, y)| {
            (-RING_EXTENT..=RING_EXTENT)
                .flat_map(move |dy| (-RING_EXTENT..=RING_EXTENT).map(move |dx| (x + dx, y + dy)))
        })
        .filter(|cell| !own.contains(cell))
        .collect()
}

impl GroupCopy {
    fn new(transform: Transform, group: &TileGroup) -> Self {
        let moved = |&(x, y): &Cell| transform.move_offset(x, y);
        let mut cells: Vec<Cell> = own_cells(&group.tiles).iter().map(moved).collect();
        cells.sort_by_key(reading_order);
        let (anchor_x, anchor_y) = *cells.first().expect("a validated group has tiles");
        let relative = |(x, y): Cell| (x - anchor_x, y - anchor_y);
        let offsets_where = |wanted: CellState| {
            let mut offsets: Vec<Cell> = group
                .neighbors
                .iter()
                .filter(|(_, state)| **state == wanted)
                .map(|(cell, _)| relative(moved(cell)))
                .collect();
            offsets.sort_by_key(reading_order);
            offsets
        };

        Self {
            transform,
            cells: cells.into_iter().map(relative).collect(),
            full: offsets_where(CellState::Full),
            empty: offsets_where(CellState::Empty),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupError {
    #[error("`{0}` must start with a letter and hold only letters and digits")]
    Name(String),

    #[error("a group must cover at least two tiles")]
    TooSmall,

    #[error("a group cannot cover tile {AIR_TILE}, which is reserved for air")]
    CoversAirTile,

    #[error("tile {0} lies outside the tileset")]
    OutOfBounds(usize),

    #[error("`{0}` names more than one group")]
    DuplicateName(String),

    #[error("`{first}` and `{second}` cover the same tiles")]
    Overlap { first: String, second: String },
}

pub const MIN_GROUP_TILES: usize = 2;

pub const RING_EXTENT: i32 = 1;

fn column_of(tile: usize) -> usize {
    tile % TILESET_SIDE
}

fn row_of(tile: usize) -> usize {
    tile / TILESET_SIDE
}

fn rect_size(top_left: usize, bottom_right: usize) -> (usize, usize) {
    (
        column_of(bottom_right) - column_of(top_left) + 1,
        row_of(bottom_right) - row_of(top_left) + 1,
    )
}

impl TileGroup {
    pub fn new(name: String, tiles: BTreeSet<usize>) -> Self {
        let mut group = Self {
            name,
            tiles,
            neighbors: BTreeMap::new(),
            chance: Chance::FULL,
            mods: TileMods::default(),
        };
        group.apply_preset(GroupPreset::Fill);

        group
    }

    pub fn apply_preset(&mut self, preset: GroupPreset) {
        self.neighbors = preset.states(&own_cells(&self.tiles));
    }

    pub fn preset(&self) -> Option<GroupPreset> {
        let own = own_cells(&self.tiles);

        GroupPreset::ALL
            .into_iter()
            .find(|preset| preset.states(&own) == self.neighbors)
    }

    pub fn set_tiles(&mut self, tiles: BTreeSet<usize>) {
        let own = own_cells(&tiles);
        let ring = ring_around(&own);
        let was_own = own_cells(&self.tiles);
        let was_ring = ring_around(&was_own);
        let kept = |cell: Cell, same_role: bool, fallback: CellState| {
            self.neighbors
                .get(&cell)
                .copied()
                .filter(|_| same_role)
                .unwrap_or(fallback)
        };

        self.neighbors = own
            .iter()
            .map(|&cell| (cell, kept(cell, was_own.contains(&cell), CellState::Full)))
            .chain(
                ring.iter()
                    .map(|&cell| (cell, kept(cell, was_ring.contains(&cell), CellState::Any))),
            )
            .collect();
        self.tiles = tiles;
    }

    pub fn own_tile_at(&self, cell: Cell) -> Option<usize> {
        self.tiles
            .iter()
            .copied()
            .find(|&tile| cell_of(tile) == cell)
    }

    pub fn bounds(&self) -> Option<(Cell, Cell)> {
        let cells = own_cells(&self.tiles);
        let xs = || cells.iter().map(|&(x, _)| x);
        let ys = || cells.iter().map(|&(_, y)| y);

        Some(((xs().min()?, ys().min()?), (xs().max()?, ys().max()?)))
    }

    pub fn rect_tiles(top_left: usize, bottom_right: usize) -> BTreeSet<usize> {
        (row_of(top_left)..=row_of(bottom_right))
            .flat_map(|row| {
                (column_of(top_left)..=column_of(bottom_right))
                    .map(move |column| row * TILESET_SIDE + column)
            })
            .collect()
    }

    pub fn footprint(&self) -> Vec<usize> {
        self.tiles.iter().copied().collect()
    }

    pub fn covers(&self, tile: usize) -> bool {
        self.tiles.contains(&tile)
    }

    pub fn anchor(&self) -> Option<usize> {
        self.tiles.first().copied()
    }

    pub fn bottom_left_tile(&self) -> Option<usize> {
        let bottom_row = row_of(*self.tiles.last()?);

        self.tiles
            .range(bottom_row * TILESET_SIDE..)
            .next()
            .copied()
    }

    pub fn bounding_corners(&self) -> Option<(usize, usize)> {
        let columns = || self.tiles.iter().map(|&tile| column_of(tile));
        let top = row_of(self.anchor()?);
        let bottom = row_of(*self.tiles.last()?);

        Some((
            top * TILESET_SIDE + columns().min()?,
            bottom * TILESET_SIDE + columns().max()?,
        ))
    }

    pub fn as_rect(&self) -> Option<(usize, usize)> {
        let (top_left, bottom_right) = self.bounding_corners()?;

        (self.tiles == Self::rect_tiles(top_left, bottom_right)).then_some((top_left, bottom_right))
    }

    pub fn size_label(&self) -> String {
        match self.as_rect() {
            Some((top_left, bottom_right)) => {
                let (width, height) = rect_size(top_left, bottom_right);
                format!("{width}×{height}")
            }
            None => format!("{} tiles", self.tiles.len()),
        }
    }

    pub fn copies(&self) -> Vec<GroupCopy> {
        Orientations::generated(self.mods)
            .transforms()
            .into_iter()
            .map(|transform| GroupCopy::new(transform, self))
            .collect()
    }

    pub fn validate(&self) -> Result<(), GroupError> {
        if !is_identifier(&self.name) {
            return Err(GroupError::Name(self.name.clone()));
        }
        if self.tiles.len() < MIN_GROUP_TILES {
            return Err(GroupError::TooSmall);
        }
        if self.covers(AIR_TILE) {
            return Err(GroupError::CoversAirTile);
        }
        if let Some(&tile) = self.tiles.iter().find(|&&tile| tile >= TILE_COUNT) {
            return Err(GroupError::OutOfBounds(tile));
        }

        Ok(())
    }
}

pub fn validate_all(groups: &[TileGroup]) -> Result<(), GroupError> {
    for group in groups {
        group.validate()?;
    }

    for (index, group) in groups.iter().enumerate() {
        for other in &groups[index + 1..] {
            if group.name == other.name {
                return Err(GroupError::DuplicateName(group.name.clone()));
            }
            if group.footprint().iter().any(|tile| other.covers(*tile)) {
                return Err(GroupError::Overlap {
                    first: group.name.clone(),
                    second: other.name.clone(),
                });
            }
        }
    }

    Ok(())
}

/// rpp names hold only letters, digits and `:`, so an underscore is as illegal
/// as a space. The emitter adds its own `:` prefix, which is what keeps these
/// names clear of rpp's keywords and of base.r's `g:`/`s:` globals.
fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    first.is_ascii_alphabetic() && characters.all(|character| character.is_ascii_alphanumeric())
}
