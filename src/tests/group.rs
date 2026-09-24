use crate::model::group::{GroupError, GroupMode, TileGroup, validate_all};
use crate::model::tile::{Chance, MASK_TILE};

fn group(name: &str, top_left: usize, width: usize, height: usize) -> TileGroup {
    TileGroup {
        name: name.to_owned(),
        top_left,
        width,
        height,
        mode: GroupMode::Fill,
        chance: Chance::FULL,
    }
}

#[test]
fn a_group_over_the_mask_tile_is_refused() {
    let corner = group("a", MASK_TILE - 17, 2, 2);

    assert!(corner.covers(MASK_TILE));
    assert!(matches!(corner.validate(), Err(GroupError::CoversMask)));
}

#[test]
fn the_bottom_right_closes_the_rectangle() {
    assert_eq!(group("a", 64, 1, 1).bottom_right(), 64);
    assert_eq!(group("a", 64, 2, 2).bottom_right(), 81);
    assert_eq!(group("a", 1, 3, 2).bottom_right(), 19);
}

#[test]
fn the_footprint_walks_the_rectangle_in_reading_order() {
    assert_eq!(group("a", 64, 2, 2).footprint(), [64, 65, 80, 81]);
    assert_eq!(group("a", 5, 3, 1).footprint(), [5, 6, 7]);
}

#[test]
fn a_group_covers_exactly_its_footprint() {
    let two_by_two = group("a", 64, 2, 2);

    for tile in two_by_two.footprint() {
        assert!(two_by_two.covers(tile), "{tile}");
    }
    for tile in [63, 66, 48, 96, 79, 82] {
        assert!(!two_by_two.covers(tile), "{tile}");
    }
}

#[test]
fn the_decorate_ring_surrounds_the_footprint() {
    let single = group("a", 64, 1, 1).ring_offsets();
    assert_eq!(
        single,
        [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ]
    );

    let two_by_two = group("a", 64, 2, 2).ring_offsets();
    assert_eq!(two_by_two.len(), 12);
    assert!(!two_by_two.contains(&(0, 0)));
    assert!(!two_by_two.contains(&(1, 1)));
    assert!(two_by_two.contains(&(2, 2)));
    assert!(two_by_two.contains(&(-1, -1)));
}

#[test]
fn names_must_be_letters_and_digits_starting_with_a_letter() {
    assert!(group("bones", 1, 2, 1).validate().is_ok());
    assert!(group("group0", 1, 2, 1).validate().is_ok());

    for bad in ["", "0bones", "big_bones", "bones!", "bönes", " bones"] {
        assert_eq!(
            group(bad, 1, 2, 1).validate(),
            Err(GroupError::Name(bad.to_owned())),
            "{bad}"
        );
    }
}

#[test]
fn a_group_must_cover_at_least_two_tiles() {
    assert_eq!(group("a", 1, 0, 1).validate(), Err(GroupError::TooSmall));
    assert_eq!(group("a", 1, 1, 1).validate(), Err(GroupError::TooSmall));

    assert!(group("a", 1, 2, 1).validate().is_ok());
    assert!(group("a", 1, 1, 2).validate().is_ok());
}

#[test]
fn a_group_cannot_start_at_zero_or_leave_the_tileset() {
    assert_eq!(
        group("a", 0, 1, 2).validate(),
        Err(GroupError::StartsAtZero)
    );

    assert!(group("a", 15, 2, 1).validate().is_err());
    assert!(group("a", 240, 1, 2).validate().is_err());
    assert!(group("a", 238, 1, 2).validate().is_ok());
}

#[test]
fn a_list_refuses_duplicate_names_and_shared_tiles() {
    let listed = [group("bones", 64, 2, 2), group("bones", 100, 2, 1)];
    assert_eq!(
        validate_all(&listed),
        Err(GroupError::DuplicateName("bones".to_owned()))
    );

    let overlapping = [group("bones", 64, 2, 2), group("pipes", 65, 2, 2)];
    assert_eq!(
        validate_all(&overlapping),
        Err(GroupError::Overlap {
            first: "bones".to_owned(),
            second: "pipes".to_owned(),
        })
    );

    let apart = [group("bones", 64, 2, 2), group("pipes", 66, 2, 2)];
    assert!(validate_all(&apart).is_ok());
}
