use std::collections::BTreeMap;

use crate::model::group::{CellState, GroupError, GroupPreset, TileGroup, validate_all};
use crate::model::tile::{TILE_COUNT, TILESET_SIDE};
use crate::model::transform::Transform;
use crate::tests::support::{QUARTER_TURN, mods};

fn group(name: &str, top_left: usize, width: usize, height: usize) -> TileGroup {
    let bottom_right = top_left + (height - 1) * TILESET_SIDE + width - 1;

    shaped(name, TileGroup::rect_tiles(top_left, bottom_right))
}

fn shaped(name: &str, tiles: impl IntoIterator<Item = usize>) -> TileGroup {
    TileGroup::new(name.to_owned(), tiles.into_iter().collect())
}

fn l_shape() -> TileGroup {
    shaped("a", [65, 81, 82])
}

fn decorated(group: &TileGroup) -> TileGroup {
    let mut decorated = group.clone();
    decorated.apply_preset(GroupPreset::Decorate);

    decorated
}

fn ring_of(group: &TileGroup) -> Vec<(i32, i32)> {
    ring_of_copy(group, 0)
}

fn ring_of_copy(group: &TileGroup, copy: usize) -> Vec<(i32, i32)> {
    let copy = decorated(group).copies().remove(copy);

    copy.full
        .into_iter()
        .filter(|cell| !copy.cells.contains(cell))
        .collect()
}

fn states_of(group: &TileGroup, wanted: CellState) -> Vec<(i32, i32)> {
    group
        .neighbors
        .iter()
        .filter(|(_, state)| **state == wanted)
        .map(|(&cell, _)| cell)
        .collect()
}

#[test]
fn a_group_without_flags_has_only_its_own_copy() {
    let copies = l_shape().copies();

    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].transform, Transform::IDENTITY);
    assert_eq!(copies[0].cells, [(0, 0), (0, 1), (1, 1)]);
}

#[test]
fn a_rotated_copy_is_anchored_on_its_own_topmost_leftmost_tile() {
    let rotating = TileGroup {
        mods: mods(false, false, true),
        ..l_shape()
    };
    let copies = rotating.copies();

    assert_eq!(copies.len(), 4);
    assert_eq!(copies[1].transform, QUARTER_TURN);
    assert_eq!(copies[1].cells, [(0, 0), (1, 0), (0, 1)]);
}

#[test]
fn the_ring_of_a_copy_follows_its_turned_shape() {
    let rotating = TileGroup {
        mods: mods(false, false, true),
        ..l_shape()
    };
    let ring = ring_of_copy(&rotating, 1);

    assert!(ring.contains(&(1, 1)));
    assert!(!ring.contains(&(0, 1)));
}

#[test]
fn a_group_may_cover_the_last_tile() {
    let corner = group("a", TILE_COUNT - 18, 2, 2);

    assert!(corner.covers(TILE_COUNT - 1));
    assert_eq!(corner.validate(), Ok(()));
}

