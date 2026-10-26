use std::ops::Range;
use std::time::{Duration, Instant};

use twmap::Tile;
use twmap::ndarray::Array2;

use crate::export::compiler::RulesCompiler;
use crate::export::r_source::{RuleSet, output_file, render};
use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::tile::{Centre, Chance, TILE_COUNT, TileRule};
use crate::model::transform::Transform;
use crate::preview::{
    Automapped, CustomShape, FALLBACK_UNMATCHED_MARKER, PlacedTile, PreviewCell, PreviewError,
    Sample, automap, unmatched_marker,
};
use crate::tests::support::index_of;

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

fn placed(index: usize, centre: Centre) -> PreviewCell {
    PreviewCell::Placed(PlacedTile {
        index,
        transform: Transform::IDENTITY,
        centre,
    })
}

fn plain(index: usize) -> PreviewCell {
    placed(index, Centre::Full)
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
    neighborhood.set_state(index_of("bottom"), NeighborState::Full);
    let tiles = [(
        AIR_DECORATION,
        TileRule {
            centre: Centre::Empty,
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

fn unmatched_count(automapped: &Automapped) -> usize {
    (0..automapped.height)
        .flat_map(|row| (0..automapped.width).map(move |column| (column, row)))
        .filter(|&(column, row)| automapped.cell(column, row) == PreviewCell::Unmatched)
        .count()
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
        centre: Centre::Full,
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
        let tiles = sample.tiles(&custom, SOLID_ID).unwrap();
        let (edge_column, edge_row) = first_cell(sample, true, false);
        let (inner_column, inner_row) = first_cell(sample, true, true);
        let (air_column, air_row) = first_cell(sample, false, false);

        let top_edges = tiles
            .indexed_iter()
            .filter(|((row, column), tile)| {
                tile.id != 0 && *row > 0 && !is_solid(&tiles, *column, row - 1)
            })
            .count();
        let solid = tiles.iter().filter(|tile| tile.id != 0).count();

        assert_eq!(automapped.cell(edge_column, edge_row), plain(7));
        assert_eq!(
            automapped.cell(inner_column, inner_row),
            PreviewCell::Unmatched
        );
        assert_eq!(automapped.cell(air_column, air_row), PreviewCell::Air);
        assert_eq!(unmatched_count(&automapped), solid - top_edges);
    }
}

#[test]
fn an_exported_empty_centre_rule_places_into_the_air_above_ground() {
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
        placed(AIR_DECORATION, Centre::Empty)
    );
    assert_eq!(automapped.cell(0, 0), PreviewCell::Air);
    assert_eq!(
        automapped.cell(0, GROUND_ROWS.start),
        PreviewCell::Unmatched
    );
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

#[test]
fn the_unmatched_marker_is_the_first_tile_nothing_claims() {
    assert_eq!(unmatched_marker(&Project::default()), 1);
}

#[test]
fn the_unmatched_marker_skips_active_rules_and_group_footprints() {
    let mut project = Project::default();
    project.add_rule(1, some_rule());
    project.add_group(TileGroup {
        name: "bones".to_owned(),
        top_left: 2,
        width: 2,
        height: 1,
        mode: GroupMode::Fill,
        chance: Chance::FULL,
    });

    assert_eq!(unmatched_marker(&project), 4);
}

#[test]
fn the_unmatched_marker_skips_tiles_a_neighbor_refers_to() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(0, NeighborState::Is(1));
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
