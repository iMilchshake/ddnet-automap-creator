use crate::blueprint::{self, BlueprintError};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Chance, MASK_TILE, TileRule};
use crate::tests::support::{mods, outer_corner};

const IMAGE: &str = "grass_main";

fn group(name: &str, top_left: usize, mode: GroupMode, percent: f32) -> TileGroup {
    TileGroup {
        name: name.to_owned(),
        top_left,
        width: 2,
        height: 2,
        mode,
        chance: Chance::new(percent).unwrap(),
    }
}

fn furnished() -> Project {
    let mut project = Project::default();

    project.set_rule(
        1,
        TileRule {
            neighborhood: outer_corner(),
            mods: mods(true, false, true),
            chance: Chance::new(2.5).unwrap(),
        },
    );
    project.set_rule(
        32,
        TileRule {
            neighborhood: Neighborhood::uniform(NeighborState::Full),
            mods: mods(false, true, false),
            chance: Chance::FULL,
        },
    );
    project.set_rule(
        200,
        TileRule::new(Neighborhood::uniform(NeighborState::Any)),
    );

    project.remove(7);
    project.remove(9);

    project.append_group(group("bones", 64, GroupMode::Decorate, 1.0));
    project.append_group(group("pipes", 100, GroupMode::Fill, 100.0));
    project.append_group(group("vines", 150, GroupMode::Fill, 33.5));

    project
}

fn round_trip(project: &Project) -> Project {
    let rule_sets = RuleSets::from_parts(vec![("Grass Main".to_owned(), project.clone())], 0);
    let text = blueprint::to_json(&rule_sets, IMAGE).unwrap();
    blueprint::parse(&text).unwrap().rule_sets.active().clone()
}

#[test]
fn a_furnished_project_survives_a_round_trip_unchanged() {
    let project = furnished();
    assert_eq!(round_trip(&project), project);
}

#[test]
fn group_priority_order_survives_a_round_trip() {
    let loaded = round_trip(&furnished());
    let names: Vec<&str> = loaded
        .groups()
        .iter()
        .map(|group| group.name.as_str())
        .collect();

    assert_eq!(names, ["bones", "pipes", "vines"]);
}

#[test]
fn the_rule_set_name_is_carried_along() {
    let rule_sets = RuleSets::from_parts(vec![("Grass Main".to_owned(), furnished())], 0);
    let text = blueprint::to_json(&rule_sets, IMAGE).unwrap();
    assert_eq!(
        blueprint::parse(&text).unwrap().rule_sets.active_name(),
        "Grass Main"
    );
}

fn load(json: &str) -> Result<Project, BlueprintError> {
    blueprint::parse(json).map(|loaded| loaded.rule_sets.active().clone())
}

fn one_tile(body: &str) -> String {
    format!(
        r#"{{"version": 1, "image": "grass_main",
            "rule_sets": [{{"name": "Grass Main", "tiles": [{body}]}}]}}"#
    )
}

const DESERT_BLUEPRINT: &str = r#"{"version": 1, "image": "desert_main",
    "rule_sets": [{"name": "Desert", "tiles": []}]}"#;

#[test]
fn parsing_accepts_another_image_and_exposes_its_name() {
    let loaded = blueprint::parse(DESERT_BLUEPRINT).unwrap();

    assert_eq!(loaded.image.as_deref(), Some("desert_main"));
    assert_eq!(loaded.rule_sets.active_name(), "Desert");
}

#[test]
fn an_unsupported_version_is_refused() {
    let json = r#"{"version": 3, "image": "grass_main", "rule_sets": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::UnsupportedVersion(3)
    ));
}

#[test]
fn a_blueprint_with_no_rule_sets_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "rule_sets": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::NoRuleSets
    ));
}

#[test]
fn a_blueprint_round_trips_several_rule_sets() {
    let mut set_a = Project::default();
    set_a.set_rule(1, TileRule::new(Neighborhood::uniform(NeighborState::Full)));
    let mut set_b = Project::default();
    set_b.set_rule(2, TileRule::new(Neighborhood::uniform(NeighborState::Any)));

    let rule_sets = RuleSets::from_parts(
        vec![("Grass".to_owned(), set_a), ("Doodads".to_owned(), set_b)],
        1,
    );
    let text = blueprint::to_json(&rule_sets, IMAGE).unwrap();
    let loaded = blueprint::parse(&text).unwrap().rule_sets;

    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.active_index(), 1);
    assert_eq!(loaded.name(0), Some("Grass"));
    assert_eq!(loaded.name(1), Some("Doodads"));
    assert_eq!(loaded.active().rule(2), rule_sets.active().rule(2));
}

#[test]
fn a_tile_outside_the_tileset_is_refused() {
    let json = one_tile(
        r#"{"id": 256, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#,
    );
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::TileId(256)
    ));
}

