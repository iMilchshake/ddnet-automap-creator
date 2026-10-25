use crate::blueprint::{self, BlueprintError};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Centre, Chance, TILE_COUNT, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tests::support::{half_turn, index_of, mods, outer_corner};

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

    project.add_rule(
        1,
        TileRule {
            mods: mods(true, false, true),
            chance: Chance::new(2.5).unwrap(),
            ..TileRule::new(outer_corner())
        },
    );
    project.add_rule(
        32,
        TileRule {
            custom: Some(half_turn()),
            ..TileRule::new(Neighborhood::uniform(NeighborState::Full))
        },
    );
    project.add_rule(
        200,
        TileRule::new(Neighborhood::uniform(NeighborState::Any)),
    );

    project.deactivate(7);
    project.deactivate(9);
    project.deactivate(32);

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

const RULE_HEAD: &str = r#"{"con": [0,0,0,0,0,0,0,0], "chance": 100.0"#;
const PLAIN_RULE: &str = r#"{"con": [0,0,0,0,0,0,0,0], "chance": 100.0, "orientations": ["N"]}"#;

fn written(project: Project) -> String {
    let rule_sets = RuleSets::from_parts(vec![("Grass Main".to_owned(), project)], 0);
    blueprint::to_json(&rule_sets, IMAGE).unwrap()
}

fn tile_of(id: usize, rules: &str) -> String {
    format!(r#"{{"id": {id}, "rules": [{rules}]}}"#)
}

fn plain_tile(id: usize) -> String {
    tile_of(id, PLAIN_RULE)
}

fn tile_with(orientations: &str) -> String {
    one_tile(&tile_of(
        5,
        &format!(r#"{RULE_HEAD}, "orientations": {orientations}}}"#),
    ))
}

fn loaded_rule(orientations: &str) -> TileRule {
    let project = load(&tile_with(orientations)).unwrap();
    project.rules_of(5)[0]
}

fn drawn_rule(drawn: Transform) -> Project {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            drawn,
            ..TileRule::new(outer_corner())
        },
    );

    project
}

#[test]
fn a_drawn_orientation_survives_a_round_trip() {
    let project = drawn_rule(Transform {
        x_flip: true,
        y_flip: false,
        rot: true,
    });

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""drawn": "VR""#));
}

fn centred(centre: Centre) -> Project {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            centre,
            ..TileRule::new(outer_corner())
        },
    );

    project
}

#[test]
fn an_empty_centre_survives_a_round_trip() {
    let project = centred(Centre::Empty);

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""centre": "empty""#));
}

#[test]
fn a_full_centre_is_not_written() {
    assert!(!written(centred(Centre::Full)).contains("centre"));
}

#[test]
fn a_missing_centre_loads_as_full() {
    assert_eq!(loaded_rule(r#"["N"]"#).centre, Centre::Full);
}

#[test]
fn an_unknown_centre_is_refused() {
    let rule = format!(r#"{RULE_HEAD}, "orientations": ["N"], "centre": "any"}}"#);

    assert!(matches!(
        load(&one_tile(&tile_of(5, &rule))).unwrap_err(),
        BlueprintError::Json(_)
    ));
}

#[test]
fn the_identity_is_not_written_as_drawn() {
    assert!(!written(drawn_rule(Transform::IDENTITY)).contains("drawn"));
}

#[test]
fn a_missing_drawn_orientation_loads_as_the_identity() {
    assert_eq!(loaded_rule(r#"["N"]"#).drawn, Transform::IDENTITY);
}

#[test]
fn an_unknown_drawn_orientation_is_rejected() {
    let rule = format!(r#"{RULE_HEAD}, "orientations": ["N"], "drawn": "Q"}}"#);

    assert!(matches!(
        load(&one_tile(&tile_of(5, &rule))),
        Err(BlueprintError::OrientationName { tile: 5, .. })
    ));
}

#[test]
fn a_preset_is_written_as_its_orientation_list() {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            mods: mods(false, false, true),
            ..TileRule::new(outer_corner())
        },
    );

    let text = written(project);

    assert!(
        text.contains(r#""orientations": [ "N", "R", "VH", "VHR" ]"#),
        "{text}"
    );
}

#[test]
fn a_custom_set_equal_to_a_preset_comes_back_as_that_preset() {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            custom: Some(Orientations::generated(mods(true, false, false))),
            ..TileRule::new(outer_corner())
        },
    );

    let loaded = round_trip(&project);
    let rule = loaded.rules_of(5)[0];

    assert_eq!(rule.mods, mods(true, false, false));
    assert_eq!(rule.custom, None);
}

#[test]
fn the_longest_rule_stays_on_one_line() {
    let mut project = Project::default();
    project.add_rule(
        TILE_COUNT - 2,
        TileRule {
            mods: mods(true, false, true),
            chance: Chance::new(1.0 / 3.0).unwrap(),
            ..TileRule::new(outer_corner())
        },
    );
    project.add_rule(TILE_COUNT - 1, TileRule::new(outer_corner()));

    let text = written(project);

    assert!(
        text.lines()
            .any(|line| line.contains("\"con\"") && line.contains("\"HR\"")),
        "{text}"
    );
}

#[test]
fn a_tile_with_two_rules_survives_a_round_trip() {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            mods: mods(false, false, true),
            chance: Chance::new(40.0).unwrap(),
            ..TileRule::new(outer_corner())
        },
    );
    project.add_rule(
        5,
        TileRule {
            custom: Some(half_turn()),
            ..TileRule::new(Neighborhood::uniform(NeighborState::Full))
        },
    );

    assert_eq!(round_trip(&project), project);
}

#[test]
fn a_tile_without_rules_is_refused() {
    let json = one_tile(&tile_of(5, ""));

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NoRules(5)
    ));
}

