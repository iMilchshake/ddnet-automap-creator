use twmap::Tile;
use twmap::automapper::{Chance, Condition, Config, IndexRule, Rule, TileCondition};
use twmap::ndarray::Array2;

const HASH_MAX: u32 = 1 << 16;
const HASH_PRIME: u32 = 31;
const MIX_SHIFTS: [u32; 4] = [17, 11, 15, 14];
const MIX_FACTORS: [u32; 3] = [0xed5ad4bb, 0xac4c1b51, 0x31848bab];

struct Layers<'tiles> {
    live: &'tiles mut Array2<Tile>,
    snapshot: Option<Array2<Tile>>,
}

impl Layers<'_> {
    fn read(&self) -> &Array2<Tile> {
        self.snapshot.as_ref().unwrap_or(&*self.live)
    }
}

pub fn run(config: &Config, seed: u32, tiles: &mut Array2<Tile>) {
    let (height, width) = tiles.dim();

    for (run_index, run) in config.runs.iter().enumerate() {
        let snapshot = run.layer_copy.then(|| tiles.clone());
        let layers = Layers {
            live: &mut *tiles,
            snapshot,
        };

        for y in 0..height {
            for x in 0..width {
                for (rule_index, rule) in run.rules.iter().enumerate() {
                    if !matches(rule, layers.read(), x, y) {
                        continue;
                    }
                    let location = [
                        seed,
                        run_index as u32,
                        rule_index as u32,
                        x as u32,
                        y as u32,
                    ];
                    if let Some(threshold) = threshold(&rule.chance)
                        && hash_location(location) >= threshold
                    {
                        continue;
                    }
                    layers.live[(y, x)] = rule.tile;
                }
            }
        }
    }
}

fn matches(rule: &IndexRule, tiles: &Array2<Tile>, x: usize, y: usize) -> bool {
    let default_holds = !rule.default_rule || tile_at(tiles, x, y, 0, 0).is_some_and(is_full);

    default_holds
        && rule
            .conditions
            .iter()
            .all(|condition| applies(condition, tiles, x, y))
}

fn is_full(tile: Tile) -> bool {
    tile.id != 0
}

fn tile_at(tiles: &Array2<Tile>, x: usize, y: usize, offset_x: i32, offset_y: i32) -> Option<Tile> {
    let column = usize::try_from(x as i64 + i64::from(offset_x)).ok()?;
    let row = usize::try_from(y as i64 + i64::from(offset_y)).ok()?;

    tiles.get((row, column)).copied()
}

fn applies(rule: &Rule, tiles: &Array2<Tile>, x: usize, y: usize) -> bool {
    let tile = tile_at(tiles, x, y, rule.offset.x, rule.offset.y);

    match &rule.condition {
        Condition::Empty => tile.map(|tile| tile.id) == Some(0),
        Condition::Full => tile.map(|tile| tile.id) != Some(0),
        Condition::WhiteList(list) => list.iter().any(|condition| tile_matches(condition, tile)),
        Condition::BlackList(list) => !list.iter().any(|condition| tile_matches(condition, tile)),
    }
}

fn tile_matches(condition: &TileCondition, tile: Option<Tile>) -> bool {
    match condition {
        TileCondition::Outside => tile.is_none(),
        TileCondition::Index(id) => tile.map(|tile| tile.id) == Some(*id),
        TileCondition::Tile(wanted) => tile == Some(*wanted),
    }
}

fn threshold(chance: &Chance) -> Option<u32> {
    let probability = match chance {
        Chance::Always => return None,
        Chance::OneOutOf(count) => count.recip(),
        Chance::Percentage(percent) => percent / 100.0,
    };

    Some((probability * HASH_MAX as f32) as u32)
}

fn mix(value: u32) -> u32 {
    let mut mixed = value.wrapping_add(1);
    for (shift, factor) in MIX_SHIFTS.into_iter().zip(MIX_FACTORS) {
        mixed ^= mixed >> shift;
        mixed = mixed.wrapping_mul(factor);
    }

    mixed ^ (mixed >> MIX_SHIFTS[3])
}

fn hash_location(location: [u32; 5]) -> u32 {
    let hash = location.into_iter().fold(1_u32, |hash, value| {
        hash.wrapping_mul(HASH_PRIME).wrapping_add(mix(value))
    });

    mix(hash.wrapping_mul(HASH_PRIME)) % HASH_MAX
}
