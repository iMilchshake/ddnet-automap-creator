//! Neighbor index convention, y pointing down:
//! ```text
//! 0 TL   1 T    2 TR
//! 3 L   [self]  4 R
//! 5 BL   6 B    7 BR
//! ```

pub const NEIGHBOR_COUNT: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeighborState {
    Empty,
    Full,
    Any,
}

impl NeighborState {
    pub fn next(self) -> Self {
        match self {
            Self::Empty => Self::Full,
            Self::Full => Self::Any,
            Self::Any => Self::Empty,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Empty => Self::Any,
            Self::Full => Self::Empty,
            Self::Any => Self::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    pub dx: i32,
    /// Positive downwards.
    pub dy: i32,
    pub rpp_name: &'static str,
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

pub fn neighbor_index_at(dx: i32, dy: i32) -> Option<usize> {
    NEIGHBORS
        .iter()
        .position(|neighbor| neighbor.dx == dx && neighbor.dy == dy)
}

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

    pub fn state(&self, index: usize) -> NeighborState {
        self.states[index]
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