#[test]
fn a_preset_orientation_list_loads_as_flags() {
    let rotating = loaded_rule(r#"["N", "R", "VH", "VHR"]"#);
    let every = loaded_rule(r#"["N", "R", "VH", "VHR", "V", "VR", "H", "HR"]"#);

    assert_eq!(rotating.mods, mods(false, false, true));
    assert_eq!(rotating.custom, None);
    assert_eq!(every.mods, mods(true, false, true));
    assert_eq!(every.custom, None);
}

#[test]
fn a_non_preset_orientation_list_loads_as_custom() {
    let rule = loaded_rule(r#"["VH"]"#);

    assert_eq!(rule.custom, Some(half_turn()));
    assert_eq!(rule.mods, TileMods::default());
}

#[test]
fn an_unknown_or_empty_orientation_list_is_refused() {
    let unknown = tile_with(r#"["X"]"#);
    let empty = tile_with("[]");

    assert!(matches!(
        load(&unknown).unwrap_err(),
        BlueprintError::OrientationName { tile: 5, name } if name == "X"
    ));
    assert!(matches!(
        load(&empty).unwrap_err(),
        BlueprintError::NoOrientations(5)
    ));
}

#[test]
fn a_tile_without_orientations_is_refused() {
    let json = one_tile(&tile_of(5, &format!("{RULE_HEAD}}}")));

    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Json(_)));
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

fn document(tiles: &str, deactivated: &str, groups: &str) -> String {
    format!(
        r#"{{"version": 1, "image": "grass_main", "active": 0,
            "rule_sets": [{{"name": "Grass Main", "normalize": true,
                "tiles": [{tiles}], "deactivated": [{deactivated}],
                "groups": [{groups}]}}]}}"#
    )
}

fn one_tile(body: &str) -> String {
    document(body, "", "")
}

const DESERT_BLUEPRINT: &str = r#"{"version": 1, "image": "desert_main", "active": 0,
    "rule_sets": [{"name": "Desert", "normalize": true, "tiles": [],
                   "deactivated": [], "groups": []}]}"#;

#[test]
fn parsing_accepts_another_image_and_exposes_its_name() {
    let loaded = blueprint::parse(DESERT_BLUEPRINT).unwrap();

    assert_eq!(loaded.image, "desert_main");
    assert_eq!(loaded.rule_sets.active_name(), "Desert");
}

#[test]
fn an_unsupported_version_is_refused() {
    let json = r#"{"version": 3, "image": "grass_main", "active": 0, "rule_sets": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::UnsupportedVersion(3)
    ));
}

#[test]
fn a_blueprint_with_no_rule_sets_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "active": 0, "rule_sets": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::NoRuleSets
    ));
}

#[test]
fn a_blueprint_round_trips_several_rule_sets() {
    let mut set_a = Project::default();
    set_a.add_rule(1, TileRule::new(Neighborhood::uniform(NeighborState::Full)));
    let mut set_b = Project::default();
    set_b.add_rule(2, TileRule::new(Neighborhood::uniform(NeighborState::Any)));

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
    assert_eq!(loaded.active().rules_of(2), rule_sets.active().rules_of(2));
}

#[test]
fn a_tile_outside_the_tileset_is_refused() {
    let json = one_tile(&plain_tile(256));
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::TileId(256)
    ));
}

#[test]
fn a_rule_on_the_last_tile_is_accepted() {
    let json = one_tile(&plain_tile(TILE_COUNT - 1));
    assert!(load(&json).unwrap().has_rules(TILE_COUNT - 1));
}

