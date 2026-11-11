use crate::export::r_source::{ExportError, RuleSet, last_wins_odds, render, render_rule_sets};
use crate::model::group::{CellState, GroupPreset, TileGroup};
use crate::model::neighbor::{Extent, NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Centre, Chance, TILE_COUNT, TileMods, TileRule};
use crate::tests::support::{HALF_TURN, mods, offset_of, outer_corner};

fn rule(neighborhood: Neighborhood, percent: f32, can_rotate: bool) -> TileRule {
    TileRule {
        mods: mods(false, false, can_rotate),
        chance: Chance::new(percent).unwrap(),
        ..TileRule::new(neighborhood)
    }
}

fn emit(tiles: &[(usize, TileRule)]) -> String {
    emit_with_groups(tiles, &[]).unwrap()
}

fn emit_with_groups(
    tiles: &[(usize, TileRule)],
    groups: &[TileGroup],
) -> Result<String, ExportError> {
    render(&[RuleSet {
        image_stem: "grass_main",
        name: "Grass_Main",
        tiles,
        deactivated: &[],
        groups,
        chance_mode: ChanceMode::Normalize,
    }])
}

fn emit_exact(tiles: &[(usize, TileRule)]) -> String {
    render(&[RuleSet {
        image_stem: "grass_main",
        name: "Grass_Main",
        tiles,
        deactivated: &[],
        groups: &[],
        chance_mode: ChanceMode::Exact,
    }])
    .unwrap()
}

fn anywhere(percent: f32) -> TileRule {
    rule(Neighborhood::uniform(NeighborState::Any), percent, false)
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
        (1, rule(surrounded.clone(), 100.0, false)),
        (2, rule(surrounded.clone(), 5.0, false)),
        (3, rule(surrounded.clone(), 1.0, false)),
        (66, rule(surrounded.clone(), 1.0, false)),
        (67, rule(surrounded, 1.0, false)),
    ];

    let source = emit(&tiles);

    assert!(source.contains("AutoMapper(\"Grass_Main\");\ng:mask = 255;\nNewRun();"));
    assert_eq!(
        body(&source),
        [
            "Insert(32).If(IndexAt([0, 0]).IsFullAt(right, bottom, bottomRight).IsEmptyAt(top, left));",
            "Insert(32.R).If(IndexAt([0, 0]).IsFullAt(left, bottomLeft, bottom).IsEmptyAt(top, right));",
            "Insert(32.VH).If(IndexAt([0, 0]).IsFullAt(topLeft, top, left).IsEmptyAt(right, bottom));",
            "Insert(32.VHR).If(IndexAt([0, 0]).IsFullAt(top, topRight, right).IsEmptyAt(left, bottom));",
            "Insert(g:mask).If(IndexAt([0, 0]).IsFullAt(topLeft, top, topRight, left, right, bottomLeft, bottom, bottomRight));",
            "Insert(1, 2, 3, 66, 67).Chance(100, 5, 1, 1, 1).If(IndexAt([0, 0]).Is(g:mask));",
        ]
    );
}

#[test]
fn a_rule_drawn_as_a_half_turn_emits_that_turn_with_the_drawn_neighborhood() {
    let drawn_rule = TileRule {
        drawn: HALF_TURN,
        ..TileRule::new(outer_corner())
    };

    assert_eq!(
        body(&emit(&[(32, drawn_rule)])),
        [
            "Insert(32.VH).If(IndexAt([0, 0]).IsFullAt(right, bottom, bottomRight).IsEmptyAt(top, left));"
        ]
    );
}

#[test]
fn exporting_a_subset_emits_only_the_chosen_rule_sets() {
    let named_set = |name: &str, tile: usize| {
        let mut project = Project::default();
        project.add_rule(tile, anywhere(100.0));
        (name.to_owned(), project)
    };
    let rule_sets = RuleSets::from_parts(
        vec![
            named_set("first", 1),
            named_set("second", 2),
            named_set("third", 3),
        ],
        0,
    );

    let source = render_rule_sets(&rule_sets.selected(&[true, false, true]), "grass_main").unwrap();

    assert!(source.contains("AutoMapper(\"first\");"));
    assert!(!source.contains("AutoMapper(\"second\");"));
    assert!(source.contains("AutoMapper(\"third\");"));
}

