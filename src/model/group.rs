use thiserror::Error;

use crate::model::tile::{Chance, TILESET_SIDE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    Fill,
    Decorate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileGroup {
    pub name: String,
    pub top_left: usize,
    pub width: usize,
    pub height: usize,
    pub mode: GroupMode,
    pub chance: Chance,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GroupError {
    #[error("`{0}` must start with a letter and hold only letters and digits")]
    Name(String),

    #[error("a group must cover at least two tiles")]
    TooSmall,

    #[error("a group cannot start at tile 0, which is always empty")]
    StartsAtZero,

    #[error("a {width}×{height} group at tile {top_left} runs off the tileset")]
    OutOfBounds {
        top_left: usize,
        width: usize,
        height: usize,
    },

    #[error("`{0}` names more than one group")]
    DuplicateName(String),

    #[error("`{first}` and `{second}` cover the same tiles")]
    Overlap { first: String, second: String },
}

impl TileGroup {
    pub fn column(&self) -> usize {
        self.top_left % TILESET_SIDE
    }

    pub fn row(&self) -> usize {
        self.top_left / TILESET_SIDE
    }

    pub fn bottom_right(&self) -> usize {
        self.top_left + (self.height - 1) * TILESET_SIDE + (self.width - 1)
    }

    pub fn footprint(&self) -> Vec<usize> {
        (0..self.height)
            .flat_map(|row| {
                (0..self.width).map(move |column| self.top_left + row * TILESET_SIDE + column)
            })
            .collect()
    }

    pub fn covers(&self, tile: usize) -> bool {
        let column = tile % TILESET_SIDE;
        let row = tile / TILESET_SIDE;

        (self.column()..self.column() + self.width).contains(&column)
            && (self.row()..self.row() + self.height).contains(&row)
    }

    /// The ring of tiles that `Decorate` additionally requires to be solid,
    /// in reading order.
    pub fn ring_offsets(&self) -> Vec<(i32, i32)> {
        let width = self.width as i32;
        let height = self.height as i32;

        (-1..=height)
            .flat_map(|y| (-1..=width).map(move |x| (x, y)))
            .filter(|&(x, y)| !(0..width).contains(&x) || !(0..height).contains(&y))
            .collect()
    }

    pub fn validate(&self) -> Result<(), GroupError> {
        if !is_identifier(&self.name) {
            return Err(GroupError::Name(self.name.clone()));
        }
        if self.width * self.height < 2 {
            return Err(GroupError::TooSmall);
        }
        if self.top_left == 0 {
            return Err(GroupError::StartsAtZero);
        }
        if self.column() + self.width > TILESET_SIDE || self.row() + self.height > TILESET_SIDE {
            return Err(GroupError::OutOfBounds {
                top_left: self.top_left,
                width: self.width,
                height: self.height,
            });
        }

        Ok(())
    }
}

pub fn validate_all(groups: &[TileGroup]) -> Result<(), GroupError> {
    for group in groups {
        group.validate()?;
    }

    for (index, group) in groups.iter().enumerate() {
        for other in &groups[index + 1..] {
            if group.name == other.name {
                return Err(GroupError::DuplicateName(group.name.clone()));
            }
            if group.footprint().iter().any(|tile| other.covers(*tile)) {
                return Err(GroupError::Overlap {
                    first: group.name.clone(),
                    second: other.name.clone(),
                });
            }
        }
    }

    Ok(())
}

/// rpp names hold only letters, digits and `:`, so an underscore is as illegal
/// as a space. The emitter adds its own `:` prefix, which is what keeps these
/// names clear of rpp's keywords and of base.r's `g:`/`s:` globals.
fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    first.is_ascii_alphabetic() && characters.all(|character| character.is_ascii_alphanumeric())
}
