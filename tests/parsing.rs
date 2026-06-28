use l2sync::book::{SCALE, format_scaled, parse_scaled};

#[test]
fn parses_exact_decimals() {
    assert_eq!(parse_scaled("0"), Some(0));
    assert_eq!(parse_scaled("123"), Some(123 * SCALE));
    assert_eq!(parse_scaled("59774.00000000"), Some(5_977_400_000_000));
    assert_eq!(parse_scaled("0.00000001"), Some(1));
    assert_eq!(parse_scaled("1.5"), Some(150_000_000));
    assert_eq!(parse_scaled("-0.5"), Some(-50_000_000));
    assert_eq!(parse_scaled("-1.00000001"), Some(-100_000_001));
}

#[test]
fn rejects_malformed() {
    assert_eq!(parse_scaled(""), None);
    assert_eq!(parse_scaled("abc"), None);
    assert_eq!(parse_scaled("1.2.3"), None);
    assert_eq!(parse_scaled("1.123456789"), None);
    assert_eq!(parse_scaled("-"), None);
    assert_eq!(parse_scaled(".5"), None);
}

#[test]
fn format_round_trips() {
    for t in [
        0_i64,
        1,
        SCALE,
        150_000_000,
        5_977_400_000_000,
        -1,
        -50_000_000,
    ] {
        assert_eq!(
            parse_scaled(&format_scaled(t)),
            Some(t),
            "round-trip of {t}"
        );
    }
}