fn emit_deactivated(
    tiles: &[(usize, TileRule)],
    deactivated: &[usize],
    groups: &[TileGroup],
) -> Result<String, ExportError> {
    render(&[RuleSet {
        image_stem: "grass_main",
        name: "Grass_Main",
        tiles,
        deactivated,
        groups,
        chance_mode: ChanceMode::Normalize,
    }])
}

#[test]
fn a_deactivated_rule_is_left_out_of_the_output() {
    let tiles = [(1, anywhere(100.0)), (2, anywhere(100.0))];

    let source = emit_deactivated(&tiles, &[1], &[]).unwrap();

    assert_eq!(body(&source), body(&emit(&tiles[1..])));
}

#[test]
fn a_rule_set_with_only_deactivated_rules_is_empty() {
    let tiles = [(1, anywhere(100.0))];

    assert!(matches!(
        emit_deactivated(&tiles, &[1], &[]),
        Err(ExportError::EmptyRuleSet(_))
    ));
}

#[test]
fn a_deactivated_rule_under_a_group_still_stops_the_export() {
    let group = tile_group("bones", 64, GroupPreset::Fill, 100.0);
    let tiles = [(65, anywhere(100.0)), (1, anywhere(100.0))];

    assert!(matches!(
        emit_deactivated(&tiles, &[65], &[group]),
        Err(ExportError::TileIsConfigured { tile: 65, .. })
    ));
}