#[test]
fn the_footprint_walks_the_tiles_in_reading_order() {
    assert_eq!(group("a", 64, 2, 2).footprint(), [64, 65, 80, 81]);
    assert_eq!(group("a", 5, 3, 1).footprint(), [5, 6, 7]);
    assert_eq!(l_shape().footprint(), [65, 81, 82]);
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
fn the_anchor_is_the_smallest_tile() {
    assert_eq!(group("a", 64, 2, 2).anchor(), Some(64));
    assert_eq!(shaped("a", [82, 65, 81]).anchor(), Some(65));
    assert_eq!(shaped("a", []).anchor(), None);
}

#[test]
fn only_a_filled_bounding_box_is_a_rectangle() {
    assert_eq!(group("a", 64, 2, 2).as_rect(), Some((64, 81)));
    assert_eq!(group("a", 1, 3, 1).as_rect(), Some((1, 3)));
    assert_eq!(l_shape().as_rect(), None);
    assert_eq!(shaped("a", [64, 66]).as_rect(), None);
}

#[test]
fn the_badge_sits_on_the_leftmost_tile_of_the_bottom_row() {
    assert_eq!(l_shape().bottom_left_tile(), Some(81));
    assert_eq!(group("a", 64, 2, 2).bottom_left_tile(), Some(80));
}

#[test]
fn the_size_label_names_rectangles_by_size_and_others_by_count() {
    assert_eq!(group("a", 64, 3, 2).size_label(), "3×2");
    assert_eq!(l_shape().size_label(), "3 tiles");
}

#[test]
fn the_decorate_ring_surrounds_the_footprint() {
    let single = ring_of(&group("a", 64, 1, 1));
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

    let two_by_two = ring_of(&group("a", 64, 2, 2));
    assert_eq!(two_by_two.len(), 12);
    assert!(!two_by_two.contains(&(0, 0)));
    assert!(!two_by_two.contains(&(1, 1)));
    assert!(two_by_two.contains(&(2, 2)));
    assert!(two_by_two.contains(&(-1, -1)));
}

#[test]
fn the_ring_of_an_l_shape_hugs_the_notch_and_stays_relative_to_the_anchor() {
    assert_eq!(
        ring_of(&l_shape()),
        [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (2, 0),
            (-1, 1),
            (2, 1),
            (-1, 2),
            (0, 2),
            (1, 2),
            (2, 2),
        ]
    );
}

#[test]
fn a_new_group_is_a_fill_group() {
    let fresh = l_shape();

    assert_eq!(fresh.preset(), Some(GroupPreset::Fill));
    assert_eq!(states_of(&fresh, CellState::Full).len(), 3);
    assert_eq!(states_of(&fresh, CellState::Any).len(), 12);
}

#[test]
fn every_preset_is_recognised_after_it_is_applied() {
    for preset in GroupPreset::ALL {
        let mut group = l_shape();
        group.apply_preset(preset);

        assert_eq!(group.preset(), Some(preset));
    }
}

#[test]
fn a_hand_edited_map_matches_no_preset() {
    let mut group = l_shape();
    group.neighbors.insert((1, 4), CellState::Empty);

    assert_eq!(group.preset(), None);
}

#[test]
fn the_decorate_preset_makes_the_whole_ring_solid() {
    let mut group = l_shape();
    group.apply_preset(GroupPreset::Decorate);

    assert_eq!(states_of(&group, CellState::Full).len(), 15);
    assert!(states_of(&group, CellState::Any).is_empty());
}

#[test]
fn the_on_floor_preset_needs_air_inside_and_ground_below() {
    let mut group = l_shape();
    group.apply_preset(GroupPreset::OnFloor);
    let copy = &group.copies()[0];

    assert_eq!(copy.empty, [(0, 0), (0, 1), (1, 1)]);
    assert_eq!(copy.full, [(0, 2), (1, 2)]);
}

#[test]
fn reshaping_keeps_states_of_cells_that_keep_their_role() {
    let mut group = group("a", 64, 2, 1);
    group.neighbors.insert((0, 4), CellState::Any);
    group.neighbors.insert((2, 3), CellState::Full);
    group.neighbors.insert((-1, 4), CellState::Empty);

    group.set_tiles(TileGroup::rect_tiles(64, 66));

    let expected = BTreeMap::from([
        ((2, 4), CellState::Full),
        ((0, 4), CellState::Any),
        ((2, 3), CellState::Full),
        ((-1, 4), CellState::Empty),
        ((3, 4), CellState::Any),
    ]);
    for (cell, state) in expected {
        assert_eq!(group.neighbors[&cell], state, "{cell:?}");
    }
    assert_eq!(group.neighbors[&(1, 4)], CellState::Full);
}

#[test]
fn reshaping_drops_cells_that_are_no_longer_part_of_the_group() {
    let mut group = group("a", 64, 3, 1);

    group.set_tiles(TileGroup::rect_tiles(64, 65));

    assert!(!group.neighbors.contains_key(&(3, 4)));
    assert_eq!(group.neighbors[&(2, 4)], CellState::Any);
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
    assert_eq!(shaped("a", []).validate(), Err(GroupError::TooSmall));
    assert_eq!(group("a", 1, 1, 1).validate(), Err(GroupError::TooSmall));

    assert!(group("a", 1, 2, 1).validate().is_ok());
    assert!(group("a", 1, 1, 2).validate().is_ok());
    assert!(shaped("a", [1, 33]).validate().is_ok());
}

#[test]
fn a_group_cannot_cover_the_air_tile_or_leave_the_tileset() {
    assert_eq!(
        group("a", 0, 1, 2).validate(),
        Err(GroupError::CoversAirTile)
    );

    assert_eq!(
        shaped("a", [255, TILE_COUNT]).validate(),
        Err(GroupError::OutOfBounds(TILE_COUNT))
    );
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
