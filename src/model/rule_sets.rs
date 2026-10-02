use crate::model::project::Project;

#[derive(Debug, Clone, PartialEq)]
pub struct RuleSets {
    sets: Vec<(String, Project)>,
    active: usize,
}

impl RuleSets {
    pub fn new(name: String) -> Self {
        Self {
            sets: vec![(name, Project::default())],
            active: 0,
        }
    }

    pub fn from_parts(sets: Vec<(String, Project)>, active: usize) -> Self {
        let active = active.min(sets.len().saturating_sub(1));
        Self { sets, active }
    }

    pub fn sets(&self) -> &[(String, Project)] {
        &self.sets
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &Project {
        &self.sets[self.active].1
    }

    pub fn active_mut(&mut self) -> &mut Project {
        &mut self.sets[self.active].1
    }

    pub fn active_name(&self) -> &str {
        &self.sets[self.active].0
    }

    pub fn active_name_mut(&mut self) -> &mut String {
        &mut self.sets[self.active].0
    }

    pub fn name(&self, index: usize) -> Option<&str> {
        self.sets.get(index).map(|(name, _)| name.as_str())
    }

    pub fn is_name_shared(&self, index: usize) -> bool {
        let Some(name) = self.name(index) else {
            return false;
        };
        self.sets
            .iter()
            .enumerate()
            .any(|(other, (existing, _))| other != index && existing == name)
    }

    pub fn select(&mut self, index: usize) {
        if index < self.sets.len() {
            self.active = index;
        }
    }

    pub fn add(&mut self) -> usize {
        let name = self.unused_name();
        self.sets.push((name, Project::default()));
        self.active = self.sets.len() - 1;
        self.active
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if self.sets.len() <= 1 || index >= self.sets.len() {
            return false;
        }

        self.sets.remove(index);
        self.active = self.active.min(self.sets.len() - 1);
        true
    }

    /// Insert a copy at `index` right after it, and select it
    pub fn duplicate(&mut self, index: usize) -> Option<usize> {
        let (name, project) = self.sets.get(index)?.clone();
        let name = self.distinct_name(name);
        let insert_at = index + 1;

        self.sets.insert(insert_at, (name, project));
        self.active = insert_at;
        Some(insert_at)
    }

    pub fn merge(&mut self, incoming: RuleSets) -> usize {
        let merged = incoming.sets.len();
        for (name, project) in incoming.sets {
            let name = self.distinct_name(name);
            self.sets.push((name, project));
        }
        merged
    }

    fn has_name(&self, name: &str) -> bool {
        self.sets.iter().any(|(existing, _)| existing == name)
    }

    fn distinct_name(&self, name: String) -> String {
        let mut candidate = name.clone();
        let mut number = 1;
        while self.has_name(&candidate) {
            number += 1;
            candidate = format!("{name}{number}");
        }
        candidate
    }

    fn unused_name(&self) -> String {
        (0..)
            .map(|number| format!("ruleset{number}"))
            .find(|name| self.sets.iter().all(|(existing, _)| existing != name))
            .expect("the candidate names never run out")
    }
}

impl Default for RuleSets {
    fn default() -> Self {
        Self::new(String::new())
    }
}
