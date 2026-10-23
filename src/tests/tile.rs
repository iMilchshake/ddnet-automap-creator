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
fn turning_a_rule_clockwise_moves_its_neighborhood_and_records_the_drawing() {
    let turned = top_empty_rule().turned(Transform::ROTATE_CLOCKWISE);

    assert_eq!(named(&turned.neighborhood, NeighborState::Empty), ["right"]);
    assert_eq!(turned.drawn, Transform::ROTATE_CLOCKWISE);
}

#[test]
fn turning_by_the_inverse_of_the_drawing_restores_the_rule() {
    let rule = distinct_rule();
    let turned = rule
        .turned(Transform::ROTATE_CLOCKWISE)
        .turned(Transform::FLIP_HORIZONTALLY);

    assert_eq!(turned.turned(turned.drawn.inverse()), rule);
}

#[test]
fn turning_a_rotating_rule_keeps_its_placements() {
    let rule = distinct_rule();
    let before = placements(rule);
    let after = placements(rule.turned(Transform::ROTATE_CLOCKWISE));

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
