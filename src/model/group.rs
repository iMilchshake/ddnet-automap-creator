use std::collections::BTreeSet;

use thiserror::Error;

use crate::model::tile::{AIR_TILE, Chance, TILE_COUNT, TILESET_SIDE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    Fill,
    Decorate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileGroup {
    pub name: String,
    pub tiles: BTreeSet<usize>,
    pub mode: GroupMode,
    pub chance: Chance,
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

const RING_REACH: i32 = 1;

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

    pub fn ring_offsets(&self) -> Vec<(i32, i32)> {
        let cells: BTreeSet<(i32, i32)> = self
            .tiles
            .iter()
            .map(|&tile| (row_of(tile) as i32, column_of(tile) as i32))
            .collect();
        let Some(&(anchor_y, anchor_x)) = cells.first() else {
            return Vec::new();
        };

        cells
            .iter()
            .flat_map(|&(y, x)| {
                (-RING_REACH..=RING_REACH)
                    .flat_map(move |dy| (-RING_REACH..=RING_REACH).map(move |dx| (y + dy, x + dx)))
            })
            .filter(|cell| !cells.contains(cell))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|(y, x)| (x - anchor_x, y - anchor_y))
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