#[test]
fn the_header_names_the_image_and_the_rule_set() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let source = emit(&tiles);

    assert!(source.starts_with(&format!(
        "// created with ddnet-automap-creator v{} (https://github.com/iMilchshake/ddnet-automap-creator)\n",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(source.contains("#include \"base.r\""));
    assert!(source.contains("#output \"grass_main.rules\""));
    assert!(source.contains("AutoMapper(\"Grass_Main\");"));
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
        (1, rule(surrounded.clone(), 100.0, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = emit(&tiles);

    assert_eq!(
        body(&source)[1],
        "Insert(1, 2).Roll().If(IndexAt([0, 0]).Is(g:mask));"
    );
}

#[test]
fn fractional_chances_keep_their_shortest_form() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (1, rule(surrounded.clone(), 2.5, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = emit(&tiles);

    assert!(body(&source)[1].contains(".Chance(2.5, 100)"));
}

#[test]
fn a_lone_tile_ignores_its_chance_when_normalized() {
    assert_eq!(body(&emit(&[(5, anywhere(30.0))])), ["Insert(5);"]);
}

#[test]
fn a_lone_tile_keeps_its_chance_when_exact() {
    assert_eq!(
        body(&emit_exact(&[(5, anywhere(30.0))])),
        ["Insert(5).Chance(30);"]
    );
}

#[test]
fn a_pool_rolls_on_a_mask_it_places_first() {
    let tiles = [(5, anywhere(30.0)), (6, anywhere(30.0))];

    assert_eq!(
        body(&emit(&tiles)),
        [
            "Insert(g:mask);",
            "Insert(5, 6).Roll().If(IndexAt([0, 0]).Is(g:mask));",
        ]
    );
}

#[test]
fn an_exact_pool_below_100_masks_only_its_total() {
    let tiles = [(5, anywhere(30.0)), (6, anywhere(30.0))];

    assert_eq!(body(&emit_exact(&tiles))[0], "Insert(g:mask).Chance(60);");
}

#[test]
fn pool_chances_below_100_are_scaled_so_every_mask_is_replaced() {
    let tiles = [(5, anywhere(10.0)), (6, anywhere(30.0))];

    for source in [emit(&tiles), emit_exact(&tiles)] {
        assert_eq!(
            body(&source)[1],
            "Insert(5, 6).Chance(25, 75).If(IndexAt([0, 0]).Is(g:mask));"
        );
    }
}

#[test]
fn a_pool_declares_the_highest_free_tile_as_mask() {
    let tiles = [(5, anywhere(30.0)), (6, anywhere(30.0))];

    assert!(emit(&tiles).contains("AutoMapper(\"Grass_Main\");\ng:mask = 255;\nNewRun();"));
}

#[test]
fn the_mask_moves_below_a_rule_on_tile_255() {
    let tiles = [
        (5, anywhere(30.0)),
        (6, anywhere(30.0)),
        (255, rule(outer_corner(), 100.0, false)),
    ];

    assert!(emit(&tiles).contains("g:mask = 254;"));
}

#[test]
fn the_mask_moves_below_a_group_on_tile_255() {
    let tiles = [(5, anywhere(30.0)), (6, anywhere(30.0))];
    let groups = [tile_group("bones", 238, GroupPreset::Fill, 100.0)];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert!(source.contains("g:mask = 253;"));
}

#[test]
fn a_pool_without_any_free_tile_is_refused() {
    let tiles: Vec<(usize, TileRule)> = (1..TILE_COUNT)
        .map(|tile| (tile, anywhere(100.0)))
        .collect();

    let error = emit_with_groups(&tiles, &[]).unwrap_err();

    assert!(matches!(error, ExportError::NoMaskTile(name) if name == "Grass_Main"));
}

#[test]
fn no_free_tile_is_fine_while_no_pool_needs_a_mask() {
    let tiles: Vec<(usize, TileRule)> = (1..TILE_COUNT)
        .map(|tile| (tile, anywhere(100.0)))
        .collect();
    let deactivated: Vec<usize> = (2..TILE_COUNT).collect();

    let source = emit_deactivated(&tiles, &deactivated, &[]).unwrap();

    assert!(!source.contains("g:mask"));
}

#[test]
fn single_tile_pools_declare_no_mask() {
    assert!(!emit(&[(5, anywhere(100.0))]).contains("g:mask"));
}

#[test]
fn a_rule_on_the_air_tile_is_refused() {
    let error = emit_with_groups(&[(AIR_TILE, anywhere(100.0))], &[]).unwrap_err();

    assert!(matches!(error, ExportError::AirTile));
}

#[test]
fn less_specific_rules_come_first() {
    let mut loose = Neighborhood::uniform(NeighborState::Any);
    loose.set_state(offset_of("top"), NeighborState::Full);

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

fn empty_at(names: &[&str]) -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    for name in names {
        neighborhood.set_state(offset_of(name), NeighborState::Empty);
    }

    neighborhood
}

#[test]
fn several_rules_of_one_tile_are_emitted_by_specificity() {
    let tiles = [
        (5, rule(empty_at(&["top", "bottom"]), 100.0, false)),
        (5, rule(empty_at(&["bottom"]), 100.0, false)),
    ];

    assert_eq!(
        body(&emit(&tiles)),
        [
            "Insert(5).If(IndexAt([0, 0]).IsEmptyAt(bottom));",
            "Insert(5).If(IndexAt([0, 0]).IsEmptyAt(top, bottom));",
        ]
    );
}

#[test]
fn a_symmetric_neighborhood_rolls_all_its_orientations() {
    let tiles = [(
        7,
        rule(Neighborhood::uniform(NeighborState::Full), 100.0, true),
    )];
    let source = emit(&tiles);
    let lines = body(&source);

    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("Insert(g:mask)"));
    assert!(lines[1].starts_with("Insert(7, 7.R"));
    assert!(lines[1].contains(".Roll()"));
}

#[test]
fn a_rule_set_name_outside_the_allowed_characters_is_refused() {
    let tiles = [(7, rule(outer_corner(), 100.0, false))];

    for name in ["", "grass main", "grass\"main", &"g".repeat(129)] {
        let error = render(&[RuleSet {
            image_stem: "grass_main",
            name,
            tiles: &tiles,
            deactivated: &[],
            groups: &[],
            chance_mode: ChanceMode::Normalize,
        }])
        .unwrap_err();

        assert!(matches!(error, ExportError::Name(_)), "accepted `{name}`");
    }
}

#[test]
fn an_empty_rule_set_is_refused() {
    let error = emit_with_groups(&[], &[]).unwrap_err();

    assert!(matches!(error, ExportError::EmptyRuleSet(name) if name == "Grass_Main"));
}

fn tile_group(name: &str, top_left: usize, preset: GroupPreset, percent: f32) -> TileGroup {
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

fn reshaped(mut group: TileGroup, tiles: &[usize], preset: GroupPreset) -> TileGroup {
    group.set_tiles(tiles.iter().copied().collect());
    group.apply_preset(preset);

    group
}

#[test]
fn every_copy_of_a_group_is_inserted_and_rolls_among_the_spots_left_over() {
    let rotating = TileGroup {
        mods: TileMods::new(false, false, true),
        ..tile_group("bones", 64, GroupPreset::Fill, 100.0)
    };

    let source = emit_with_groups(&[], &[rotating]).unwrap();

    assert!(source.contains("InsertObject(o:bones).Chance(25).If("));
    assert!(source.contains(&format!(
        "InsertObject(o:bones.R).Chance({}).If(",
        100.0_f32 / 3.0
    )));
    assert!(source.contains("InsertObject(o:bones.VH).Chance(50).If("));
    assert!(source.contains("InsertObject(o:bones.VHR).If("));
    assert!(source.contains("IsNotOverlapping(o:bones, o:bones.R, o:bones.VH, o:bones.VHR)"));
}

#[test]
fn a_decorating_copy_tests_the_ring_of_its_own_shape() {
    let rotating = TileGroup {
        mods: TileMods::new(false, false, true),
        ..reshaped(
            tile_group("bones", 64, GroupPreset::Fill, 100.0),
            &[65, 81, 82],
            GroupPreset::Decorate,
        )
    };

    let source = emit_with_groups(&[], &[rotating]).unwrap();
    let rings: Vec<&str> = source
        .lines()
        .filter(|line| line.contains("IndexAt("))
        .collect();

    assert_eq!(rings.len(), 4);
    assert_ne!(rings[0], rings[1]);
}

#[test]
fn groups_become_objects_declared_before_the_automapper() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let groups = [
        tile_group("bones", 64, GroupPreset::Fill, 100.0),
        tile_group("pipes", 100, GroupPreset::Fill, 100.0),
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
fn a_group_that_is_not_a_rectangle_is_declared_through_indices() {
    let group = reshaped(
        tile_group("bones", 64, GroupPreset::Fill, 100.0),
        &[65, 66, 81, 97, 98, 99],
        GroupPreset::Fill,
    );
    let source = emit_with_groups(&[], &[group]).unwrap();

    assert!(
        source.contains("object o:bones = Indices(65_66, 81, 97_99);\n"),
        "{source}"
    );
}

#[test]
fn a_run_of_indices_does_not_wrap_across_tileset_rows() {
    let group = reshaped(
        tile_group("bones", 64, GroupPreset::Fill, 100.0),
        &[15, 16, 40],
        GroupPreset::Fill,
    );
    let source = emit_with_groups(&[], &[group]).unwrap();

    assert!(
        source.contains("object o:bones = Indices(15, 16, 40);\n"),
        "{source}"
    );
}

#[test]
fn a_decorate_group_also_tests_the_ring_around_its_footprint() {
    let groups = [tile_group("bones", 64, GroupPreset::Decorate, 1.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).Chance(1).If(\n\
         \x20   Object().IsNotOverlapping(o:bones),\n\
         \x20   Object().HasSpace(),\n\
         \x20   IndexAt([-1, -1], [0, -1], [1, -1], [2, -1], \
         [-1, 0], [2, 0], \
         [-1, 1], [2, 1], \
         [-1, 2], [0, 2], [1, 2], [2, 2]).IsFull()\n\
         );"
        ),
        "{source}"
    );
}

#[test]
fn a_fill_group_tests_its_own_tiles_for_solid_and_overlap() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).If(\n\
         \x20   Object().IsNotOverlapping(o:bones),\n\
         \x20   Object().HasSpace()\n\
         );"
        ),
        "{source}"
    );
    assert!(!source.contains("IsFull"));
    assert!(!source.contains("IsEmpty"));
}

#[test]
fn a_group_with_a_mixed_footprint_lists_its_own_cells_explicitly() {
    let mut group = tile_group("bones", 64, GroupPreset::Fill, 100.0);
    group.neighbors.insert((1, 5), CellState::Any);
    let source = emit_with_groups(&[], &[group]).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).If(\n\
         \x20   Object().IsNotOverlapping(o:bones),\n\
         \x20   IndexAt([0, 0], [1, 0], [0, 1]).IsFull()\n\
         );"
        ),
        "{source}"
    );
    assert!(!source.contains("HasSpace"));
}

