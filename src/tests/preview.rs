use std::collections::BTreeSet;
use std::ops::{Range, RangeInclusive};
use std::time::{Duration, Instant};

use twmap::Tile;
use twmap::ndarray::Array2;

use crate::blueprint;
use crate::export::compiler::RulesCompiler;
use crate::export::r_source::{RuleSet, output_file, render};
use crate::model::group::{GroupPreset, TileGroup};
use crate::model::neighbor::{Extent, NEIGHBORS, NeighborState, Neighborhood, Offset, Side};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{Center, Chance, RuleCenter, TILE_COUNT, TileMods, TileRule};
use crate::model::transform::Transform;
use crate::preview::{
    Automapped, CustomShape, FALLBACK_UNMATCHED_MARKER, PlacedTile, PreviewCell, PreviewError,
    Sample, automap, placing_rule, unmatched_marker,
};
use crate::tests::support::offset_of;

const SEED: u32 = 42;
const SOLID_ID: u8 = 200;
const SHAPE_SIZE: usize = 4;
const GROUND_ROWS: Range<usize> = 2..SHAPE_SIZE;
const DRAWN_COLUMN: usize = 1;
const DRAWN_ROW: usize = 2;
const LARGER_SIZE: usize = 6;
const PADDED_TORUS_SIZE: (usize, usize) = (20, 36);
const PADDED_MINIMAL_SIZE: (usize, usize) = (11, 11);
const SHAPES_SIZE: (usize, usize) = (17, 32);
const UNMATCHED_MARKER: u8 = SOLID_ID;
const FLIPPED_EVERYWHERE: &str = "[Test]\nIndex 5 XFLIP\n";
const TOP_EDGES: &str = "[Test]\nIndex 2\nIndex 7\nPos 0 -1 EMPTY\n";
const ONLY_TOP_EDGES: &str = "[Test]\nIndex 7\nPos 0 -1 EMPTY\n";
const TILE_ONE_ON_TOP_EDGES: &str = "[Test]\nIndex 1\nPos 0 -1 EMPTY\n";
const IMAGE_STEM: &str = "grass_main";
const COMPILE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const COMPILE_TIMEOUT: Duration = Duration::from_secs(30);
const AIR_DECORATION: usize = 9;
const ELL_TILES: [usize; 4] = [65, 81, 97, 98];
const COUNTED_TILE: usize = 65;
const DOMINO_TILES: [usize; 2] = [COUNTED_TILE, COUNTED_TILE + 1];
const DOMINO_SEEDS: RangeInclusive<u32> = 1..=32;
const WALLED_FLOOR_WIDTH: usize = 16;
const WALLED_FLOOR_HEIGHT: usize = 6;
const LOW_CHANCE_PERCENT: f32 = 5.0;
const LOW_CHANCE_SIDE: usize = 256;
const LOW_CHANCE_SEEDS: RangeInclusive<u32> = 1..=8;
const MIN_TOTAL_SHARE: f32 = 0.7;
const MAX_TOTAL_SHARE: f32 = 1.05;
const READS_LATER_CELL: &str = "[Test]\nNoLayerCopy\nIndex 1\nIndex 2\nPos 1 0 INDEX 1\n";
const SHARE_TOLERANCE: f32 = 0.12;

fn placed(index: usize, center: Center) -> PreviewCell {
    PreviewCell::Placed(PlacedTile {
        index,
        transform: Transform::IDENTITY,
        center,
    })
}

fn plain(index: usize) -> PreviewCell {
    placed(index, Center::Full)
}

fn compiled(source: String) -> String {
    let mut compiler = RulesCompiler::new();
    compiler.start(source, output_file(IMAGE_STEM));

    let deadline = Instant::now() + COMPILE_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(result) = compiler.poll() {
            return result.unwrap().rules;
        }
        std::thread::sleep(COMPILE_POLL_INTERVAL);
    }

    panic!("rpp did not finish within {COMPILE_TIMEOUT:?}");
}

fn empty_above_ground() -> String {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("bottom"), NeighborState::Full);
    let tiles = [(
        AIR_DECORATION,
        TileRule {
            center: RuleCenter::Empty,
            ..TileRule::new(neighborhood)
        },
    )];

    render(&[RuleSet {
        image_stem: IMAGE_STEM,
        name: "Test",
        tiles: &tiles,
        deactivated: &[],
        groups: &[],
        chance_mode: ChanceMode::Normalize,
    }])
    .unwrap()
}

fn some_rule() -> TileRule {
    TileRule::new(Neighborhood::uniform(NeighborState::Full))
}

