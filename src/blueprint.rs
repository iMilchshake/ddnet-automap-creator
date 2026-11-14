use json_pretty_compact::PrettyCompactFormatter;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::group::{self, Cell, CellState, GroupError, RING_EXTENT, TileGroup};
use crate::model::neighbor::{Extent, NeighborState, Neighborhood};
use crate::model::pool::ChanceMode;
use crate::model::project::Project;
use crate::model::rule_sets::RuleSets;
use crate::model::tile::{
    AIR_TILE, Chance, InvalidChance, RuleCenter, TILE_COUNT, TileMods, TileRule,
};
use crate::model::transform::{Orientations, Transform};

const VERSION: u32 = 2;

// for serialization of orientations
const IDENTITY_LETTER: char = 'N';
const X_FLIP_LETTER: char = 'V';
const Y_FLIP_LETTER: char = 'H';
const ROTATE_LETTER: char = 'R';

const EMPTY_LETTER: char = 'E';
const FULL_LETTER: char = 'F';
const ANY_LETTER: char = 'A';
const TILE_OPEN: char = '[';
const TILE_CLOSE: char = ']';

const STATE_LETTERS: [(char, NeighborState); 3] = [
    (EMPTY_LETTER, NeighborState::Empty),
    (FULL_LETTER, NeighborState::Full),
    (ANY_LETTER, NeighborState::Any),
];

/// Wide enough to keep one rule on one line; the default 120 breaks them up.
const MAX_LINE: u32 = 150;

const BEGIN_MARKER: &str = "# ddnet-automap-creator blueprint begin";
const END_MARKER: &str = "# ddnet-automap-creator blueprint end";
const COMMENT_PREFIX: &str = "# ";
const BARE_COMMENT: &str = "#";

