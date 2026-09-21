//! Neighbor index convention, y pointing down:
//!
//! ```text
//! 0 TL   1 T    2 TR
//! 3 L   [self]  4 R
//! 5 BL   6 B    7 BR
//! ```

pub const NEIGHBOR_COUNT: usize = 8;

/// What a rule demands of one neighboring cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighborState {
    Empty,
    Full,
    /// Matches either; only rpp can express this.
    // TODO: no producer until the tile config dialog exists.
    #[allow(dead_code)]
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    pub dx: i32,
    /// Positive downwards.
    pub dy: i32,
    /// Identifier rpp uses in `IsFullAt` / `IsEmptyAt`.
    pub rpp_name: &'static str,
    /// Position in the packed 8-bit form, most significant first.
    pub bit: u32,
}

const fn neighbor(dx: i32, dy: i32, rpp_name: &'static str, bit: u32) -> Neighbor {
    Neighbor {
        dx,
        dy,
        rpp_name,
        bit,
    }
}

pub const NEIGHBORS: [Neighbor; NEIGHBOR_COUNT] = [
    neighbor(-1, -1, "topLeft", 7),
    neighbor(0, -1, "top", 6),
    neighbor(1, -1, "topRight", 5),
    neighbor(-1, 0, "left", 4),
    neighbor(1, 0, "right", 3),
    neighbor(-1, 1, "bottomLeft", 2),
    neighbor(0, 1, "bottom", 1),
    neighbor(1, 1, "bottomRight", 0),
];

/// The 8 neighbor states of one tile, in index order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Neighborhood {
    states: [NeighborState; NEIGHBOR_COUNT],
}

impl Neighborhood {
    pub fn uniform(state: NeighborState) -> Self {
        Self {
            states: [state; NEIGHBOR_COUNT],
        }
    }

    pub fn set_state(&mut self, index: usize, state: NeighborState) {
        self.states[index] = state;
    }

    pub fn states(&self) -> &[NeighborState; NEIGHBOR_COUNT] {
        &self.states
    }
}

impl Default for Neighborhood {
    fn default() -> Self {
        Self::uniform(NeighborState::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::{NEIGHBOR_COUNT, NEIGHBORS, NeighborState, Neighborhood};

    #[test]
    fn bit_is_seven_minus_index() {
        for (index, neighbor) in NEIGHBORS.iter().enumerate() {
            assert_eq!(neighbor.bit as usize, 7 - index, "index {index}");
        }
    }

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
    fn uniform_sets_every_state() {
        let neighborhood = Neighborhood::uniform(NeighborState::Any);
        assert_eq!(neighborhood.states(), &[NeighborState::Any; NEIGHBOR_COUNT]);
    }
}