fn is_solid(tiles: &Array2<Tile>, column: usize, row: usize) -> bool {
    tiles[(row, column)].id != 0
}

fn ground_shape() -> CustomShape {
    let mut shape = CustomShape::empty(SHAPE_SIZE, SHAPE_SIZE);
    for row in GROUND_ROWS {
        for column in 0..SHAPE_SIZE {
            shape.set(column, row, true);
        }
    }
    shape
}

fn first_cell(sample: Sample, solid: bool, solid_above: bool) -> (usize, usize) {
    let tiles = sample.tiles(&ground_shape(), SOLID_ID).unwrap();
    let (height, width) = tiles.dim();

    (1..height)
        .flat_map(|row| (0..width).map(move |column| (column, row)))
        .find(|&(column, row)| {
            is_solid(&tiles, column, row) == solid
                && is_solid(&tiles, column, row - 1) == solid_above
        })
        .unwrap()
}

#[test]
fn compiled_rules_start_with_a_comment_naming_the_version() {
    let rules = compiled(empty_above_ground());

    let header = rules.lines().next().unwrap();
    assert!(header.starts_with("# "));
    assert!(header.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn compiled_rules_with_the_header_still_parse() {
    let rules = compiled(empty_above_ground());

    let automapped = automap(
        &rules,
        Sample::default(),
        &ground_shape(),
        SEED,
        UNMATCHED_MARKER,
    );

    automapped.unwrap();
}

#[test]
fn every_sample_has_ground_and_air() {
    for sample in Sample::ALL {
        let tiles = sample.tiles(&ground_shape(), SOLID_ID).unwrap();

        assert!(tiles.iter().any(|tile| tile.id != 0), "{}", sample.name());
        assert!(tiles.iter().any(|tile| tile.id == 0), "{}", sample.name());
    }
}

#[test]
fn padded_samples_are_surrounded_by_air() {
    for (sample, (height, width)) in [
        (Sample::DeBruijnTorus, PADDED_TORUS_SIZE),
        (Sample::Minimal, PADDED_MINIMAL_SIZE),
    ] {
        let tiles = sample.tiles(&ground_shape(), SOLID_ID).unwrap();

        assert_eq!(tiles.dim(), (height, width), "{}", sample.name());
        for ((row, column), tile) in tiles.indexed_iter() {
            let border = row == 0 || column == 0 || row == height - 1 || column == width - 1;
            assert!(!border || tile.id == 0, "{} {column},{row}", sample.name());
        }
    }
}

#[test]
fn shapes_keep_their_size() {
    let tiles = Sample::Shapes.tiles(&ground_shape(), SOLID_ID).unwrap();

    assert_eq!(tiles.dim(), SHAPES_SIZE);
}

#[test]
fn a_rule_without_conditions_replaces_every_solid_cell() {
    let flipped = PreviewCell::Placed(PlacedTile {
        index: 5,
        transform: Transform {
            x_flip: true,
            y_flip: false,
            rot: false,
        },
        center: Center::Full,
    });

    for sample in Sample::ALL {
        let custom = ground_shape();
        let automapped =
            automap(FLIPPED_EVERYWHERE, sample, &custom, SEED, UNMATCHED_MARKER).unwrap();

        for ((row, column), tile) in sample.tiles(&custom, SOLID_ID).unwrap().indexed_iter() {
            let expected = match tile.id {
                0 => PreviewCell::Air,
                _ => flipped,
            };
            assert_eq!(automapped.cell(column, row), expected, "{column},{row}");
        }
    }
}

#[test]
fn later_rules_win_where_their_conditions_hold() {
    for sample in Sample::ALL {
        let automapped =
            automap(TOP_EDGES, sample, &ground_shape(), SEED, UNMATCHED_MARKER).unwrap();
        let (edge_column, edge_row) = first_cell(sample, true, false);
        let (inner_column, inner_row) = first_cell(sample, true, true);
        let (air_column, air_row) = first_cell(sample, false, false);

        assert_eq!(automapped.cell(edge_column, edge_row), plain(7));
        assert_eq!(automapped.cell(inner_column, inner_row), plain(2));
        assert_eq!(automapped.cell(air_column, air_row), PreviewCell::Air);
    }
}

#[test]
fn rules_run_cell_by_cell_the_way_ddnet_does() {
    let mut shape = CustomShape::empty(3, 1);
    for column in 0..3 {
        shape.set(column, 0, true);
    }

    let automapped = automap(
        READS_LATER_CELL,
        Sample::Custom,
        &shape,
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();

    for column in 0..3 {
        assert_eq!(automapped.cell(column, 0), plain(1), "{column}");
    }
}

#[test]
fn a_chance_rpp_rounds_to_random_1_places_everywhere() {
    let sample = Sample::default();
    let automapped = automap(
        "[Test]\nIndex 5\nRandom 1\n",
        sample,
        &CustomShape::default(),
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();
    let (column, row) = first_cell(sample, true, false);

    assert_eq!(automapped.cell(column, row), plain(5));
}

#[test]
fn unreadable_rules_are_reported() {
    assert!(matches!(
        automap(
            "[Test]\nIndex banana\n",
            Sample::default(),
            &CustomShape::default(),
            SEED,
            UNMATCHED_MARKER
        ),
        Err(PreviewError::Syntax(_))
    ));
}

#[test]
fn rules_without_a_rule_set_are_reported() {
    assert!(matches!(
        automap(
            "",
            Sample::default(),
            &CustomShape::default(),
            SEED,
            UNMATCHED_MARKER
        ),
        Err(PreviewError::NoRuleSet)
    ));
}

#[test]
fn another_seed_rolls_chances_differently() {
    let halves = "[Test]\nIndex 5\nRandom 2\n";
    let sample = Sample::default();
    let custom = CustomShape::default();
    let first = automap(halves, sample, &custom, SEED, UNMATCHED_MARKER).unwrap();
    let second = automap(halves, sample, &custom, SEED + 1, UNMATCHED_MARKER).unwrap();

    assert_ne!(first, second);
    assert_eq!(
        first,
        automap(halves, sample, &custom, SEED, UNMATCHED_MARKER).unwrap()
    );
}

#[test]
fn solid_cells_no_rule_touched_are_unmatched() {
    for sample in Sample::ALL {
        let custom = ground_shape();
        let automapped = automap(ONLY_TOP_EDGES, sample, &custom, SEED, UNMATCHED_MARKER).unwrap();
        let (edge_column, edge_row) = first_cell(sample, true, false);
        let (inner_column, inner_row) = first_cell(sample, true, true);
        let (air_column, air_row) = first_cell(sample, false, false);

        assert_eq!(automapped.cell(edge_column, edge_row), plain(7));
        assert_eq!(
            automapped.cell(inner_column, inner_row),
            PreviewCell::Unmatched
        );
        assert_eq!(automapped.cell(air_column, air_row), PreviewCell::Air);
    }
}

#[test]
fn an_unmapped_sample_marks_every_solid_cell_unmatched() {
    for sample in Sample::ALL {
        let custom = ground_shape();
        let automapped = Automapped::unmapped(sample, &custom).unwrap();

        for ((row, column), tile) in sample.tiles(&custom, SOLID_ID).unwrap().indexed_iter() {
            let expected = match tile.id {
                0 => PreviewCell::Air,
                _ => PreviewCell::Unmatched,
            };
            assert_eq!(automapped.cell(column, row), expected, "{column},{row}");
        }
    }
}

#[test]
fn an_exported_rule_with_outer_cells_compiles_to_positioned_tests() {
    let extent = Extent {
        left: 2,
        right: 2,
        up: 2,
        down: 1,
    };
    let mut neighborhood = Neighborhood::with_extent(extent, NeighborState::Any);
    neighborhood.set_state((2, 0), NeighborState::Full);
    neighborhood.set_state((0, -2), NeighborState::Empty);
    neighborhood.set_state((-2, 0), NeighborState::Is(17));
    let tiles = [(AIR_DECORATION, TileRule::new(neighborhood))];

    let rules = compiled(
        render(&[RuleSet {
            image_stem: IMAGE_STEM,
            name: "Test",
            tiles: &tiles,
            deactivated: &[],
            groups: &[],
            chance_mode: ChanceMode::Normalize,
        }])
        .unwrap(),
    );

    assert!(rules.contains("Pos 2 0 FULL"), "{rules}");
    assert!(rules.contains("Pos 0 -2 EMPTY"), "{rules}");
    assert!(rules.contains("Pos -2 0 INDEX 17"), "{rules}");
}

#[test]
fn an_exported_empty_center_rule_places_into_the_air_above_ground() {
    let rules = compiled(empty_above_ground());
    assert!(rules.contains("NoDefaultRule"), "{rules}");
    assert!(rules.contains("Pos 0 0 EMPTY"), "{rules}");
    assert!(
        rules.contains("Pos 0 1 FULL\nPos 0 1 NOTINDEX -1\n"),
        "{rules}"
    );

    let all_air = automap(
        &rules,
        Sample::Custom,
        &CustomShape::empty(SHAPE_SIZE, SHAPE_SIZE),
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();
    assert_eq!(all_air.cell(0, SHAPE_SIZE - 1), PreviewCell::Air);

    let automapped = automap(
        &rules,
        Sample::Custom,
        &ground_shape(),
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();

    assert_eq!(
        automapped.cell(0, GROUND_ROWS.start - 1),
        placed(AIR_DECORATION, Center::Empty)
    );
    assert_eq!(automapped.cell(0, 0), PreviewCell::Air);
    assert_eq!(
        automapped.cell(0, GROUND_ROWS.start),
        PreviewCell::Unmatched
    );
}

fn mirror_symmetric_flippable() -> TileRule {
    TileRule {
        mods: TileMods::new(true, false, false),
        chance: Chance::new(50.0).unwrap(),
        ..rule_with(&[("top", NeighborState::Full)])
    }
}

#[test]
fn an_exported_symmetric_rule_places_both_the_plain_and_the_flipped_tile() {
    let tiles = [(AIR_DECORATION, mirror_symmetric_flippable())];

    let rules = compiled(
        render(&[RuleSet {
            image_stem: IMAGE_STEM,
            name: "Test",
            tiles: &tiles,
            deactivated: &[],
            groups: &[],
            chance_mode: ChanceMode::Normalize,
        }])
        .unwrap(),
    );

    let index = format!("Index {AIR_DECORATION}");
    assert!(rules.lines().any(|line| line == index), "{rules}");
    assert!(
        rules.lines().any(|line| line == format!("{index} XFLIP")),
        "{rules}"
    );
}

#[test]
fn a_flipped_placement_of_a_symmetric_rule_resolves_to_that_rule() {
    let project = project_with(&[mirror_symmetric_flippable()]);
    let solid_above = solid_at(&["top"]);

    for transform in [Transform::IDENTITY, Transform::FLIP_HORIZONTALLY] {
        let placed = placed_tile(transform, Center::Full);

        assert_eq!(placing_rule(&project, placed, &solid_above), Some(0));
    }
}

#[test]
fn a_rule_placing_tile_one_is_not_mistaken_for_unmatched() {
    let sample = Sample::default();
    let automapped = automap(
        TILE_ONE_ON_TOP_EDGES,
        sample,
        &CustomShape::default(),
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();
    let (column, row) = first_cell(sample, true, false);

    assert_eq!(automapped.cell(column, row), plain(1));
}

#[test]
fn a_custom_map_has_the_size_and_solid_cells_that_were_drawn() {
    let mut shape = CustomShape::empty(LARGER_SIZE, SHAPE_SIZE);
    shape.set(DRAWN_COLUMN, DRAWN_ROW, true);

    let tiles = Sample::Custom.tiles(&shape, SOLID_ID).unwrap();

    assert_eq!(tiles.dim(), (SHAPE_SIZE, LARGER_SIZE));
    assert_eq!(tiles.iter().filter(|tile| tile.id == SOLID_ID).count(), 1);
    assert!(is_solid(&tiles, DRAWN_COLUMN, DRAWN_ROW));
}

#[test]
fn a_cell_set_back_to_air_is_air() {
    let mut shape = ground_shape();
    shape.set(0, GROUND_ROWS.start, false);

    let tiles = Sample::Custom.tiles(&shape, SOLID_ID).unwrap();

    assert!(!is_solid(&tiles, 0, GROUND_ROWS.start));
    assert!(is_solid(&tiles, 1, GROUND_ROWS.start));
}

#[test]
fn clearing_a_custom_map_leaves_only_air() {
    let mut shape = ground_shape();
    shape.clear();

    let tiles = Sample::Custom.tiles(&shape, SOLID_ID).unwrap();

    assert!(tiles.iter().all(|tile| tile.id == 0));
}

#[test]
fn growing_a_custom_map_keeps_its_cells_and_fills_the_rest_with_air() {
    let mut shape = ground_shape();
    shape.resize(LARGER_SIZE, LARGER_SIZE);

    let tiles = Sample::Custom.tiles(&shape, SOLID_ID).unwrap();

    assert_eq!(tiles.dim(), (LARGER_SIZE, LARGER_SIZE));
    for ((row, column), tile) in tiles.indexed_iter() {
        let kept = column < SHAPE_SIZE && GROUND_ROWS.contains(&row);
        assert_eq!(tile.id != 0, kept, "{column},{row}");
    }
}

#[test]
fn shrinking_a_custom_map_keeps_the_overlap() {
    let mut shape = ground_shape();
    shape.resize(SHAPE_SIZE - 1, GROUND_ROWS.start + 1);

    let tiles = Sample::Custom.tiles(&shape, SOLID_ID).unwrap();

    assert_eq!(tiles.dim(), (GROUND_ROWS.start + 1, SHAPE_SIZE - 1));
    assert!(is_solid(&tiles, 0, GROUND_ROWS.start));
    assert!(!is_solid(&tiles, 0, 0));
}

fn rule_with(states: &[(&str, NeighborState)]) -> TileRule {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    for (name, state) in states {
        neighborhood.set_state(offset_of(name), *state);
    }

    TileRule::new(neighborhood)
}

fn solid_at(names: &[&str]) -> impl Fn(Offset) -> bool {
    let solid: Vec<Offset> = names.iter().map(|name| offset_of(name)).collect();

    move |offset| solid.contains(&offset)
}

fn placed_tile(transform: Transform, center: Center) -> PlacedTile {
    PlacedTile {
        index: 1,
        transform,
        center,
    }
}

#[test]
fn is_solid_near_reads_the_sample_and_counts_outside_cells_as_solid() {
    let automapped = Automapped::unmapped(Sample::Custom, &ground_shape()).unwrap();
    let row = GROUND_ROWS.start;
    let expected = solid_at(&["left", "right", "bottom", "bottomLeft", "bottomRight"]);

    for neighbor in NEIGHBORS {
        let offset = (neighbor.dx, neighbor.dy);
        assert_eq!(
            automapped.is_solid_near(1, row, offset),
            expected(offset),
            "{}",
            neighbor.rpp_name
        );
    }
    assert!(automapped.is_solid_near(0, 0, offset_of("left")));
    assert!(!automapped.is_solid_near(0, 0, offset_of("bottom")));
}

#[test]
fn a_rule_requiring_a_solid_cell_two_away_matches_only_where_the_map_has_it() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any).grown(Side::Right);
    neighborhood.set_state((2, 0), NeighborState::Full);
    let project = project_with(&[TileRule::new(neighborhood)]);
    let placed = placed_tile(Transform::IDENTITY, Center::Full);

    let solid_two_away = |offset: Offset| offset == (2, 0);

    assert_eq!(placing_rule(&project, placed, solid_two_away), Some(0));
    assert_eq!(placing_rule(&project, placed, solid_at(&["right"])), None);
}

fn project_with(rules: &[TileRule]) -> Project {
    let mut project = Project::default();
    for rule in rules {
        project.add_rule(1, rule.clone());
    }

    project
}

#[test]
fn the_more_specific_rule_is_the_one_that_placed_the_tile() {
    let project = project_with(&[rule_with(&[]), rule_with(&[("top", NeighborState::Empty)])]);
    let placed = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(placing_rule(&project, placed, solid_at(&[])), Some(1));
    assert_eq!(placing_rule(&project, placed, solid_at(&["top"])), Some(0));
}

#[test]
fn a_rule_repeated_by_a_later_one_stays_with_the_earlier_one() {
    let project = project_with(&[rule_with(&[]), rule_with(&[])]);
    let placed = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(placing_rule(&project, placed, solid_at(&[])), Some(0));
}

#[test]
fn a_specific_tile_neighbor_counts_as_solid() {
    let project = project_with(&[rule_with(&[("top", NeighborState::Is(3))])]);
    let placed = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(placing_rule(&project, placed, solid_at(&["top"])), Some(0));
    assert_eq!(placing_rule(&project, placed, solid_at(&[])), None);
}

#[test]
fn a_tile_specific_rule_beats_a_more_specific_plain_rule() {
    let project = project_with(&[
        rule_with(&[
            ("top", NeighborState::Empty),
            ("left", NeighborState::Empty),
            ("right", NeighborState::Empty),
        ]),
        rule_with(&[("bottom", NeighborState::Is(3))]),
    ]);
    let placed = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(
        placing_rule(&project, placed, solid_at(&["bottom"])),
        Some(1)
    );
}

#[test]
fn a_rotated_variant_reads_the_turned_neighborhood() {
    let rotating = TileRule {
        mods: TileMods::new(false, false, true),
        ..rule_with(&[
            ("bottom", NeighborState::Full),
            ("top", NeighborState::Empty),
        ])
    };
    let project = project_with(&[rotating]);
    let rotated = placed_tile(Transform::ROTATE_CLOCKWISE, Center::Full);
    let upright = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(
        placing_rule(&project, rotated, solid_at(&["left"])),
        Some(0)
    );
    assert_eq!(placing_rule(&project, rotated, solid_at(&["bottom"])), None);
    assert_eq!(
        placing_rule(&project, upright, solid_at(&["bottom"])),
        Some(0)
    );
}

#[test]
fn only_rules_with_the_placed_center_are_considered() {
    let project = project_with(&[
        rule_with(&[]),
        TileRule {
            center: RuleCenter::Empty,
            ..rule_with(&[])
        },
    ]);

    let empty = placed_tile(Transform::IDENTITY, Center::Empty);
    let full = placed_tile(Transform::IDENTITY, Center::Full);

    assert_eq!(placing_rule(&project, empty, solid_at(&[])), Some(1));
    assert_eq!(placing_rule(&project, full, solid_at(&[])), Some(0));
}

#[test]
fn the_unmatched_marker_is_the_first_tile_nothing_claims() {
    assert_eq!(unmatched_marker(&Project::default()), 1);
}

#[test]
fn the_unmatched_marker_skips_active_rules_and_group_footprints() {
    let mut project = Project::default();
    project.add_rule(1, some_rule());
    project.add_group(TileGroup::new(
        "bones".to_owned(),
        TileGroup::rect_tiles(2, 3),
    ));

    assert_eq!(unmatched_marker(&project), 4);
}

#[test]
fn the_unmatched_marker_skips_tiles_a_neighbor_refers_to() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("topLeft"), NeighborState::Is(1));
    let mut project = Project::default();
    project.add_rule(5, TileRule::new(neighborhood));

    assert_eq!(unmatched_marker(&project), 2);
}

#[test]
fn the_unmatched_marker_ignores_deactivated_rules() {
    let mut project = Project::default();
    project.add_rule(1, some_rule());
    project.deactivate(1);

    assert_eq!(unmatched_marker(&project), 1);
}

#[test]
fn the_unmatched_marker_falls_back_when_only_the_mask_is_free() {
    let mut project = Project::default();
    for tile in 1..TILE_COUNT - 1 {
        project.add_rule(tile, some_rule());
    }

    let marker = unmatched_marker(&project);

    assert_eq!(marker, FALLBACK_UNMATCHED_MARKER);
    assert_ne!(Some(usize::from(marker)), project.mask_tile());
}

#[test]
fn a_group_of_any_shape_compiles() {
    let mut group = TileGroup::new("bones".to_owned(), [65, 81, 82].into_iter().collect());
    group.apply_preset(GroupPreset::Decorate);
    let source = render(&[RuleSet {
        image_stem: IMAGE_STEM,
        name: "Test",
        tiles: &[],
        deactivated: &[],
        groups: &[group],
        chance_mode: ChanceMode::Normalize,
    }])
    .unwrap();

    assert!(!compiled(source).is_empty());
}

fn ell(mods: TileMods, percent: f32) -> TileGroup {
    shaped(&ELL_TILES, mods, percent)
}

fn shaped(tiles: &[usize], mods: TileMods, percent: f32) -> TileGroup {
    TileGroup {
        chance: Chance::new(percent).unwrap(),
        mods,
        ..TileGroup::new("ell".to_owned(), tiles.iter().copied().collect())
    }
}

fn compiled_group(group: &TileGroup) -> String {
    let source = render(&[RuleSet {
        image_stem: IMAGE_STEM,
        name: "Test",
        tiles: &[],
        deactivated: &[],
        groups: std::slice::from_ref(group),
        chance_mode: ChanceMode::Normalize,
    }])
    .unwrap();

    compiled(source)
}

fn rule_blocks(rules: &str) -> Vec<Vec<&str>> {
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in rules.lines() {
        if line.starts_with("Index ") {
            blocks.push(Vec::new());
        }
        if let Some(block) = blocks.last_mut() {
            block.push(line);
        }
    }

    blocks
}

fn full_positions(block: &[&str]) -> BTreeSet<(i32, i32)> {
    block
        .iter()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["Pos", x, y, "FULL"] => Some((x.parse().unwrap(), y.parse().unwrap())),
                _ => None,
            }
        })
        .collect()
}

