use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};
use crate::model::pool::{self, ChanceMode, Pool, Variant};
use crate::model::project::is_free_tile;
use crate::model::tile::{AIR_TILE, Centre, Chance, TileRule, highest_free_tile, referenced_tiles};
use crate::model::transform::Transform;

/// Group names live under a prefix of our own. rpp keywords and base.r globals
/// hold no `:`, and base.r reserves only the `g:` and `s:` prefixes, so nothing
/// emitted here can collide with them.
const OBJECT_PREFIX: &str = "o:";

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY_URL: &str = "https://github.com/iMilchshake/ddnet-automap-creator";

const MASK: &str = "g:mask";
const ON_MASK: &str = ".If(IndexAt([0, 0]).Is(g:mask))";
const NO_DEFAULT_RULE: &str = "util:NoDefaultPosRule();";
const INSIDE_MAP_CONDITION: &str = "util:IsNotOut();";
const EMPTY_CENTRE_CONDITION: &str = "util:NewPosRule([0, 0]); util:IsEmpty();";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("rule set `{0}` has no tile and no group, there is nothing to export")]
    EmptyRuleSet(String),

    #[error("`{0}` names more than one rule set")]
    DuplicateName(String),

    #[error("rule set name `{0}` must be 1 to 128 letters, digits, `_` or `-`")]
    Name(String),

    #[error("group `{group}` covers tile {tile}, which is also configured on its own")]
    TileIsConfigured { group: String, tile: usize },

    #[error("no free tile left for rpp's mask in rule set `{0}`, unconfigure one")]
    NoMaskTile(String),

    #[error("tile {AIR_TILE} is reserved for air and cannot hold a rule")]
    AirTile,

    #[error(transparent)]
    Group(#[from] GroupError),
}

pub struct RuleSet<'a> {
    pub image_stem: &'a str,
    pub name: &'a str,
    pub tiles: &'a [(usize, TileRule)],
    pub deactivated: &'a [usize],
    pub groups: &'a [TileGroup],
    pub chance_mode: ChanceMode,
}

impl RuleSet<'_> {
    fn active_tiles(&self) -> Vec<(usize, TileRule)> {
        self.tiles
            .iter()
            .filter(|(tile, _)| !self.deactivated.contains(tile))
            .copied()
            .collect()
    }

    fn mask_tile(&self) -> Option<usize> {
        let referenced = referenced_tiles(self.tiles);
        highest_free_tile(|tile| {
            let has_rule = self.tiles.iter().any(|(used, _)| *used == tile);
            is_free_tile(tile, has_rule, self.groups) && !referenced.contains(&tile)
        })
    }

    fn needs_mask(&self) -> bool {
        pool::pools(&self.active_tiles()).iter().any(|pool| {
            pool.centre == Centre::Full
                && !pool.neighborhood.is_tile_specific()
                && pool.members.len() > 1
        })
    }

    fn mask_line(&self) -> Result<Option<String>, ExportError> {
        if !self.needs_mask() {
            return Ok(None);
        }

        match self.mask_tile() {
            Some(tile) => Ok(Some(format!("{MASK} = {tile};"))),
            None => Err(ExportError::NoMaskTile(self.name.to_owned())),
        }
    }
}

pub fn credit() -> String {
    format!("created with ddnet-automap-creator v{APP_VERSION} ({REPOSITORY_URL})")
}

pub fn output_file(image_stem: &str) -> String {
    format!("{image_stem}.rules")
}

pub fn render(rule_sets: &[RuleSet]) -> Result<String, ExportError> {
    for rule_set in rule_sets {
        if rule_set.active_tiles().is_empty() && rule_set.groups.is_empty() {
            return Err(ExportError::EmptyRuleSet(rule_set.name.to_owned()));
        }
        if !is_valid_name(rule_set.name) {
            return Err(ExportError::Name(rule_set.name.to_owned()));
        }
        if rule_set.tiles.iter().any(|(tile, _)| *tile == AIR_TILE) {
            return Err(ExportError::AirTile);
        }
        group::validate_all(rule_set.groups)?;
        reject_claimed_tiles(rule_set.groups, rule_set.tiles)?;
    }
    if let Some(duplicate) = duplicate_name(rule_sets) {
        return Err(ExportError::DuplicateName(duplicate));
    }

    let Some(first) = rule_sets.first() else {
        return Ok(String::new());
    };
    // We only need prefixes to fix naming collisions, if we have more than one rule set.
    let namespaced = rule_sets.len() > 1;

    let mut lines = vec![
        format!("// {}", credit()),
        "// compile with rpp (https://github.com/Aerll/rpp), base.r has to sit next to this file"
            .to_owned(),
        "#include \"base.r\"".to_owned(),
        format!("#output \"{}\"", output_file(first.image_stem)),
        String::new(),
    ];

    for (index, rule_set) in rule_sets.iter().enumerate() {
        render_rule_set(&mut lines, index, namespaced, rule_set)?;
    }

    Ok(lines.join("\n"))
}

