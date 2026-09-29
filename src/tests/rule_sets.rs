use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::rule_sets::RuleSets;
use crate::model::tile::TileRule;

#[test]
fn a_fresh_collection_holds_one_set_and_selects_it() {
    let sets = RuleSets::new("grass_main".to_owned());

    assert_eq!(sets.len(), 1);
    assert_eq!(sets.active_index(), 0);
    assert_eq!(sets.active_name(), "grass_main");
}

#[test]
fn adding_selects_the_new_set_with_a_fresh_name() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    let index = sets.add();

    assert_eq!(index, 1);
    assert_eq!(sets.active_index(), 1);
    assert_eq!(sets.active_name(), "ruleset0");

    sets.add();
    assert_eq!(sets.active_name(), "ruleset1");
}

#[test]
fn the_last_rule_set_cannot_be_removed() {
    let mut sets = RuleSets::new("grass_main".to_owned());

    assert!(!sets.remove(0));
    assert_eq!(sets.len(), 1);
}

#[test]
fn removing_the_active_set_selects_a_neighbor() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.add();
    sets.add();
    sets.select(2);

    assert!(sets.remove(2));
    assert_eq!(sets.len(), 2);
    assert_eq!(sets.active_index(), 1);
}

#[test]
fn removing_a_set_before_the_active_one_shifts_the_active_index() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.add();
    sets.select(1);

    assert!(sets.remove(0));
    assert_eq!(sets.active_index(), 0);
    assert_eq!(sets.active_name(), "ruleset0");
}

#[test]
fn selecting_out_of_range_is_ignored() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.select(5);

    assert_eq!(sets.active_index(), 0);
}

#[test]
fn renaming_the_active_set_edits_it_in_place() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    *sets.active_name_mut() = "renamed".to_owned();

    assert_eq!(sets.active_name(), "renamed");
    assert_eq!(sets.name(0), Some("renamed"));
}

#[test]
fn duplicating_inserts_a_copy_right_after_and_selects_it() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.active_mut()
        .set_rule(5, TileRule::new(Neighborhood::uniform(NeighborState::Full)));
    sets.add();

    let index = sets.duplicate(0).unwrap();

    assert_eq!(index, 1);
    assert_eq!(sets.len(), 3);
    assert_eq!(sets.active_index(), 1);
    assert_eq!(sets.active_name(), "grass_main copy");
    assert_eq!(sets.active(), &sets.sets()[0].1);
}

#[test]
fn duplicating_avoids_an_existing_copy_name() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.duplicate(0);
    sets.duplicate(0);

    assert_eq!(sets.name(1), Some("grass_main copy 2"));
    assert_eq!(sets.name(2), Some("grass_main copy"));
}

#[test]
fn from_parts_clamps_an_out_of_range_active_index() {
    let sets = RuleSets::from_parts(
        vec![
            ("a".to_owned(), Default::default()),
            ("b".to_owned(), Default::default()),
        ],
        9,
    );

    assert_eq!(sets.active_index(), 1);
}