fn flags_of(index_line: &str) -> Transform {
    let words: Vec<&str> = index_line.split_whitespace().collect();

    Transform {
        x_flip: words.contains(&"XFLIP"),
        y_flip: words.contains(&"YFLIP"),
        rot: words.contains(&"ROTATE"),
    }
}

#[test]
fn rpp_turns_every_group_copy_the_way_the_model_does() {
    let group = ell(TileMods::new(true, false, true), 100.0);
    let rules = compiled_group(&group);
    let blocks = rule_blocks(&rules);
    let copies = group.copies();

    assert_eq!(copies.len(), Transform::ALL.len());
    assert!(blocks.len() >= copies.len(), "{rules}");
    for (copy, block) in copies.iter().zip(&blocks) {
        let expected: BTreeSet<(i32, i32)> = copy.cells.iter().copied().collect();

        assert_eq!(full_positions(block), expected, "{:?}", copy.transform);
        assert_eq!(flags_of(block[0]), copy.transform, "{}", block[0]);
    }
}

fn solid_everywhere(side: usize) -> CustomShape {
    let mut shape = CustomShape::empty(side, side);
    for row in 0..side {
        for column in 0..side {
            shape.set(column, row, true);
        }
    }

    shape
}

fn anchors(automapped: &Automapped) -> Vec<PlacedTile> {
    (0..automapped.height)
        .flat_map(|row| (0..automapped.width).map(move |column| (column, row)))
        .filter_map(|(column, row)| match automapped.cell(column, row) {
            PreviewCell::Placed(placed) if placed.index == COUNTED_TILE => Some(placed),
            _ => None,
        })
        .collect()
}

