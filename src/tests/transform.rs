use crate::model::neighbor::{NeighborState, Neighborhood};
use crate::model::tile::TileMods;
use crate::model::transform::{Orientations, Transform};
use crate::tests::support::{all_distinct, half_turn, index_of, mods, named, outer_corner};

fn transform(x_flip: bool, y_flip: bool, rot: bool) -> Transform {
    Transform {
        x_flip,
        y_flip,
        rot,
    }
}

fn every_transform() -> Vec<Transform> {
    Orientations::generated(mods(true, true, true)).transforms()
}

fn generated_suffixes(mods: TileMods) -> Vec<String> {
    Orientations::generated(mods)
        .transforms()
        .iter()
        .map(|transform| transform.suffix())
        .collect()
}

#[test]
fn the_full_group_holds_each_of_the_eight_symmetries_once() {
    let transforms = every_transform();
    assert_eq!(transforms.len(), 8);

    for (index, transform) in transforms.iter().enumerate() {
        assert!(
            !transforms[index + 1..].contains(transform),
            "{transform:?} appears twice"
        );
    }
}

#[test]
fn suffixes_follow_the_v_h_r_order() {
    let suffixes: Vec<String> = every_transform()
        .iter()
        .map(|transform| transform.suffix())
        .collect();

    assert_eq!(suffixes, ["", "R", "VH", "VHR", "V", "VR", "H", "HR"]);
}

#[test]
fn every_transform_keeps_each_state_on_the_ring() {
    let base = outer_corner();
    for transform in every_transform() {
        let moved = transform.apply(&base);

        for state in [
            NeighborState::Empty,
            NeighborState::Full,
            NeighborState::Any,
        ] {
            assert_eq!(
                named(&moved, state).len(),
                named(&base, state).len(),
                "{transform:?} lost a {state:?} neighbor"
            );
        }
    }
}

#[test]
fn rotating_a_neighborhood_four_times_returns_it() {
    let rotate = transform(false, false, true);
    let base = outer_corner();

    let mut turned = base;
    for _ in 0..4 {
        turned = rotate.apply(&turned);
    }

    assert_eq!(turned, base);
}

#[test]
fn rotating_an_outer_corner_walks_it_around_the_tile() {
    let base = outer_corner();
    let turned = [
        Transform::IDENTITY,
        transform(false, false, true),
        transform(true, true, false),
        transform(true, true, true),
    ]
    .map(|transform| transform.apply(&base));

    assert_eq!(
        named(&turned[0], NeighborState::Full),
        ["right", "bottom", "bottomRight"]
    );
    assert_eq!(named(&turned[0], NeighborState::Empty), ["top", "left"]);

    assert_eq!(
        named(&turned[1], NeighborState::Full),
        ["left", "bottomLeft", "bottom"]
    );
    assert_eq!(named(&turned[1], NeighborState::Empty), ["top", "right"]);

    assert_eq!(
        named(&turned[2], NeighborState::Full),
        ["topLeft", "top", "left"]
    );
    assert_eq!(named(&turned[2], NeighborState::Empty), ["right", "bottom"]);

    assert_eq!(
        named(&turned[3], NeighborState::Full),
        ["top", "topRight", "right"]
    );
    assert_eq!(named(&turned[3], NeighborState::Empty), ["left", "bottom"]);
}

#[test]
fn permissions_generate_their_subgroup_in_enumeration_order() {
    assert_eq!(generated_suffixes(mods(false, false, false)), [""]);
    assert_eq!(generated_suffixes(mods(true, false, false)), ["", "V"]);
    assert_eq!(generated_suffixes(mods(false, true, false)), ["", "H"]);
    assert_eq!(
        generated_suffixes(mods(true, true, false)),
        ["", "VH", "V", "H"]
    );
    assert_eq!(
        generated_suffixes(mods(false, false, true)),
        ["", "R", "VH", "VHR"]
    );

    let every = ["", "R", "VH", "VHR", "V", "VR", "H", "HR"];
    assert_eq!(generated_suffixes(mods(true, false, true)), every);
    assert_eq!(generated_suffixes(mods(false, true, true)), every);
    assert_eq!(generated_suffixes(mods(true, true, true)), every);
}

