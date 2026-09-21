use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::TileRule;

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