fn copy_counts(mods: TileMods) -> Vec<usize> {
    let group = ell(mods, LOW_CHANCE_PERCENT);
    let rules = compiled_group(&group);
    let shape = solid_everywhere(LOW_CHANCE_SIDE);

    let copies = group.copies();
    let mut counts = vec![0_usize; copies.len()];
    for seed in LOW_CHANCE_SEEDS {
        let automapped = automap(&rules, Sample::Custom, &shape, seed, UNMATCHED_MARKER).unwrap();
        for placed in anchors(&automapped) {
            let position = copies
                .iter()
                .position(|copy| copy.transform == placed.transform)
                .unwrap();
            counts[position] += 1;
        }
    }
    eprintln!("{mods:?} copy counts {counts:?}");

    counts
}

#[test]
fn every_copy_of_a_low_chance_group_gets_an_equal_share_of_the_anchors() {
    for mods in [
        TileMods::new(true, false, false),
        TileMods::new(false, false, true),
    ] {
        let counts = copy_counts(mods);
        let mean = counts.iter().sum::<usize>() as f32 / counts.len() as f32;

        for count in &counts {
            assert!(
                (*count as f32 - mean).abs() <= SHARE_TOLERANCE * mean,
                "{mods:?} {counts:?}"
            );
        }
    }
}