#[test]
fn a_rule_on_the_air_tile_is_refused() {
    let json = one_tile(&plain_tile(AIR_TILE));
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::AirTile));
}

fn deactivating(tile: usize) -> String {
    document("", &tile.to_string(), "")
}

#[test]
fn deactivating_the_air_tile_is_refused() {
    assert!(matches!(
        load(&deactivating(AIR_TILE)).unwrap_err(),
        BlueprintError::AirTile
    ));
}

#[test]
fn the_chance_mode_survives_a_round_trip() {
    let mut project = furnished();
    project.set_chance_mode(ChanceMode::Exact);

    assert_eq!(round_trip(&project).chance_mode(), ChanceMode::Exact);
}

#[test]
fn a_repeated_tile_is_refused() {
    let tile = plain_tile(5);
    let json = one_tile(&format!("{tile}, {tile}"));
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::DuplicateTile(5)
    ));
}

#[test]
fn an_unknown_neighbor_code_is_refused() {
    let json = one_tile(&tile_of(
        5,
        r#"{"con": [0,1,2,3,0,0,0,0], "chance": 100.0, "orientations": ["N"]}"#,
    ));
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NeighborCode(3)
    ));
}

#[test]
fn a_chance_outside_its_range_is_refused() {
    let json = one_tile(&tile_of(
        5,
        r#"{"con": [0,0,0,0,0,0,0,0], "chance": 0.0, "orientations": ["N"]}"#,
    ));
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::Chance(_)
    ));
}

#[test]
fn an_invalid_group_is_refused() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "top_left": 64, "width": 1, "height": 1,
            "mode": "fill", "chance": 100.0}"#,
    );
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn two_groups_sharing_a_name_are_refused() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "top_left": 64, "width": 2, "height": 2,
            "mode": "fill", "chance": 100.0},
           {"name": "bones", "top_left": 100, "width": 2, "height": 2,
            "mode": "fill", "chance": 100.0}"#,
    );
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn a_group_over_a_configured_tile_is_refused() {
    let json = document(
        &plain_tile(65),
        "",
        r#"{"name": "bones", "top_left": 64, "width": 2, "height": 2,
            "mode": "fill", "chance": 100.0}"#,
    );
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::Claimed { tile: 65, .. }
    ));
}

#[test]
fn deactivated_tiles_survive_with_their_rules() {
    let original = furnished();
    let project = round_trip(&original);

    assert_eq!(project.deactivated_tiles(), [7, 9, 32]);
    assert!(project.is_deactivated(7));
    assert!(!project.has_rules(7));
    assert!(project.is_deactivated(32));
    assert_eq!(project.rules_of(32), original.rules_of(32));
    assert!(!project.is_deactivated(1));
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
    let project = load(&with_con("[2,0,2,0,1,2,1,1]")).unwrap();

    assert_eq!(project.rules_of(5)[0].neighborhood, outer_corner());
}

fn with_con(con: &str) -> String {
    one_tile(&tile_of(
        5,
        &format!(r#"{{"con": {con}, "chance": 100.0, "orientations": ["N"]}}"#),
    ))
}

#[test]
fn a_tile_requirement_survives_a_round_trip() {
    let mut neighborhood = outer_corner();
    neighborhood.set_state(index_of("topLeft"), NeighborState::Is(17));
    let mut project = Project::default();
    project.add_rule(5, TileRule::new(neighborhood));

    let text = written(project.clone());

    assert!(text.contains(r#"{ "is": 17 }"#), "{text}");
    assert_eq!(round_trip(&project), project);
}

#[test]
fn a_tile_requirement_loads_from_its_object_form() {
    let project = load(&with_con(r#"[{"is": 17}, 0, 2, 0, 1, 2, 1, 1]"#)).unwrap();
    let neighborhood = project.rules_of(5)[0].neighborhood;

    assert_eq!(neighborhood.state(0), NeighborState::Is(17));
    assert_eq!(neighborhood.state(4), NeighborState::Full);
}

#[test]
fn the_air_tile_as_a_neighbor_tile_is_refused() {
    let json = with_con(r#"[{"is": 0}, 0, 0, 0, 0, 0, 0, 0]"#);

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::AirNeighborTile
    ));
}

#[test]
fn a_neighbor_tile_outside_the_tileset_is_refused() {
    let json = with_con(r#"[{"is": 256}, 0, 0, 0, 0, 0, 0, 0]"#);

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::TileId(256)
    ));
}

#[test]
fn a_con_of_the_wrong_length_is_refused() {
    for con in ["[2,0,2,0,1,2,1]", "[2,0,2,0,1,2,1,1,1]"] {
        assert!(
            matches!(load(&with_con(con)).unwrap_err(), BlueprintError::Json(_)),
            "{con}"
        );
    }
}
