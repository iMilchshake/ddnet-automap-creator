use thiserror::Error;

use crate::model::group::{self, Cell, GroupCopy, GroupError, TileGroup};
use crate::model::neighbor::{NeighborState, Neighborhood, neighbor_at};
use crate::model::pool::{self, ChanceMode, Pool, PoolRuns, Variant};
use crate::model::project::{Project, is_free_tile};
use crate::model::tile::{
    AIR_TILE, Center, Chance, TILESET_SIDE, TileRule, highest_free_tile, referenced_tiles,
};
use crate::model::transform::Transform;

/// Group names live under a prefix of our own. rpp keywords and base.r globals
/// hold no `:`, and base.r reserves only the `g:` and `s:` prefixes, so nothing
/// emitted here can collide with them.
const OBJECT_PREFIX: &str = "o:";

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY_URL: &str = "https://github.com/iMilchshake/ddnet-automap-creator";

const MASK: &str = "g:mask";
const ON_MASK: &str = ".If(IndexAt([0, 0]).Is(g:mask))";
const CENTER_TEST: &str = "IndexAt([0, 0])";
const NO_DEFAULT_POS_RULE: &str = ".NoDefaultPosRule()";
const FILL_OBJECTS: &str = "Run().FillObjects();";
const HAS_SPACE: &str = "Object().HasSpace()";
const ANCHOR: Cell = (0, 0);

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Ruleset `{0}` has no rules")]
    EmptyRuleSet(String),

    #[error("Ruleset name is already in use")]
    DuplicateName(String),

    #[error("Ruleset names need 1 to 128 letters, digits, _ or -")]
    Name(String),

    #[error("Group `{group}` can't overlap with tile {tile}")]
    TileIsConfigured { group: String, tile: usize },

    #[error("Ruleset `{0}` uses every tile, but one must stay free")]
    NoMaskTile(String),

    #[error("Tile {AIR_TILE} (air) can't have rules")]
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
            .cloned()
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
            pool.center == Center::Full
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

pub fn render_rule_sets(
    rule_sets: &[(String, Project)],
    image_stem: &str,
) -> Result<String, ExportError> {
    let tiles: Vec<Vec<(usize, TileRule)>> = rule_sets
        .iter()
        .map(|(_, project)| project.all_rules())
        .collect();
    let deactivated: Vec<Vec<usize>> = rule_sets
        .iter()
        .map(|(_, project)| project.deactivated_tiles())
        .collect();

    let sets: Vec<RuleSet> = rule_sets
        .iter()
        .zip(tiles.iter().zip(&deactivated))
        .map(|((name, project), (tiles, deactivated))| RuleSet {
            image_stem,
            name,
            tiles,
            deactivated,
            groups: project.groups(),
            chance_mode: project.chance_mode(),
        })
        .collect();

    render(&sets)
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
                .flat_map(|group| object_lines(index, namespaced, group)),
        );
        lines.push(String::new());
    }

    lines.push(format!("AutoMapper(\"{}\");", rule_set.name));
    lines.extend(mask_line);

    let pools = pool::pools(&rule_set.active_tiles());
    let PoolRuns {
        plain,
        specific,
        empty_centered,
    } = PoolRuns::split(&pools);

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
    if !empty_centered.is_empty() {
        let rules = empty_centered
            .iter()
            .flat_map(|pool| copy_run_lines(pool, rule_set.chance_mode));
        runs.push(new_run(false, rules));
    }
    if !rule_set.groups.is_empty() {
        let objects = rule_set
            .groups
            .iter()
            .map(|group| insert_object_block(index, namespaced, group, rule_set.groups))
            .chain([FILL_OBJECTS.to_owned()]);
        runs.push(new_run(true, objects));
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
    format!("{OBJECT_PREFIX}{}", scoped_name(index, namespaced, group))
}

