//! Public growth claims: log-log exponent, limits, and a rendered series.

use ph_bench_build::{
    Character, Group, Growth, LINEAR_EXPONENT, LINEAR_LIMIT, MIN_SCALE, MIN_SCALES,
    NOISE_ALLOWANCE, Point, QUADRATIC_EXPONENT, QUADRATIC_LIMIT, Scale, Series,
};

#[test]
fn limits_are_the_exponents_plus_the_noise_allowance() {
    assert_eq!(LINEAR_EXPONENT, 1.0);
    assert_eq!(QUADRATIC_EXPONENT, 2.0);
    assert_eq!(NOISE_ALLOWANCE, 0.25);
    assert_eq!(LINEAR_LIMIT, 1.25);
    assert_eq!(QUADRATIC_LIMIT, 1.75);
    assert_eq!(MIN_SCALES, 2);
    assert_eq!(MIN_SCALE, 1);
}

#[test]
fn character_uses_the_limits() {
    assert_eq!(Character::from_exponent(0.0), Character::Linear);
    assert_eq!(Character::from_exponent(LINEAR_LIMIT), Character::Linear);
    assert_eq!(
        Character::from_exponent(LINEAR_LIMIT + 0.001),
        Character::Superlinear
    );
    assert_eq!(
        Character::from_exponent(QUADRATIC_LIMIT - 0.001),
        Character::Superlinear
    );
    assert_eq!(
        Character::from_exponent(QUADRATIC_LIMIT),
        Character::Quadratic
    );
    assert_eq!(Character::from_exponent(3.0), Character::Quadratic);
    assert_eq!(Character::from_exponent(f64::NAN), Character::Quadratic);
    assert_eq!(
        Character::from_exponent(f64::INFINITY),
        Character::Quadratic
    );
}

#[test]
fn exact_linear_and_quadratic_ratios() {
    let linear = Growth::Linear
        .check(&[point(16, 1.0), point(64, 4.0), point(256, 16.0)])
        .unwrap();
    assert!((linear.exponent() - 1.0).abs() < 1e-9);
    assert_eq!(linear.character(), Character::Linear);
    Growth::NotQuadratic
        .check(&[point(16, 1.0), point(256, 16.0)])
        .unwrap();

    let quadratic =
        Growth::NotQuadratic.check(&[point(16, 1.0), point(64, 16.0), point(256, 256.0)]);
    let error = quadratic.unwrap_err().to_string();
    assert!(error.contains("quadratic"), "{error}");
    assert!(error.contains("1.75"), "{error}");
    let linear_error = Growth::Linear
        .check(&[point(16, 1.0), point(256, 256.0)])
        .unwrap_err()
        .to_string();
    assert!(linear_error.contains("not linear"), "{linear_error}");
    assert!(linear_error.contains("1.25"), "{linear_error}");
}

#[test]
fn two_points_are_the_log_ratio() {
    let fit = Growth::Linear
        .check(&[point(16, 1.0), point(256, 32.0)])
        .unwrap();
    assert!((fit.exponent() - 1.25).abs() < 1e-9, "{}", fit.exponent());
    assert_eq!(fit.character(), Character::Linear);

    let over = Growth::Linear.check(&[point(16, 1.0), point(256, 32.0 * 1.01)]);
    assert!(over.is_err());

    let below_quadratic = Growth::NotQuadratic
        .check(&[point(16, 1.0), point(256, 128.0 / 1.01)])
        .unwrap();
    assert_eq!(below_quadratic.character(), Character::Superlinear);

    let at_quadratic = Growth::NotQuadratic.check(&[point(16, 1.0), point(256, 128.0)]);
    let error = at_quadratic.unwrap_err().to_string();
    assert!(error.contains("quadratic"), "{error}");
}

#[test]
fn superlinear_is_not_linear_and_not_quadratic() {
    let points = [point(16, 1.0), point(256, 64.0)];
    assert!(Growth::Linear.check(&points).is_err());
    let fit = Growth::NotQuadratic.check(&points).unwrap();
    assert!((fit.exponent() - 1.5).abs() < 1e-9, "{}", fit.exponent());
    assert_eq!(fit.character(), Character::Superlinear);
}

#[test]
fn fixed_overhead_and_jitter_still_count_as_linear() {
    let flat = Growth::Linear
        .check(&[point(64, 2.0), point(1024, 2.2), point(8192, 3.0)])
        .unwrap();
    assert!(flat.exponent() < LINEAR_LIMIT);

    let noisy = Growth::Linear
        .check(&[
            point(64, 64.0 * 1.2),
            point(256, 256.0 * 0.9),
            point(1024, 1024.0 * 1.1),
        ])
        .unwrap();
    assert_eq!(noisy.character(), Character::Linear);
}

