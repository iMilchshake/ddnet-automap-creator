use crate::export::r_source::{RuleSet, render};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::{self, ChanceMode};
use crate::model::project::Project;
use crate::model::tile::{AIR_TILE, Chance, TILE_COUNT, TileRule};

fn some_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Full))
}

fn other_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Any))
}

fn half_rule() -> TileRule {
    TileRule {
        chance: Chance::new(50.0).unwrap(),
        ..some_rule()
    }
}

#[test]
fn the_last_tile_is_free_like_any_other() {
    assert!(Project::default().is_free(TILE_COUNT - 1));
}

#[test]
fn the_mask_is_the_highest_free_tile() {
    assert_eq!(Project::default().mask_tile(), Some(TILE_COUNT - 1));
}

#[test]
fn the_mask_skips_tiles_with_a_rule_even_when_deactivated() {
    let mut project = Project::default();
    project.add_rule(TILE_COUNT - 1, some_rule());
    project.add_rule(TILE_COUNT - 2, some_rule());
    project.deactivate(TILE_COUNT - 2);

    assert_eq!(project.mask_tile(), Some(TILE_COUNT - 3));
}

#[test]
fn the_mask_skips_tiles_inside_a_group() {
    let mut project = Project::default();
    project.add_group(TileGroup {
        name: "bones".to_owned(),
        top_left: TILE_COUNT - 18,
        width: 2,
        height: 2,
        mode: GroupMode::Fill,
        chance: Chance::FULL,
    });

    assert_eq!(project.mask_tile(), Some(TILE_COUNT - 3));
}

#[test]
fn the_mask_skips_tiles_a_neighbor_refers_to() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(0, NeighborState::Is(TILE_COUNT - 1));
    neighborhood.set_state(1, NeighborState::Is(TILE_COUNT - 2));
    let mut project = Project::default();
    project.add_rule(7, TileRule::new(neighborhood));

    assert_eq!(project.mask_tile(), Some(TILE_COUNT - 3));
}

#[test]
fn there_is_no_mask_when_every_tile_is_used() {
    let mut project = Project::default();
    for tile in 1..TILE_COUNT {
        project.add_rule(tile, some_rule());
    }

    assert_eq!(project.mask_tile(), None);
}

#[test]
fn the_air_tile_is_never_free() {
    assert!(!Project::default().is_free(AIR_TILE));
}

fn emitted(project: &Project) -> String {
    render(&[RuleSet {
        image_stem: "grass_main",
        name: "Grass_Main",
        tiles: &project.all_rules(),
        deactivated: &project.deactivated_tiles(),
        groups: &[],
        chance_mode: ChanceMode::Normalize,
    }])
    .unwrap()
}

#[test]
fn configuring_a_deactivated_tile_keeps_it_deactivated() {
    let mut project = Project::default();
    project.deactivate(7);
    project.add_rule(7, some_rule());

    assert!(project.is_deactivated(7));
    assert_eq!(project.rule_count(), 1);
}

#[test]
fn deactivating_a_tile_keeps_its_rule() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.deactivate(7);

    assert_eq!(project.rules_of(7), [some_rule()]);
    assert!(project.is_deactivated(7));
    assert_eq!(project.deactivated_rule_count(), 1);
}

#[test]
fn reactivating_makes_the_rule_active_again() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.deactivate(7);
    project.reactivate(7);

    assert!(!project.is_deactivated(7));
    assert_eq!(project.rules(), [(7, some_rule())]);
}

#[test]
fn clearing_the_rule_of_a_deactivated_tile_keeps_the_mark() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.deactivate(7);
    project.remove_rule(7, 0);

    assert!(project.is_deactivated(7));
    assert!(!project.has_rules(7));
}

#[test]
fn deactivated_rules_are_left_out_of_the_active_rules() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(8, some_rule());
    project.deactivate(7);

    assert_eq!(project.rules(), [(8, some_rule())]);
    assert_eq!(project.all_rules().len(), 2);
}

#[test]
fn a_deactivated_rule_claims_its_tile() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.deactivate(7);

    assert!(!project.is_free(7));
}

#[test]
fn deactivated_rules_are_left_out_of_the_export() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(8, some_rule());
    project.deactivate(7);

    let source = emitted(&project);

    assert!(source.contains("Insert(8)"));
    assert!(!source.contains("Insert(7)"));
}

#[test]
fn deactivated_rules_are_left_out_of_the_pools() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(8, some_rule());
    project.deactivate(7);

    let pools = pool::pools(&project.rules());

    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0].members.len(), 1);
    assert_eq!(pools[0].members[0].tile, 8);
}

#[test]
fn a_tile_is_free_only_when_no_rule_and_no_group_claim_it() {
    let mut project = Project::default();
    assert!(project.is_free(64));

    project.add_rule(64, some_rule());
    assert!(!project.is_free(64));

    let mut grouped = Project::default();
    grouped.add_group(TileGroup {
        name: "bones".to_owned(),
        top_left: 64,
        width: 2,
        height: 2,
        mode: GroupMode::Fill,
        chance: Chance::FULL,
    });

    for tile in [64, 65, 80, 81] {
        assert!(!grouped.is_free(tile), "{tile}");
    }
    assert!(grouped.is_free(66));
}

#[test]
fn adding_rules_appends_them_and_returns_their_index() {
    let mut project = Project::default();

    assert!(project.rules_of(7).is_empty());
    assert!(!project.has_rules(7));
    assert_eq!(project.add_rule(7, some_rule()), 0);
    assert_eq!(project.add_rule(7, other_rule()), 1);

    assert!(project.has_rules(7));
    assert_eq!(project.rules_of(7), [some_rule(), other_rule()]);
}

#[test]
fn replacing_a_rule_leaves_the_others_alone() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(7, other_rule());

    project.replace_rule(7, 1, half_rule());
    project.replace_rule(7, 2, other_rule());
    project.replace_rule(8, 0, other_rule());

    assert_eq!(project.rules_of(7), [some_rule(), half_rule()]);
    assert!(!project.has_rules(8));
}

#[test]
fn removing_a_rule_shifts_the_later_ones_up() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(7, other_rule());
    project.add_rule(7, half_rule());

    project.remove_rule(7, 9);
    assert_eq!(project.rules_of(7).len(), 3);

    project.remove_rule(7, 0);
    assert_eq!(project.rules_of(7), [other_rule(), half_rule()]);
}

#[test]
fn removing_the_last_rule_frees_the_tile() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());

    project.remove_rule(7, 0);

    assert!(!project.has_rules(7));
    assert!(project.is_free(7));
    assert_eq!(project.rule_count(), 0);
}

#[test]
fn rules_are_flattened_in_tile_then_rule_order() {
    let mut project = Project::default();
    project.add_rule(9, some_rule());
    project.add_rule(7, some_rule());
    project.add_rule(8, half_rule());
    project.add_rule(7, other_rule());

    assert_eq!(
        project.all_rules(),
        [
            (7, some_rule()),
            (7, other_rule()),
            (8, half_rule()),
            (9, some_rule()),
        ]
    );
}

#[test]
fn counts_cover_every_rule_and_deactivation_stays_per_tile() {
    let mut project = Project::default();
    project.add_rule(7, some_rule());
    project.add_rule(7, other_rule());
    project.add_rule(8, half_rule());
    project.deactivate(7);

    assert_eq!(project.rule_count(), 3);
    assert_eq!(project.deactivated_rule_count(), 2);
    assert_eq!(project.rules(), [(8, half_rule())]);
}
