use crate::model::neighbor::NeighborState;
use crate::model::transform::{Transform, variants};
use crate::tests::support::{mods, named, outer_corner};

fn transform(x_flip: bool, y_flip: bool, rot: bool) -> Transform {
    Transform {
        x_flip,
        y_flip,
        rot,
    }
}

fn every_transform() -> Vec<Transform> {
    variants(mods(true, true, true))
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
    let suffixes = |mods| -> Vec<String> { variants(mods).iter().map(|t| t.suffix()).collect() };

    assert_eq!(suffixes(mods(false, false, false)), [""]);
    assert_eq!(suffixes(mods(true, false, false)), ["", "V"]);
    assert_eq!(suffixes(mods(false, true, false)), ["", "H"]);
    assert_eq!(suffixes(mods(true, true, false)), ["", "VH", "V", "H"]);
    assert_eq!(suffixes(mods(false, false, true)), ["", "R", "VH", "VHR"]);

    let every = ["", "R", "VH", "VHR", "V", "VR", "H", "HR"];
    assert_eq!(suffixes(mods(true, false, true)), every);
    assert_eq!(suffixes(mods(false, true, true)), every);
    assert_eq!(suffixes(mods(true, true, true)), every);
}
