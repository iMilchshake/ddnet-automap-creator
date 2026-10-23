use crate::export::r_source::{ExportError, RuleSet, last_wins_odds, render};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::tile::{AIR_TILE, Chance, TILE_COUNT, TileRule};
use crate::tests::support::{HALF_TURN, index_of, mods, outer_corner};

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
        (1, rule(surrounded, 100.0, false)),
        (2, rule(surrounded, 5.0, false)),
        (3, rule(surrounded, 1.0, false)),
        (66, rule(surrounded, 1.0, false)),
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
    let group = TileGroup {
        name: "bones".to_owned(),
        top_left: 64,
        width: 2,
        height: 2,
        mode: GroupMode::Fill,
        chance: Chance::FULL,
    };
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

    assert!(source.starts_with(
        "// created with ddnet-automap-creator (https://github.com/iMilchshake/ddnet-automap-creator)\n"
    ));
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
        (1, rule(surrounded, 100.0, false)),
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
        (1, rule(surrounded, 2.5, false)),
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
    let groups = [tile_group("bones", 238, GroupMode::Fill, 100.0)];

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

fn empty_at(names: &[&str]) -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    for name in names {
        neighborhood.set_state(index_of(name), NeighborState::Empty);
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
fn groups_alone_start_their_run_right_after_the_automapper_line() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let source = emit_with_groups(&[], &groups).unwrap();

    assert!(
        source.contains("AutoMapper(\"Grass_Main\");\nNewRun();\nOverrideLayer();\nInsertObject(")
    );
}

#[test]
fn runs_are_separated_by_one_blank_line() {
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let tiles = [
        (1, anywhere(100.0)),
        (2, rule(tile_specific(), 100.0, false)),
    ];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert_eq!(source.matches("\n\nNewRun();\n").count(), 3);
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
    let groups_a = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let groups_b = [tile_group("bones", 100, GroupMode::Fill, 100.0)];

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
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];

    let source = render(&[rule_set("SetA", &tiles, &groups)]).unwrap();

    assert!(source.contains("object o:bones = Rect(64, 81);"));
}

fn tile_specific() -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(index_of("top"), NeighborState::Is(17));
    neighborhood.set_state(index_of("right"), NeighborState::Is(18));
    neighborhood.set_state(index_of("bottom"), NeighborState::Empty);

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

    assert_eq!(body(&source), ["Insert(1);"]);
    assert!(!source.contains("g:mask"));
    assert!(!source.contains("OverrideLayer"));
    assert!(
        source.ends_with(
            "Insert(1);\n\
             \n\
             NewRun();\n\
             util:NewRule(); util:InsertIndex(2);\n\
             util:NewPosRule([0, -1]); util:Is(17);\n\
             util:NewPosRule([1, 0]); util:Is(18);\n\
             util:NewPosRule([0, 1]); util:IsEmpty();\n\
             util:NewRule(); util:InsertIndex(3); util:Chance(2);\n\
             util:NewPosRule([0, -1]); util:Is(17);\n\
             util:NewPosRule([1, 0]); util:Is(18);\n\
             util:NewPosRule([0, 1]); util:IsEmpty();\n\
             util:NewRule(); util:InsertIndex(2.R);\n\
             util:NewPosRule([-1, 0]); util:IsEmpty();\n\
             util:NewPosRule([1, 0]); util:Is(17);\n\
             util:NewPosRule([0, 1]); util:Is(18);\n\
             util:NewRule(); util:InsertIndex(2.VH);\n\
             util:NewPosRule([0, -1]); util:IsEmpty();\n\
             util:NewPosRule([-1, 0]); util:Is(18);\n\
             util:NewPosRule([0, 1]); util:Is(17);\n\
             util:NewRule(); util:InsertIndex(2.VHR);\n\
             util:NewPosRule([0, -1]); util:Is(18);\n\
             util:NewPosRule([-1, 0]); util:Is(17);\n\
             util:NewPosRule([1, 0]); util:IsEmpty();\n"
        ),
        "{source}"
    );
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
        source.ends_with(
            "Insert(g:mask);\n\
             Insert(5, 6).Roll().If(IndexAt([0, 0]).Is(g:mask));\n\
             \n\
             NewRun();\n\
             util:NewRule(); util:InsertIndex(7);\n\
             util:NewPosRule([0, -1]); util:Is(17);\n\
             util:NewPosRule([1, 0]); util:Is(18);\n\
             util:NewPosRule([0, 1]); util:IsEmpty();\n"
        ),
        "{source}"
    );
}

#[test]
fn only_tile_specific_rules_start_with_the_copy_run() {
    let source = emit(&[(2, rule(tile_specific(), 100.0, false))]);

    assert!(source.contains("AutoMapper(\"Grass_Main\");\nNewRun();\nutil:NewRule();"));
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
    let groups = [tile_group("bones", 64, GroupMode::Fill, 100.0)];
    let tiles = [(2, rule(tile_specific(), 100.0, false))];

    let source = emit_with_groups(&tiles, &groups).unwrap();

    assert!(source.find("util:NewRule()") < source.find("InsertObject"));
    assert!(source.contains("util:InsertIndex(2);\n"));
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

    assert!(source.contains("util:InsertIndex(2);\n"));
    assert!(source.contains("util:InsertIndex(3); util:Chance(2);\n"));
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

    assert!(emit_exact(&specific).contains("util:InsertIndex(1);\n"));
    assert!(emit_exact(&plain).contains("Insert(g:mask);\n"));
}

#[test]
fn the_mask_skips_tiles_a_neighbor_refers_to() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(index_of("top"), NeighborState::Is(255));
    neighborhood.set_state(index_of("bottom"), NeighborState::Is(254));
    let tiles = [
        (5, anywhere(30.0)),
        (6, anywhere(30.0)),
        (7, rule(neighborhood, 100.0, false)),
    ];

    assert!(emit(&tiles).contains("g:mask = 253;"));
}

#[test]
fn two_rule_sets_sharing_a_name_are_refused() {
    let tiles = [(1, rule(Neighborhood::default(), 100.0, false))];

    let error =
        render(&[rule_set("SetA", &tiles, &[]), rule_set("SetA", &tiles, &[])]).unwrap_err();

    assert!(matches!(error, ExportError::DuplicateName(name) if name == "SetA"));
}
