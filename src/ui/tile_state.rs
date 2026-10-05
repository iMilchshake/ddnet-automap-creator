use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{AIR_TILE, MASK_TILE, TileRule};
use crate::tileset::{TileKind, Tileset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileState {
    Locked,
    Grouped,
    Unconfigured,
    Configured(TileRule),
    Removed,
}

impl TileState {
    pub fn is_editable(self) -> bool {
        !matches!(self, Self::Locked | Self::Grouped)
    }
}

pub fn tile_state(tileset: &Tileset, project: &Project, tile: usize) -> TileState {
    if tile == MASK_TILE || tile == AIR_TILE {
        return TileState::Locked;
    }
    if project.group_at(tile).is_some() {
        return TileState::Grouped;
    }
    if let Some(rule) = project.rule(tile) {
        return TileState::Configured(*rule);
    }

    if tileset.tiles[tile].kind == TileKind::Locked {
        return TileState::Locked;
    }

    match project.is_removed(tile) {
        true => TileState::Removed,
        false => TileState::Unconfigured,
    }
}

pub fn seed_rule(tileset: &Tileset, project: &Project, tile: usize) -> TileRule {
    if let Some(rule) = project.rule(tile) {
        return *rule;
    }

    match tileset.tiles[tile].kind {
        TileKind::Open => TileRule::new(Neighborhood::uniform(NeighborState::Any)),
        TileKind::Locked => TileRule::new(Neighborhood::default()),
    }
}