#[test]
fn an_on_floor_group_tests_air_inside_ground_below_and_skips_the_default_rule() {
    let groups = [tile_group("bones", 64, GroupPreset::OnFloor, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains(
            "InsertObject(o:bones).If(\n\
         \x20   Object().IsNotOverlapping(o:bones),\n\
         \x20   IndexAt([0, 2], [1, 2]).IsFull(),\n\
         \x20   IndexAt([0, 0], [1, 0], [0, 1], [1, 1]).IsEmpty()\n\
         ).NoDefaultPosRule();"
        ),
        "{source}"
    );
}

#[test]
fn every_group_is_told_not_to_overlap_any_group_including_itself() {
    let groups = [
        tile_group("bones", 64, GroupPreset::Fill, 100.0),
        tile_group("pipes", 100, GroupPreset::Fill, 100.0),
    ];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert_eq!(
        source.matches("IsNotOverlapping(o:bones, o:pipes)").count(),
        2
    );
}

#[test]
fn groups_alone_start_their_run_right_after_the_automapper_line() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains("AutoMapper(\"Grass_Main\");\nNewRun();\nOverrideLayer();\nInsertObject(")
    );
}

#[test]
fn runs_are_separated_by_one_blank_line() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let tiles = [
        (1, anywhere(100.0)),
        (2, rule(tile_specific(), 100.0, false)),
    ];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert_eq!(source.matches("\n\nNewRun();\n").count(), 2);
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
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];

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
    let groups = [tile_group("big_bones", 64, GroupPreset::Fill, 100.0)];

    let error = emit_with_groups(&[], &groups).unwrap_err();
    assert!(matches!(error, ExportError::Group(_)));
}

