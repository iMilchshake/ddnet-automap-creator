use json_pretty_compact::PrettyCompactFormatter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBOR_COUNT, NeighborState, Neighborhood};
use crate::model::project::Project;
use crate::model::tile::{Chance, InvalidChance, TILE_COUNT, TileMods, TileRule};

const VERSION: u32 = 1;

/// Wide enough to keep one tile on one line; the default 120 breaks them up.
const MAX_LINE: u32 = 140;

#[derive(Debug, Error)]
pub enum BlueprintError {
    #[error("not a readable blueprint: {0}")]
    Json(#[from] serde_json::Error),

    #[error("blueprint version {0} is not supported, this app writes version {VERSION}")]
    UnsupportedVersion(u32),

    #[error("blueprint was made for `{wanted}`, but `{loaded}` is open")]
    WrongImage { wanted: String, loaded: String },

    #[error("tile {0} is outside the tileset")]
    TileId(usize),

    #[error("tile {0} appears more than once")]
    DuplicateTile(usize),

    #[error("`{0}` is not a neighbor state, expected 0 (empty), 1 (full) or 2 (any)")]
    NeighborCode(u8),

    #[error(transparent)]
    Chance(#[from] InvalidChance),

    #[error(transparent)]
    Group(#[from] GroupError),

    #[error("group `{group}` covers tile {tile}, which is also configured on its own")]
    Claimed { group: String, tile: usize },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blueprint {
    pub version: u32,
    pub image: Option<String>,
    #[serde(default)]
    pub rule_set: String,
    pub tiles: Vec<BlueprintTile>,
    #[serde(default)]
    pub removed: Vec<usize>,
    #[serde(default)]
    pub groups: Vec<BlueprintGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintTile {
    pub id: usize,
    pub con: [u8; NEIGHBOR_COUNT],
    pub chance: f32,
    pub mods: BlueprintMods,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintMods {
    pub x_flip: bool,
    pub y_flip: bool,
    pub rot: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintGroup {
    pub name: String,
    pub top_left: usize,
    pub width: usize,
    pub height: usize,
    pub mode: BlueprintMode,
    pub chance: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlueprintMode {
    Fill,
    Decorate,
}

pub struct Loaded {
    pub project: Project,
    pub rule_set: String,
}

pub fn to_json(project: &Project, image: &str, rule_set: &str) -> Result<String, BlueprintError> {
    let blueprint = Blueprint {
        version: VERSION,
        image: Some(image.to_owned()),
        rule_set: rule_set.to_owned(),
        tiles: project.rules().into_iter().map(store_tile).collect(),
        removed: project.removed_tiles(),
        groups: project.groups().iter().map(store_group).collect(),
    };

    let mut target = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(
        &mut target,
        PrettyCompactFormatter::new().with_max_line_length(MAX_LINE),
    );
    blueprint.serialize(&mut serializer)?;

    Ok(String::from_utf8(target).expect("serde_json writes utf-8"))
}

pub fn from_json(text: &str, image: &str) -> Result<Loaded, BlueprintError> {
    let blueprint: Blueprint = serde_json::from_str(text)?;
    if blueprint.version != VERSION {
        return Err(BlueprintError::UnsupportedVersion(blueprint.version));
    }

    if let Some(wanted) = &blueprint.image
        && wanted != image
    {
        return Err(BlueprintError::WrongImage {
            wanted: wanted.clone(),
            loaded: image.to_owned(),
        });
    }

    let mut project = Project::default();
    for tile in &blueprint.tiles {
        if tile.id >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile.id));
        }
        if project.rule(tile.id).is_some() {
            return Err(BlueprintError::DuplicateTile(tile.id));
        }
        project.set_rule(tile.id, load_tile(tile)?);
    }

    for &tile in &blueprint.removed {
        if tile >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile));
        }
        project.remove(tile);
    }

    let groups: Vec<TileGroup> = blueprint
        .groups
        .iter()
        .map(load_group)
        .collect::<Result<_, _>>()?;
    group::validate_all(&groups)?;
    for group in groups {
        for tile in group.footprint() {
            if project.rule(tile).is_some() {
                return Err(BlueprintError::Claimed {
                    group: group.name.clone(),
                    tile,
                });
            }
        }
        project.append_group(group);
    }

    Ok(Loaded {
        project,
        rule_set: blueprint.rule_set,
    })
}

fn store_tile((id, rule): (usize, TileRule)) -> BlueprintTile {
    let mut con = [0; NEIGHBOR_COUNT];
    for (slot, state) in con.iter_mut().zip(rule.neighborhood.states()) {
        *slot = store_state(*state);
    }

    BlueprintTile {
        id,
        con,
        chance: rule.chance.percent(),
        mods: BlueprintMods {
            x_flip: rule.mods.can_x_flip,
            y_flip: rule.mods.can_y_flip,
            rot: rule.mods.can_rotate,
        },
    }
}

fn load_tile(tile: &BlueprintTile) -> Result<TileRule, BlueprintError> {
    let mut neighborhood = Neighborhood::default();
    for (index, &code) in tile.con.iter().enumerate() {
        neighborhood.set_state(index, load_state(code)?);
    }

    Ok(TileRule {
        neighborhood,
        mods: TileMods {
            can_x_flip: tile.mods.x_flip,
            can_y_flip: tile.mods.y_flip,
            can_rotate: tile.mods.rot,
        },
        chance: Chance::new(tile.chance)?,
    })
}

fn store_group(group: &TileGroup) -> BlueprintGroup {
    BlueprintGroup {
        name: group.name.clone(),
        top_left: group.top_left,
        width: group.width,
        height: group.height,
        mode: match group.mode {
            GroupMode::Fill => BlueprintMode::Fill,
            GroupMode::Decorate => BlueprintMode::Decorate,
        },
        chance: group.chance.percent(),
    }
}

fn load_group(group: &BlueprintGroup) -> Result<TileGroup, BlueprintError> {
    Ok(TileGroup {
        name: group.name.clone(),
        top_left: group.top_left,
        width: group.width,
        height: group.height,
        mode: match group.mode {
            BlueprintMode::Fill => GroupMode::Fill,
            BlueprintMode::Decorate => GroupMode::Decorate,
        },
        chance: Chance::new(group.chance)?,
    })
}

fn store_state(state: NeighborState) -> u8 {
    match state {
        NeighborState::Empty => 0,
        NeighborState::Full => 1,
        NeighborState::Any => 2,
    }
}

fn load_state(code: u8) -> Result<NeighborState, BlueprintError> {
    match code {
        0 => Ok(NeighborState::Empty),
        1 => Ok(NeighborState::Full),
        2 => Ok(NeighborState::Any),
        _ => Err(BlueprintError::NeighborCode(code)),
    }
}
