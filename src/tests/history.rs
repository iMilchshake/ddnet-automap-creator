use crate::model::rule_sets::RuleSets;
use crate::ui::history::{History, MAX_STEPS};

fn named(name: &str) -> RuleSets {
    RuleSets::new(name.to_owned())
}

#[test]
fn an_unchanged_document_commits_nothing() {
    let mut history = History::new(named("a"));
    history.commit(&named("a"));

    assert!(!history.can_undo(&named("a")));
}

#[test]
fn undo_restores_the_state_before_the_commit() {
    let mut history = History::new(named("a"));
    history.commit(&named("b"));

    assert_eq!(history.undo(&named("b")), Some(named("a")));
    assert_eq!(history.undo(&named("a")), None);
}

#[test]
fn undo_commits_a_pending_change_first() {
    let mut history = History::new(named("a"));

    assert_eq!(history.undo(&named("b")), Some(named("a")));
    assert_eq!(history.redo(&named("a")), Some(named("b")));
}

#[test]
fn redo_replays_what_was_undone() {
    let mut history = History::new(named("a"));
    history.commit(&named("b"));
    history.commit(&named("c"));
    history.undo(&named("c"));
    history.undo(&named("b"));

    assert!(history.can_redo(&named("a")));
    assert_eq!(history.redo(&named("a")), Some(named("b")));
    assert_eq!(history.redo(&named("b")), Some(named("c")));
    assert_eq!(history.redo(&named("c")), None);
}

#[test]
fn a_new_edit_clears_redo() {
    let mut history = History::new(named("a"));
    history.commit(&named("b"));
    history.undo(&named("b"));
    history.commit(&named("c"));

    assert!(!history.can_redo(&named("c")));
    assert_eq!(history.redo(&named("c")), None);
}

#[test]
fn selecting_another_rule_set_is_not_an_undo_step() {
    let mut two_sets = named("a");
    two_sets.add();
    let mut history = History::new(two_sets.clone());

    two_sets.select(0);
    history.commit(&two_sets);

    assert!(!history.can_undo(&two_sets));
}

#[test]
fn reset_forgets_everything() {
    let mut history = History::new(named("a"));
    history.commit(&named("b"));
    history.reset(named("c"));

    assert!(!history.can_undo(&named("c")));
}

#[test]
fn the_oldest_steps_are_dropped_beyond_the_cap() {
    let mut history = History::new(named("0"));
    for step in 1..=MAX_STEPS + 1 {
        history.commit(&named(&step.to_string()));
    }

    let mut current = named(&(MAX_STEPS + 1).to_string());
    let mut undone = 0;
    while let Some(previous) = history.undo(&current) {
        current = previous;
        undone += 1;
    }

    assert_eq!(undone, MAX_STEPS);
    assert_eq!(current, named("1"));
}