#[test]
fn a_low_chance_group_places_about_its_chance_of_the_cells() {
    let counts = copy_counts(TileMods::new(true, false, false));
    let total = counts.iter().sum::<usize>() as f32;
    let cells = (LOW_CHANCE_SIDE * LOW_CHANCE_SIDE * LOW_CHANCE_SEEDS.count()) as f32;
    let expected = cells * LOW_CHANCE_PERCENT / Chance::FULL.percent();
    eprintln!("total anchors {total} of expected {expected}");

    assert!(
        (MIN_TOTAL_SHARE * expected..=MAX_TOTAL_SHARE * expected).contains(&total),
        "{counts:?} of expected {expected}"
    );
}

#[test]
fn only_one_copy_is_placed_where_two_copies_fit_the_same_cells() {
    let domino = shaped(
        &DOMINO_TILES,
        TileMods::new(true, false, false),
        Chance::FULL.percent(),
    );
    let rules = compiled_group(&domino);
    let mut shape = CustomShape::empty(DOMINO_TILES.len(), 1);
    for column in 0..DOMINO_TILES.len() {
        shape.set(column, 0, true);
    }

    let mut winners = Vec::new();
    for seed in DOMINO_SEEDS {
        let automapped = automap(&rules, Sample::Custom, &shape, seed, UNMATCHED_MARKER).unwrap();
        let placed = anchors(&automapped);

        assert_eq!(placed.len(), 1, "seed {seed}");
        winners.push(placed[0].transform);
    }

    for copy in domino.copies() {
        assert!(winners.contains(&copy.transform), "{:?}", copy.transform);
    }
}