#[test]
fn every_preset_round_trips_through_its_generated_set() {
    for preset in [
        mods(false, false, false),
        mods(true, false, false),
        mods(false, true, false),
        mods(false, false, true),
        mods(true, true, false),
        mods(true, false, true),
    ] {
        assert_eq!(Orientations::generated(preset).preset(), Some(preset));
    }
}

#[test]
fn a_set_no_preset_generates_has_no_preset() {
    let turn_only: Orientations = [Transform::IDENTITY, transform(false, false, true)]
        .into_iter()
        .collect();

    assert_eq!(half_turn().preset(), None);
    assert_eq!(turn_only.preset(), None);
}

#[test]
fn a_collected_set_holds_exactly_its_transforms() {
    let half_turn = half_turn();

    assert_eq!(half_turn.len(), 1);
    assert!(half_turn.contains(transform(true, true, false)));
    assert!(!half_turn.contains(Transform::IDENTITY));
}

#[test]
fn with_and_without_add_and_remove_one_orientation() {
    let identity_only = Orientations::generated(mods(false, false, false));
    let quarter_turn = transform(false, false, true);
    let with_turn = identity_only.with(quarter_turn);

    assert_eq!(with_turn.len(), 2);
    assert_eq!(with_turn.without(quarter_turn), identity_only);
    assert!(identity_only.without(Transform::IDENTITY).is_empty());
}

#[test]
fn applying_after_the_identity_changes_nothing() {
    for transform in Transform::ALL {
        assert_eq!(transform.after(Transform::IDENTITY), transform);
        assert_eq!(Transform::IDENTITY.after(transform), transform);
    }
}

#[test]
fn after_applies_the_first_transform_first() {
    let base = all_distinct();
    for second in Transform::ALL {
        for first in Transform::ALL {
            assert_eq!(
                second.after(first).apply(&base),
                second.apply(&first.apply(&base)),
                "{second:?} after {first:?}"
            );
        }
    }
}

#[test]
fn after_stays_within_the_eight_transforms() {
    for second in Transform::ALL {
        for first in Transform::ALL {
            assert!(Transform::ALL.contains(&second.after(first)));
        }
    }
}

fn only_empty(name: &str) -> Neighborhood {
    let mut neighborhood = Neighborhood::uniform(NeighborState::Any);
    neighborhood.set_state(index_of(name), NeighborState::Empty);

    neighborhood
}

fn moved_empty(by: Transform, name: &str) -> Vec<&'static str> {
    named(&by.apply(&only_empty(name)), NeighborState::Empty)
}

#[test]
fn rotating_clockwise_moves_top_to_right_to_bottom_to_left() {
    let rotate = Transform::ROTATE_CLOCKWISE;

    assert_eq!(moved_empty(rotate, "top"), ["right"]);
    assert_eq!(moved_empty(rotate, "right"), ["bottom"]);
    assert_eq!(moved_empty(rotate, "bottom"), ["left"]);
    assert_eq!(moved_empty(rotate, "left"), ["top"]);
}

#[test]
fn flipping_horizontally_mirrors_left_and_right_only() {
    let flip = Transform::FLIP_HORIZONTALLY;

    assert_eq!(moved_empty(flip, "left"), ["right"]);
    assert_eq!(moved_empty(flip, "topLeft"), ["topRight"]);
    assert_eq!(moved_empty(flip, "top"), ["top"]);
}

#[test]
fn flipping_vertically_mirrors_top_and_bottom_only() {
    let flip = Transform::FLIP_VERTICALLY;

    assert_eq!(moved_empty(flip, "top"), ["bottom"]);
    assert_eq!(moved_empty(flip, "topLeft"), ["bottomLeft"]);
    assert_eq!(moved_empty(flip, "left"), ["left"]);
}

#[test]
fn rotating_four_times_or_flipping_twice_is_the_identity() {
    let repeated = |by: Transform, times: usize| {
        (0..times).fold(Transform::IDENTITY, |total, _| by.after(total))
    };

    assert_eq!(
        repeated(Transform::ROTATE_CLOCKWISE, 4),
        Transform::IDENTITY
    );
    assert_eq!(
        repeated(Transform::FLIP_HORIZONTALLY, 2),
        Transform::IDENTITY
    );
    assert_eq!(repeated(Transform::FLIP_VERTICALLY, 2), Transform::IDENTITY);
}