fn render_rule_set(
    lines: &mut Vec<String>,
    index: usize,
    namespaced: bool,
    rule_set: &RuleSet,
) -> Result<(), ExportError> {
    let mask_line = rule_set.mask_line()?;

    if !rule_set.groups.is_empty() {
        lines.extend(
            rule_set
                .groups
                .iter()
                .map(|group| object_line(index, namespaced, group)),
        );
        lines.push(String::new());
    }

    lines.push(format!("AutoMapper(\"{}\");", rule_set.name));
    lines.extend(mask_line);

    let pools = pool::pools(&rule_set.active_tiles());
    let (empty_centred, full_centred): (Vec<&Pool>, Vec<&Pool>) =
        pools.iter().partition(|pool| pool.centre == Centre::Empty);
    let (specific, plain): (Vec<&Pool>, Vec<&Pool>) = full_centred
        .into_iter()
        .partition(|pool| pool.neighborhood.is_tile_specific());

    let mut runs: Vec<Vec<String>> = Vec::new();
    if !plain.is_empty() {
        let inserts = plain
            .iter()
            .flat_map(|pool| insert_lines(pool, rule_set.chance_mode));
        runs.push(new_run(false, inserts));
    }
    if !specific.is_empty() {
        let rules = specific
            .iter()
            .flat_map(|pool| copy_run_lines(pool, rule_set.chance_mode));
        runs.push(new_run(false, rules));
    }
    if !rule_set.groups.is_empty() {
        let objects = rule_set
            .groups
            .iter()
            .map(|group| insert_object_block(index, namespaced, group, rule_set.groups));
        runs.push(new_run(true, objects));
        runs.push(new_run(true, ["Run().FillObjects();".to_owned()]));
    }
    if !empty_centred.is_empty() {
        let rules = empty_centred
            .iter()
            .flat_map(|pool| copy_run_lines(pool, rule_set.chance_mode));
        runs.push(new_run(false, rules));
    }

    lines.extend(runs.join(&String::new()));
    lines.push(String::new());

    Ok(())
}

fn new_run(overriding: bool, body: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut run = vec!["NewRun();".to_owned()];
    if overriding {
        run.push("OverrideLayer();".to_owned());
    }
    run.extend(body);

    run
}

// Checks whether rules with colliding names exist.
fn duplicate_name(rule_sets: &[RuleSet]) -> Option<String> {
    let mut seen: Vec<&str> = Vec::with_capacity(rule_sets.len());
    for rule_set in rule_sets {
        if seen.contains(&rule_set.name) {
            return Some(rule_set.name.to_owned());
        }
        seen.push(rule_set.name);
    }
    None
}

fn is_valid_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
}

fn reject_claimed_tiles(
    groups: &[TileGroup],
    tiles: &[(usize, TileRule)],
) -> Result<(), ExportError> {
    for group in groups {
        for (tile, _) in tiles {
            if group.covers(*tile) {
                return Err(ExportError::TileIsConfigured {
                    group: group.name.clone(),
                    tile: *tile,
                });
            }
        }
    }

    Ok(())
}

/// Construct a globally unique object name which does not conflict with rpp's
/// reserved keywords or names of different rule sets by adding unique prefixes
fn object_name(index: usize, namespaced: bool, group: &TileGroup) -> String {
    match namespaced {
        true => format!("{OBJECT_PREFIX}{index}{}", group.name),
        false => format!("{OBJECT_PREFIX}{}", group.name),
    }
}

fn object_line(index: usize, namespaced: bool, group: &TileGroup) -> String {
    format!(
        "object {} = Rect({}, {});",
        object_name(index, namespaced, group),
        group.top_left,
        group.bottom_right()
    )
}

fn insert_object_block(
    index: usize,
    namespaced: bool,
    group: &TileGroup,
    every_group: &[TileGroup],
) -> String {
    let names: Vec<String> = every_group
        .iter()
        .map(|group| object_name(index, namespaced, group))
        .collect();

    let mut tests = vec![
        "    Object().HasSpace()".to_owned(),
        format!("    Object().IsNotOverlapping({})", names.join(", ")),
    ];
    if group.mode == GroupMode::Decorate {
        let offsets: Vec<String> = group
            .ring_offsets()
            .iter()
            .map(|(x, y)| format!("[{x}, {y}]"))
            .collect();
        tests.push(format!("    IndexAt({}).IsFull()", offsets.join(", ")));
    }

    format!(
        "InsertObject({}){}.If(\n{}\n);",
        object_name(index, namespaced, group),
        chance_call(group.chance),
        tests.join(",\n"),
    )
}

fn chance_call(chance: Chance) -> String {
    percent_call(chance.percent())
}

fn percent_call(percent: f32) -> String {
    match percent >= Chance::FULL.percent() {
        true => String::new(),
        false => format!(".Chance({percent})"),
    }
}

fn index_name(tile: usize, transform: Transform) -> String {
    let suffix = transform.suffix();
    if suffix.is_empty() {
        return tile.to_string();
    }

    format!("{tile}.{suffix}")
}

