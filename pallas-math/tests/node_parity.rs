use pallas_math::math::{ExpOrdering, FixedDecimal, FixedPrecision};

const SCALE: &str = "10000000000000000000000000000000000";
const THREE_SCALE: &str = "30000000000000000000000000000000000";

fn raw(value: &str) -> FixedDecimal {
    FixedDecimal::from_str(value, 34).expect("valid raw E34 integer")
}

#[test]
fn signed_raw_division_floors_instead_of_truncating() {
    let cases = [
        ("7", THREE_SCALE, "2"),
        ("-7", THREE_SCALE, "-3"),
        ("7", "-30000000000000000000000000000000000", "-3"),
        ("-7", "-30000000000000000000000000000000000", "2"),
        ("6", THREE_SCALE, "2"),
        ("-6", THREE_SCALE, "-2"),
        ("6", "-30000000000000000000000000000000000", "-2"),
        ("-6", "-30000000000000000000000000000000000", "2"),
        ("0", THREE_SCALE, "0"),
        ("0", "-30000000000000000000000000000000000", "0"),
    ];
    let mut mismatches = Vec::new();
    for (numerator, denominator, expected) in cases {
        let numerator_value = raw(numerator);
        let denominator_value = raw(denominator);
        let expected_value = raw(expected);
        let borrowed = &numerator_value / &denominator_value;
        let owned = numerator_value / denominator_value;
        if borrowed != expected_value || owned != expected_value {
            mismatches.push(format!(
                "raw {numerator} / {denominator}: borrowed={borrowed}, owned={owned}, expected={expected_value}"
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn signed_integral_ratios_floor_at_e34_precision() {
    let cases = [
        (1_i64, 3_i64, "3333333333333333333333333333333333"),
        (-1, 3, "-3333333333333333333333333333333334"),
        (1, -3, "-3333333333333333333333333333333334"),
        (-1, -3, "3333333333333333333333333333333333"),
        (6, 3, "20000000000000000000000000000000000"),
        (-6, 3, "-20000000000000000000000000000000000"),
        (6, -3, "-20000000000000000000000000000000000"),
        (-6, -3, "20000000000000000000000000000000000"),
        (0, 3, "0"),
        (0, -3, "0"),
    ];
    let mut mismatches = Vec::new();
    for (numerator, denominator, expected) in cases {
        let numerator_value = FixedDecimal::from(numerator);
        let denominator_value = FixedDecimal::from(denominator);
        let expected_value = raw(expected);
        let borrowed = &numerator_value / &denominator_value;
        let owned = numerator_value / denominator_value;
        if borrowed != expected_value || owned != expected_value {
            mismatches.push(format!(
                "{numerator}/{denominator}: borrowed={borrowed}, owned={owned}, expected={expected_value}"
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn signed_multiplication_preserves_already_correct_flooring() {
    let cases = [
        ("1", "1", "0"),
        ("-1", "1", "-1"),
        ("1", "-1", "-1"),
        ("-1", "-1", "0"),
        ("7", SCALE, "7"),
        ("-7", SCALE, "-7"),
        ("7", "-10000000000000000000000000000000000", "-7"),
        ("-7", "-10000000000000000000000000000000000", "7"),
        ("0", "-1", "0"),
    ];
    for (left, right, expected) in cases {
        let left_value = raw(left);
        let right_value = raw(right);
        let expected_value = raw(expected);
        assert_eq!(
            &left_value * &right_value,
            expected_value,
            "borrowed raw multiplication {left} * {right}"
        );
        assert_eq!(
            left_value * right_value,
            expected_value,
            "owned raw multiplication {left} * {right}"
        );
    }
}

#[test]
fn every_direct_node_comparison_matches_ordering_and_iterations() {
    let fixture = include_str!("data/node-comparisons.tsv");
    assert!(fixture.ends_with('\n'), "truncated comparator fixture");
    let mut rows = 0;
    let mut mismatches = Vec::new();
    for (index, line) in fixture.lines().enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "comparator row {index} field count");
        let compare = raw(fields[0]);
        let x = raw(fields[1]);
        let expected_ordering = match fields[2] {
            "ABOVE" => ExpOrdering::GT,
            "BELOW" => ExpOrdering::LT,
            "MAX_REACHED" => ExpOrdering::UNKNOWN,
            other => panic!("unknown node ordering {other:?} at row {index}"),
        };
        let expected_iterations: u64 = fields[3].parse().expect("valid iteration count");
        let actual = x.exp_cmp(1000, 3, &compare);
        if actual.estimation != expected_ordering || actual.iterations != expected_iterations {
            mismatches.push(format!(
                "row {index}, qRaw={}, xRaw={}: got {:?}/{}; expected {:?}/{}",
                fields[0],
                fields[1],
                actual.estimation,
                actual.iterations,
                expected_ordering,
                expected_iterations
            ));
        }
        rows += 1;
    }
    assert!(rows > 0, "direct node comparator fixture must not be empty");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn pinned_node_primitive_rows_match_all_operator_forms() {
    use dashu_int::IBig;
    use std::str::FromStr;

    let fixture = include_str!("data/node-primitives.tsv");
    assert!(fixture.ends_with('\n'), "truncated primitive fixture");
    let mut rows = 0;
    for (index, line) in fixture.lines().enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "primitive row {index}");
        let (a, b) = if fields[0] == "ratio" {
            (
                FixedDecimal::from(IBig::from_str(fields[1]).unwrap()),
                FixedDecimal::from(IBig::from_str(fields[2]).unwrap()),
            )
        } else {
            (raw(fields[1]), raw(fields[2]))
        };
        let expected = raw(fields[3]);
        let mut assigned = a.clone();
        let mut borrowed_assigned = a.clone();
        let (borrowed, owned) = match fields[0] {
            "multiply" => {
                assigned *= b.clone();
                let mut target = &mut borrowed_assigned;
                target *= &b;
                (&a * &b, a * b)
            }
            "divide" | "ratio" => {
                assigned /= b.clone();
                let mut target = &mut borrowed_assigned;
                target /= &b;
                (&a / &b, a / b)
            }
            other => panic!("primitive row {index}: unknown operation {other}"),
        };
        assert_eq!(borrowed, expected, "borrowed primitive row {index}");
        assert_eq!(owned, expected, "owned primitive row {index}");
        assert_eq!(assigned, expected, "assignment primitive row {index}");
        assert_eq!(
            borrowed_assigned, expected,
            "borrowed assignment primitive row {index}"
        );
        rows += 1;
    }
    assert!(rows > 0, "primitive fixture must not be empty");
}

#[test]
fn comparator_unknown_means_exact_budget_exhaustion() {
    let one = raw(SCALE);
    let zero = raw("0");
    let result = zero.exp_cmp(0, 3, &one);
    assert_eq!(result.estimation, ExpOrdering::UNKNOWN);
    assert_eq!(result.iterations, 0);
    assert_eq!(result.approx, one);

    let enormous_x = FixedDecimal::from(1_000_000u64);
    for budget in [1, 2, 1000] {
        let result = enormous_x.exp_cmp(budget, 3, &one);
        assert_eq!(result.estimation, ExpOrdering::UNKNOWN);
        assert_eq!(result.iterations, budget);
    }
}

#[test]
#[should_panic]
fn primitive_division_by_zero_remains_a_panic() {
    let _ = raw("1") / raw("0");
}

#[test]
fn extra_node_arithmetic_matches_signed_and_log_scaling_cases() {
    let fixture = include_str!("data/node-extra-arithmetic.tsv");
    assert!(
        fixture.ends_with('\n'),
        "truncated extra arithmetic fixture"
    );
    let one = FixedDecimal::from(1u64);
    let c = &one - &(&one / &FixedDecimal::from(10u64));
    let ln_c = c.ln();
    let mut rows = 0;
    let mut singular_rows = 0;
    for (index, line) in fixture.lines().enumerate() {
        let fields: Vec<_> = line.split('\t').collect();
        assert!(
            fields.len() == 5 || fields.len() == 8,
            "extra arithmetic row {index}"
        );
        let x = raw(fields[0]);
        let a = raw(fields[1]);
        let b = raw(fields[2]);
        if fields.len() == 5 {
            assert_eq!(a, one, "singular reciprocal row {index}");
            assert_eq!(
                &fields[3..],
                &["ERROR", "divide-by-zero"],
                "oracle error row {index}"
            );
            assert!(
                std::panic::catch_unwind(|| &one / &(&one - &a)).is_err(),
                "production reciprocal must panic at oracle singular row {index}"
            );
            singular_rows += 1;
            rows += 1;
            continue;
        }
        assert_eq!(x.exp().to_string(), fields[3], "exp row {index}");
        assert_eq!((-a.ln()).to_string(), fields[4], "negative ln row {index}");
        assert_eq!(
            (&one - &c.pow(&b)).to_string(),
            fields[5],
            "threshold row {index}"
        );
        let q = &one / &(&one - &a);
        let comparison = (-(&b * &ln_c)).exp_cmp(1000, 3, &q);
        let expected = match fields[6] {
            "ABOVE" => ExpOrdering::GT,
            "BELOW" => ExpOrdering::LT,
            "MAX_REACHED" => ExpOrdering::UNKNOWN,
            other => panic!("extra arithmetic row {index}: unknown ordering {other}"),
        };
        assert_eq!(comparison.estimation, expected, "comparator row {index}");
        assert_eq!(
            comparison.iterations,
            fields[7].parse::<u64>().unwrap(),
            "comparator iterations row {index}"
        );
        rows += 1;
    }
    assert_eq!(rows, 7 * 5 * 4, "complete Cartesian arithmetic inputs");
    assert_eq!(singular_rows, 7 * 4, "all singular reciprocal inputs");
}
