use thiserror::Error;

use crate::model::group::{self, GroupError, GroupMode, TileGroup};
use crate::model::neighbor::{NEIGHBORS, NeighborState, Neighborhood};
use crate::model::tile::{Chance, TileRule};
use crate::model::transform::{self, Transform};

/// Group names live under a prefix of our own. rpp keywords and base.r globals
/// hold no `:`, and base.r reserves only the `g:` and `s:` prefixes, so nothing
/// emitted here can collide with them.
const OBJECT_PREFIX: &str = "o:";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("no tile and no group is configured, there is nothing to export")]
    NothingConfigured,

    #[error("group `{group}` is anchored on tile {tile}, which is also configured on its own")]
    AnchorIsConfigured { group: String, tile: usize },

    #[error(transparent)]
    Group(#[from] GroupError),
}

pub struct RuleSet<'a> {
    pub image_stem: &'a str,
    pub name: &'a str,
    pub tiles: &'a [(usize, TileRule)],
    pub groups: &'a [TileGroup],
}

pub fn render(rule_set: &RuleSet) -> Result<String, ExportError> {
    if rule_set.tiles.is_empty() && rule_set.groups.is_empty() {
        return Err(ExportError::NothingConfigured);
    }
    group::validate_all(rule_set.groups)?;
    reject_claimed_anchors(rule_set)?;

    let mut lines = vec![
        "// created with ddnet-automap-creator (https://github.com/iMilchshake/ddnet-automap-creator)"
            .to_owned(),
        "// compile with rpp (https://github.com/Aerll/rpp), base.r has to sit next to this file"
            .to_owned(),
        "#include \"base.r\"".to_owned(),
        format!("#output \"{}.rules\"", rule_set.image_stem),
        String::new(),
    ];

    if !rule_set.groups.is_empty() {
        lines.extend(rule_set.groups.iter().map(object_line));
        lines.push(String::new());
    }

    lines.push(format!("AutoMapper(\"{}\");", rule_set.name));
    lines.push("NewRun();".to_owned());
    lines.push(String::new());

    lines.extend(pool(expand(rule_set.tiles)).iter().map(insert_line));

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

fn reject_claimed_anchors(rule_set: &RuleSet) -> Result<(), ExportError> {
    for group in rule_set.groups {
        if rule_set
            .tiles
            .iter()
            .any(|(tile, _)| *tile == group.top_left)
        {
            return Err(ExportError::AnchorIsConfigured {
                group: group.name.clone(),
                tile: group.top_left,
            });
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
    match chance.is_full() {
        true => String::new(),
        false => format!(".Chance({})", chance.percent()),
    }
}

struct Variant {
    tile: usize,
    index: String,
    neighborhood: Neighborhood,
    chance: Chance,
}

struct Pool {
    neighborhood: Neighborhood,
    members: Vec<Variant>,
}

fn expand(tiles: &[(usize, TileRule)]) -> Vec<Variant> {
    let mut variants = Vec::new();

    for &(tile, rule) in tiles {
        let mut seen = Vec::new();
        for transform in transform::variants(rule.mods) {
            let neighborhood = transform.apply(&rule.neighborhood);
            if seen.contains(&neighborhood) {
                continue;
            }
            seen.push(neighborhood);

            variants.push(Variant {
                tile,
                index: index_name(tile, transform),
                neighborhood,
                chance: rule.chance,
            });
        }
    }

    variants.sort_by_key(|variant| (specificity(&variant.neighborhood), variant.tile));
    variants
}

fn pool(variants: Vec<Variant>) -> Vec<Pool> {
    let mut pools: Vec<Pool> = Vec::new();

    for variant in variants {
        match pools
            .iter_mut()
            .find(|pool| pool.neighborhood == variant.neighborhood)
        {
            Some(pool) => pool.members.push(variant),
            None => pools.push(Pool {
                neighborhood: variant.neighborhood,
                members: vec![variant],
            }),
        }
    }

    pools
}

fn specificity(neighborhood: &Neighborhood) -> usize {
    neighborhood
        .states()
        .iter()
        .filter(|state| **state != NeighborState::Any)
        .count()
}

fn index_name(tile: usize, transform: Transform) -> String {
    let suffix = transform.suffix();
    if suffix.is_empty() {
        return tile.to_string();
    }

    format!("{tile}.{suffix}")
}

fn insert_line(pool: &Pool) -> String {
    let indices: Vec<&str> = pool
        .members
        .iter()
        .map(|member| member.index.as_str())
        .collect();

    format!(
        "Insert({}){}{};",
        indices.join(", "),
        chance_clause(pool),
        test_clause(&pool.neighborhood),
    )
}

fn chance_clause(pool: &Pool) -> String {
    let every_chance_full = pool.members.iter().all(|member| member.chance.is_full());
    if every_chance_full {
        return match pool.members.len() {
            1 => String::new(),
            _ => ".Roll()".to_owned(),
        };
    }

    let chances: Vec<String> = pool
        .members
        .iter()
        .map(|member| member.chance.percent().to_string())
        .collect();

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