#[test]
fn an_on_floor_group_lands_on_air_above_solid_ground_and_never_inside_it() {
    let mut floor = shaped(&DOMINO_TILES, TileMods::default(), Chance::FULL.percent());
    floor.apply_preset(GroupPreset::OnFloor);
    let rules = compiled_group(&floor);

    let automapped = automap(
        &rules,
        Sample::Custom,
        &ground_shape(),
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();
    let covered = group_cells(&automapped);

    assert!(!covered.is_empty());
    for (_, row, placed) in covered {
        assert_eq!(row, GROUND_ROWS.start - 1);
        assert_eq!(placed.center, Center::Empty);
    }
}

fn placed_cells(automapped: &Automapped) -> Vec<(usize, usize, PlacedTile)> {
    (0..automapped.height)
        .flat_map(|row| (0..automapped.width).map(move |column| (column, row)))
        .filter_map(|(column, row)| match automapped.cell(column, row) {
            PreviewCell::Placed(placed) => Some((column, row, placed)),
            _ => None,
        })
        .collect()
}

fn group_cells(automapped: &Automapped) -> Vec<(usize, usize, PlacedTile)> {
    placed_cells(automapped)
        .into_iter()
        .filter(|(_, _, placed)| DOMINO_TILES.contains(&placed.index))
        .collect()
}

fn shape_of(
    width: usize,
    height: usize,
    solid: impl IntoIterator<Item = (usize, usize)>,
) -> CustomShape {
    let mut shape = CustomShape::empty(width, height);
    for (column, row) in solid {
        shape.set(column, row, true);
    }

    shape
}

fn on_floor_domino(mods: TileMods, percent: f32) -> TileGroup {
    let mut floor = shaped(&DOMINO_TILES, mods, percent);
    floor.apply_preset(GroupPreset::OnFloor);

    floor
}

#[test]
fn an_empty_center_tile_never_lands_right_of_a_group_tile_on_air() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(offset_of("left"), NeighborState::Full);
    let tiles = [(
        AIR_DECORATION,
        TileRule {
            center: RuleCenter::Empty,
            ..TileRule::new(neighborhood)
        },
    )];
    let floor = on_floor_domino(TileMods::default(), Chance::FULL.percent());
    let source = render(&[RuleSet {
        image_stem: IMAGE_STEM,
        name: "Test",
        tiles: &tiles,
        deactivated: &[],
        groups: std::slice::from_ref(&floor),
        chance_mode: ChanceMode::Normalize,
    }])
    .unwrap();

    let wall = (0..GROUND_ROWS.start).map(|row| (0, row));
    let floor_cells = GROUND_ROWS.flat_map(|row| (0..SHAPE_SIZE).map(move |column| (column, row)));
    let shape = shape_of(SHAPE_SIZE, SHAPE_SIZE, wall.chain(floor_cells));

    let automapped = automap(
        &compiled(source),
        Sample::Custom,
        &shape,
        SEED,
        UNMATCHED_MARKER,
    )
    .unwrap();

    assert_eq!(automapped.cell(1, 1), placed(AIR_DECORATION, Center::Empty));
    let covered: Vec<(usize, usize)> = group_cells(&automapped)
        .into_iter()
        .map(|(column, row, _)| (column, row))
        .collect();
    assert_eq!(covered, [(2, 1), (3, 1)]);
}