#[test]
fn groups_alone_are_enough_to_export() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    assert!(emit_with_groups(&[], &groups).is_ok());
}

fn rule_set<'a>(
    name: &'a str,
    tiles: &'a [(usize, TileRule)],
    groups: &'a [TileGroup],
) -> RuleSet<'a> {
    RuleSet {
        image_stem: "grass_main",
        name,
        tiles,
        deactivated: &[],
        groups,
        chance_mode: ChanceMode::Normalize,
    }
}

#[test]
fn several_rule_sets_share_one_output_and_header() {
    let tiles_a = [(1, rule(Neighborhood::default(), 100.0, false))];
    let tiles_b = [(2, rule(Neighborhood::default(), 100.0, false))];

    let source = render(&[
        rule_set("SetA", &tiles_a, &[]),
        rule_set("SetB", &tiles_b, &[]),
    ])
    .unwrap();

    assert_eq!(source.matches("#output").count(), 1);
    assert!(source.contains("AutoMapper(\"SetA\");"));
    assert!(source.contains("AutoMapper(\"SetB\");"));
    assert!(source.find("AutoMapper(\"SetA\")") < source.find("AutoMapper(\"SetB\")"));
}

#[test]
fn several_rule_sets_namespace_same_named_groups() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let groups_a = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let groups_b = [tile_group("bones", 100, GroupPreset::Fill, 100.0)];

    let source = render(&[
        rule_set("SetA", &tiles, &groups_a),
        rule_set("SetB", &tiles, &groups_b),
    ])
    .unwrap();

    assert!(source.contains("object o:0bones = Rect(64, 81);"));
    assert!(source.contains("object o:1bones = Rect(100, 117);"));
    assert!(!source.contains("object o:bones"));
}

#[test]
fn a_single_rule_set_keeps_unnamespaced_object_names() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];

    let source = render(&[rule_set("SetA", &tiles, &groups)]).unwrap();

    assert!(source.contains("object o:bones = Rect(64, 81);"));
}

const SPECIFIC_TESTS: &str =
    ".If(IndexAt([0, 0]).IsEmptyAt(bottom), IndexAt([0, -1]).Is(17), IndexAt([1, 0]).Is(18))";

fn tile_specific() -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("top"), NeighborState::Is(17));
    neighborhood.set_state(offset_of("right"), NeighborState::Is(18));
    neighborhood.set_state(offset_of("bottom"), NeighborState::Empty);

    neighborhood
}

