use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{AIR_TILE, TileRule};
use crate::tileset::{TileKind, Tileset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileState {
    Locked,
    Grouped,
    Unconfigured,
    Configured(TileRule),
    Deactivated(Option<TileRule>),
}

impl TileState {
    pub fn is_editable(self) -> bool {
        !matches!(self, Self::Locked | Self::Grouped)
    }
}

pub fn tile_state(tileset: &Tileset, project: &Project, tile: usize) -> TileState {
    if tile == AIR_TILE {
        return TileState::Locked;
    }
    if project.group_at(tile).is_some() {
        return TileState::Grouped;
    }
    let rule = project.rules_of(tile).first().copied();
    if rule.is_none() && tileset.tiles[tile].kind == TileKind::Locked {
        return TileState::Locked;
    }

    match (rule, project.is_deactivated(tile)) {
        (Some(rule), false) => TileState::Configured(rule),
        (rule, true) => TileState::Deactivated(rule),
        (None, false) => TileState::Unconfigured,
    }
}

pub fn seed_rule(tileset: &Tileset, tile: usize) -> TileRule {
    match tileset.tiles[tile].kind {
        TileKind::Open => TileRule::new(Neighborhood::uniform(NeighborState::Any)),
        TileKind::Locked => TileRule::new(Neighborhood::default()),
    }
}
