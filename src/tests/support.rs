use crate::model::neighbor::{NEIGHBOR_COUNT, NEIGHBORS, NeighborState, Neighborhood};
use crate::model::tile::TileMods;
use crate::model::transform::{Orientations, Transform};

pub fn index_of(rpp_name: &str) -> usize {
    NEIGHBORS
        .iter()
        .position(|neighbor| neighbor.rpp_name == rpp_name)
        .unwrap_or_else(|| panic!("no neighbor is called {rpp_name}"))
}

pub fn named(neighborhood: &Neighborhood, state: NeighborState) -> Vec<&'static str> {
    NEIGHBORS
        .iter()
        .enumerate()
        .filter(|(index, _)| neighborhood.state(*index) == state)
        .map(|(_, neighbor)| neighbor.rpp_name)
        .collect()
}

pub fn mods(can_x_flip: bool, can_y_flip: bool, can_rotate: bool) -> TileMods {
    TileMods::new(can_x_flip, can_y_flip, can_rotate)
}

pub const HALF_TURN: Transform = Transform {
    x_flip: true,
    y_flip: true,
    rot: false,
};

pub const QUARTER_TURN: Transform = Transform {
    x_flip: false,
    y_flip: false,
    rot: true,
};

pub fn half_turn() -> Orientations {
    [HALF_TURN].into_iter().collect()
}

pub fn all_distinct() -> Neighborhood {
    let mut neighborhood = Neighborhood::default();
    for index in 0..NEIGHBOR_COUNT {
        neighborhood.set_state(index, NeighborState::Is(index + 1));
    }

    neighborhood
}

/// Solid to the right and below, open above and to the left.
pub fn outer_corner() -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    for name in ["right", "bottom", "bottomRight"] {
        neighborhood.set_state(index_of(name), NeighborState::Full);
    }
    for name in ["top", "left"] {
        neighborhood.set_state(index_of(name), NeighborState::Empty);
    }

    neighborhood
}
