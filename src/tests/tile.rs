use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::pools;
use crate::model::tile::{Chance, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tests::support::{all_distinct, half_turn, index_of, mods, named};

#[test]
fn a_rule_places_its_preset_unless_custom_is_set() {
    let rule = TileRule {
        mods: mods(true, false, true),
        ..TileRule::new(Neighborhood::uniform(NeighborState::Any))
    };
    let custom = TileRule {
        custom: Some(half_turn()),
        ..rule
    };

    assert_eq!(rule.orientations(), Orientations::generated(rule.mods));
    assert_eq!(custom.orientations(), half_turn());
}

fn top_empty_rule() -> TileRule {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(index_of("top"), NeighborState::Empty);

    TileRule::new(neighborhood)
}

fn distinct_rule() -> TileRule {
    TileRule {
        mods: mods(false, false, true),
        ..TileRule::new(all_distinct())
    }
}

fn placements(rule: TileRule) -> Vec<(Transform, Neighborhood)> {
    pools(&[(1, rule)])
        .into_iter()
        .flat_map(|pool| {
            pool.members
                .into_iter()
                .map(move |member| (member.transform, pool.neighborhood))
        })
        .collect()
}

#[test]
fn turning_the_neighbors_clockwise_moves_only_the_neighborhood() {
    let rule = top_empty_rule();
    let turned = rule.turned_neighbors(Transform::ROTATE_CLOCKWISE);

    assert_eq!(named(&turned.neighborhood, NeighborState::Empty), ["right"]);
    assert_eq!(turned.drawn, rule.drawn);
}

#[test]
fn turning_the_tile_clockwise_records_only_the_drawing() {
    let rule = top_empty_rule();
    let turned = rule.turned_tile(Transform::ROTATE_CLOCKWISE);

    assert_eq!(turned.drawn, Transform::ROTATE_CLOCKWISE);
    assert_eq!(turned.neighborhood, rule.neighborhood);
}

#[test]
fn turning_the_tile_clockwise_four_times_restores_the_rule() {
    let rule = distinct_rule();
    let turned = (0..4).fold(rule, |turned, _| {
        turned.turned_tile(Transform::ROTATE_CLOCKWISE)
    });

    assert_eq!(turned, rule);
}

#[test]
fn turning_a_rotating_rule_keeps_its_placements() {
    let rule = distinct_rule();
    let before = placements(rule);
    let turned = rule
        .turned_neighbors(Transform::ROTATE_CLOCKWISE)
        .turned_tile(Transform::ROTATE_CLOCKWISE);
    let after = placements(turned);

    assert_eq!(before.len(), after.len());
    assert!(before.iter().all(|placement| after.contains(placement)));
}

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