#[test]
fn point_order_does_not_change_the_exponent() {
    let forward = Growth::Linear
        .check(&[point(8, 0.8), point(32, 3.2)])
        .unwrap();
    let backward = Growth::Linear
        .check(&[point(32, 3.2), point(8, 0.8)])
        .unwrap();
    assert!((forward.exponent() - backward.exponent()).abs() < 1e-12);
}

#[test]
fn bad_points_are_rejected() {
    assert!(Growth::Linear.check(&[point(8, 1.0)]).is_err());
    assert!(
        Growth::Linear
            .check(&[point(0, 1.0), point(8, 2.0)])
            .is_err()
    );
    assert!(
        Growth::Linear
            .check(&[point(8, 1.0), point(8, 2.0)])
            .is_err()
    );
    assert!(
        Growth::Linear
            .check(&[point(8, 0.0), point(16, 1.0)])
            .is_err()
    );
    assert!(
        Growth::Linear
            .check(&[point(8, -1.0), point(16, 1.0)])
            .is_err()
    );
    assert!(
        Growth::Linear
            .check(&[point(8, f64::NAN), point(16, 1.0)])
            .is_err()
    );
    assert!(
        Growth::Linear
            .check(&[point(8, f64::INFINITY), point(16, 1.0)])
            .is_err()
    );
}

#[test]
fn a_series_declares_generator_scales_and_claim() {
    let series = Series::new("items")
        .group(Group::Debug)
        .scales([32, 8])
        .expect(Growth::NotQuadratic)
        .render(|scale| format!("struct Item{};\n", scale.count()))
        .unwrap();
    assert_eq!(series.name(), "items");
    assert_eq!(series.scale_values().len(), 2);
    assert_eq!(series.scale_values()[0].count(), 32);
    assert_eq!(series.source(Scale::new(8)), Some("struct Item8;\n"));
    assert_eq!(series.source(Scale::new(32)), Some("struct Item32;\n"));
    let fit = series.check_seconds(&[3.2, 0.8]).unwrap();
    assert!((fit.exponent() - 1.0).abs() < 1e-9, "{}", fit.exponent());
}

#[test]
fn an_unfinished_series_does_not_render() {
    let missing_group = Series::new("items").scales([8, 16]).expect(Growth::Linear);
    assert!(missing_group.render(source).is_err());

    let one_scale = Series::new("items")
        .group(Group::Release)
        .scales([8])
        .expect(Growth::Linear);
    assert!(one_scale.render(source).is_err());

    let zero = Series::new("items")
        .group(Group::Debug)
        .scales([0, 8])
        .expect(Growth::Linear);
    assert!(zero.render(source).is_err());

    let duplicate = Series::new("items")
        .group(Group::Debug)
        .scales([8, 8])
        .expect(Growth::Linear);
    assert!(duplicate.render(source).is_err());

    let unnamed = Series::new("")
        .group(Group::Debug)
        .scales([8, 16])
        .expect(Growth::Linear);
    assert!(unnamed.render(source).is_err());

    let no_claim = Series::new("items").group(Group::Debug).scales([8, 16]);
    assert!(no_claim.render(source).is_err());

    let empty = Series::new("items")
        .group(Group::Incremental)
        .scales([8, 16])
        .expect(Growth::Linear)
        .render(|_| String::new());
    assert!(empty.is_err());
}

#[test]
fn check_seconds_requires_a_time_per_scale() {
    let series = Series::new("items")
        .group(Group::Debug)
        .scales([8, 16])
        .expect(Growth::Linear)
        .render(source)
        .unwrap();
    let error = series.check_seconds(&[0.1]).unwrap_err().to_string();
    assert!(error.contains("2 scales"), "{error}");
    assert!(error.contains("but 1"), "{error}");

    let bare = Series::new("items");
    assert!(bare.check_seconds(&[0.1, 0.2]).is_err());

    let changed = series.scales([8, 16, 32]);
    assert!(changed.check_seconds(&[0.1, 0.2, 0.4]).is_err());
}

fn point(count: u32, seconds: f64) -> Point {
    Point::new(Scale::new(count), seconds)
}

fn source(scale: Scale) -> String {
    format!("struct Item{};\n", scale.count())
}
