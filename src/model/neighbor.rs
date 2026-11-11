pub type Offset = (i32, i32);

pub const CENTRE: Offset = (0, 0);
pub const MIN_EXTENT: i32 = 1;

const NEIGHBOR_COUNT: usize = 8;

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

pub fn neighbor_at((dx, dy): Offset) -> Option<&'static Neighbor> {
    NEIGHBORS
        .iter()
        .find(|neighbor| neighbor.dx == dx && neighbor.dy == dy)
}

pub fn offset_label(offset: Offset) -> String {
    if let Some(neighbor) = neighbor_at(offset) {
        return neighbor.label.to_owned();
    }

    let (dx, dy) = offset;
    let horizontal = match dx.signum() {
        1 => Some(format!("{dx} right")),
        -1 => Some(format!("{} left", -dx)),
        _ => None,
    };
    let vertical = match dy.signum() {
        1 => Some(format!("{dy} down")),
        -1 => Some(format!("{} up", -dy)),
        _ => None,
    };

    [horizontal, vertical]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Up,
    Down,
}

impl Side {
    pub const ALL: [Self; 4] = [Self::Left, Self::Right, Self::Up, Self::Down];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Extent {
    pub left: i32,
    pub right: i32,
    pub up: i32,
    pub down: i32,
}

impl Extent {
    pub const DEFAULT: Self = Self::uniform(MIN_EXTENT);

    pub const fn uniform(extent: i32) -> Self {
        Self {
            left: extent,
            right: extent,
            up: extent,
            down: extent,
        }
    }

    pub fn is_valid(self) -> bool {
        Side::ALL
            .into_iter()
            .all(|side| self.side(side) >= MIN_EXTENT)
    }

    pub fn width(self) -> i32 {
        self.left + 1 + self.right
    }

    pub fn height(self) -> i32 {
        self.up + 1 + self.down
    }

    pub fn cell_count(self) -> usize {
        (self.width() * self.height() - 1) as usize
    }

    pub fn contains(self, (dx, dy): Offset) -> bool {
        (-self.left..=self.right).contains(&dx) && (-self.up..=self.down).contains(&dy)
    }

    pub fn side(self, side: Side) -> i32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
            Side::Up => self.up,
            Side::Down => self.down,
        }
    }

    pub fn with_side(self, side: Side, extent: i32) -> Self {
        match side {
            Side::Left => Self {
                left: extent,
                ..self
            },
            Side::Right => Self {
                right: extent,
                ..self
            },
            Side::Up => Self { up: extent, ..self },
            Side::Down => Self {
                down: extent,
                ..self
            },
        }
    }

    pub fn offsets(self) -> impl Iterator<Item = Offset> {
        (-self.up..=self.down).flat_map(move |dy| (-self.left..=self.right).map(move |dx| (dx, dy)))
    }

    fn cell_index(self, offset: Offset) -> Option<usize> {
        if !self.contains(offset) || offset == CENTRE {
            return None;
        }

        let (dx, dy) = offset;
        let position = (dy + self.up) * self.width() + dx + self.left;
        let centre = self.up * self.width() + self.left;

        Some((position - i32::from(position > centre)) as usize)
    }
}

impl Default for Extent {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Neighborhood {
    extent: Extent,
    states: Vec<NeighborState>,
}

impl Neighborhood {
    pub fn uniform(state: NeighborState) -> Self {
        Self::with_extent(Extent::DEFAULT, state)
    }

    pub fn with_extent(extent: Extent, state: NeighborState) -> Self {
        Self {
            extent,
            states: vec![state; extent.cell_count()],
        }
    }

    pub fn from_states(extent: Extent, states: Vec<NeighborState>) -> Option<Self> {
        (extent.is_valid() && states.len() == extent.cell_count())
            .then_some(Self { extent, states })
    }

    pub fn extent(&self) -> Extent {
        self.extent
    }

    pub fn state(&self, offset: Offset) -> NeighborState {
        match self.extent.cell_index(offset) {
            Some(index) => self.states[index],
            None => NeighborState::Any,
        }
    }

    pub fn set_state(&mut self, offset: Offset, state: NeighborState) {
        let index = self
            .extent
            .cell_index(offset)
            .expect("the offset is a cell of the neighborhood");
        self.states[index] = state;
    }

    pub fn cells(&self) -> impl Iterator<Item = (Offset, NeighborState)> {
        self.extent
            .offsets()
            .filter(|offset| *offset != CENTRE)
            .zip(self.states.iter().copied())
    }

    pub fn constraints(&self) -> Vec<(Offset, NeighborState)> {
        self.cells()
            .filter(|(_, state)| *state != NeighborState::Any)
            .collect()
    }

    pub fn specificity(&self) -> usize {
        self.constraints().len()
    }

    pub fn grown(&self, side: Side) -> Self {
        self.resized(self.extent.with_side(side, self.extent.side(side) + 1))
    }

    pub fn shrunk(&self, side: Side) -> Option<Self> {
        self.can_shrink(side)
            .then(|| self.resized(self.extent.with_side(side, self.extent.side(side) - 1)))
    }

    pub fn can_shrink(&self, side: Side) -> bool {
        self.extent.side(side) > MIN_EXTENT
    }

    pub fn is_tile_specific(&self) -> bool {
        self.cells().any(|(_, state)| state.tile().is_some())
    }

    fn resized(&self, extent: Extent) -> Self {
        let mut resized = Self::with_extent(extent, NeighborState::Any);
        for (offset, state) in self.cells().filter(|(offset, _)| extent.contains(*offset)) {
            resized.set_state(offset, state);
        }

        resized
    }
}

impl Default for Neighborhood {
    fn default() -> Self {
        Self::uniform(NeighborState::Empty)
    }
}
