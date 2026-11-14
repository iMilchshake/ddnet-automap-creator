use crate::blueprint::{self, BlueprintError};
use crate::model::group::{CellState, GroupError, GroupPreset, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood, Side};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Chance, RuleCenter, TILE_COUNT, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};
use crate::tests::support::{half_turn, mods, offset_of, outer_corner};

const IMAGE: &str = "grass_main";

fn group(name: &str, top_left: usize, preset: GroupPreset, percent: f32) -> TileGroup {
    let mut group = TileGroup::new(
        name.to_owned(),
        TileGroup::rect_tiles(top_left, top_left + 17),
    );
    group.apply_preset(preset);

    TileGroup {
        chance: Chance::new(percent).unwrap(),
        ..group
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

    project.append_group(group("bones", 64, GroupPreset::Decorate, 1.0));
    project.append_group(group("pipes", 100, GroupPreset::Fill, 100.0));
    project.append_group(group("vines", 150, GroupPreset::OnFloor, 33.5));

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

const RULE_HEAD: &str = r#"{"neighbors": "EEEEEEEE", "chance": 100.0"#;
const PLAIN_RULE: &str = r#"{"neighbors": "EEEEEEEE", "chance": 100.0, "orientations": ["N"]}"#;

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
    project.rules_of(5)[0].clone()
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

fn centered(center: RuleCenter) -> Project {
    let mut project = Project::default();
    project.add_rule(
        5,
        TileRule {
            center,
            ..TileRule::new(outer_corner())
        },
    );

    project
}

#[test]
fn an_empty_center_survives_a_round_trip() {
    let project = centered(RuleCenter::Empty);

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""center": "E""#));
}

#[test]
fn an_any_center_survives_a_round_trip() {
    let project = centered(RuleCenter::Any);

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""center": "A""#));
}

#[test]
fn a_full_center_is_not_written() {
    assert!(!written(centered(RuleCenter::Full)).contains("center"));
}

#[test]
fn a_missing_center_loads_as_full() {
    assert_eq!(loaded_rule(r#"["N"]"#).center, RuleCenter::Full);
}

#[test]
fn an_unknown_center_is_refused() {
    let rule = format!(r#"{RULE_HEAD}, "orientations": ["N"], "center": "S"}}"#);

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
    let rule = &loaded.rules_of(5)[0];

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
            .any(|line| line.contains("\"neighbors\": \"") && line.contains("\"HR\"")),
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
fn a_group_of_any_shape_survives_a_round_trip() {
    let mut l_shape = group("bones", 64, GroupPreset::Fill, 100.0);
    l_shape.set_tiles([65, 81, 82].into_iter().collect());
    let mut project = Project::default();
    project.append_group(l_shape.clone());

    assert_eq!(round_trip(&project).groups(), [l_shape]);
}

#[test]
fn the_copies_of_a_group_survive_a_round_trip() {
    let mut project = Project::default();
    project.append_group(TileGroup {
        mods: mods(false, false, true),
        ..group("bones", 64, GroupPreset::Fill, 100.0)
    });

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""orientations": [ "N", "R", "VH", "VHR" ]"#));
}

#[test]
fn a_custom_neighborhood_survives_a_round_trip() {
    let mut custom = group("bones", 64, GroupPreset::Fill, 100.0);
    custom.neighbors.insert((0, 4), CellState::Empty);
    custom.neighbors.insert((-1, 3), CellState::Full);
    let mut project = Project::default();
    project.append_group(custom);

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(r#""neighbors""#));
}

#[test]
fn a_group_neighborhood_is_read_row_by_row_from_the_grown_bounding_box() {
    const FULL_OFFSETS: [(i32, i32); 14] = [
        (-1, -1),
        (1, -1),
        (-2, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (2, 1),
        (-1, 2),
        (0, 2),
        (1, 2),
        (2, 2),
        (0, 3),
    ];
    const EMPTY_OFFSETS: [(i32, i32); 4] = [(0, -1), (-1, 0), (-2, 1), (1, 3)];
    const ANCHOR: (i32, i32) = (8, 1);
    const NEIGHBORS: &str = "AFEFAFEFFAEFFFFAFFFFAAFEA";
    let json = document(
        "",
        "",
        &format!(
            r#"{{"name": "bones", "tiles": [24, 25, 39, 40, 41, 56, 57],
                "neighbors": "{NEIGHBORS}", "chance": 100.0}}"#
        ),
    );

    let project = load(&json).unwrap();
    let neighbors = &project.groups()[0].neighbors;
    let state_at = |(dx, dy): (i32, i32)| neighbors.get(&(ANCHOR.0 + dx, ANCHOR.1 + dy));

    for offset in FULL_OFFSETS {
        assert_eq!(state_at(offset), Some(&CellState::Full), "{offset:?}");
    }
    for offset in EMPTY_OFFSETS {
        assert_eq!(state_at(offset), Some(&CellState::Empty), "{offset:?}");
    }
    assert_eq!(
        neighbors
            .values()
            .filter(|state| **state != CellState::Any)
            .count(),
        FULL_OFFSETS.len() + EMPTY_OFFSETS.len()
    );
    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(&format!(r#""neighbors": "{NEIGHBORS}""#)));
}

#[test]
fn a_neighborhood_of_the_wrong_length_is_refused() {
    for neighbors in ["", "AAA", "AAAA", "AAAAAAAAAAAAAAAAA"] {
        let json = document(
            "",
            "",
            &format!(
                r#"{{"name": "bones", "tiles": [64, 65, 80, 81],
                    "neighbors": "{neighbors}", "chance": 100.0}}"#
            ),
        );

        assert!(
            matches!(
                load(&json).unwrap_err(),
                BlueprintError::GroupNeighborShape(name) if name == "bones"
            ),
            "{neighbors}"
        );
    }
}

#[test]
fn a_condition_on_a_corner_away_from_a_non_rectangular_group_is_refused() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [24, 25, 39, 40, 41, 56, 57],
            "neighbors": "FFEFAFEFFAEFFFFAFFFFAAFEA", "chance": 100.0}"#,
    );

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::GroupNeighborCell(name) if name == "bones"
    ));
}

#[test]
fn a_specific_tile_in_a_group_neighborhood_is_refused() {
    for neighbors in ["[5]AAA", "[0]AAA", "[9999]AAA"] {
        let json = document(
            "",
            "",
            &format!(
                r#"{{"name": "bones", "tiles": [64, 65, 80, 81],
                    "neighbors": "{neighbors}", "chance": 100.0}}"#
            ),
        );

        assert!(
            matches!(
                load(&json).unwrap_err(),
                BlueprintError::GroupNeighborTile(name) if name == "bones"
            ),
            "{neighbors}"
        );
    }
}

