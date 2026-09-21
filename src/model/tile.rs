use thiserror::Error;

use crate::model::neighbor::Neighborhood;

pub const TILESET_SIDE: usize = 16;
pub const TILE_COUNT: usize = TILESET_SIDE * TILESET_SIDE;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileRule {
    pub neighborhood: Neighborhood,
    pub mods: TileMods,
    pub chance: Chance,
}

impl TileRule {
    pub fn new(neighborhood: Neighborhood) -> Self {
        Self {
            neighborhood,
            mods: TileMods::default(),
            chance: Chance::FULL,
        }
    }
}
