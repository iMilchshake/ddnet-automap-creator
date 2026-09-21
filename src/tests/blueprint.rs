use crate::blueprint::{self, BlueprintError};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, TileRule};
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
    let text = blueprint::to_json(project, IMAGE, "Grass Main").unwrap();
    blueprint::from_json(&text, IMAGE).unwrap().project
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
    let text = blueprint::to_json(&furnished(), IMAGE, "Grass Main").unwrap();
    assert_eq!(
        blueprint::from_json(&text, IMAGE).unwrap().rule_set,
        "Grass Main"
    );
}

fn load(json: &str) -> Result<Project, BlueprintError> {
    blueprint::from_json(json, IMAGE).map(|loaded| loaded.project)
}

fn one_tile(body: &str) -> String {
    format!(r#"{{"version": 1, "image": "grass_main", "tiles": [{body}]}}"#)
}

#[test]
fn a_blueprint_for_another_image_is_refused() {
    let json = r#"{"version": 1, "image": "desert_main", "tiles": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::WrongImage { .. }
    ));
}

#[test]
fn an_unsupported_version_is_refused() {
    let json = r#"{"version": 2, "image": "grass_main", "tiles": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::UnsupportedVersion(2)
    ));
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
    let json = r#"{"version": 1, "image": "grass_main", "tiles": [],
        "groups": [{"name": "bones", "top_left": 64, "width": 1, "height": 1,
                    "mode": "fill", "chance": 100.0}]}"#;
    assert!(matches!(load(json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn two_groups_sharing_a_name_are_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "tiles": [],
        "groups": [{"name": "bones", "top_left": 64, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0},
                   {"name": "bones", "top_left": 100, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0}]}"#;
    assert!(matches!(load(json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn a_group_over_a_configured_tile_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main",
        "tiles": [{"id": 65, "con": [0,0,0,0,0,0,0,0], "chance": 100.0,
                   "mods": {"x_flip": false, "y_flip": false, "rot": false}}],
        "groups": [{"name": "bones", "top_left": 64, "width": 2, "height": 2,
                    "mode": "fill", "chance": 100.0}]}"#;
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
    let text = blueprint::to_json(&furnished(), IMAGE, "Grass Main").unwrap();
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