#[test]
fn a_group_without_tiles_reports_that_it_is_too_small() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [],
            "neighbors": "", "chance": 100.0}"#,
    );

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::Group(GroupError::TooSmall)
    ));
}

#[test]
fn a_group_with_the_old_mode_field_is_refused() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [64, 65, 80, 81],
            "mode": "fill", "chance": 100.0}"#,
    );

    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Json(_)));
}

#[test]
fn a_group_without_orientations_loads_as_the_identity_only() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [64, 65, 80, 81],
            "neighbors": "AAAAAAAAAAAAAAAA", "chance": 100.0}"#,
    );

    assert_eq!(load(&json).unwrap().groups()[0].mods, TileMods::default());
}

#[test]
fn group_orientations_no_flags_can_make_are_refused() {
    for list in [r#"["N", "R"]"#, "[]"] {
        let json = document(
            "",
            "",
            &format!(
                r#"{{"name": "bones", "tiles": [64, 65, 80, 81],
                    "neighbors": "AAAAAAAAAAAAAAAA", "chance": 100.0,
                    "orientations": {list}}}"#
            ),
        );

        assert!(
            matches!(
                load(&json).unwrap_err(),
                BlueprintError::GroupOrientations(_)
            ),
            "{list}"
        );
    }
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
        r#"{{"version": 2, "image": "grass_main", "active": 0,
            "rule_sets": [{{"name": "Grass Main", "normalize": true,
                "tiles": [{tiles}], "deactivated": [{deactivated}],
                "groups": [{groups}]}}]}}"#
    )
}

