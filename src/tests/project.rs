use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, TileRule};

fn some_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Full))
}

#[test]
fn configuring_a_tile_clears_its_removal() {
    let mut project = Project::default();
    project.remove(7);
    project.set_rule(7, some_rule());

    assert!(!project.is_removed(7));
    assert_eq!(project.rule_count(), 1);
}

#[test]
fn removing_a_tile_drops_its_rule() {
    let mut project = Project::default();
    project.set_rule(7, some_rule());
    project.remove(7);

    assert!(project.rule(7).is_none());
    assert!(project.is_removed(7));
}

#[test]
fn restoring_leaves_the_tile_unconfigured() {
    let mut project = Project::default();
    project.remove(7);
    project.restore(7);

    assert!(!project.is_removed(7));
    assert!(project.rule(7).is_none());
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
