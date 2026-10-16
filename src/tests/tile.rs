use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::tile::{Chance, TileRule};
use crate::model::transform::Orientations;
use crate::tests::support::{half_turn, mods};

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