fn one_tile(body: &str) -> String {
    document(body, "", "")
}

fn fixtures() -> Vec<(String, String)> {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/src/tests/fixtures");

    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(path).unwrap())
        })
        .collect()
}

fn fixture_version(name: &str) -> u32 {
    name.strip_prefix("blueprint_v")
        .and_then(|rest| rest.strip_suffix(".json"))
        .and_then(|number| number.parse().ok())
        .unwrap_or_else(|| panic!("{name} is not named blueprint_v<N>.json"))
}

const DESERT_BLUEPRINT: &str = r#"{"version": 2, "image": "desert_main", "active": 0,
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
fn a_version_1_blueprint_is_refused() {
    let json = r#"{"version": 1, "image": "grass_main", "active": 0, "rule_sets": []}"#;
    assert!(matches!(
        load(json).unwrap_err(),
        BlueprintError::UnsupportedVersion(1)
    ));
}

#[test]
fn every_fixture_blueprint_loads() {
    let fixtures = fixtures();

    assert!(!fixtures.is_empty());
    for (name, text) in fixtures {
        if let Err(error) = blueprint::parse(&text) {
            panic!("{name}: {error}");
        }
    }
}

#[test]
fn the_newest_fixture_blueprint_round_trips_byte_for_byte() {
    let (name, text) = fixtures()
        .into_iter()
        .max_by_key(|(name, _)| fixture_version(name))
        .unwrap();

    let loaded = blueprint::parse(&text).unwrap();

    assert_eq!(
        blueprint::to_json(&loaded.rule_sets, &loaded.image).unwrap(),
        text,
        "{name}"
    );
}

#[test]
fn a_blueprint_with_no_rule_sets_is_refused() {
    let json = r#"{"version": 2, "image": "grass_main", "active": 0, "rule_sets": []}"#;
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
fn an_unknown_neighbor_letter_is_refused() {
    let json = with_neighbors("EFAXEEEE");
    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NeighborLetter('X')
    ));
}

#[test]
fn a_chance_outside_its_range_is_refused() {
    let json = one_tile(&tile_of(
        5,
        r#"{"neighbors": "EEEEEEEE", "chance": 0.0, "orientations": ["N"]}"#,
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
        r#"{"name": "bones", "tiles": [64],
            "neighbors": "AAAAAAAAA", "chance": 100.0}"#,
    );
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn group_tiles_must_ascend_without_repeats() {
    for tiles in ["64, 64, 65", "65, 64"] {
        let json = document(
            "",
            "",
            &format!(
                r#"{{"name": "bones", "tiles": [{tiles}],
                    "neighbors": "AA", "chance": 100.0}}"#
            ),
        );
        assert!(matches!(
            load(&json).unwrap_err(),
            BlueprintError::GroupTiles(_)
        ));
    }
}

#[test]
fn two_groups_sharing_a_name_are_refused() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [64, 65, 80, 81],
            "neighbors": "AAAAAAAAAAAAAAAA", "chance": 100.0},
           {"name": "bones", "tiles": [100, 101, 116, 117],
            "neighbors": "AAAAAAAAAAAAAAAA", "chance": 100.0}"#,
    );
    assert!(matches!(load(&json).unwrap_err(), BlueprintError::Group(_)));
}

