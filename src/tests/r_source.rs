use crate::export::r_source::{ExportError, RuleSet, render};
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

    let source = render(&RuleSet {
        image_stem: "grass_main",
        name: "Grass",
        tiles: &tiles,
    })
    .unwrap();

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
    let source = render(&RuleSet {
        image_stem: "grass_main",
        name: "Grass Main",
        tiles: &tiles,
    })
    .unwrap();

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
    let source = render(&RuleSet {
        image_stem: "any",
        name: "Any",
        tiles: &tiles,
    })
    .unwrap();

    assert_eq!(body(&source), ["Insert(5);"]);
}

#[test]
fn a_pool_at_full_chance_rolls_instead_of_listing_chances() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (1, rule(surrounded, 100.0, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = render(&RuleSet {
        image_stem: "roll",
        name: "Roll",
        tiles: &tiles,
    })
    .unwrap();

    assert!(body(&source)[0].starts_with("Insert(1, 2).Roll()"));
}

#[test]
fn fractional_chances_keep_their_shortest_form() {
    let surrounded = Neighborhood::uniform(NeighborState::Full);
    let tiles = [
        (1, rule(surrounded, 2.5, false)),
        (2, rule(surrounded, 100.0, false)),
    ];
    let source = render(&RuleSet {
        image_stem: "fraction",
        name: "Fraction",
        tiles: &tiles,
    })
    .unwrap();

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
    let source = render(&RuleSet {
        image_stem: "order",
        name: "Order",
        tiles: &tiles,
    })
    .unwrap();

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
    let source = render(&RuleSet {
        image_stem: "symmetric",
        name: "Symmetric",
        tiles: &tiles,
    })
    .unwrap();

    assert_eq!(body(&source).len(), 1);
    assert!(body(&source)[0].starts_with("Insert(7)."));
}

#[test]
fn an_empty_rule_set_is_refused() {
    let error = render(&RuleSet {
        image_stem: "empty",
        name: "Empty",
        tiles: &[],
    })
    .unwrap_err();

    assert!(matches!(error, ExportError::NothingConfigured));
}
