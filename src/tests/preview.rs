use twmap::Tile;
use twmap::ndarray::Array2;

use crate::model::transform::Transform;
use crate::preview::{PlacedTile, PreviewError, Sample, automap};

const SEED: u32 = 42;
const FLIPPED_EVERYWHERE: &str = "[Test]\nIndex 5 XFLIP\n";
const TOP_EDGES: &str = "[Test]\nIndex 2\nIndex 7\nPos 0 -1 EMPTY\n";

fn plain(index: usize) -> Option<PlacedTile> {
    Some(PlacedTile {
        index,
        transform: Transform::IDENTITY,
    })
}

fn is_solid(tiles: &Array2<Tile>, column: usize, row: usize) -> bool {
    tiles[(row, column)].id != 0
}

fn first_cell(sample: Sample, solid: bool, solid_above: bool) -> (usize, usize) {
    let tiles = sample.tiles().unwrap();
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
        let tiles = sample.tiles().unwrap();

        assert!(tiles.iter().any(|tile| tile.id != 0), "{}", sample.name());
        assert!(tiles.iter().any(|tile| tile.id == 0), "{}", sample.name());
    }
}

#[test]
fn a_rule_without_conditions_replaces_every_solid_cell() {
    let flipped = Some(PlacedTile {
        index: 5,
        transform: Transform {
            x_flip: true,
            y_flip: false,
            rot: false,
        },
    });

    for sample in Sample::ALL {
        let automapped = automap(FLIPPED_EVERYWHERE, sample, SEED).unwrap();

        for ((row, column), tile) in sample.tiles().unwrap().indexed_iter() {
            let expected = match tile.id {
                0 => None,
                _ => flipped,
            };
            assert_eq!(automapped.tile(column, row), expected, "{column},{row}");
        }
    }
}

#[test]
fn later_rules_win_where_their_conditions_hold() {
    for sample in Sample::ALL {
        let automapped = automap(TOP_EDGES, sample, SEED).unwrap();
        let (edge_column, edge_row) = first_cell(sample, true, false);
        let (inner_column, inner_row) = first_cell(sample, true, true);
        let (air_column, air_row) = first_cell(sample, false, false);

        assert_eq!(automapped.tile(edge_column, edge_row), plain(7));
        assert_eq!(automapped.tile(inner_column, inner_row), plain(2));
        assert_eq!(automapped.tile(air_column, air_row), None);
    }
}

#[test]
fn a_chance_rpp_rounds_to_random_1_places_everywhere() {
    let sample = Sample::default();
    let automapped = automap("[Test]\nIndex 5\nRandom 1\n", sample, SEED).unwrap();
    let (column, row) = first_cell(sample, true, false);

    assert_eq!(automapped.tile(column, row), plain(5));
}

#[test]
fn unreadable_rules_are_reported() {
    assert!(matches!(
        automap("[Test]\nIndex banana\n", Sample::default(), SEED),
        Err(PreviewError::Syntax(_))
    ));
}

#[test]
fn rules_without_a_rule_set_are_reported() {
    assert!(matches!(
        automap("", Sample::default(), SEED),
        Err(PreviewError::NoRuleSet)
    ));
}

#[test]
fn another_seed_rolls_chances_differently() {
    let halves = "[Test]\nIndex 5\nRandom 2\n";
    let sample = Sample::default();
    let first = automap(halves, sample, SEED).unwrap();
    let second = automap(halves, sample, SEED + 1).unwrap();

    assert_ne!(first, second);
    assert_eq!(first, automap(halves, sample, SEED).unwrap());
}
