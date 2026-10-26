use std::collections::{BTreeMap, BTreeSet};

use crate::model::group::TileGroup;
use crate::model::pool::ChanceMode;
use crate::model::tile::{AIR_TILE, TileRule, highest_free_tile, referenced_tiles};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Project {
    rules: BTreeMap<usize, Vec<TileRule>>,
    deactivated: BTreeSet<usize>,
    groups: Vec<TileGroup>,
    chance_mode: ChanceMode,
}

impl Project {
    pub fn chance_mode(&self) -> ChanceMode {
        self.chance_mode
    }

    pub fn set_chance_mode(&mut self, mode: ChanceMode) {
        self.chance_mode = mode;
    }

    pub fn rules_of(&self, tile: usize) -> &[TileRule] {
        self.rules.get(&tile).map_or(&[], Vec::as_slice)
    }

    pub fn has_rules(&self, tile: usize) -> bool {
        self.rules.contains_key(&tile)
    }

    pub fn configured_tiles(&self) -> impl Iterator<Item = (usize, &[TileRule])> {
        self.rules
            .iter()
            .map(|(&tile, rules)| (tile, rules.as_slice()))
    }

    pub fn is_deactivated(&self, tile: usize) -> bool {
        self.deactivated.contains(&tile)
    }

    pub fn add_rule(&mut self, tile: usize, rule: TileRule) -> usize {
        let rules = self.rules.entry(tile).or_default();
        rules.push(rule);

        rules.len() - 1
    }

    pub fn replace_rule(&mut self, tile: usize, index: usize, rule: TileRule) {
        if let Some(slot) = self
            .rules
            .get_mut(&tile)
            .and_then(|rules| rules.get_mut(index))
        {
            *slot = rule;
        }
    }

    pub fn remove_rule(&mut self, tile: usize, index: usize) {
        let Some(rules) = self.rules.get_mut(&tile) else {
            return;
        };
        if index < rules.len() {
            rules.remove(index);
        }
        if rules.is_empty() {
            self.rules.remove(&tile);
        }
    }

    pub fn deactivate(&mut self, tile: usize) {
        self.deactivated.insert(tile);
    }

    pub fn reactivate(&mut self, tile: usize) {
        self.deactivated.remove(&tile);
    }

    pub fn deactivated_tiles(&self) -> Vec<usize> {
        self.deactivated.iter().copied().collect()
    }

    pub fn rules(&self) -> Vec<(usize, TileRule)> {
        self.all_rules()
            .into_iter()
            .filter(|(tile, _)| !self.is_deactivated(*tile))
            .collect()
    }

    pub fn all_rules(&self) -> Vec<(usize, TileRule)> {
        self.rules
            .iter()
            .flat_map(|(&tile, rules)| rules.iter().map(move |&rule| (tile, rule)))
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

    pub fn move_group(&mut self, from: usize, before: usize) -> Option<usize> {
        if from >= self.groups.len() || before > self.groups.len() {
            return None;
        }

        let group = self.groups.remove(from);
        let target = match before > from {
            true => before - 1,
            false => before,
        };
        self.groups.insert(target, group);

        Some(target)
    }

    pub fn is_free(&self, tile: usize) -> bool {
        is_free_tile(tile, self.has_rules(tile), &self.groups)
    }

    pub fn mask_tile(&self) -> Option<usize> {
        let referenced = referenced_tiles(&self.all_rules());
        highest_free_tile(|tile| self.is_free(tile) && !referenced.contains(&tile))
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

pub fn is_free_tile(tile: usize, has_rule: bool, groups: &[TileGroup]) -> bool {
    tile != AIR_TILE && !has_rule && groups.iter().all(|group| !group.covers(tile))
}