#[test]
fn a_group_over_a_configured_tile_is_refused() {
    let json = document(
        &plain_tile(65),
        "",
        r#"{"name": "bones", "tiles": [64, 65, 80, 81],
            "neighbors": "AAAAAAAAAAAAAAAA", "chance": 100.0}"#,
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
    assert!(text.contains(r#""neighbors": "AEAEFAFF""#), "{text}");
}

#[test]
fn a_neighborhood_is_read_back_in_index_order() {
    let project = load(&with_neighbors("AEAEFAFF")).unwrap();

    assert_eq!(project.rules_of(5)[0].neighborhood, outer_corner());
}

fn with_neighbors(neighbors: &str) -> String {
    one_tile(&tile_of(
        5,
        &format!(r#"{{"neighbors": "{neighbors}", "chance": 100.0, "orientations": ["N"]}}"#),
    ))
}

#[test]
fn a_tile_requirement_survives_a_round_trip() {
    let mut neighborhood = outer_corner();
    neighborhood.set_state(offset_of("topLeft"), NeighborState::Is(17));
    let mut project = Project::default();
    project.add_rule(5, TileRule::new(neighborhood));

    let text = written(project.clone());

    assert!(text.contains(r#""neighbors": "[17]"#), "{text}");
    assert_eq!(round_trip(&project), project);
}

#[test]
fn a_tile_requirement_loads_from_brackets() {
    let project = load(&with_neighbors("[17]EAEFAFF")).unwrap();
    let neighborhood = &project.rules_of(5)[0].neighborhood;

    assert_eq!(
        neighborhood.state(offset_of("topLeft")),
        NeighborState::Is(17)
    );
    assert_eq!(neighborhood.state(offset_of("right")), NeighborState::Full);
}

#[test]
fn the_air_tile_as_a_neighbor_tile_is_refused() {
    let json = with_neighbors("[0]EEEEEEE");

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::AirNeighborTile
    ));
}

#[test]
fn a_neighbor_tile_outside_the_tileset_is_refused() {
    let json = with_neighbors("[256]EEEEEEE");

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::TileId(256)
    ));
}

#[test]
fn a_malformed_bracket_is_refused() {
    for neighbors in [
        "[17EEEEEEE",
        "EEEEEEE[17",
        "[]EEEEEEE",
        "[x]EEEEEEE",
        "[+17]EEEEEEE",
    ] {
        assert!(
            matches!(
                load(&with_neighbors(neighbors)).unwrap_err(),
                BlueprintError::NeighborTileText(_)
            ),
            "{neighbors}"
        );
    }
}

#[test]
fn a_neighbor_list_of_the_wrong_length_is_refused() {
    for neighbors in ["AEAEFAF", "AEAEFAFFF"] {
        assert!(
            matches!(
                load(&with_neighbors(neighbors)).unwrap_err(),
                BlueprintError::NeighborhoodShape(5)
            ),
            "{neighbors}"
        );
    }
}

fn with_extent(neighbors: &str, extent: &str) -> String {
    one_tile(&tile_of(
        5,
        &format!(
            r#"{{"neighbors": "{neighbors}", "extent": {extent}, "chance": 100.0, "orientations": ["N"]}}"#
        ),
    ))
}

#[test]
fn a_non_square_neighborhood_survives_a_round_trip() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any)
        .grown(Side::Left)
        .grown(Side::Left)
        .grown(Side::Down);
    neighborhood.set_state((-2, 1), NeighborState::Full);
    neighborhood.set_state((0, 2), NeighborState::Empty);
    neighborhood.set_state((-3, 0), NeighborState::Is(17));
    let mut project = Project::default();
    project.add_rule(5, TileRule::new(neighborhood));

    let text = written(project.clone());

    assert!(text.contains(r#""extent""#), "{text}");
    assert_eq!(round_trip(&project), project);
}

#[test]
fn a_three_by_three_neighborhood_writes_no_extent() {
    let mut project = Project::default();
    project.add_rule(5, TileRule::new(Neighborhood::uniform(NeighborState::Any)));

    assert!(!written(project).contains("extent"));
}

#[test]
fn a_two_by_two_fill_group_writes_its_full_ring_without_an_extent() {
    let mut project = Project::default();
    project.append_group(group("bones", 64, GroupPreset::Fill, 100.0));

    let text = written(project);

    assert!(
        text.contains(r#""neighbors": "AAAAAFFAAFFAAAAA""#),
        "{text}"
    );
    assert!(!text.contains("extent"), "{text}");
}

#[test]
fn a_two_by_two_group_loads_and_saves_back_identically() {
    const NEIGHBORS: &str = "EFAAAFEAAFFAEAAE";
    let json = document(
        "",
        "",
        &format!(
            r#"{{"name": "bones", "tiles": [5, 6, 21, 22],
                "neighbors": "{NEIGHBORS}", "chance": 100.0}}"#
        ),
    );

    let project = load(&json).unwrap();

    assert_eq!(round_trip(&project), project);
    assert!(written(project).contains(&format!(r#""neighbors": "{NEIGHBORS}""#)));
}

#[test]
fn a_bad_group_neighbor_letter_names_the_group() {
    let json = document(
        "",
        "",
        r#"{"name": "bones", "tiles": [64, 65, 80, 81],
            "neighbors": "AAAX", "chance": 100.0}"#,
    );

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::GroupNeighborText { group, .. } if group == "bones"
    ));
}

