use std::collections::BTreeMap;

use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::tile::{Chance, TileRule};
use crate::model::transform::{self, Transform};

const ALL_CELLS: f32 = 100.0;

/// How the chances of tiles sharing a neighborhood turn into what they place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChanceMode {
    /// Chances are weights: every matching cell gets one of the tiles.
    #[default]
    Normalize,
    /// Chances are taken as they are, a total below 100 leaves cells unchanged.
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variant {
    pub tile: usize,
    pub transform: Transform,
    pub neighborhood: Neighborhood,
    pub chance: Chance,
}

/// Every variant declaring one exact neighborhood; rpp rolls between them.
#[derive(Debug, Clone, PartialEq)]
pub struct Pool {
    pub neighborhood: Neighborhood,
    pub members: Vec<Variant>,
}

impl Pool {
    /// The percentage of matching cells each member ends up on, in member order.
    pub fn shares(&self, mode: ChanceMode) -> Vec<f32> {
        let total = self.total();
        let scale = match mode {
            ChanceMode::Normalize => ALL_CELLS / total,
            ChanceMode::Exact => (ALL_CELLS / total).min(1.0),
        };

        self.members
            .iter()
            .map(|member| member.chance.percent() * scale)
            .collect()
    }

    /// The percentage of matching cells the pool places anything on at all.
    pub fn coverage(&self, mode: ChanceMode) -> f32 {
        match mode {
            ChanceMode::Normalize => ALL_CELLS,
            ChanceMode::Exact => self.total().min(ALL_CELLS),
        }
    }

    pub fn total(&self) -> f32 {
        self.members
            .iter()
            .map(|member| member.chance.percent())
            .sum()
    }

    pub fn contains(&self, tile: usize) -> bool {
        self.members.iter().any(|member| member.tile == tile)
    }
}

/// What each configured tile places where its own, untransformed neighborhood
/// matches.
pub fn base_shares(pools: &[Pool], mode: ChanceMode) -> BTreeMap<usize, f32> {
    let mut shares = BTreeMap::new();
    for pool in pools {
        for (member, share) in pool.members.iter().zip(pool.shares(mode)) {
            if member.transform == Transform::IDENTITY {
                shares.insert(member.tile, share);
            }
        }
    }

    shares
}

/// Pools in the order they must be emitted: least specific first, so the
/// more specific ones are applied last and win where they overlap.
pub fn pools(tiles: &[(usize, TileRule)]) -> Vec<Pool> {
    let mut pools: Vec<Pool> = Vec::new();

    for variant in expand(tiles) {
        match pools
            .iter_mut()
            .find(|pool| pool.neighborhood == variant.neighborhood)
        {
            Some(pool) => pool.members.push(variant),
            None => pools.push(Pool {
                neighborhood: variant.neighborhood,
                members: vec![variant],
            }),
        }
    }

    pools
}

fn expand(tiles: &[(usize, TileRule)]) -> Vec<Variant> {
    let mut variants = Vec::new();

    for &(tile, rule) in tiles {
        let mut seen = Vec::new();
        for transform in transform::variants(rule.mods) {
            let neighborhood = transform.apply(&rule.neighborhood);
            if seen.contains(&neighborhood) {
                continue;
            }
            seen.push(neighborhood);

            variants.push(Variant {
                tile,
                transform,
                neighborhood,
                chance: rule.chance,
            });
        }
    }

    variants.sort_by_key(|variant| (specificity(&variant.neighborhood), variant.tile));
    variants
}

fn specificity(neighborhood: &Neighborhood) -> usize {
    neighborhood
        .states()
        .iter()
        .filter(|state| **state != NeighborState::Any)
        .count()
}
