use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::TileRule;
use crate::ui::overview::Overview;

fn any_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Any))
}

#[test]
fn turned_off_tiles_are_not_counted_as_tiles_with_rules() {
    let mut project = Project::default();
    project.add_rule(1, any_rule());
    project.deactivate(1);

    let overview = Overview::new(&project);

    assert_eq!(overview.tiles_with_rules, 0);
    assert_eq!(overview.rules, 0);
    assert_eq!(overview.deactivated_rules, 1);
}

#[test]
fn rules_of_turned_off_tiles_are_listed_as_deactivated() {
    let mut project = Project::default();
    project.add_rule(1, any_rule());
    project.add_rule(1, any_rule());
    project.add_rule(2, any_rule());
    project.deactivate(2);

    assert_eq!(Overview::new(&project).rules_text(), "2 (+1 deactivated)");
}

#[test]
fn rules_text_has_no_suffix_without_deactivated_rules() {
    let mut project = Project::default();
    project.add_rule(1, any_rule());

    assert_eq!(Overview::new(&project).rules_text(), "1");
}
