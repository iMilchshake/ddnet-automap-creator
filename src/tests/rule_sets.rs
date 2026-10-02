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
    assert_eq!(sets.active_name(), "grass_main2");
    assert_eq!(sets.active(), &sets.sets()[0].1);
}

#[test]
fn duplicating_avoids_an_existing_numbered_name() {
    let mut sets = RuleSets::new("grass_main".to_owned());
    sets.duplicate(0);
    sets.duplicate(0);

    assert_eq!(sets.name(1), Some("grass_main3"));
    assert_eq!(sets.name(2), Some("grass_main2"));
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

fn named_sets(names: &[&str], active: usize) -> RuleSets {
    let sets = names
        .iter()
        .map(|name| (name.to_string(), Default::default()))
        .collect();
    RuleSets::from_parts(sets, active)
}

fn names(sets: &RuleSets) -> Vec<&str> {
    sets.sets().iter().map(|(name, _)| name.as_str()).collect()
}

#[test]
fn only_sets_sharing_a_name_are_reported_as_shared() {
    let unique = named_sets(&["grass", "rock"], 0);
    assert!(!unique.is_name_shared(0));
    assert!(!unique.is_name_shared(1));

    let sets = named_sets(&["grass", "rock", "grass"], 0);
    assert!(sets.is_name_shared(0));
    assert!(!sets.is_name_shared(1));
    assert!(sets.is_name_shared(2));
    assert!(!sets.is_name_shared(3));
}

#[test]
fn merging_appends_in_order_and_keeps_the_active_selection() {
    let mut sets = named_sets(&["grass", "rock"], 1);

    let merged = sets.merge(named_sets(&["sand", "snow"], 0));

    assert_eq!(merged, 2);
    assert_eq!(names(&sets), ["grass", "rock", "sand", "snow"]);
    assert_eq!(sets.active_index(), 1);
}

#[test]
fn merging_a_colliding_name_appends_the_smallest_free_number() {
    let mut sets = named_sets(&["grass"], 0);

    sets.merge(named_sets(&["grass", "grass"], 0));

    assert_eq!(names(&sets), ["grass", "grass2", "grass3"]);
}

#[test]
fn merging_skips_numbered_names_that_already_exist() {
    let mut sets = named_sets(&["grass", "grass2"], 0);

    sets.merge(named_sets(&["grass"], 0));

    assert_eq!(names(&sets), ["grass", "grass2", "grass3"]);
}