#[test]
fn a_rotating_on_floor_group_at_half_chance_only_covers_air() {
    let floor = on_floor_domino(TileMods::new(false, false, true), 50.0);
    let rules = compiled_group(&floor);

    let walls = (0..WALLED_FLOOR_HEIGHT).flat_map(|row| [(0, row), (WALLED_FLOOR_WIDTH - 1, row)]);
    let floor_cells = (WALLED_FLOOR_HEIGHT - GROUND_ROWS.len()..WALLED_FLOOR_HEIGHT)
        .flat_map(|row| (0..WALLED_FLOOR_WIDTH).map(move |column| (column, row)));
    let shape = shape_of(
        WALLED_FLOOR_WIDTH,
        WALLED_FLOOR_HEIGHT,
        walls.chain(floor_cells),
    );

    let group_tiles: Vec<PlacedTile> = DOMINO_SEEDS
        .flat_map(|seed| {
            let automapped = automap(&rules, Sample::Custom, &shape, seed, UNMATCHED_MARKER);
            group_cells(&automapped.unwrap())
        })
        .map(|(_, _, placed)| placed)
        .collect();

    assert!(!group_tiles.is_empty());
    assert!(group_tiles.iter().all(|p| p.center == Center::Empty));
}

#[test]
fn rules_with_an_embedded_blueprint_automap_like_the_plain_rules() {
    let rules = compiled(empty_above_ground());
    let sets = RuleSets::new("Test".to_owned());
    let json = blueprint::to_json(&sets, IMAGE_STEM).unwrap();
    let embedded = blueprint::embed(&rules, &json);
    let automap_of = |text: &str| {
        automap(
            text,
            Sample::default(),
            &ground_shape(),
            SEED,
            UNMATCHED_MARKER,
        )
        .unwrap()
    };

    assert_eq!(automap_of(&embedded), automap_of(&rules));
}
