use crate::model::neighbor::Neighborhood;
use crate::model::project::Project;
use crate::model::tile::TileRule;
use crate::tileset::{TileKind, Tileset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileState {
    Locked,
    Guessed(Neighborhood),
    Configured(TileRule),
    Removed,
}

pub fn tile_state(tileset: &Tileset, project: &Project, tile: usize) -> TileState {
    let TileKind::Guess(guess) = tileset.tiles[tile].kind else {
        return TileState::Locked;
    };

    match project.rule(tile) {
        Some(rule) => TileState::Configured(*rule),
        None if project.is_removed(tile) => TileState::Removed,
        None => TileState::Guessed(guess),
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