#[test]
fn tile_specific_pools_get_their_own_copy_run_after_the_plain_inserts() {
    let tiles = [
        (1, anywhere(100.0)),
        (2, rule(tile_specific(), 100.0, true)),
        (3, rule(tile_specific(), 100.0, false)),
    ];

    let source = emit(&tiles);

    assert_eq!(
        body(&source),
        [
            "Insert(1);",
            &format!("Insert(2){SPECIFIC_TESTS};"),
            &format!("Insert(3){SPECIFIC_TESTS}; util:Chance(2);"),
            "Insert(2.R).If(IndexAt([0, 0]).IsEmptyAt(left), IndexAt([1, 0]).Is(17), IndexAt([0, 1]).Is(18));",
            "Insert(2.VH).If(IndexAt([0, 0]).IsEmptyAt(top), IndexAt([-1, 0]).Is(18), IndexAt([0, 1]).Is(17));",
            "Insert(2.VHR).If(IndexAt([0, 0]).IsEmptyAt(right), IndexAt([0, -1]).Is(18), IndexAt([-1, 0]).Is(17));",
        ]
    );
    assert!(!source.contains("g:mask"));
    assert!(!source.contains("OverrideLayer"));
    assert!(!source.contains("util:NewRule"));
    assert!(
        source.ends_with(&format!(
            "Insert(1);\n\
             \n\
             NewRun();\n\
             Insert(2){SPECIFIC_TESTS};\n\
             Insert(3){SPECIFIC_TESTS}; util:Chance(2);\n\
             Insert(2.R).If(IndexAt([0, 0]).IsEmptyAt(left), IndexAt([1, 0]).Is(17), IndexAt([0, 1]).Is(18));\n\
             Insert(2.VH).If(IndexAt([0, 0]).IsEmptyAt(top), IndexAt([-1, 0]).Is(18), IndexAt([0, 1]).Is(17));\n\
             Insert(2.VHR).If(IndexAt([0, 0]).IsEmptyAt(right), IndexAt([0, -1]).Is(18), IndexAt([-1, 0]).Is(17));\n"
        )),
        "{source}"
    );
}

#[test]
fn the_same_picked_tile_at_two_positions_shares_one_test() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("topRight"), NeighborState::Is(16));
    neighborhood.set_state(offset_of("right"), NeighborState::Is(16));

    let source = emit(&[(40, rule(neighborhood, 100.0, false))]);

    assert!(source.contains("\nInsert(40).If(IndexAt([1, -1], [1, 0]).Is(16));\n"));
}

#[test]
fn a_rolled_regular_pool_is_followed_by_the_tile_specific_run() {
    let tiles = [
        (5, anywhere(30.0)),
        (6, anywhere(30.0)),
        (7, rule(tile_specific(), 100.0, false)),
    ];

    let source = emit(&tiles);

    assert!(
        source.ends_with(&format!(
            "Insert(g:mask);\n\
             Insert(5, 6).Roll().If(IndexAt([0, 0]).Is(g:mask));\n\
             \n\
             NewRun();\n\
             Insert(7){SPECIFIC_TESTS};\n"
        )),
        "{source}"
    );
}

#[test]
fn only_tile_specific_rules_start_with_the_copy_run() {
    let source = emit(&[(2, rule(tile_specific(), 100.0, false))]);

    assert!(source.contains("AutoMapper(\"Grass_Main\");\nNewRun();\nInsert(2).If("));
    assert_eq!(source.matches("NewRun();").count(), 1);
}

#[test]
fn rule_sets_without_tile_specific_neighbors_get_no_extra_run() {
    let source = emit(&[(1, anywhere(100.0))]);

    assert_eq!(source.matches("NewRun();").count(), 1);
    assert!(!source.contains("util:"));
}

#[test]
fn the_copy_run_sits_before_the_group_runs() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let tiles = [(2, rule(tile_specific(), 100.0, false))];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert!(source.find("Insert(2).If(") < source.find("InsertObject"));
    assert!(source.contains(&format!("Insert(2){SPECIFIC_TESTS};\n")));
}

#[test]
fn fill_objects_directly_follows_the_last_insert_object_block() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let tiles = [(1, anywhere(100.0))];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert!(source.contains("\n);\nRun().FillObjects();\n"));
    assert!(!source.contains("OverrideLayer();\nRun().FillObjects();"));
}

