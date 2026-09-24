use crate::model::transform::Transform;
use crate::preview::{PlacedTile, PreviewError, SAMPLE, automap};

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
    let automapped = automap(FLIPPED_EVERYWHERE).unwrap();
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
    let automapped = automap(TOP_EDGES).unwrap();

    assert_eq!(automapped.tile(2, 1), plain(7));
    assert_eq!(automapped.tile(2, 2), plain(2));
    assert_eq!(automapped.tile(0, 0), None);
}

#[test]
fn unreadable_rules_are_reported() {
    assert!(matches!(
        automap("[Test]\nIndex banana\n"),
        Err(PreviewError::Syntax(_))
    ));
}

#[test]
fn rules_without_a_rule_set_are_reported() {
    assert!(matches!(automap(""), Err(PreviewError::NoRuleSet)));
}
