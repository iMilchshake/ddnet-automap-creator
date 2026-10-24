use std::collections::BTreeSet;

use thiserror::Error;

use crate::model::neighbor::Neighborhood;
use crate::model::transform::{Orientations, Transform};

pub const TILESET_SIDE: usize = 16;
pub const TILE_COUNT: usize = TILESET_SIDE * TILESET_SIDE;

pub const AIR_TILE: usize = 0;

pub fn highest_free_tile(is_free: impl Fn(usize) -> bool) -> Option<usize> {
    (AIR_TILE + 1..TILE_COUNT).rev().find(|&tile| is_free(tile))
}

pub fn referenced_tiles(tiles: &[(usize, TileRule)]) -> BTreeSet<usize> {
    tiles
        .iter()
        .flat_map(|(_, rule)| rule.neighborhood.states())
        .filter_map(|state| state.tile())
        .collect()
}

#[derive(Debug, Error)]
#[error("chance must be greater than 0 and at most 100, got {0}")]
pub struct InvalidChance(pub f32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chance(f32);

impl Chance {
    pub const FULL: Self = Self(100.0);

    pub fn new(percent: f32) -> Result<Self, InvalidChance> {
        if !percent.is_finite() || percent <= 0.0 || percent > 100.0 {
            return Err(InvalidChance(percent));
        }

        Ok(Self(percent))
    }

    pub fn percent(self) -> f32 {
        self.0
    }

    pub fn is_full(self) -> bool {
        self.0 >= 100.0
    }
}

impl Default for Chance {
    fn default() -> Self {
        Self::FULL
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TileMods {
    pub can_x_flip: bool,
    pub can_y_flip: bool,
    pub can_rotate: bool,
}

impl TileMods {
    pub const fn new(can_x_flip: bool, can_y_flip: bool, can_rotate: bool) -> Self {
        Self {
            can_x_flip,
            can_y_flip,
            can_rotate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRule {
    pub neighborhood: Neighborhood,
    pub mods: TileMods,
    pub custom: Option<Orientations>,
    pub drawn: Transform,
    pub chance: Chance,
}

impl TileRule {
    pub fn new(neighborhood: Neighborhood) -> Self {
        Self {
            neighborhood,
            mods: TileMods::default(),
            custom: None,
            drawn: Transform::IDENTITY,
            chance: Chance::FULL,
        }
    }

    pub fn orientations(&self) -> Orientations {
        Orientations::resolved(self.custom, self.mods)
    }

    pub fn turned_neighbors(self, by: Transform) -> Self {
        Self {
            neighborhood: by.apply(&self.neighborhood),
            ..self
        }
    }

    pub fn turned_tile(self, by: Transform) -> Self {
        Self {
            drawn: by.after(self.drawn),
            ..self
        }
    }
}