fn insert_lines(pool: &Pool, mode: ChanceMode) -> Vec<String> {
    let test = test_clause(&pool.neighborhood);
    let [member] = pool.members.as_slice() else {
        return rolled_lines(pool, mode, &test);
    };

    let chance = match mode {
        ChanceMode::Normalize => String::new(),
        ChanceMode::Exact => chance_call(member.chance),
    };
    let index = index_name(member.tile, member.transform);

    vec![format!("Insert({index}){chance}{test};")]
}

/// rpp's documented way to randomize: mark the cells with its mask, then roll
/// the pool on the mask only, so exactly one member replaces each mark.
fn rolled_lines(pool: &Pool, mode: ChanceMode, test: &str) -> Vec<String> {
    let indices: Vec<String> = pool
        .members
        .iter()
        .map(|member| index_name(member.tile, member.transform))
        .collect();

    vec![
        format!("Insert({MASK}){}{test};", percent_call(pool.coverage(mode))),
        format!(
            "Insert({}){}{ON_MASK};",
            indices.join(", "),
            pool_chance(pool)
        ),
    ]
}

/// Every mark must be replaced, so chances summing below 100 are scaled up;
/// rpp scales totals above 100 down on its own.
fn pool_chance(pool: &Pool) -> String {
    let first = pool.members[0].chance;
    if pool.members.iter().all(|member| member.chance == first) {
        return ".Roll()".to_owned();
    }

    let chances: Vec<f32> = match pool.total() >= Chance::FULL.percent() {
        true => pool
            .members
            .iter()
            .map(|member| member.chance.percent())
            .collect(),
        false => pool.shares(ChanceMode::Normalize),
    };
    let chances: Vec<String> = chances.iter().map(f32::to_string).collect();

    format!(".Chance({})", chances.join(", "))
}

fn test_clause(neighborhood: &Neighborhood) -> String {
    let full = names_with(neighborhood, NeighborState::Full);
    let empty = names_with(neighborhood, NeighborState::Empty);
    if full.is_empty() && empty.is_empty() {
        return String::new();
    }

    let mut clause = String::from(".If(IndexAt([0, 0])");
    if !full.is_empty() {
        clause.push_str(&format!(".IsFullAt({})", full.join(", ")));
    }
    if !empty.is_empty() {
        clause.push_str(&format!(".IsEmptyAt({})", empty.join(", ")));
    }
    clause.push(')');

    clause
}

fn names_with(neighborhood: &Neighborhood, state: NeighborState) -> Vec<&'static str> {
    NEIGHBORS
        .iter()
        .enumerate()
        .filter(|(index, _)| neighborhood.state(*index) == state)
        .map(|(_, neighbor)| neighbor.rpp_name)
        .collect()
}

pub fn last_wins_odds(shares: &[f32], covers_all: bool) -> Vec<Option<f32>> {
    let full = Chance::FULL.percent();

    let mut odds: Vec<Option<f32>> = shares
        .iter()
        .rev()
        .scan(0.0, |later, share| {
            let one_out_of = (full - *later) / share;
            *later += share;

            Some(Some(one_out_of))
        })
        .collect();
    odds.reverse();

    if covers_all && let Some(first) = odds.first_mut() {
        *first = None;
    }

    odds
}

fn copy_run_lines(pool: &Pool, mode: ChanceMode) -> Vec<String> {
    let covers_all = pool.coverage(mode) >= Chance::FULL.percent();
    let odds = last_wins_odds(&pool.shares(mode), covers_all);

    pool.members
        .iter()
        .zip(odds)
        .flat_map(|(member, odds)| index_rule_lines(member, odds))
        .collect()
}

fn index_rule_lines(member: &Variant, odds: Option<f32>) -> Vec<String> {
    let chance = odds
        .map(|odds| format!(" util:Chance({odds});"))
        .unwrap_or_default();
    let (no_default, centre_condition) = match member.centre {
        Centre::Full => (String::new(), None),
        Centre::Empty => (
            format!(" {NO_DEFAULT_RULE}"),
            Some(EMPTY_CENTRE_CONDITION.to_owned()),
        ),
    };
    let head = format!(
        "util:NewRule(); util:InsertIndex({});{chance}{no_default}",
        index_name(member.tile, member.transform)
    );

    let conditions = NEIGHBORS.iter().enumerate().flat_map(|(index, neighbor)| {
        let state = member.neighborhood.state(index);
        let position = format!("util:NewPosRule([{}, {}]);", neighbor.dx, neighbor.dy);
        let keep_inside = member.centre == Centre::Empty && state == NeighborState::Full;

        [
            neighbor_condition(state),
            keep_inside.then(|| INSIDE_MAP_CONDITION.to_owned()),
        ]
        .into_iter()
        .flatten()
        .map(move |condition| format!("{position} {condition}"))
    });

    std::iter::once(head)
        .chain(centre_condition)
        .chain(conditions)
        .collect()
}

fn neighbor_condition(state: NeighborState) -> Option<String> {
    match state {
        NeighborState::Any => None,
        NeighborState::Full => Some("util:IsFull();".to_owned()),
        NeighborState::Empty => Some("util:IsEmpty();".to_owned()),
        NeighborState::Is(tile) => Some(format!("util:Is({tile});")),
    }
}