#[derive(Debug, Error)]
pub enum BlueprintError {
    #[error("not a valid blueprint: {0}")]
    Json(#[from] serde_json::Error),

    #[error("old blueprint that is no longer supported")]
    UnsupportedVersion(u32),

    #[error("tile {0} exceeds tileset bounds")]
    TileId(usize),

    #[error("tile {0} is listed twice")]
    DuplicateTile(usize),

    #[error("Tile {AIR_TILE} (air) can't have rules")]
    AirTile,

    #[error("group `{0}` has unsorted or repeated tiles")]
    GroupTiles(String),

    #[error("group `{0}` has invalid orientations")]
    GroupOrientations(String),

    #[error("group `{0}` has wrong neighborhood size")]
    GroupNeighborShape(String),

    #[error("group `{0}` has neighbors too far away")]
    GroupNeighborCell(String),

    #[error("group `{group}`: {source}")]
    GroupNeighborText {
        group: String,
        source: Box<BlueprintError>,
    },

    #[error("group `{0}` can't have tile neighbors, only E/F/A")]
    GroupNeighborTile(String),

    #[error("`{0}` isn't a valid neighbor, only E/F/A/[tile]")]
    NeighborLetter(char),

    #[error("`{0}` isn't a valid neighbor tile")]
    NeighborTileText(String),

    #[error("tile {0} has wrong neighborhood size")]
    NeighborhoodShape(usize),

    #[error("Tile {AIR_TILE} (air) can't be a neighbor tile")]
    AirNeighborTile,

    #[error("tile {tile} has unknown orientation `{name}`")]
    OrientationName { tile: usize, name: String },

    #[error("tile {0} needs at least 1 rule")]
    NoRules(usize),

    #[error("tile {0} needs at least 1 orientation")]
    NoOrientations(usize),

    #[error(transparent)]
    Chance(#[from] InvalidChance),

    #[error(transparent)]
    Group(#[from] GroupError),

    #[error("group `{group}` can't overlap with tile {tile}")]
    Claimed { group: String, tile: usize },

    #[error("blueprint has no rulesets")]
    NoRuleSets,

    #[error("the file has no embedded blueprint")]
    NoEmbedded,

    #[error("embedded blueprint is incomplete")]
    IncompleteEmbedded,
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
    pub rules: Vec<BlueprintRule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintRule {
    pub neighbors: String,
    #[serde(default, skip_serializing_if = "BlueprintExtent::is_default")]
    pub extent: BlueprintExtent,
    pub chance: f32,
    pub orientations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawn: Option<String>,
    #[serde(default, skip_serializing_if = "BlueprintCenter::is_full")]
    pub center: BlueprintCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintExtent([i32; 4]);

impl BlueprintExtent {
    fn is_default(&self) -> bool {
        Extent::from(*self) == Extent::DEFAULT
    }
}

impl Default for BlueprintExtent {
    fn default() -> Self {
        Extent::DEFAULT.into()
    }
}

impl From<Extent> for BlueprintExtent {
    fn from(extent: Extent) -> Self {
        Self([extent.left, extent.right, extent.up, extent.down])
    }
}

impl From<BlueprintExtent> for Extent {
    fn from(BlueprintExtent([left, right, up, down]): BlueprintExtent) -> Self {
        Self {
            left,
            right,
            up,
            down,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "char", try_from = "char")]
pub enum BlueprintCenter {
    #[default]
    Full,
    Empty,
    Any,
}

impl BlueprintCenter {
    fn is_full(&self) -> bool {
        *self == Self::Full
    }
}

impl From<BlueprintCenter> for char {
    fn from(center: BlueprintCenter) -> Self {
        match center {
            BlueprintCenter::Full => FULL_LETTER,
            BlueprintCenter::Empty => EMPTY_LETTER,
            BlueprintCenter::Any => ANY_LETTER,
        }
    }
}

impl TryFrom<char> for BlueprintCenter {
    type Error = String;

    fn try_from(letter: char) -> Result<Self, Self::Error> {
        match letter {
            FULL_LETTER => Ok(Self::Full),
            EMPTY_LETTER => Ok(Self::Empty),
            ANY_LETTER => Ok(Self::Any),
            other => Err(format!("unknown center letter {other}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintGroup {
    pub name: String,
    pub tiles: Vec<usize>,
    pub neighbors: String,
    pub chance: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientations: Option<Vec<String>>,
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

pub fn embed(rules: &str, blueprint_json: &str) -> String {
    let commented: String = blueprint_json
        .lines()
        .map(|line| format!("{COMMENT_PREFIX}{line}\n"))
        .collect();

    format!(
        "{}\n{BEGIN_MARKER}\n{commented}{END_MARKER}\n",
        rules.trim_end()
    )
}

pub fn parse_embedded(rules: &str) -> Result<Loaded, BlueprintError> {
    let lines: Vec<&str> = rules.lines().map(str::trim_end).collect();
    let begin = lines
        .iter()
        .position(|line| *line == BEGIN_MARKER)
        .ok_or(BlueprintError::NoEmbedded)?;
    let body = &lines[begin + 1..];
    let end = body
        .iter()
        .position(|line| *line == END_MARKER)
        .ok_or(BlueprintError::IncompleteEmbedded)?;

    parse(&uncommented(&body[..end]))
}

fn uncommented(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|line| {
            line.strip_prefix(COMMENT_PREFIX)
                .or_else(|| line.strip_prefix(BARE_COMMENT))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        if project.has_rules(tile.id) {
            return Err(BlueprintError::DuplicateTile(tile.id));
        }
        if tile.rules.is_empty() {
            return Err(BlueprintError::NoRules(tile.id));
        }
        for rule in &tile.rules {
            project.add_rule(tile.id, load_rule(tile.id, rule)?);
        }
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
            if project.has_rules(tile) {
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
        tiles: project.configured_tiles().map(store_tile).collect(),
        deactivated: project.deactivated_tiles(),
        groups: project.groups().iter().map(store_group).collect(),
    }
}

fn store_tile((id, rules): (usize, &[TileRule])) -> BlueprintTile {
    BlueprintTile {
        id,
        rules: rules.iter().map(store_rule).collect(),
    }
}

fn store_rule(rule: &TileRule) -> BlueprintRule {
    let orientations = rule.orientations().transforms();

    BlueprintRule {
        neighbors: store_states(rule.neighborhood.cells().map(|(_, state)| state)),
        extent: rule.neighborhood.extent().into(),
        chance: rule.chance.percent(),
        orientations: orientations.into_iter().map(orientation_name).collect(),
        drawn: (rule.drawn != Transform::IDENTITY).then(|| orientation_name(rule.drawn)),
        center: match rule.center {
            RuleCenter::Full => BlueprintCenter::Full,
            RuleCenter::Empty => BlueprintCenter::Empty,
            RuleCenter::Any => BlueprintCenter::Any,
        },
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

fn parse_orientation(name: &str) -> Option<Transform> {
    Transform::ALL
        .into_iter()
        .find(|transform| orientation_name(*transform) == name)
}

fn load_orientation(tile: usize, name: &str) -> Result<Transform, BlueprintError> {
    parse_orientation(name).ok_or_else(|| BlueprintError::OrientationName {
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

fn load_rule(tile: usize, rule: &BlueprintRule) -> Result<TileRule, BlueprintError> {
    let states = load_states(&rule.neighbors)?;
    let neighborhood = Neighborhood::from_states(rule.extent.into(), states)
        .ok_or(BlueprintError::NeighborhoodShape(tile))?;

    let orientations = load_orientations(tile, &rule.orientations)?;
    let drawn = match &rule.drawn {
        Some(name) => load_orientation(tile, name)?,
        None => Transform::IDENTITY,
    };
    let (mods, custom) = match orientations.preset() {
        Some(mods) => (mods, None),
        None => (TileMods::default(), Some(orientations)),
    };

    Ok(TileRule {
        neighborhood,
        center: match rule.center {
            BlueprintCenter::Full => RuleCenter::Full,
            BlueprintCenter::Empty => RuleCenter::Empty,
            BlueprintCenter::Any => RuleCenter::Any,
        },
        mods,
        custom,
        drawn,
        chance: Chance::new(rule.chance)?,
    })
}

fn neighbor_box(((min_x, min_y), (max_x, max_y)): (Cell, Cell)) -> impl Iterator<Item = Cell> {
    (min_y - RING_EXTENT..=max_y + RING_EXTENT)
        .flat_map(move |y| (min_x - RING_EXTENT..=max_x + RING_EXTENT).map(move |x| (x, y)))
}

fn store_group(group: &TileGroup) -> BlueprintGroup {
    let bounds = group.bounds().expect("a validated group has tiles");

    BlueprintGroup {
        name: group.name.clone(),
        tiles: group.footprint(),
        neighbors: store_states(neighbor_box(bounds).map(|cell| {
            group
                .neighbors
                .get(&cell)
                .copied()
                .unwrap_or(CellState::Any)
                .into()
        })),
        chance: group.chance.percent(),
        orientations: store_group_orientations(group.mods),
    }
}

fn load_group_states(group: &TileGroup, text: &str) -> Result<Vec<CellState>, BlueprintError> {
    if text.contains(TILE_OPEN) {
        return Err(BlueprintError::GroupNeighborTile(group.name.clone()));
    }

    let states = load_states(text).map_err(|source| BlueprintError::GroupNeighborText {
        group: group.name.clone(),
        source: Box::new(source),
    })?;

    states
        .into_iter()
        .map(|state| {
            CellState::from_neighbor(state)
                .ok_or_else(|| BlueprintError::GroupNeighborTile(group.name.clone()))
        })
        .collect()
}

fn load_neighbors(group: &mut TileGroup, text: &str) -> Result<(), BlueprintError> {
    let bounds = group.bounds().ok_or(GroupError::TooSmall)?;
    let states = load_group_states(group, text)?;
    let cells: Vec<Cell> = neighbor_box(bounds).collect();
    if cells.len() != states.len() {
        return Err(BlueprintError::GroupNeighborShape(group.name.clone()));
    }

    let conditions: Vec<(Cell, CellState)> = cells
        .into_iter()
        .zip(states)
        .filter(|(_, state)| *state != CellState::Any)
        .collect();
    if conditions
        .iter()
        .any(|(cell, _)| !group.neighbors.contains_key(cell))
    {
        return Err(BlueprintError::GroupNeighborCell(group.name.clone()));
    }

    group
        .neighbors
        .values_mut()
        .for_each(|state| *state = CellState::Any);
    group.neighbors.extend(conditions);

    Ok(())
}

fn store_group_orientations(mods: TileMods) -> Option<Vec<String>> {
    (mods != TileMods::default()).then(|| {
        Orientations::generated(mods)
            .transforms()
            .into_iter()
            .map(orientation_name)
            .collect()
    })
}

fn load_group_mods(group: &BlueprintGroup) -> Result<TileMods, BlueprintError> {
    let Some(names) = &group.orientations else {
        return Ok(TileMods::default());
    };

    names
        .iter()
        .map(|name| parse_orientation(name))
        .collect::<Option<Orientations>>()
        .and_then(Orientations::preset)
        .ok_or_else(|| BlueprintError::GroupOrientations(group.name.clone()))
}

fn load_group(group: &BlueprintGroup) -> Result<TileGroup, BlueprintError> {
    if group.tiles.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(BlueprintError::GroupTiles(group.name.clone()));
    }

    let mut loaded = TileGroup {
        chance: Chance::new(group.chance)?,
        mods: load_group_mods(group)?,
        ..TileGroup::new(group.name.clone(), group.tiles.iter().copied().collect())
    };
    load_neighbors(&mut loaded, &group.neighbors)?;

    Ok(loaded)
}

fn store_states(states: impl Iterator<Item = NeighborState>) -> String {
    states.fold(String::new(), |mut text, state| {
        match state {
            NeighborState::Is(tile) => {
                text.push(TILE_OPEN);
                text.push_str(&tile.to_string());
                text.push(TILE_CLOSE);
            }
            NeighborState::Empty => text.push(EMPTY_LETTER),
            NeighborState::Full => text.push(FULL_LETTER),
            NeighborState::Any => text.push(ANY_LETTER),
        }
        text
    })
}

fn load_tile_state(digits: &str) -> Result<NeighborState, BlueprintError> {
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(BlueprintError::NeighborTileText(digits.to_owned()));
    }
    match digits.parse::<usize>() {
        Err(_) => Err(BlueprintError::NeighborTileText(digits.to_owned())),
        Ok(tile) if tile >= TILE_COUNT => Err(BlueprintError::TileId(tile)),
        Ok(tile) if tile == AIR_TILE => Err(BlueprintError::AirNeighborTile),
        Ok(tile) => Ok(NeighborState::Is(tile)),
    }
}

fn load_states(text: &str) -> Result<Vec<NeighborState>, BlueprintError> {
    let mut chars = text.chars();
    let mut states = Vec::new();
    while let Some(next) = chars.next() {
        let state = match next {
            TILE_OPEN => {
                let (digits, rest) = chars
                    .as_str()
                    .split_once(TILE_CLOSE)
                    .ok_or_else(|| BlueprintError::NeighborTileText(chars.as_str().to_owned()))?;
                chars = rest.chars();
                load_tile_state(digits)?
            }
            letter => STATE_LETTERS
                .iter()
                .find(|(candidate, _)| *candidate == letter)
                .map(|(_, state)| *state)
                .ok_or(BlueprintError::NeighborLetter(letter))?,
        };
        states.push(state);
    }

    Ok(states)
}
