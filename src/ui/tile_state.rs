use crate::model::neighbor::Neighborhood;
use crate::model::project::Project;
use crate::model::tile::TileRule;
use crate::tileset::{TileKind, Tileset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileState {
    Locked,
    Grouped,
    Guessed(Neighborhood),
    Configured(TileRule),
    Removed,
}

impl TileState {
    pub fn is_editable(self) -> bool {
        !matches!(self, Self::Locked | Self::Grouped)
    }
}

pub fn tile_state(tileset: &Tileset, project: &Project, tile: usize) -> TileState {
    if project.group_at(tile).is_some() {
        return TileState::Grouped;
    }
    if let Some(rule) = project.rule(tile) {
        return TileState::Configured(*rule);
    }

    let TileKind::Guess(guess) = tileset.tiles[tile].kind else {
        return TileState::Locked;
    };

    match project.is_removed(tile) {
        true => TileState::Removed,
        false => TileState::Guessed(guess),
    }
}

pub fn seed_rule(tileset: &Tileset, project: &Project, tile: usize) -> TileRule {
    if let Some(rule) = project.rule(tile) {
        return *rule;
    }

    match tileset.tiles[tile].kind {
        TileKind::Guess(guess) => TileRule::new(guess),
        TileKind::Locked => TileRule::new(Neighborhood::default()),
    }
}
