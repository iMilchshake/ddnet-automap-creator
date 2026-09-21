use crate::export::r_source::{ExportError, RuleSet, render};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::tile::{Chance, TileRule};
use crate::tests::support::{index_of, mods, outer_corner};

fn rule(neighborhood: Neighborhood, percent: f32, can_rotate: bool) -> TileRule {
    TileRule {
        neighborhood,
        mods: mods(false, false, can_rotate),
        chance: Chance::new(percent).unwrap(),
    }
}

fn emit(tiles: &[(usize, TileRule)]) -> String {
    emit_with_groups(tiles, &[]).unwrap()
}

fn emit_with_groups(
    tiles: &[(usize, TileRule)],
    groups: &[TileGroup],
) -> Result<String, ExportError> {
    render(&RuleSet {
        image_stem: "grass_main",
        name: "Grass Main",
        tiles,
        groups,
    })
}

fn body(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| line.starts_with("Insert("))
        .collect()
}

#[test]
fn a_rotated_corner_and_a_weighted_pool_match_the_reference_output() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (32, rule(outer_corner(), 100.0, true)),
        (1, rule(surrounded, 100.0, false)),
        (2, rule(surrounded, 5.0, false)),
        (3, rule(surrounded, 1.0, false)),
        (66, rule(surrounded, 1.0, false)),
        (67, rule(surrounded, 1.0, false)),
    ];

    let source = emit(&tiles);

    assert_eq!(
        body(&source),
        [
            "Insert(32).If(IndexAt([0, 0]).IsFullAt(right, bottom, bottomRight).IsEmptyAt(top, left));",
            "Insert(32.R).If(IndexAt([0, 0]).IsFullAt(left, bottomLeft, bottom).IsEmptyAt(top, right));",
            "Insert(32.VH).If(IndexAt([0, 0]).IsFullAt(topLeft, top, left).IsEmptyAt(right, bottom));",
            "Insert(32.VHR).If(IndexAt([0, 0]).IsFullAt(top, topRight, right).IsEmptyAt(left, bottom));",
            "Insert(1, 2, 3, 66, 67).Chance(100, 5, 1, 1, 1).If(IndexAt([0, 0]).IsFullAt(topLeft, top, topRight, left, right, bottomLeft, bottom, bottomRight));",
        ]
    );
}

#[test]
fn the_header_names_the_image_and_the_rule_set() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let source = emit(&tiles);

    assert!(source.starts_with(
        "// created with ddnet-automap-creator (https://github.com/iMilchshake/ddnet-automap-creator)\n"
    ));
    assert!(source.contains("#include \"base.r\""));
    assert!(source.contains("#output \"grass_main.rules\""));
    assert!(source.contains("AutoMapper(\"Grass Main\");"));
    assert!(source.contains("NewRun();"));
    assert!(source.ends_with('\n'));
}

#[test]
fn an_all_any_neighborhood_drops_the_test() {
    let tiles = [(
        5,
        rule(Neighborhood::uniform(NeighborState::Any), 100.0, false),
    )];
    let source = emit(&tiles);

    assert_eq!(body(&source), ["Insert(5);"]);
}

