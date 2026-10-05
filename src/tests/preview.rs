use twmap::Tile;
use twmap::ndarray::Array2;

use crate::model::group::{GroupMode, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, MASK_TILE, TileRule};
use crate::model::transform::Transform;
use crate::preview::{
    Automapped, FALLBACK_UNMATCHED_MARKER, PlacedTile, PreviewCell, PreviewError, Sample, automap,
    unmatched_marker,
};

const SEED: u32 = 42;
const SOLID_ID: u8 = 200;
const UNMATCHED_MARKER: u8 = SOLID_ID;
const FLIPPED_EVERYWHERE: &str = "[Test]\nIndex 5 XFLIP\n";
const TOP_EDGES: &str = "[Test]\nIndex 2\nIndex 7\nPos 0 -1 EMPTY\n";
const ONLY_TOP_EDGES: &str = "[Test]\nIndex 7\nPos 0 -1 EMPTY\n";
const TILE_ONE_ON_TOP_EDGES: &str = "[Test]\nIndex 1\nPos 0 -1 EMPTY\n";

fn plain(index: usize) -> PreviewCell {
    PreviewCell::Placed(PlacedTile {
        index,
        transform: Transform::IDENTITY,
    })
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

fn first_cell(sample: Sample, solid: bool, solid_above: bool) -> (usize, usize) {
    let tiles = sample.tiles(SOLID_ID).unwrap();
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
fn every_sample_has_ground_and_air() {
    for sample in Sample::ALL {
        let tiles = sample.tiles(SOLID_ID).unwrap();

        assert!(tiles.iter().any(|tile| tile.id != 0), "{}", sample.name());
        assert!(tiles.iter().any(|tile| tile.id == 0), "{}", sample.name());
    }
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
    });

    for sample in Sample::ALL {
        let automapped = automap(FLIPPED_EVERYWHERE, sample, SEED, UNMATCHED_MARKER).unwrap();

        for ((row, column), tile) in sample.tiles(SOLID_ID).unwrap().indexed_iter() {
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
        let automapped = automap(TOP_EDGES, sample, SEED, UNMATCHED_MARKER).unwrap();
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
            SEED,
            UNMATCHED_MARKER
        ),
        Err(PreviewError::Syntax(_))
    ));
}

#[test]
fn rules_without_a_rule_set_are_reported() {
    assert!(matches!(
        automap("", Sample::default(), SEED, UNMATCHED_MARKER),
        Err(PreviewError::NoRuleSet)
    ));
}

#[test]
fn another_seed_rolls_chances_differently() {
    let halves = "[Test]\nIndex 5\nRandom 2\n";
    let sample = Sample::default();
    let first = automap(halves, sample, SEED, UNMATCHED_MARKER).unwrap();
    let second = automap(halves, sample, SEED + 1, UNMATCHED_MARKER).unwrap();

    assert_ne!(first, second);
    assert_eq!(
        first,
        automap(halves, sample, SEED, UNMATCHED_MARKER).unwrap()
    );
}

#[test]
fn solid_cells_no_rule_touched_are_unmatched() {
    for sample in Sample::ALL {
        let automapped = automap(ONLY_TOP_EDGES, sample, SEED, UNMATCHED_MARKER).unwrap();
        let tiles = sample.tiles(SOLID_ID).unwrap();
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
fn a_rule_placing_tile_one_is_not_mistaken_for_unmatched() {
    let sample = Sample::default();
    let automapped = automap(TILE_ONE_ON_TOP_EDGES, sample, SEED, UNMATCHED_MARKER).unwrap();
    let (column, row) = first_cell(sample, true, false);

    assert_eq!(automapped.cell(column, row), plain(1));
}

#[test]
fn the_unmatched_marker_is_the_first_tile_nothing_claims() {
    assert_eq!(unmatched_marker(&Project::default()), 1);
}

#[test]
fn the_unmatched_marker_skips_active_rules_and_group_footprints() {
    let mut project = Project::default();
    project.set_rule(1, some_rule());
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
fn the_unmatched_marker_ignores_deactivated_rules() {
    let mut project = Project::default();
    project.set_rule(1, some_rule());
    project.deactivate(1);

    assert_eq!(unmatched_marker(&project), 1);
}

#[test]
fn the_unmatched_marker_falls_back_when_every_tile_is_used() {
    let mut project = Project::default();
    for tile in 1..MASK_TILE {
        project.set_rule(tile, some_rule());
    }

    assert_eq!(unmatched_marker(&project), FALLBACK_UNMATCHED_MARKER);
}
