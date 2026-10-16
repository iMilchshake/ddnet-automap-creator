use json_pretty_compact::PrettyCompactFormatter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBOR_COUNT, NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{AIR_TILE, Chance, InvalidChance, TILE_COUNT, TileMods, TileRule};
use crate::model::transform::{Orientations, Transform};

const VERSION: u32 = 1;

// for serialization of orientations
const IDENTITY_LETTER: char = 'N';
const X_FLIP_LETTER: char = 'V';
const Y_FLIP_LETTER: char = 'H';
const ROTATE_LETTER: char = 'R';

/// Wide enough to keep one tile on one line; the default 120 breaks them up.
const MAX_LINE: u32 = 150;

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

    #[error("tile {AIR_TILE} is reserved for air and cannot hold a rule")]
    AirTile,

    #[error("`{0}` is not a neighbor state, expected 0 (empty), 1 (full) or 2 (any)")]
    NeighborCode(u8),

    #[error("tile {tile}: `{name}` is not an orientation name")]
    OrientationName { tile: usize, name: String },

    #[error("tile {0} needs at least one orientation")]
    NoOrientations(usize),

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
    image: String,
    active: usize,
    rule_sets: Vec<BlueprintRuleSet>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlueprintRuleSet {
    name: String,
    normalize: bool,
    tiles: Vec<BlueprintTile>,
    deactivated: Vec<usize>,
    groups: Vec<BlueprintGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintTile {
    pub id: usize,
    pub con: [u8; NEIGHBOR_COUNT],
    pub chance: f32,
    pub orientations: Vec<String>,
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
    pub rule_sets: RuleSets,
    pub image: String,
}

pub fn to_json(rule_sets: &RuleSets, image: &str) -> Result<String, BlueprintError> {
    let blueprint = Blueprint {
        version: VERSION,
        image: image.to_owned(),
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
            let project = load_project(&rule_set)?;
            Ok((rule_set.name, project))
        })
        .collect::<Result<_, BlueprintError>>()?;

    Ok(Loaded {
        rule_sets: RuleSets::from_parts(sets, blueprint.active),
        image: blueprint.image,
    })
}

fn load_project(rule_set: &BlueprintRuleSet) -> Result<Project, BlueprintError> {
    let mut project = Project::default();
    project.set_chance_mode(match rule_set.normalize {
        true => ChanceMode::Normalize,
        false => ChanceMode::Exact,
    });
    for tile in &rule_set.tiles {
        if tile.id >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile.id));
        }
        if tile.id == AIR_TILE {
            return Err(BlueprintError::AirTile);
        }
        if project.rule(tile.id).is_some() {
            return Err(BlueprintError::DuplicateTile(tile.id));
        }
        project.set_rule(tile.id, load_tile(tile)?);
    }

    for &tile in &rule_set.deactivated {
        if tile >= TILE_COUNT {
            return Err(BlueprintError::TileId(tile));
        }
        if tile == AIR_TILE {
            return Err(BlueprintError::AirTile);
        }
        project.deactivate(tile);
    }

    let loaded_groups: Vec<TileGroup> = rule_set
        .groups
        .iter()
        .map(load_group)
        .collect::<Result<_, _>>()?;
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
        tiles: project.all_rules().into_iter().map(store_tile).collect(),
        deactivated: project.deactivated_tiles(),
        groups: project.groups().iter().map(store_group).collect(),
    }
}

fn store_tile((id, rule): (usize, TileRule)) -> BlueprintTile {
    let mut con = [0; NEIGHBOR_COUNT];
    for (slot, state) in con.iter_mut().zip(rule.neighborhood.states()) {
        *slot = store_state(*state);
    }

    let orientations = rule.orientations().transforms();

    BlueprintTile {
        id,
        con,
        chance: rule.chance.percent(),
        orientations: orientations.into_iter().map(orientation_name).collect(),
    }
}

fn orientation_name(transform: Transform) -> String {
    let letters: String = [
        (transform.x_flip, X_FLIP_LETTER),
        (transform.y_flip, Y_FLIP_LETTER),
        (transform.rot, ROTATE_LETTER),
    ]
    .into_iter()
    .filter(|(applied, _)| *applied)
    .map(|(_, letter)| letter)
    .collect();

    match letters.is_empty() {
        true => IDENTITY_LETTER.to_string(),
        false => letters,
    }
}

fn load_orientation(tile: usize, name: &str) -> Result<Transform, BlueprintError> {
    Transform::ALL
        .into_iter()
        .find(|transform| orientation_name(*transform) == name)
        .ok_or_else(|| BlueprintError::OrientationName {
            tile,
            name: name.to_owned(),
        })
}

fn load_orientations(tile: usize, names: &[String]) -> Result<Orientations, BlueprintError> {
    let orientations: Orientations = names
        .iter()
        .map(|name| load_orientation(tile, name))
        .collect::<Result<_, _>>()?;

    match orientations.is_empty() {
        true => Err(BlueprintError::NoOrientations(tile)),
        false => Ok(orientations),
    }
}

fn load_tile(tile: &BlueprintTile) -> Result<TileRule, BlueprintError> {
    let mut neighborhood = Neighborhood::default();
    for (index, &code) in tile.con.iter().enumerate() {
        neighborhood.set_state(index, load_state(code)?);
    }

    let orientations = load_orientations(tile.id, &tile.orientations)?;
    let (mods, custom) = match orientations.preset() {
        Some(mods) => (mods, None),
        None => (TileMods::default(), Some(orientations)),
    };

    Ok(TileRule {
        neighborhood,
        mods,
        custom,
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
