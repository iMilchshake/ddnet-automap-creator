use crate::model::transform::Transform;
use crate::preview::{FIRST_SEED, PlacedTile, PreviewError, SAMPLE, automap, next_seed};

const FLIPPED_EVERYWHERE: &str = "[Test]\nIndex 5 XFLIP\n";
const TOP_EDGES: &str = "[Test]\nIndex 2\nIndex 7\nPos 0 -1 EMPTY\n";

fn plain(index: usize) -> Option<PlacedTile> {
    Some(PlacedTile {
        index,
        transform: Transform::IDENTITY,
    })
}

#[test]
fn every_sample_row_has_the_same_width() {
    assert!(SAMPLE.iter().all(|row| row.len() == SAMPLE[0].len()));
}

#[test]
fn a_rule_without_conditions_replaces_every_solid_cell() {
    let automapped = automap(FLIPPED_EVERYWHERE, FIRST_SEED).unwrap();
    let flipped = Some(PlacedTile {
        index: 5,
        transform: Transform {
            x_flip: true,
            y_flip: false,
            rot: false,
        },
    });

    for (row, line) in SAMPLE.iter().enumerate() {
        for (column, cell) in line.chars().enumerate() {
            let expected = match cell {
                '#' => flipped,
                _ => None,
            };
            assert_eq!(automapped.tile(column, row), expected, "{column},{row}");
        }
    }
}

#[test]
fn later_rules_win_where_their_conditions_hold() {
    let automapped = automap(TOP_EDGES, FIRST_SEED).unwrap();

    assert_eq!(automapped.tile(2, 1), plain(7));
    assert_eq!(automapped.tile(2, 2), plain(2));
    assert_eq!(automapped.tile(0, 0), None);
}

#[test]
fn a_chance_rpp_rounds_to_random_1_places_everywhere() {
    let automapped = automap("[Test]\nIndex 5\nRandom 1\n", FIRST_SEED).unwrap();

    assert_eq!(automapped.tile(2, 1), plain(5));
}

#[test]
fn unreadable_rules_are_reported() {
    assert!(matches!(
        automap("[Test]\nIndex banana\n", FIRST_SEED),
        Err(PreviewError::Syntax(_))
    ));
}

#[test]
fn rules_without_a_rule_set_are_reported() {
    assert!(matches!(
        automap("", FIRST_SEED),
        Err(PreviewError::NoRuleSet)
    ));
}

#[test]
fn another_seed_rolls_chances_differently() {
    let halves = "[Test]\nIndex 5\nRandom 2\n";
    let first = automap(halves, FIRST_SEED).unwrap();
    let second = automap(halves, next_seed(FIRST_SEED)).unwrap();

    assert_ne!(first, second);
    assert_eq!(first, automap(halves, FIRST_SEED).unwrap());
}

#[test]
fn the_next_seed_counts_up_and_never_hands_twmap_a_zero() {
    assert_eq!(next_seed(FIRST_SEED), FIRST_SEED + 1);
    assert_eq!(next_seed(u32::MAX), FIRST_SEED);
}
