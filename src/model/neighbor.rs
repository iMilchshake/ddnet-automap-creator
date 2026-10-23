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
    Is(usize),
}

impl NeighborState {
    pub fn toggled_solid(self) -> Self {
        match self {
            Self::Empty | Self::Any | Self::Is(_) => Self::Full,
            Self::Full => Self::Any,
        }
    }

    pub fn tile(self) -> Option<usize> {
        match self {
            Self::Is(tile) => Some(tile),
            Self::Empty | Self::Full | Self::Any => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Neighbor {
    pub dx: i32,
    /// Positive downwards.
    pub dy: i32,
    pub rpp_name: &'static str,
    pub label: &'static str,
}

const fn neighbor(dx: i32, dy: i32, rpp_name: &'static str, label: &'static str) -> Neighbor {
    Neighbor {
        dx,
        dy,
        rpp_name,
        label,
    }
}

pub const NEIGHBORS: [Neighbor; NEIGHBOR_COUNT] = [
    neighbor(-1, -1, "topLeft", "top left"),
    neighbor(0, -1, "top", "top"),
    neighbor(1, -1, "topRight", "top right"),
    neighbor(-1, 0, "left", "left"),
    neighbor(1, 0, "right", "right"),
    neighbor(-1, 1, "bottomLeft", "bottom left"),
    neighbor(0, 1, "bottom", "bottom"),
    neighbor(1, 1, "bottomRight", "bottom right"),
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

    pub fn is_tile_specific(&self) -> bool {
        self.states.iter().any(|state| state.tile().is_some())
    }
}

impl Default for Neighborhood {
    fn default() -> Self {
        Self::uniform(NeighborState::Empty)
    }
}
