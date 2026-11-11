use std::collections::BTreeMap;

use crate::model::neighbor::Neighborhood;
use crate::model::tile::{Centre, Chance, TileRule};
use crate::model::transform::Transform;

const ALL_CELLS: f32 = 100.0;
const CHANCE_RESOLUTION: f32 = 1000.0;

/// How the chances of tiles sharing a neighborhood turn into what they place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChanceMode {
    /// Chances are weights: every matching cell gets one of the tiles.
    #[default]
    Normalize,
    /// Chances are taken as they are, a total below 100 leaves cells unchanged.
    Exact,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub tile: usize,
    pub rule: usize,
    pub transform: Transform,
    pub primary: bool,
    pub neighborhood: Neighborhood,
    pub centre: Centre,
    pub chance: Chance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pool {
    pub neighborhood: Neighborhood,
    pub centre: Centre,
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
        let sum: f32 = self
            .members
            .iter()
            .map(|member| member.chance.percent())
            .sum();

        (sum * CHANCE_RESOLUTION).round() / CHANCE_RESOLUTION
    }

    pub fn contains(&self, tile: usize) -> bool {
        self.members.iter().any(|member| member.tile == tile)
    }

    pub fn places_primary(&self, tile: usize) -> bool {
        self.members
            .iter()
            .any(|member| member.tile == tile && member.primary)
    }

    pub fn share_of(&self, tile: usize, mode: ChanceMode) -> Option<f32> {
        self.members
            .iter()
            .zip(self.shares(mode))
            .filter(|(member, _)| member.tile == tile)
            .map(|(_, share)| share)
            .reduce(|sum, share| sum + share)
    }
}

pub fn pools_of(pools: &[Pool], tile: usize) -> Vec<&Pool> {
    let mut joined: Vec<&Pool> = pools.iter().filter(|pool| pool.contains(tile)).collect();
    joined.sort_by_key(|pool| !pool.places_primary(tile));

    joined
}

/// What each configured tile places where its own, first-orientation
/// neighborhood matches.
pub fn base_shares(pools: &[Pool], mode: ChanceMode) -> BTreeMap<usize, f32> {
    let mut shares = BTreeMap::new();
    for pool in pools {
        for member in pool.members.iter().filter(|member| member.primary) {
            shares.extend(
                pool.share_of(member.tile, mode)
                    .map(|share| (member.tile, share)),
            );
        }
    }

    shares
}

/// Pools in the order they must be emitted: least specific first, so the
/// more specific ones are applied last and win where they overlap.
pub fn pools(tiles: &[(usize, TileRule)]) -> Vec<Pool> {
    let mut pools: Vec<Pool> = Vec::new();

    for variant in expand(tiles) {
        let constraints = variant.neighborhood.constraints();
        match pools.iter_mut().find(|pool| {
            pool.neighborhood.constraints() == constraints && pool.centre == variant.centre
        }) {
            Some(pool) => pool.members.push(variant),
            None => pools.push(Pool {
                neighborhood: variant.neighborhood.clone(),
                centre: variant.centre,
                members: vec![variant],
            }),
        }
    }

    pools
}

pub struct PoolRuns<'a> {
    pub plain: Vec<&'a Pool>,
    pub specific: Vec<&'a Pool>,
    pub empty_centred: Vec<&'a Pool>,
}

impl<'a> PoolRuns<'a> {
    pub fn split(pools: &'a [Pool]) -> Self {
        let (empty_centred, full_centred): (Vec<&Pool>, Vec<&Pool>) =
            pools.iter().partition(|pool| pool.centre == Centre::Empty);
        let (specific, plain) = full_centred
            .into_iter()
            .partition(|pool| pool.neighborhood.is_tile_specific());

        Self {
            plain,
            specific,
            empty_centred,
        }
    }

    pub fn lowest_priority_first(&self) -> impl Iterator<Item = &'a Pool> {
        self.plain
            .iter()
            .chain(&self.specific)
            .chain(&self.empty_centred)
            .copied()
    }
}

fn expand(tiles: &[(usize, TileRule)]) -> Vec<Variant> {
    let mut variants = Vec::new();

    let mut rules_by_tile: BTreeMap<usize, Vec<&TileRule>> = BTreeMap::new();
    for (tile, rule) in tiles {
        rules_by_tile.entry(*tile).or_default().push(rule);
    }

    for (tile, rules) in rules_by_tile {
        let mut of_tile: Vec<Variant> = rules
            .into_iter()
            .enumerate()
            .flat_map(|(index, rule)| rule_variants(tile, index, rule))
            .collect();
        if let Some(first) = of_tile.first_mut() {
            first.primary = true;
        }
        variants.extend(of_tile);
    }

    variants.sort_by_key(|variant| (variant.neighborhood.specificity(), variant.tile));
    variants
}

fn rule_variants(tile: usize, index: usize, rule: &TileRule) -> Vec<Variant> {
    let placements: Vec<(Neighborhood, Transform)> = rule
        .orientations()
        .transforms()
        .into_iter()
        .map(|relative| {
            (
                relative.apply(&rule.neighborhood),
                relative.after(rule.drawn),
            )
        })
        .collect();

    placements
        .iter()
        .map(|(neighborhood, transform)| {
            let sharing = placements
                .iter()
                .filter(|(other, _)| other.constraints() == neighborhood.constraints())
                .count();

            Variant {
                tile,
                rule: index,
                transform: *transform,
                primary: false,
                neighborhood: neighborhood.clone(),
                centre: rule.centre,
                chance: rule.chance.split(sharing),
            }
        })
        .collect()
}