fn scoped_name(index: usize, namespaced: bool, group: &TileGroup) -> String {
    match namespaced {
        true => format!("{index}{}", group.name),
        false => group.name.clone(),
    }
}

fn object_lines(index: usize, namespaced: bool, group: &TileGroup) -> Vec<String> {
    let object = object_name(index, namespaced, group);
    if let Some((top_left, bottom_right)) = group.as_rect() {
        return vec![format!(
            "object {object} = Rect({top_left}, {bottom_right});"
        )];
    }

    let entries: Vec<String> = row_runs(&group.footprint())
        .into_iter()
        .map(|(first, last)| match first == last {
            true => first.to_string(),
            false => format!("{first}_{last}"),
        })
        .collect();

    vec![format!(
        "object {object} = Indices({});",
        entries.join(", ")
    )]
}

fn row_runs(sorted_tiles: &[usize]) -> Vec<(usize, usize)> {
    sorted_tiles.iter().fold(Vec::new(), |mut runs, &tile| {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == tile && tile % TILESET_SIDE != 0 => *last = tile,
            _ => runs.push((tile, tile)),
        }
        runs
    })
}

fn insert_object_block(
    index: usize,
    namespaced: bool,
    group: &TileGroup,
    every_group: &[TileGroup],
) -> String {
    let names: Vec<String> = every_group
        .iter()
        .flat_map(|other| copy_names(index, namespaced, other))
        .collect();
    let object = object_name(index, namespaced, group);
    let copies = group.copies();

    copies
        .iter()
        .enumerate()
        .map(|(position, copy)| {
            format!(
                "InsertObject({}){}.If(\n{}\n){};",
                with_suffix(object.clone(), copy.transform),
                percent_call(copy_percent(group.chance, position, copies.len())),
                copy_tests(copy, &names).join(",\n"),
                anchor_call(copy),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn anchor_call(copy: &GroupCopy) -> &'static str {
    match copy.full.contains(&ANCHOR) {
        true => "",
        false => NO_DEFAULT_POS_RULE,
    }
}

fn copy_tests(copy: &GroupCopy, names: &[String]) -> Vec<String> {
    let all_own_full = copy.cells.iter().all(|cell| copy.full.contains(cell));
    let (space_test, full): (Option<String>, Vec<Cell>) = match all_own_full {
        true => (
            Some(HAS_SPACE.to_owned()),
            copy.full
                .iter()
                .filter(|cell| !copy.cells.contains(cell))
                .copied()
                .collect(),
        ),
        false => (None, copy.full.clone()),
    };

    [
        Some(format!("Object().IsNotOverlapping({})", names.join(", "))),
        space_test,
        index_test(&full, "IsFull"),
        index_test(&copy.empty, "IsEmpty"),
    ]
    .into_iter()
    .flatten()
    .map(|test| format!("    {test}"))
    .collect()
}

fn index_test(offsets: &[Cell], check: &str) -> Option<String> {
    (!offsets.is_empty()).then(|| format!("IndexAt({}).{check}()", positions(offsets).join(", ")))
}

fn copy_names(index: usize, namespaced: bool, group: &TileGroup) -> Vec<String> {
    let object = object_name(index, namespaced, group);

    group
        .copies()
        .iter()
        .map(|copy| with_suffix(object.clone(), copy.transform))
        .collect()
}

fn copy_percent(chance: Chance, position: usize, count: usize) -> f32 {
    let percent = chance.percent();
    let taken_before = position as f32 * percent / Chance::FULL.percent();

    percent / (count as f32 - taken_before)
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

fn with_suffix(name: String, transform: Transform) -> String {
    let suffix = transform.suffix();
    if suffix.is_empty() {
        return name;
    }

    format!("{name}.{suffix}")
}

fn index_name(tile: usize, transform: Transform) -> String {
    with_suffix(tile.to_string(), transform)
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
    if_clause(&center_tests(neighborhood, Center::Full))
}

fn if_clause(tests: &[String]) -> String {
    match tests.is_empty() {
        true => String::new(),
        false => format!(".If({})", tests.join(", ")),
    }
}

fn center_tests(neighborhood: &Neighborhood, center: Center) -> Vec<String> {
    let full = names_with(neighborhood, NeighborState::Full);
    let empty = names_with(neighborhood, NeighborState::Empty);
    let empty_center = center == Center::Empty;

    let named = (empty_center || !full.is_empty() || !empty.is_empty()).then(|| {
        let mut test = CENTER_TEST.to_owned();
        if empty_center {
            test.push_str(".IsEmpty()");
        }
        if !full.is_empty() {
            test.push_str(&format!(".IsFullAt({})", full.join(", ")));
        }
        if !empty.is_empty() {
            test.push_str(&format!(".IsEmptyAt({})", empty.join(", ")));
        }
        test
    });
    let outer_full = outer_offsets_where(neighborhood, NeighborState::Full);
    let outer_empty = outer_offsets_where(neighborhood, NeighborState::Empty);

    named
        .into_iter()
        .chain(index_test(&outer_full, "IsFull"))
        .chain(index_test(&outer_empty, "IsEmpty"))
        .collect()
}

fn offsets_where(
    neighborhood: &Neighborhood,
    matches: impl Fn(NeighborState) -> bool,
) -> Vec<Cell> {
    neighborhood
        .cells()
        .filter(|(_, state)| matches(*state))
        .map(|(offset, _)| offset)
        .collect()
}

fn outer_offsets_where(neighborhood: &Neighborhood, wanted: NeighborState) -> Vec<Cell> {
    offsets_where(neighborhood, |state| state == wanted)
        .into_iter()
        .filter(|offset| neighbor_at(*offset).is_none())
        .collect()
}

fn positions(offsets: &[Cell]) -> Vec<String> {
    offsets.iter().map(|(x, y)| format!("[{x}, {y}]")).collect()
}

fn picked_tests(neighborhood: &Neighborhood) -> Vec<String> {
    let mut tiles: Vec<usize> = Vec::new();
    for (_, state) in neighborhood.cells() {
        if let Some(tile) = state.tile()
            && !tiles.contains(&tile)
        {
            tiles.push(tile);
        }
    }

    tiles
        .into_iter()
        .map(|tile| {
            let offsets = offsets_where(neighborhood, |state| state.tile() == Some(tile));
            format!("IndexAt({}).Is({tile})", positions(&offsets).join(", "))
        })
        .collect()
}

fn inside_map_test(member: &Variant) -> Option<String> {
    if member.center != Center::Empty {
        return None;
    }

    let offsets = offsets_where(&member.neighborhood, |state| state == NeighborState::Full);

    (!offsets.is_empty()).then(|| format!("IndexAt({}).IsNotOut()", positions(&offsets).join(", ")))
}

fn names_with(neighborhood: &Neighborhood, wanted: NeighborState) -> Vec<&'static str> {
    neighborhood
        .cells()
        .filter(|(_, state)| *state == wanted)
        .filter_map(|(offset, _)| neighbor_at(offset))
        .map(|neighbor| neighbor.rpp_name)
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
        .map(|(member, odds)| index_rule_line(member, odds))
        .collect()
}

fn index_rule_line(member: &Variant, odds: Option<f32>) -> String {
    let no_default = match member.center {
        Center::Full => "",
        Center::Empty => NO_DEFAULT_POS_RULE,
    };
    let tests: Vec<String> = center_tests(&member.neighborhood, member.center)
        .into_iter()
        .chain(picked_tests(&member.neighborhood))
        .chain(inside_map_test(member))
        .collect();
    let chance = odds
        .map(|odds| format!(" util:Chance({odds});"))
        .unwrap_or_default();

    format!(
        "Insert({}){no_default}{};{chance}",
        index_name(member.tile, member.transform),
        if_clause(&tests)
    )
}
