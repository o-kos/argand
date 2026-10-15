use super::*;

#[test]
fn the_identity_leaves_values_alone() {
    let mut values = [0.25f32, -0.5, 0.75, -1.0];
    Adjustment::IDENTITY.apply(&mut values, 2, 3);
    assert_eq!(values, [0.25, -0.5, 0.75, -1.0]);
    assert_eq!(Adjustment::default(), Adjustment::IDENTITY);
}

#[test]
fn inverting_a_complex_capture_conjugates_every_sample() {
    let mut values = [0.25f32, -0.5, 0.75, 1.0];
    Adjustment::IDENTITY.inverted().apply(&mut values, 2, 7);
    assert_eq!(values, [0.25, 0.5, 0.75, -1.0]);
}

#[test]
fn inverting_a_real_capture_flips_every_odd_sample_of_the_capture() {
    let inverted = Adjustment::IDENTITY.inverted();
    let mut from_even = [1.0f32, 1.0, 1.0, 1.0];
    inverted.apply(&mut from_even, 1, 4);
    assert_eq!(from_even, [1.0, -1.0, 1.0, -1.0]);
    let mut from_odd = [1.0f32, 1.0, 1.0];
    inverted.apply(&mut from_odd, 1, 5);
    assert_eq!(from_odd, [-1.0, 1.0, -1.0]);
}

#[test]
fn the_gain_scales_i_and_q_alike() {
    let adjustment = Adjustment::new(true, 2.0).expect("a gain");
    let mut values = [0.25f32, -0.5];
    adjustment.apply(&mut values, 2, 0);
    assert_eq!(values, [0.5, 1.0]);
    assert_eq!(adjustment.factor(2, 0, 0), 2.0);
    assert_eq!(adjustment.factor(2, 1, 0), -2.0);
}

#[test]
fn a_gain_must_be_positive_and_finite() {
    assert_eq!(Adjustment::new(false, 0.0), None);
    assert_eq!(Adjustment::new(false, -1.0), None);
    assert_eq!(Adjustment::new(false, f64::NAN), None);
    assert_eq!(Adjustment::new(false, f64::INFINITY), None);
    assert_eq!(Adjustment::IDENTITY.with_gain(0.5).map(Adjustment::gain), Some(0.5));
}

#[test]
fn inverting_twice_is_the_identity() {
    assert!(Adjustment::IDENTITY.inverted().inverted().is_identity());
    assert!(!Adjustment::IDENTITY.inverted().is_identity());
}