#[test]
fn a_pool_at_full_chance_rolls_instead_of_listing_chances() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (1, rule(surrounded, 100.0, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = emit(&tiles);

    assert!(body(&source)[0].starts_with("Insert(1, 2).Roll()"));
}

#[test]
fn fractional_chances_keep_their_shortest_form() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (1, rule(surrounded, 2.5, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = emit(&tiles);

    assert!(body(&source)[0].contains(".Chance(2.5, 100)"));
}

#[test]
fn less_specific_rules_come_first() {
    let mut loose = Neighborhood::uniform(NeighborState::Any);
    loose.set_state(index_of("top"), NeighborState::Full);

    let tiles = [
        (
            9,
            rule(Neighborhood::uniform(NeighborState::Full), 100.0, false),
        ),
        (4, rule(loose, 100.0, false)),
    ];
    let source = emit(&tiles);

    let lines = body(&source);
    assert!(lines[0].starts_with("Insert(4)"));
    assert!(lines[1].starts_with("Insert(9)"));
}

#[test]
fn a_symmetric_neighborhood_keeps_only_the_first_of_its_duplicates() {
    let tiles = [(
        7,
        rule(Neighborhood::uniform(NeighborState::Full), 100.0, true),
    )];
    let source = emit(&tiles);

    assert_eq!(body(&source).len(), 1);
    assert!(body(&source)[0].starts_with("Insert(7)."));
}

#[test]
fn an_empty_rule_set_is_refused() {
    let error = emit_with_groups(&[], &[]).unwrap_err();

    assert!(matches!(error, ExportError::NothingConfigured));
}

fn tile_group(name: &str, top_left: usize, mode: GroupMode, percent: f32) -> TileGroup {
    TileGroup {
        name: name.to_owned(),
        top_left,
        width: 2,
        height: 2,
        mode,
        chance: Chance::new(percent).unwrap(),
    }
}

#[test]
fn groups_become_objects_declared_before_the_automapper() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let groups = [
        tile_group("bones", 64, GroupMode::Fill, 100.0),
        tile_group("pipes", 100, GroupMode::Fill, 100.0),
    ];

    let source = emit_with_groups(&tiles, &groups).unwrap();
    let lines: Vec<&str> = source.lines().collect();

    let objects = lines
        .iter()
        .position(|line| line.starts_with("object "))
        .unwrap();
    let automapper = lines
        .iter()
        .position(|line| line.starts_with("AutoMapper("))
        .unwrap();

    assert!(objects < automapper);
    assert_eq!(lines[objects], "object o:bones = Rect(64, 81);");
    assert_eq!(lines[objects + 1], "object o:pipes = Rect(100, 117);");
}

#[test]
fn a_decorate_group_also_tests_the_ring_around_its_footprint() {
    let groups = [tile_group("bones", 64, GroupMode::Decorate, 1.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).Chance(1).If(\n\
         \x20   Object().HasSpace(),\n\
         \x20   Object().IsNotOverlapping(o:bones),\n\
         \x20   IndexAt([-1, -1], [0, -1], [1, -1], [2, -1], [-1, 0], [2, 0], \
         [-1, 1], [2, 1], [-1, 2], [0, 2], [1, 2], [2, 2]).IsFull()\n\
         );"
        ),
        "{source}"
    );
}

#[test]
fn a_fill_group_at_full_chance_tests_nothing_but_space_and_overlap() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).If(\n\
         \x20   Object().HasSpace(),\n\
         \x20   Object().IsNotOverlapping(o:bones)\n\
         );"
        ),
        "{source}"
    );
    assert!(!source.contains("IsFull()"));
}

#[test]
fn every_group_is_told_not_to_overlap_any_group_including_itself() {
    let groups = [
        tile_group("bones", 64, GroupMode::Fill, 100.0),
        tile_group("pipes", 100, GroupMode::Fill, 100.0),
    ];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert_eq!(
        source.matches("IsNotOverlapping(o:bones, o:pipes)").count(),
        2
    );
}

#[test]
fn groups_add_a_closing_fill_objects_run() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(source.ends_with("NewRun();\nOverrideLayer();\nRun().FillObjects();\n"));
}

#[test]
fn a_rule_set_without_groups_never_mentions_objects() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let source = emit(&tiles);

    assert!(!source.contains("object "));
    assert!(!source.contains("InsertObject"));
    assert!(!source.contains("FillObjects"));
}

#[test]
fn a_tile_cannot_be_configured_and_grouped_at_once() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];

    for tile in [64, 65, 80, 81] {
        let tiles = [(tile, rule(Neighborhood::default(), 100.0, false))];
        let error = emit_with_groups(&tiles, &groups).unwrap_err();
        assert!(
            matches!(error, ExportError::TileIsConfigured { .. }),
            "tile {tile}"
        );
    }
}

#[test]
fn an_invalid_group_stops_the_export() {
    let groups = [tile_group("big_bones", 64, GroupMode::Fill, 100.0)];

    let error = emit_with_groups(&[], &groups).unwrap_err();
    assert!(matches!(error, ExportError::Group(_)));
}

#[test]
fn groups_alone_are_enough_to_export() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    assert!(emit_with_groups(&[], &groups).is_ok());
}