#[test]
fn a_rule_on_the_mask_tile_is_refused() {
    let json = one_tile(&format!(
        r#"{{"id": {MASK_TILE}, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
            "mods": {{"x_flip": false, "y_flip": false, "rot": false}}}}"#
    ));
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::MaskTile));
}

#[test]
fn a_rule_on_the_air_tile_is_refused() {
    let json = one_tile(&format!(
        r#"{{"id": {AIR_TILE}, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
            "mods": {{"x_flip": false, "y_flip": false, "rot": false}}}}"#
    ));
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::AirTile));
}

#[test]
fn the_chance_mode_survives_a_round_trip() {
    let mut project = furnished();
    project.set_chance_mode(ChanceMode::Exact);

    assert_eq!(round_trip(&project).chance_mode(), ChanceMode::Exact);
}

#[test]
fn a_file_without_a_chance_mode_normalizes() {
    let json = r#"{"version": 1, "image": "grass_main",
        "rule_sets": [{"name": "Grass Main", "tiles": []}]}"#;

    assert_eq!(load(json).unwrap().chance_mode(), ChanceMode::Normalize);
}

#[test]
fn a_repeated_tile_is_refused() {
    let tile = r#"{"id": 5, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
                   "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#;
    let json = one_tile(&format!("{tile}, {tile}"));
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::DuplicateTile(5)
    ));
}

#[test]
fn an_unknown_neighbor_code_is_refused() {
    let json = one_tile(
        r#"{"id": 5, "con": [0,1,2,3,0,0,0,0], "chance": 100.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#,
    );
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NeighborCode(3)
    ));
}

#[test]
fn a_chance_outside_its_range_is_refused() {
    let json = one_tile(
        r#"{"id": 5, "con": [0,0,0,0,0,0,0,0], "chance": 0.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#,
    );
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::Chance(_)
    ));
}

#[test]
fn an_invalid_group_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "rule_sets": [{"name": "Grass Main",
        "tiles": [],
        "groups": [{"name": "bones", "top_left": 64, "width": 1, "height": 1,
                    "mode": "fill", "chance": 100.0}]}]}"#;
    assert!(matches!(load(json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn two_groups_sharing_a_name_are_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "rule_sets": [{"name": "Grass Main",
        "tiles": [],
        "groups": [{"name": "bones", "top_left": 64, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0},
                   {"name": "bones", "top_left": 100, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0}]}]}"#;
    assert!(matches!(load(json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn a_group_over_a_configured_tile_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "rule_sets": [{"name": "Grass Main",
        "tiles": [{"id": 65, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
                   "mods": {"x_flip": false, "y_flip": false, "rot": false}}],
        "groups": [{"name": "bones", "top_left": 64, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0}]}]}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::Claimed { tile: 65, .. }
    ));
}

#[test]
fn a_file_without_removed_or_groups_loads_as_empty() {
    let json = one_tile(
        r#"{"id": 5, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#,
    );
    let project = load(&json).unwrap();

    assert_eq!(project.rule_count(), 1);
    assert!(project.removed_tiles().is_empty());
    assert!(project.groups().is_empty());
}

#[test]
fn the_retired_empty_flag_is_ignored_rather_than_refused() {
    let json = one_tile(
        r#"{"id": 5, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false, "empty": true}}"#,
    );
    assert_eq!(load(&json).unwrap().rule_count(), 1);
}

#[test]
fn removed_tiles_are_kept_apart_from_rules() {
    let project = round_trip(&furnished());

    assert_eq!(project.removed_tiles(), [7, 9]);
    assert!(project.is_removed(7));
    assert!(project.rule(7).is_none());
}

#[test]
fn a_neighborhood_is_stored_in_index_order() {
    let rule_sets = RuleSets::from_parts(vec![("Grass Main".to_owned(), furnished())], 0);
    let text = blueprint::to_json(&rule_sets, IMAGE).unwrap();
    assert!(
        text.contains("\"con\": [ 2, 0, 2, 0, 1, 2, 1, 1 ]"),
        "{text}"
    );
}

#[test]
fn a_neighborhood_is_read_back_in_index_order() {
    let json = one_tile(
        r#"{"id": 5, "con": [2,0,2,0,1,2,1,1], "chance": 100.0,
            "mods": {"x_flip": false, "y_flip": false, "rot": false}}"#,
    );
    let project = load(&json).unwrap();

    assert_eq!(project.rule(5).unwrap().neighborhood, outer_corner());
}

#[test]
fn a_con_of_the_wrong_length_is_refused() {
    for con in ["[2,0,2,0,1,2,1]", "[2,0,2,0,1,2,1,1,1]"] {
        let json = one_tile(&format!(
            r#"{{"id": 5, "con": {con}, "chance": 100.0,
                "mods": {{"x_flip": false, "y_flip": false, "rot": false}}}}"#
        ));
        assert!(
            matches!(load(&json).unwrap_err(), BlueprintError::Json(_)),
            "{con}"
        );
    }
}