fn full_at_right() -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("right"), NeighborState::Full);

    neighborhood
}

const EMPTY_CENTRE_AT_RIGHT: &str =
    ".NoDefaultPosRule().If(IndexAt([0, 0]).IsEmpty().IsFullAt(right), IndexAt([1, 0]).IsNotOut())";

fn empty_centred(neighborhood: Neighborhood, percent: f32) -> TileRule {
    TileRule {
        centre: Centre::Empty,
        ..rule(neighborhood, percent, false)
    }
}

#[test]
fn empty_centre_rules_get_a_copy_run_before_the_group_runs() {
    let groups = [tile_group("bones", 64, GroupPreset::Fill, 100.0)];
    let tiles = [(2, empty_centred(full_at_right(), 100.0))];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert!(
        source.contains(
            "NewRun();\n\
             Insert(2).NoDefaultPosRule().If(IndexAt([0, 0]).IsEmpty().IsFullAt(right), IndexAt([1, 0]).IsNotOut());\n\
             \n\
             NewRun();\n\
             OverrideLayer();\n\
             InsertObject("
        ),
        "{source}"
    );
    assert!(source.ends_with("Run().FillObjects();\n"), "{source}");
    assert!(!source.contains("util:NewRule"));
}

#[test]
fn an_empty_centre_rule_without_neighbors_tests_only_the_centre() {
    let source = emit(&[(
        52,
        empty_centred(Neighborhood::uniform(NeighborState::Any), 100.0),
    )]);

    assert!(source.contains("\nInsert(52).NoDefaultPosRule().If(IndexAt([0, 0]).IsEmpty());\n"));
}

#[test]
fn empty_centre_pools_split_their_cells_like_other_copy_runs() {
    let tiles = [
        (2, empty_centred(full_at_right(), 100.0)),
        (3, empty_centred(full_at_right(), 100.0)),
    ];

    let source = emit(&tiles);

    assert!(source.contains(&format!("Insert(2){EMPTY_CENTRE_AT_RIGHT};\n")));
    assert!(source.contains(&format!(
        "Insert(3){EMPTY_CENTRE_AT_RIGHT}; util:Chance(2);\n"
    )));
    assert!(!source.contains("g:mask"));
}

#[test]
fn a_full_and_an_empty_centre_rule_on_one_neighborhood_do_not_compete() {
    let tiles = [
        (1, rule(full_at_right(), 100.0, false)),
        (2, empty_centred(full_at_right(), 100.0)),
    ];

    let source = emit(&tiles);

    assert_eq!(
        body(&source),
        [
            "Insert(1).If(IndexAt([0, 0]).IsFullAt(right));",
            &format!("Insert(2){EMPTY_CENTRE_AT_RIGHT};"),
        ]
    );
    assert!(!source.contains("g:mask"));
}

#[test]
fn only_empty_centre_rules_keep_full_neighbors_inside_the_map() {
    let tiles = [
        (1, rule(full_at_right(), 100.0, false)),
        (2, rule(tile_specific(), 100.0, false)),
        (3, empty_centred(empty_at(&["top"]), 100.0)),
    ];

    assert!(!emit(&tiles).contains("IsNotOut"));
}

#[test]
fn rule_sets_without_empty_centres_never_mention_the_default_rule() {
    let tiles = [
        (1, anywhere(100.0)),
        (2, rule(tile_specific(), 100.0, false)),
    ];

    assert!(!emit(&tiles).contains("NoDefaultPosRule"));
}

#[test]
fn the_last_member_keeps_its_share_and_earlier_ones_compensate() {
    assert_eq!(
        last_wins_odds(&[30.0, 20.0, 50.0], true),
        [None, Some(2.5), Some(2.0)]
    );
}

#[test]
fn shares_below_100_leave_the_rest_unchanged() {
    assert_eq!(
        last_wins_odds(&[20.0, 10.0], false),
        [Some(4.5), Some(10.0)]
    );
}

#[test]
fn a_lone_partial_share_keeps_its_chance() {
    assert_eq!(last_wins_odds(&[30.0], false), [Some(100.0 / 30.0)]);
}

#[test]
fn a_lone_full_share_needs_no_chance() {
    assert_eq!(last_wins_odds(&[100.0], true), [None]);
}

