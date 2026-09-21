use crate::model::tile::Chance;

#[test]
fn chance_rejects_values_outside_its_range() {
    assert!(Chance::new(0.0).is_err());
    assert!(Chance::new(-1.0).is_err());
    assert!(Chance::new(100.1).is_err());
    assert!(Chance::new(f32::NAN).is_err());
}

#[test]
fn chance_accepts_fractions_and_the_bounds() {
    assert_eq!(Chance::new(2.5).unwrap().percent(), 2.5);
    assert!(Chance::new(100.0).unwrap().is_full());
    assert!(!Chance::new(99.9).unwrap().is_full());
}
