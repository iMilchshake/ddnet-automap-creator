use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::{ChanceMode, Pool, Variant, base_shares, pools, pools_of};
use crate::model::tile::{Chance, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tests::support::{HALF_TURN, QUARTER_TURN, half_turn, index_of, mods};

const TOLERANCE: f32 = 0.001;

fn anywhere(percent: f32) -> TileRule {
    TileRule {
        chance: Chance::new(percent).unwrap(),
        ..TileRule::new(Neighborhood::uniform(NeighborState::Any))
    }
}

fn half_turn_only() -> TileRule {
    TileRule {
        custom: Some(half_turn()),
        ..TileRule::new(only("left"))
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
fn a_tile_requirement_counts_towards_specificity() {
    let mut requiring = Neighborhood::uniform(NeighborState::Any);
    requiring.set_state(index_of("top"), NeighborState::Is(17));

    let pools = pools(&[(1, TileRule::new(requiring)), (3, anywhere(100.0))]);

    let order: Vec<usize> = pools.iter().map(|pool| pool.members[0].tile).collect();
    assert_eq!(order, [3, 1]);
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
        mods: mods(true, false, false),
        ..TileRule::new(only("left"))
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
        mods: mods(true, false, false),
        ..TileRule::new(only("left"))
    };
    let plain = TileRule::new(only("right"));

    let shares = base_shares(&pools(&[(3, flippable), (8, plain)]), ChanceMode::Normalize);

    assert_eq!(shares.get(&3), Some(&100.0));
    assert_eq!(shares.get(&8), Some(&50.0));
}

fn rotating(rotate: bool) -> TileRule {
    TileRule {
        mods: mods(false, false, rotate),
        ..TileRule::new(only("left"))
    }
}

#[test]
fn a_half_turn_only_rule_expands_to_the_turned_neighborhood() {
    let pools = pools(&[(3, half_turn_only())]);
    let members: Vec<&Variant> = pools.iter().flat_map(|pool| &pool.members).collect();

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].transform.suffix(), "VH");
    assert_eq!(members[0].neighborhood, only("right"));
}

#[test]
fn a_tile_without_the_identity_still_gets_a_share() {
    let shares = base_shares(&pools(&[(3, half_turn_only())]), ChanceMode::Normalize);

    assert_close(&[shares[&3]], &[100.0]);
}

#[test]
fn a_rotating_tile_joins_every_turned_pool_after_its_own() {
    let tiles: Vec<(usize, TileRule)> = (1..=4).map(|tile| (tile, rotating(true))).collect();
    let pools = pools(&tiles);

    let joined = pools_of(&pools, 1);
    let mates: Vec<usize> = joined[0].members.iter().map(|member| member.tile).collect();

    assert_eq!(joined.len(), 4);
    assert_eq!(joined[0].neighborhood, only("left"));
    assert_eq!(mates, [1, 2, 3, 4]);
}

#[test]
fn a_turned_pool_can_split_differently_than_the_tiles_own() {
    let tiles = [
        (1, rotating(true)),
        (2, rotating(true)),
        (3, rotating(false)),
    ];
    let pools = pools(&tiles);

    let shares: Vec<f32> = pools_of(&pools, 1)
        .iter()
        .filter_map(|pool| pool.share_of(1, ChanceMode::Normalize))
        .collect();

    assert_close(&shares, &[33.333_3, 50.0, 50.0, 50.0]);
}

fn variants(tiles: &[(usize, TileRule)]) -> Vec<Variant> {
    pools(tiles)
        .into_iter()
        .flat_map(|pool| pool.members)
        .collect()
}

#[test]
fn a_rule_drawn_upside_down_places_that_orientation_with_the_drawn_neighborhood() {
    let drawn_rule = TileRule {
        drawn: HALF_TURN,
        ..TileRule::new(only("left"))
    };

    let variants = variants(&[(3, drawn_rule)]);

    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0].transform, HALF_TURN);
    assert_eq!(variants[0].neighborhood, only("left"));
}

