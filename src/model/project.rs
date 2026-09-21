use std::collections::{BTreeMap, BTreeSet};

use crate::model::tile::TileRule;

#[derive(Debug, Clone, Default)]
pub struct Project {
    rules: BTreeMap<usize, TileRule>,
    removed: BTreeSet<usize>,
}

impl Project {
    pub fn rule(&self, tile: usize) -> Option<&TileRule> {
        self.rules.get(&tile)
    }

    pub fn is_removed(&self, tile: usize) -> bool {
        self.removed.contains(&tile)
    }

    pub fn set_rule(&mut self, tile: usize, rule: TileRule) {
        self.removed.remove(&tile);
        self.rules.insert(tile, rule);
    }

    pub fn remove(&mut self, tile: usize) {
        self.rules.remove(&tile);
        self.removed.insert(tile);
    }

    pub fn restore(&mut self, tile: usize) {
        self.removed.remove(&tile);
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn rules(&self) -> Vec<(usize, TileRule)> {
        self.rules
            .iter()
            .map(|(&tile, &rule)| (tile, rule))
            .collect()
    }
}