#[test]
fn an_exact_pool_above_100_is_scaled_before_the_copy_run() {
    let tiles = [
        (2, rule(tile_specific(), 100.0, false)),
        (3, rule(tile_specific(), 100.0, false)),
    ];

    let source = emit_exact(&tiles);

    assert!(source.contains(&format!("Insert(2){SPECIFIC_TESTS};\n")));
    assert!(source.contains(&format!("Insert(3){SPECIFIC_TESTS}; util:Chance(2);\n")));
}

const SUMMING_TO_100: [f32; 4] = [33.3, 33.3, 33.3, 0.1];

#[test]
fn an_exact_pool_summing_to_100_with_float_noise_still_covers_every_cell() {
    let specific: Vec<(usize, TileRule)> = SUMMING_TO_100
        .iter()
        .enumerate()
        .map(|(index, percent)| (index + 1, rule(tile_specific(), *percent, false)))
        .collect();
    let plain: Vec<(usize, TileRule)> = SUMMING_TO_100
        .iter()
        .enumerate()
        .map(|(index, percent)| (index + 1, anywhere(*percent)))
        .collect();

    assert!(emit_exact(&specific).contains(&format!("Insert(1){SPECIFIC_TESTS};\n")));
    assert!(emit_exact(&plain).contains("Insert(g:mask);\n"));
}

#[test]
fn the_mask_skips_tiles_a_neighbor_refers_to() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("top"), NeighborState::Is(255));
    neighborhood.set_state(offset_of("bottom"), NeighborState::Is(254));
    let tiles = [
        (5, anywhere(30.0)),
        (6, anywhere(30.0)),
        (7, rule(neighborhood, 100.0, false)),
    ];

    assert!(emit(&tiles).contains("g:mask = 253;"));
}

fn extending_two_cells() -> Neighborhood {
    let extent = Extent {
        left: 2,
        right: 2,
        up: 2,
        down: 1,
    };

    Neighborhood::with_extent(extent, NeighborState::Any)
}

#[test]
fn outer_cells_are_tested_by_position_next_to_the_named_ones() {
    let mut neighborhood = extending_two_cells();
    neighborhood.set_state(offset_of("top"), NeighborState::Full);
    neighborhood.set_state((2, 0), NeighborState::Full);
    neighborhood.set_state((0, -2), NeighborState::Empty);

    let source = emit(&[(5, rule(neighborhood, 100.0, false))]);

    assert_eq!(
        body(&source),
        [
            "Insert(5).If(IndexAt([0, 0]).IsFullAt(top), IndexAt([2, 0]).IsFull(), IndexAt([0, -2]).IsEmpty());"
        ]
    );
}

#[test]
fn outer_cells_are_tested_in_the_tile_specific_copy_run() {
    let mut neighborhood = extending_two_cells();
    neighborhood.set_state(offset_of("top"), NeighborState::Empty);
    neighborhood.set_state((2, 0), NeighborState::Full);
    neighborhood.set_state((0, -2), NeighborState::Empty);
    neighborhood.set_state((-2, 0), NeighborState::Is(17));

    let source = emit(&[(7, rule(neighborhood, 100.0, false))]);

    assert_eq!(
        body(&source),
        [
            "Insert(7).If(IndexAt([0, 0]).IsEmptyAt(top), IndexAt([2, 0]).IsFull(), IndexAt([0, -2]).IsEmpty(), IndexAt([-2, 0]).Is(17));"
        ]
    );
}

#[test]
fn outer_cells_that_are_any_do_not_change_the_output() {
    let mut small = Neighborhood::uniform(NeighborState::Any);
    small.set_state(offset_of("top"), NeighborState::Full);
    let mut large = extending_two_cells();
    large.set_state(offset_of("top"), NeighborState::Full);

    assert_eq!(
        emit(&[(5, rule(small, 100.0, false))]),
        emit(&[(5, rule(large, 100.0, false))])
    );
}

#[test]
fn two_rule_sets_sharing_a_name_are_refused() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];

    let error =
        render(&[rule_set("SetA", &tiles, &[]), rule_set("SetA", &tiles, &[])]).unwrap_err();

    assert!(matches!(error, ExportError::DuplicateName(name) if name == "SetA"));
}