#[test]
fn neighbors_matching_the_extent_are_loaded() {
    let json = with_extent("AAAAAAAAAAA", "[2,1,1,1]");
    let project = load(&json).unwrap();

    assert_eq!(project.rules_of(5)[0].neighborhood.extent().left, 2);
}

#[test]
fn neighbors_not_matching_the_extent_are_refused() {
    let json = with_extent("AAAAAAAA", "[2,1,1,1]");

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NeighborhoodShape(5)
    ));
}

#[test]
fn an_extent_below_one_is_refused() {
    let json = with_extent("AAAAA", "[0,1,1,1]");

    assert!(matches!(
        load(&json).unwrap_err(),
        BlueprintError::NeighborhoodShape(5)
    ));
}

const RULES_TEXT: &str = "# credit\n[Test]\nIndex 1\n";
const BEGIN_MARKER: &str = "# ddnet-automap-creator blueprint begin";
const END_MARKER: &str = "# ddnet-automap-creator blueprint end";

fn furnished_sets() -> RuleSets {
    RuleSets::from_parts(vec![("Grass Main".to_owned(), furnished())], 0)
}

fn embedded_rules() -> String {
    let json = blueprint::to_json(&furnished_sets(), IMAGE).unwrap();
    blueprint::embed(RULES_TEXT, &json)
}

#[test]
fn an_embedded_blueprint_survives_a_round_trip() {
    let loaded = blueprint::parse_embedded(&embedded_rules()).unwrap();

    assert_eq!(loaded.rule_sets.sets(), furnished_sets().sets());
    assert_eq!(loaded.image, IMAGE);
}

#[test]
fn everything_after_the_rules_is_a_comment() {
    let embedded = embedded_rules();
    let appended = embedded.strip_prefix(RULES_TEXT).unwrap();

    assert!(appended.ends_with(&format!("{END_MARKER}\n")));
    assert!(appended.lines().all(|line| line.starts_with('#')));
}

#[test]
fn embedding_keeps_exactly_one_newline_before_the_block() {
    let json = blueprint::to_json(&furnished_sets(), IMAGE).unwrap();
    let embedded = blueprint::embed("rules\n\n\n", &json);

    assert!(embedded.starts_with(&format!("rules\n{BEGIN_MARKER}\n")));
}

#[test]
fn an_embedded_blueprint_loads_from_a_crlf_file() {
    let crlf = embedded_rules().replace('\n', "\r\n");

    let loaded = blueprint::parse_embedded(&crlf).unwrap();

    assert_eq!(loaded.rule_sets.sets(), furnished_sets().sets());
}

#[test]
fn rules_without_a_block_have_no_embedded_blueprint() {
    assert!(matches!(
        blueprint::parse_embedded(RULES_TEXT),
        Err(BlueprintError::NoEmbedded)
    ));
}

#[test]
fn a_block_without_an_end_is_incomplete() {
    let truncated = embedded_rules().replace(END_MARKER, "");

    assert!(matches!(
        blueprint::parse_embedded(&truncated),
        Err(BlueprintError::IncompleteEmbedded)
    ));
}
