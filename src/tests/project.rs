use crate::export::r_source::{RuleSet, render};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::{self, ChanceMode};
use crate::model::project::Project;
use crate::model::tile::{AIR_TILE, Chance, MASK_TILE, TileRule};

fn some_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Full))
}

#[test]
fn the_mask_tile_is_never_free() {
    assert!(!Project::default().is_free(MASK_TILE));
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
    project.set_rule(7, some_rule());

    assert!(project.is_deactivated(7));
    assert_eq!(project.rule_count(), 1);
}

#[test]
fn deactivating_a_tile_keeps_its_rule() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.deactivate(7);

    assert_eq!(project.rule(7), Some(&some_rule()));
    assert!(project.is_deactivated(7));
    assert_eq!(project.deactivated_rule_count(), 1);
}

#[test]
fn reactivating_makes_the_rule_active_again() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.deactivate(7);
    project.reactivate(7);

    assert!(!project.is_deactivated(7));
    assert_eq!(project.rules(), [(7, some_rule())]);
}

#[test]
fn clearing_the_rule_of_a_deactivated_tile_keeps_the_mark() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.deactivate(7);
    project.clear_rule(7);

    assert!(project.is_deactivated(7));
    assert!(project.rule(7).is_none());
}

#[test]
fn deactivated_rules_are_left_out_of_the_active_rules() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.set_rule(8, some_rule());
    project.deactivate(7);

    assert_eq!(project.rules(), [(8, some_rule())]);
    assert_eq!(project.all_rules().len(), 2);
}

#[test]
fn a_deactivated_rule_claims_its_tile() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.deactivate(7);

    assert!(!project.is_free(7));
}

#[test]
fn deactivated_rules_are_left_out_of_the_export() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.set_rule(8, some_rule());
    project.deactivate(7);

    let source = emitted(&project);

    assert!(source.contains("Insert(8)"));
    assert!(!source.contains("Insert(7)"));
}

#[test]
fn deactivated_rules_are_left_out_of_the_pools() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.set_rule(8, some_rule());
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

    project.set_rule(64, some_rule());
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
