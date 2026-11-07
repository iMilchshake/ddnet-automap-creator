use std::collections::VecDeque;
use std::mem;

use crate::model::rule_sets::RuleSets;

pub const MAX_STEPS: usize = 200;

pub struct History {
    undo: VecDeque<RuleSets>,
    redo: Vec<RuleSets>,
    committed: RuleSets,
}

impl History {
    pub fn new(rule_sets: RuleSets) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            committed: rule_sets,
        }
    }

    pub fn reset(&mut self, rule_sets: RuleSets) {
        *self = Self::new(rule_sets);
    }

    pub fn commit(&mut self, current: &RuleSets) {
        if self.matches_committed(current) {
            return;
        }

        let previous = mem::replace(&mut self.committed, current.clone());
        self.push_undo(previous);
        self.redo.clear();
    }

    pub fn can_undo(&self, current: &RuleSets) -> bool {
        !self.undo.is_empty() || !self.matches_committed(current)
    }

    pub fn can_redo(&self, current: &RuleSets) -> bool {
        !self.redo.is_empty() && self.matches_committed(current)
    }

    pub fn undo(&mut self, current: &RuleSets) -> Option<RuleSets> {
        self.commit(current);
        let previous = self.undo.pop_back()?;
        self.redo
            .push(mem::replace(&mut self.committed, previous.clone()));
        Some(previous)
    }

    pub fn redo(&mut self, current: &RuleSets) -> Option<RuleSets> {
        self.commit(current);
        let next = self.redo.pop()?;
        let previous = mem::replace(&mut self.committed, next.clone());
        self.push_undo(previous);
        Some(next)
    }

    fn matches_committed(&self, current: &RuleSets) -> bool {
        current.sets() == self.committed.sets()
    }

    fn push_undo(&mut self, snapshot: RuleSets) {
        if self.undo.len() == MAX_STEPS {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
    }
}
