use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::{ChanceMode, Pool, base_shares, pools};
use crate::model::tile::{Chance, TileRule};
use crate::tests::support::{index_of, mods};

const TOLERANCE: f32 = 0.001;

fn anywhere(percent: f32) -> TileRule {
    TileRule {
        neighborhood: Neighborhood::uniform(NeighborState::Any),
        mods: mods(false, false, false),
        chance: Chance::new(percent).unwrap(),
    }
}

fn only(name: &str) -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(index_of(name), NeighborState::Full);

    neighborhood
}

fn single_pool(percents: &[f32]) -> Pool {
    let tiles: Vec<(usize, TileRule)> = percents
        .iter()
        .enumerate()
        .map(|(tile, percent)| (tile + 1, anywhere(*percent)))
        .collect();

    let mut pools = pools(&tiles);
    assert_eq!(pools.len(), 1);
    pools.remove(0)
}

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < TOLERANCE,
            "{actual} != {expected}"
        );
    }
}

#[test]
fn normalized_chances_always_fill_every_cell() {
    for (percents, expected) in [
        (vec![100.0, 100.0], vec![50.0, 50.0]),
        (vec![30.0, 30.0], vec![50.0, 50.0]),
        (vec![100.0, 5.0], vec![95.238_1, 4.761_9]),
        (vec![30.0], vec![100.0]),
    ] {
        let pool = single_pool(&percents);
        assert_close(&pool.shares(ChanceMode::Normalize), &expected);
        assert_eq!(pool.coverage(ChanceMode::Normalize), 100.0);
    }
}

#[test]
fn exact_chances_below_100_leave_the_rest_unchanged() {
    let pool = single_pool(&[30.0, 30.0]);

    assert_close(&pool.shares(ChanceMode::Exact), &[30.0, 30.0]);
    assert_eq!(pool.coverage(ChanceMode::Exact), 60.0);
}

#[test]
fn exact_chances_above_100_are_scaled_down() {
    let pool = single_pool(&[100.0, 100.0]);

    assert_close(&pool.shares(ChanceMode::Exact), &[50.0, 50.0]);
    assert_eq!(pool.coverage(ChanceMode::Exact), 100.0);
}

#[test]
fn a_transformed_variant_pools_with_the_tile_it_matches() {
    let flippable = TileRule {
        neighborhood: only("left"),
        mods: mods(true, false, false),
        chance: Chance::FULL,
    };
    let plain = TileRule::new(only("right"));

    let pools = pools(&[(3, flippable), (8, plain)]);
    let shared = pools
        .iter()
        .find(|pool| pool.neighborhood == only("right"))
        .unwrap();

    let members: Vec<(usize, bool)> = shared
        .members
        .iter()
        .map(|member| (member.tile, member.transform.x_flip))
        .collect();
    assert_eq!(members, [(3, true), (8, false)]);
}

#[test]
fn base_shares_follow_the_untransformed_variant() {
    let flippable = TileRule {
        neighborhood: only("left"),
        mods: mods(true, false, false),
        chance: Chance::FULL,
    };
    let plain = TileRule::new(only("right"));

    let shares = base_shares(&pools(&[(3, flippable), (8, plain)]), ChanceMode::Normalize);

    assert_eq!(shares.get(&3), Some(&100.0));
    assert_eq!(shares.get(&8), Some(&50.0));
}
