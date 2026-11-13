use crate::model::neighbor::{
    CENTER, Extent, NEIGHBORS, NeighborState, Neighborhood, Side, neighbor_at, offset_label,
};

const DIRECT_NEIGHBORS: usize = 8;

#[test]
fn offsets_cover_the_ring_without_the_center() {
    let mut offsets: Vec<(i32, i32)> = NEIGHBORS
        .iter()
        .map(|neighbor| (neighbor.dx, neighbor.dy))
        .collect();
    offsets.sort();

    let mut expected: Vec<(i32, i32)> = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .filter(|&offset| offset != (0, 0))
        .collect();
    expected.sort();

    assert_eq!(offsets.len(), DIRECT_NEIGHBORS);
    assert_eq!(offsets, expected);
}

#[test]
fn rpp_names_are_unique() {
    let mut names: Vec<&str> = NEIGHBORS.iter().map(|neighbor| neighbor.rpp_name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), DIRECT_NEIGHBORS);
}

#[test]
fn toggling_solid_moves_empty_to_full_and_swaps_full_with_any() {
    assert_eq!(NeighborState::Empty.toggled_solid(), NeighborState::Full);
    assert_eq!(NeighborState::Full.toggled_solid(), NeighborState::Any);
    assert_eq!(NeighborState::Any.toggled_solid(), NeighborState::Full);
}

#[test]
fn toggling_solid_turns_a_tile_requirement_into_full() {
    assert_eq!(NeighborState::Is(17).toggled_solid(), NeighborState::Full);
}

#[test]
fn only_tile_requirements_expose_a_tile() {
    assert_eq!(NeighborState::Any.tile(), None);
    assert_eq!(NeighborState::Is(1).tile(), Some(1));
}

#[test]
fn uniform_sets_every_state() {
    let neighborhood = Neighborhood::uniform(NeighborState::Any);
    let states: Vec<NeighborState> = neighborhood.cells().map(|(_, state)| state).collect();

    assert_eq!(states, [NeighborState::Any; DIRECT_NEIGHBORS]);
}

#[test]
fn cells_walk_the_rectangle_in_reading_order_without_the_center() {
    let extent = Extent {
        left: 2,
        right: 1,
        up: 1,
        down: 1,
    };
    let neighborhood = Neighborhood::with_extent(extent, NeighborState::Any);
    let offsets: Vec<(i32, i32)> = neighborhood.cells().map(|(offset, _)| offset).collect();

    assert_eq!(offsets.len(), extent.cell_count());
    assert_eq!(offsets[0], (-2, -1));
    assert_eq!(offsets[2], (0, -1));
    assert!(!offsets.contains(&CENTER));
    assert_eq!(offsets.last(), Some(&(1, 1)));
}

#[test]
fn growing_adds_any_cells_and_shrinking_drops_them() {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Full);
    neighborhood.set_state((1, 0), NeighborState::Empty);

    let grown = neighborhood.grown(Side::Right);

    assert_eq!(grown.extent().right, 2);
    assert_eq!(grown.state((2, 1)), NeighborState::Any);
    assert_eq!(grown.state((1, 0)), NeighborState::Empty);
    assert_eq!(grown.shrunk(Side::Right), Some(neighborhood));
}

#[test]
fn the_three_by_three_core_cannot_be_shrunk() {
    let neighborhood = Neighborhood::default();

    for side in Side::ALL {
        assert!(!neighborhood.can_shrink(side));
        assert_eq!(neighborhood.shrunk(side), None);
    }
}

#[test]
fn constraints_ignore_any_cells_and_extent() {
    let mut small = Neighborhood::uniform(NeighborState::Any);
    small.set_state((1, 0), NeighborState::Full);
    let grown = small.grown(Side::Up).grown(Side::Left);

    assert_ne!(small, grown);
    assert_eq!(small.constraints(), grown.constraints());
    assert_eq!(grown.specificity(), 1);
}

#[test]
fn direct_neighbors_are_named_and_outer_cells_are_not() {
    assert_eq!(
        neighbor_at((0, -1)).map(|neighbor| neighbor.rpp_name),
        Some("top")
    );
    assert!(neighbor_at((2, 0)).is_none());
    assert!(neighbor_at(CENTER).is_none());
}

#[test]
fn outer_cells_are_labelled_by_distance() {
    assert_eq!(offset_label((-1, 0)), "left");
    assert_eq!(offset_label((2, -1)), "2 right, 1 up");
    assert_eq!(offset_label((0, 3)), "3 down");
}
