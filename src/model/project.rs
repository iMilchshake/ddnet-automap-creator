use std::collections::{BTreeMap, BTreeSet};

use crate::model::group::TileGroup;
use crate::model::tile::TileRule;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Project {
    rules: BTreeMap<usize, TileRule>,
    removed: BTreeSet<usize>,
    groups: Vec<TileGroup>,
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

    pub fn removed_tiles(&self) -> Vec<usize> {
        self.removed.iter().copied().collect()
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

    pub fn groups(&self) -> &[TileGroup] {
        &self.groups
    }

    /// New groups take priority over existing ones, so they go to the front.
    pub fn add_group(&mut self, group: TileGroup) {
        self.groups.insert(0, group);
    }

    /// Keeps the given order, unlike `add_group`, which promotes to the front.
    pub fn append_group(&mut self, group: TileGroup) {
        self.groups.push(group);
    }

    pub fn replace_group(&mut self, index: usize, group: TileGroup) {
        if let Some(slot) = self.groups.get_mut(index) {
            *slot = group;
        }
    }

    pub fn remove_group(&mut self, index: usize) {
        if index < self.groups.len() {
            self.groups.remove(index);
        }
    }

    pub fn raise_group(&mut self, index: usize) {
        if index > 0 && index < self.groups.len() {
            self.groups.swap(index - 1, index);
        }
    }

    pub fn lower_group(&mut self, index: usize) {
        if index + 1 < self.groups.len() {
            self.groups.swap(index, index + 1);
        }
    }

    pub fn is_free(&self, tile: usize) -> bool {
        self.rule(tile).is_none() && self.group_at(tile).is_none()
    }

    pub fn group_at(&self, tile: usize) -> Option<usize> {
        self.groups.iter().position(|group| group.covers(tile))
    }

    pub fn unused_group_name(&self) -> String {
        (0..)
            .map(|number| format!("group{number}"))
            .find(|name| self.groups.iter().all(|group| group.name != *name))
            .expect("the candidate names never run out")
    }
}
