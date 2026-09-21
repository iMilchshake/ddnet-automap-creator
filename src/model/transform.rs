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

impl Transform {
    pub const IDENTITY: Self = transform(false, false, false);

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

pub fn variants(mods: TileMods) -> Vec<Transform> {
    let subgroup = subgroup(mods);
    ENUMERATION
        .into_iter()
        .filter(|transform| subgroup.contains(transform))
        .collect()
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
