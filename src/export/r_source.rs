use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};
use crate::model::pool::{self, ChanceMode, Pool};
use crate::model::tile::{Chance, MASK_TILE, TileRule};
use crate::model::transform::Transform;

/// Group names live under a prefix of our own. rpp keywords and base.r globals
/// hold no `:`, and base.r reserves only the `g:` and `s:` prefixes, so nothing
/// emitted here can collide with them.
const OBJECT_PREFIX: &str = "o:";

const MASK: &str = "g:mask";
const ON_MASK: &str = ".If(IndexAt([0, 0]).Is(g:mask))";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("no tile and no group is configured, there is nothing to export")]
    NothingConfigured,

    #[error("rule set name `{0}` must be 1 to 128 letters, digits, `_` or `-`")]
    Name(String),

    #[error("group `{group}` covers tile {tile}, which is also configured on its own")]
    TileIsConfigured { group: String, tile: usize },

    #[error("tile {MASK_TILE} is reserved as rpp's mask and cannot hold a rule")]
    MaskTile,

    #[error(transparent)]
    Group(#[from] GroupError),
}

pub struct RuleSet<'a> {
    pub image_stem: &'a str,
    pub name: &'a str,
    pub tiles: &'a [(usize, TileRule)],
    pub groups: &'a [TileGroup],
    pub chance_mode: ChanceMode,
}

pub fn output_file(image_stem: &str) -> String {
    format!("{image_stem}.rules")
}

pub fn render(rule_set: &RuleSet) -> Result<String, ExportError> {
    if rule_set.tiles.is_empty() && rule_set.groups.is_empty() {
        return Err(ExportError::NothingConfigured);
    }
    if !is_valid_name(rule_set.name) {
        return Err(ExportError::Name(rule_set.name.to_owned()));
    }
    if rule_set.tiles.iter().any(|(tile, _)| *tile == MASK_TILE) {
        return Err(ExportError::MaskTile);
    }
    group::validate_all(rule_set.groups)?;
    reject_claimed_tiles(rule_set)?;

    let mut lines = vec![
        "// created with ddnet-automap-creator (https://github.com/iMilchshake/ddnet-automap-creator)"
            .to_owned(),
        "// compile with rpp (https://github.com/Aerll/rpp), base.r has to sit next to this file"
            .to_owned(),
        "#include \"base.r\"".to_owned(),
        format!("#output \"{}\"", output_file(rule_set.image_stem)),
        String::new(),
    ];

    if !rule_set.groups.is_empty() {
        lines.extend(rule_set.groups.iter().map(object_line));
        lines.push(String::new());
    }

    lines.push(format!("AutoMapper(\"{}\");", rule_set.name));
    lines.push("NewRun();".to_owned());
    lines.push(String::new());

    lines.extend(
        pool::pools(rule_set.tiles)
            .iter()
            .flat_map(|pool| insert_lines(pool, rule_set.chance_mode)),
    );

    if !rule_set.groups.is_empty() {
        lines.push(String::new());
        lines.push("NewRun();".to_owned());
        lines.push("OverrideLayer();".to_owned());
        lines.extend(
            rule_set
                .groups
                .iter()
                .map(|group| insert_object_block(group, rule_set.groups)),
        );

        lines.push(String::new());
        lines.push("NewRun();".to_owned());
        lines.push("OverrideLayer();".to_owned());
        lines.push("Run().FillObjects();".to_owned());
    }

    lines.push(String::new());

    Ok(lines.join("\n"))
}

fn is_valid_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character))
}

fn reject_claimed_tiles(rule_set: &RuleSet) -> Result<(), ExportError> {
    for group in rule_set.groups {
        for (tile, _) in rule_set.tiles {
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

fn object_name(group: &TileGroup) -> String {
    format!("{OBJECT_PREFIX}{}", group.name)
}

fn object_line(group: &TileGroup) -> String {
    format!(
        "object {} = Rect({}, {});",
        object_name(group),
        group.top_left,
        group.bottom_right()
    )
}

fn insert_object_block(group: &TileGroup, every_group: &[TileGroup]) -> String {
    let names: Vec<String> = every_group.iter().map(object_name).collect();

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
        object_name(group),
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
