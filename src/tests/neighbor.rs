use crate::model::neighbor::{
    NEIGHBOR_COUNT, NEIGHBORS, NeighborState, Neighborhood, neighbor_index_at,
};

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

    assert_eq!(offsets.len(), NEIGHBOR_COUNT);
    assert_eq!(offsets, expected);
}

#[test]
fn rpp_names_are_unique() {
    let mut names: Vec<&str> = NEIGHBORS.iter().map(|neighbor| neighbor.rpp_name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), NEIGHBOR_COUNT);
}

#[test]
fn cycling_a_state_three_times_returns_to_the_start() {
    for state in [
        NeighborState::Empty,
        NeighborState::Full,
        NeighborState::Any,
    ] {
        assert_eq!(state.next().next().next(), state);
        assert_eq!(state.next().previous(), state);
    }
}

#[test]
fn uniform_sets_every_state() {
    let neighborhood = Neighborhood::uniform(NeighborState::Any);
    assert_eq!(neighborhood.states(), &[NeighborState::Any; NEIGHBOR_COUNT]);
}

#[test]
fn walking_the_three_by_three_grid_skips_the_center_in_reading_order() {
    let indices: Vec<Option<usize>> = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| neighbor_index_at(dx, dy)))
        .collect();

    assert_eq!(
        indices,
        [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            None,
            Some(4),
            Some(5),
            Some(6),
            Some(7),
        ]
    );
}
