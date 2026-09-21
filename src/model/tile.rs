use thiserror::Error;

use crate::model::neighbor::Neighborhood;

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

#[cfg(test)]
mod tests {
    use super::Chance;

    #[test]
    fn chance_rejects_values_outside_its_range() {
        assert!(Chance::new(0.0).is_err());
        assert!(Chance::new(-1.0).is_err());
        assert!(Chance::new(100.1).is_err());
        assert!(Chance::new(f32::NAN).is_err());
    }

    #[test]
    fn chance_accepts_fractions_and_the_bounds() {
        assert_eq!(Chance::new(2.5).unwrap().percent(), 2.5);
        assert!(Chance::new(100.0).unwrap().is_full());
        assert!(!Chance::new(99.9).unwrap().is_full());
    }
}
