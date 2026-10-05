use json_pretty_compact::PrettyCompactFormatter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBOR_COUNT, NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{
    AIR_TILE, Chance, InvalidChance, MASK_TILE, TILE_COUNT, TileMods, TileRule,
};

const VERSION: u32 = 1;

/// Wide enough to keep one tile on one line; the default 120 breaks them up.
const MAX_LINE: u32 = 140;

#[derive(Debug, Error)]
pub enum BlueprintError {
    #[error("not a readable blueprint: {0}")]
    Json(#[from] serde_json::Error),

    #[error("blueprint version {0} is not supported, this app writes version {VERSION}")]
    UnsupportedVersion(u32),

    #[error("tile {0} is outside the tileset")]
    TileId(usize),

    #[error("tile {0} appears more than once")]
    DuplicateTile(usize),

    #[error("tile {MASK_TILE} is reserved as rpp's mask and cannot hold a rule")]
    MaskTile,

    #[error("tile {AIR_TILE} is reserved for air and cannot hold a rule")]
    AirTile,

    #[error("`{0}` is not a neighbor state, expected 0 (empty), 1 (full) or 2 (any)")]
    NeighborCode(u8),

    #[error(transparent)]
    Chance(#[from] InvalidChance),

    #[error(transparent)]
    Group(#[from] GroupError),

    #[error("group `{group}` covers tile {tile}, which is also configured on its own")]
    Claimed { group: String, tile: usize },

    #[error("blueprint holds no rule sets")]
    NoRuleSets,
}

#[derive(Debug, Serialize, Deserialize)]
struct Blueprint {
    version: u32,
    image: Option<String>,
    #[serde(default)]
    active: usize,
    rule_sets: Vec<BlueprintRuleSet>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlueprintRuleSet {
    name: String,
    #[serde(default = "default_normalize")]
    normalize: bool,
    tiles: Vec<BlueprintTile>,
    #[serde(default)]
    removed: Vec<usize>,
    #[serde(default)]
    groups: Vec<BlueprintGroup>,
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

fn default_normalize() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlueprintMode {
    Fill,
    Decorate,
}

pub struct Loaded {
    pub rule_sets: RuleSets,
    pub image: Option<String>,
}

pub fn to_json(rule_sets: &RuleSets, image: &str) -> Result<String, BlueprintError> {
    let blueprint = Blueprint {
        version: VERSION,
        image: Some(image.to_owned()),
        active: rule_sets.active_index(),
        rule_sets: rule_sets
            .sets()
            .iter()
            .map(|(name, project)| store_rule_set(name, project))
            .collect(),
    };

    let mut target = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(
        &mut target,
        PrettyCompactFormatter::new().with_max_line_length(MAX_LINE),
    );
    blueprint.serialize(&mut serializer)?;

    Ok(String::from_utf8(target).expect("serde_json writes utf-8"))
}

pub fn parse(text: &str) -> Result<Loaded, BlueprintError> {
    build(read(text)?)
}

fn read(text: &str) -> Result<Blueprint, BlueprintError> {
    let blueprint: Blueprint = serde_json::from_str(text)?;
    match blueprint.version == VERSION {
        true => Ok(blueprint),
        false => Err(BlueprintError::UnsupportedVersion(blueprint.version)),
    }
}

fn build(blueprint: Blueprint) -> Result<Loaded, BlueprintError> {
    if blueprint.rule_sets.is_empty() {
        return Err(BlueprintError::NoRuleSets);
    }

    let sets = blueprint
        .rule_sets
        .into_iter()
        .map(|rule_set| {
            let project = load_project(
                rule_set.normalize,
                &rule_set.tiles,
                &rule_set.removed,
                &rule_set.groups,
            )?;
            Ok((rule_set.name, project))
        })
        .collect::<Result<_, BlueprintError>>()?;

    Ok(Loaded {
        rule_sets: RuleSets::from_parts(sets, blueprint.active),
        image: blueprint.image,
    })
}

fn load_project(
    normalize: bool,
    tiles: &[BlueprintTile],
    removed: &[usize],
    groups: &[BlueprintGroup],
) -> Result<Project, BlueprintError> {
    let mut project = Project::default();
    project.set_chance_mode(match normalize {
        true => ChanceMode::Normalize,
        false => ChanceMode::Exact,
    });
    for tile in tiles {
        if tile.id >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile.id));
        }
        if tile.id == MASK_TILE {
            return Err(BlueprintError::MaskTile);
        }
        if tile.id == AIR_TILE {
            return Err(BlueprintError::AirTile);
        }
        if project.rule(tile.id).is_some() {
            return Err(BlueprintError::DuplicateTile(tile.id));
        }
        project.set_rule(tile.id, load_tile(tile)?);
    }

    for &tile in removed {
        if tile >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile));
        }
        project.remove(tile);
    }

    let loaded_groups: Vec<TileGroup> = groups.iter().map(load_group).collect::<Result<_, _>>()?;
    group::validate_all(&loaded_groups)?;
    for group in loaded_groups {
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

    Ok(project)
}

fn store_rule_set(name: &str, project: &Project) -> BlueprintRuleSet {
    BlueprintRuleSet {
        name: name.to_owned(),
        normalize: project.chance_mode() == ChanceMode::Normalize,
        tiles: project.rules().into_iter().map(store_tile).collect(),
        removed: project.removed_tiles(),
        groups: project.groups().iter().map(store_group).collect(),
    }
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