#[test]
fn rotating_a_rule_drawn_rotated_places_every_rotation_of_the_drawn_neighborhood() {
    let drawn_rule = TileRule {
        drawn: QUARTER_TURN,
        ..rotating(true)
    };

    let variants = variants(&[(3, drawn_rule)]);

    let mut placed: Vec<Transform> = variants.iter().map(|variant| variant.transform).collect();
    let mut rotations = Orientations::generated(mods(false, false, true)).transforms();
    placed.sort_by_key(|transform| transform.suffix());
    rotations.sort_by_key(|transform| transform.suffix());
    assert_eq!(placed, rotations);

    for variant in &variants {
        let relative = rotations
            .iter()
            .find(|rotation| rotation.after(QUARTER_TURN) == variant.transform)
            .unwrap();
        assert_eq!(variant.neighborhood, relative.apply(&only("left")));
    }
}

#[test]
fn only_the_first_variant_of_a_tiles_first_rule_is_primary() {
    let variants = variants(&[(3, rotating(true)), (3, TileRule::new(only("topLeft")))]);

    let primary: Vec<&Variant> = variants.iter().filter(|variant| variant.primary).collect();

    assert_eq!(primary.len(), 1);
    assert_eq!(primary[0].rule, 0);
    assert_eq!(primary[0].transform, Transform::IDENTITY);
}

#[test]
fn a_variant_knows_the_rule_it_came_from() {
    let variants = variants(&[
        (3, rotating(true)),
        (3, TileRule::new(only("topLeft"))),
        (4, TileRule::new(only("right"))),
    ]);

    let rules_of = |tile: usize| -> Vec<usize> {
        variants
            .iter()
            .filter(|variant| variant.tile == tile)
            .map(|variant| variant.rule)
            .collect()
    };

    assert_eq!(rules_of(3).iter().filter(|&&rule| rule == 0).count(), 4);
    assert_eq!(rules_of(3).iter().filter(|&&rule| rule == 1).count(), 1);
    assert_eq!(rules_of(4), [0]);
}

#[test]
fn rules_of_one_tile_need_not_be_adjacent() {
    let variants = variants(&[
        (5, TileRule::new(only("left"))),
        (6, TileRule::new(only("left"))),
        (5, anywhere(50.0)),
    ]);

    let mut rules_of_five: Vec<usize> = variants
        .iter()
        .filter(|variant| variant.tile == 5)
        .map(|variant| variant.rule)
        .collect();
    rules_of_five.sort();

    assert_eq!(rules_of_five, [0, 1]);
    assert_eq!(
        variants
            .iter()
            .filter(|variant| variant.tile == 5 && variant.primary)
            .count(),
        1
    );
}

#[test]
fn a_later_rule_reaching_a_known_neighborhood_by_another_transform_is_dropped() {
    let turned = TileRule {
        custom: Some(half_turn()),
        ..TileRule::new(only("right"))
    };

    let variants = variants(&[(3, TileRule::new(only("left"))), (3, turned)]);

    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0].rule, 0);
}

#[test]
fn a_placement_repeated_by_a_later_rule_of_the_tile_is_dropped() {
    let pools = pools(&[
        (3, TileRule::new(only("left"))),
        (3, anywhere(50.0)),
        (3, TileRule::new(only("left"))),
        (4, TileRule::new(only("left"))),
    ]);

    let shared = pools
        .iter()
        .find(|pool| pool.neighborhood == only("left"))
        .unwrap();
    let members: Vec<(usize, usize)> = shared
        .members
        .iter()
        .map(|member| (member.tile, member.rule))
        .collect();

    assert_eq!(members, [(3, 0), (4, 0)]);
    assert_eq!(pools.iter().flat_map(|pool| &pool.members).count(), 3);
}
