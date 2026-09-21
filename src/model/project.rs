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
}

#[cfg(test)]
mod tests {
    use super::Project;
    use crate::model::neighbor::{NeighborState, Neighborhood};
    use crate::model::tile::TileRule;

    fn some_rule() -> TileRule {
        TileRule::new(Neighborhood::uniform(NeighborState::Full))
    }

    #[test]
    fn configuring_a_tile_clears_its_removal() {
        let mut project = Project::default();
        project.remove(7);
        project.set_rule(7, some_rule());

        assert!(!project.is_removed(7));
        assert_eq!(project.rule_count(), 1);
    }

    #[test]
    fn removing_a_tile_drops_its_rule() {
        let mut project = Project::default();
        project.set_rule(7, some_rule());
        project.remove(7);

        assert!(project.rule(7).is_none());
        assert!(project.is_removed(7));
    }

    #[test]
    fn restoring_leaves_the_tile_unconfigured() {
        let mut project = Project::default();
        project.remove(7);
        project.restore(7);

        assert!(!project.is_removed(7));
        assert!(project.rule(7).is_none());
    }
}
