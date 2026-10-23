use crate::model::neighbor::{NEIGHBORS, Neighborhood, neighbor_index_at};
use crate::model::tile::TileMods;

/// A variant's transform is `R^rot ∘ X^x_flip ∘ Y^y_flip`, applied right to
/// left: flips, then rotation. The reverse order picks different tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transform {
    pub x_flip: bool,
    pub y_flip: bool,
    pub rot: bool,
}

const fn transform(x_flip: bool, y_flip: bool, rot: bool) -> Transform {
    Transform {
        x_flip,
        y_flip,
        rot,
    }
}

const ENUMERATION: [Transform; 8] = [
    transform(false, false, false),
    transform(false, false, true),
    transform(true, true, false),
    transform(true, true, true),
    transform(true, false, false),
    transform(true, false, true),
    transform(false, true, false),
    transform(false, true, true),
];

const BASIS: [(i32, i32); 2] = [(1, 0), (0, 1)];

impl Transform {
    pub const IDENTITY: Self = transform(false, false, false);
    pub const ALL: [Self; 8] = ENUMERATION;
    pub const ROTATE_CLOCKWISE: Self = transform(false, false, true);
    pub const FLIP_HORIZONTALLY: Self = transform(true, false, false);
    pub const FLIP_VERTICALLY: Self = transform(false, true, false);

    pub fn inverse(self) -> Self {
        Self::ALL
            .into_iter()
            .find(|candidate| self.after(*candidate) == Self::IDENTITY)
            .expect("every transform has an inverse")
    }

    pub fn suffix(self) -> String {
        let mut suffix = String::new();
        if self.x_flip {
            suffix.push('V');
        }
        if self.y_flip {
            suffix.push('H');
        }
        if self.rot {
            suffix.push('R');
        }

        suffix
    }

    pub fn apply(self, base: &Neighborhood) -> Neighborhood {
        let mut moved = Neighborhood::default();
        for (index, neighbor) in NEIGHBORS.iter().enumerate() {
            let (dx, dy) = self.move_offset(neighbor.dx, neighbor.dy);
            let target = neighbor_index_at(dx, dy).expect("a ring offset stays on the ring");
            moved.set_state(target, base.state(index));
        }

        moved
    }

    pub fn after(self, first: Self) -> Self {
        Self::ALL
            .into_iter()
            .find(|candidate| {
                BASIS.into_iter().all(|(dx, dy)| {
                    let (first_dx, first_dy) = first.move_offset(dx, dy);
                    candidate.move_offset(dx, dy) == self.move_offset(first_dx, first_dy)
                })
            })
            .expect("the transforms are closed under composition")
    }

    fn move_offset(self, dx: i32, dy: i32) -> (i32, i32) {
        let (mut dx, mut dy) = (dx, dy);
        if self.y_flip {
            dy = -dy;
        }
        if self.x_flip {
            dx = -dx;
        }
        if self.rot {
            (dx, dy) = (-dy, dx);
        }

        (dx, dy)
    }

    fn rotated(self) -> Self {
        match self.rot {
            true => transform(!self.x_flip, !self.y_flip, false),
            false => transform(self.x_flip, self.y_flip, true),
        }
    }

    fn x_flipped(self) -> Self {
        match self.rot {
            true => transform(self.x_flip, !self.y_flip, true),
            false => transform(!self.x_flip, self.y_flip, false),
        }
    }

    fn y_flipped(self) -> Self {
        match self.rot {
            true => transform(!self.x_flip, self.y_flip, true),
            false => transform(self.x_flip, !self.y_flip, false),
        }
    }
}

const PRESETS: [TileMods; 6] = [
    TileMods::new(false, false, false),
    TileMods::new(true, false, false),
    TileMods::new(false, true, false),
    TileMods::new(false, false, true),
    TileMods::new(true, true, false),
    TileMods::new(true, false, true),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orientations(u8);

impl Orientations {
    pub fn preset(self) -> Option<TileMods> {
        PRESETS
            .into_iter()
            .find(|mods| Self::generated(*mods) == self)
    }

    pub fn generated(mods: TileMods) -> Self {
        subgroup(mods).into_iter().collect()
    }

    pub fn resolved(custom: Option<Self>, mods: TileMods) -> Self {
        custom.unwrap_or_else(|| Self::generated(mods))
    }

    pub fn contains(self, transform: Transform) -> bool {
        self.0 & bit(transform) != 0
    }

    pub fn with(self, transform: Transform) -> Self {
        Self(self.0 | bit(transform))
    }

    pub fn without(self, transform: Transform) -> Self {
        Self(self.0 & !bit(transform))
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn transforms(self) -> Vec<Transform> {
        ENUMERATION
            .into_iter()
            .filter(|transform| self.contains(*transform))
            .collect()
    }
}

impl FromIterator<Transform> for Orientations {
    fn from_iter<Transforms: IntoIterator<Item = Transform>>(transforms: Transforms) -> Self {
        Self(
            transforms
                .into_iter()
                .fold(0, |bits, transform| bits | bit(transform)),
        )
    }
}

fn bit(transform: Transform) -> u8 {
    let position = ENUMERATION
        .iter()
        .position(|candidate| *candidate == transform)
        .expect("every transform is enumerated");

    1 << position
}

fn subgroup(mods: TileMods) -> Vec<Transform> {
    let mut members = vec![Transform::IDENTITY];
    let mut next = 0;

    while next < members.len() {
        let current = members[next];
        next += 1;

        let mut reachable = Vec::new();
        if mods.can_x_flip {
            reachable.push(current.x_flipped());
        }
        if mods.can_y_flip {
            reachable.push(current.y_flipped());
        }
        if mods.can_rotate {
            reachable.push(current.rotated());
        }

        for transform in reachable {
            if !members.contains(&transform) {
                members.push(transform);
            }
        }
    }

    members
}
