//! Exact money arithmetic in millionths of the currency unit.
//!
//! GCP exports costs with up to a dozen decimals. Summing those as `f64`
//! gives drifting totals, so amounts are parsed once into integer micro-units
//! (rounded half away from zero) and summed exactly.

use std::fmt;
use std::ops::Neg;

const MICROS: i128 = 1_000_000;

/// An amount in micro-units (1 000 000 = 1.00).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Money(i64);

impl Money {
    /// Zero.
    pub const ZERO: Money = Money(0);

    /// Builds an amount from raw micro-units.
    pub const fn from_micros(micros: i64) -> Money {
        Money(micros)
    }

    /// Raw micro-units.
    pub const fn micros(self) -> i64 {
        self.0
    }

    /// Lossy conversion for percentages and JSON.
    pub fn as_f64(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    /// Sum, or `None` when it does not fit.
    pub fn checked_add(self, rhs: Money) -> Option<Money> {
        self.0.checked_add(rhs.0).map(Money)
    }

    /// Difference, or `None` when it does not fit.
    pub fn checked_sub(self, rhs: Money) -> Option<Money> {
        self.0.checked_sub(rhs.0).map(Money)
    }

    /// Absolute value (saturating at `i64::MAX`).
    pub fn abs(self) -> Money {
        Money(self.0.saturating_abs())
    }

    /// Parses a plain decimal such as `12`, `-0.5`, `+3.25` or `1.2e-5`.
    ///
    /// Thousands separators and currency symbols are rejected on purpose:
    /// an export uses `.` as decimal point and nothing else.
    pub fn parse(text: &str) -> Result<Money, String> {
        let text = text.trim();
        let err = || format!("invalid amount {text:?}");
        if text.is_empty() {
            return Err(err());
        }
        let (negative, rest) = match text.as_bytes()[0] {
            b'-' => (true, &text[1..]),
            b'+' => (false, &text[1..]),
            _ => (false, text),
        };
        let (mantissa, exponent) = match rest.find(['e', 'E']) {
            Some(i) => {
                let exp: i32 = rest[i + 1..].parse().map_err(|_| err())?;
                (&rest[..i], exp.clamp(-1000, 1000))
            }
            None => (rest, 0),
        };
        let (int_part, frac_part) = match mantissa.split_once('.') {
            Some((i, f)) => (i, f),
            None => (mantissa, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(err());
        }
        if !int_part
            .bytes()
            .chain(frac_part.bytes())
            .all(|b| b.is_ascii_digit())
        {
            return Err(err());
        }
        let digits: String = format!("{int_part}{frac_part}");
        let digits = digits.trim_start_matches('0');
        // value = digits * 10^(exponent - frac_len); micros = value * 10^6
        let shift = 6 + exponent - frac_part.len() as i32;
        let magnitude: i128 = if digits.is_empty() {
            0
        } else if shift >= 0 {
            if digits.len() as i32 + shift > 18 {
                return Err(format!("amount {text:?} is too large"));
            }
            digits.parse::<i128>().map_err(|_| err())? * 10i128.pow(shift as u32)
        } else {
            let drop = (-shift) as usize;
            if drop > digits.len() {
                0
            } else {
                let kept = &digits[..digits.len() - drop];
                let first_dropped = digits.as_bytes()[digits.len() - drop];
                let base: i128 = if kept.is_empty() {
                    0
                } else {
                    kept.parse().map_err(|_| err())?
                };
                if first_dropped >= b'5' {
                    base.checked_add(1)
                        .ok_or_else(|| format!("amount {text:?} is too large"))?
                } else {
                    base
                }
            }
        };
        let signed = if negative { -magnitude } else { magnitude };
        i64::try_from(signed)
            .map(Money)
            .map_err(|_| format!("amount {text:?} is too large"))
    }

    /// Rounds to cents (half away from zero) and returns the cent count.
    fn cents(self) -> i128 {
        let v = self.0 as i128;
        let half = MICROS / 200; // 5 000 micro = half a cent
        if v >= 0 {
            (v + half) / (MICROS / 100)
        } else {
            -((-v + half) / (MICROS / 100))
        }
    }

    /// Two decimals with thousands separators: `1,234.50`.
    pub fn to_display(self) -> String {
        let cents = self.cents();
        let sign = if cents < 0 { "-" } else { "" };
        let cents = cents.abs();
        let whole = (cents / 100).to_string();
        let mut grouped = String::new();
        for (i, c) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(c);
        }
        format!("{sign}{grouped}.{:02}", cents % 100)
    }

    /// Exact plain decimal, at least two and at most six decimals: `0.000123`.
    pub fn to_plain(self) -> String {
        let v = self.0 as i128;
        let sign = if v < 0 { "-" } else { "" };
        let v = v.abs();
        let mut frac = format!("{:06}", v % MICROS);
        while frac.len() > 2 && frac.ends_with('0') {
            frac.pop();
        }
        format!("{sign}{}.{frac}", v / MICROS)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain())
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(self.0.saturating_neg())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> i64 {
        Money::parse(s).unwrap().micros()
    }

    #[test]
    fn parses_integers_and_decimals() {
        assert_eq!(p("12"), 12_000_000);
        assert_eq!(p("0.5"), 500_000);
        assert_eq!(p("12.345678"), 12_345_678);
    }

    #[test]
    fn amount_at_the_i128_limit_is_too_large_and_does_not_panic() {
        let e = Money::parse("170141183460469231731687303715884.1057275").unwrap_err();
        assert!(e.contains("too large"), "{e}");
    }

    #[test]
    fn parses_signs() {
        assert_eq!(p("-3.25"), -3_250_000);
        assert_eq!(p("+3.25"), 3_250_000);
    }

    #[test]
    fn parses_leading_and_trailing_dot_forms() {
        assert_eq!(p(".5"), 500_000);
        assert_eq!(p("5."), 5_000_000);
    }

    #[test]
    fn rounds_half_away_from_zero_at_the_sixth_decimal() {
        assert_eq!(p("0.0000005"), 1);
        assert_eq!(p("0.0000004"), 0);
        assert_eq!(p("-0.0000005"), -1);
        assert_eq!(p("1.9999995"), 2_000_000);
    }

    #[test]
    fn long_gcp_style_decimals() {
        assert_eq!(p("0.000123456789"), 123);
        assert_eq!(p("12.123456789012"), 12_123_457);
    }

    #[test]
    fn parses_exponent_notation() {
        assert_eq!(p("1.2e-5"), 12);
        assert_eq!(p("3E2"), 300_000_000);
        assert_eq!(p("1e-9"), 0);
    }

    #[test]
    fn zero_forms() {
        assert_eq!(p("0"), 0);
        assert_eq!(p("-0.0"), 0);
        assert_eq!(p("0e50"), 0);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(p("  7.5 "), 7_500_000);
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "", " ", "abc", "1,5", "$5", "1.2.3", "--1", ".", "-", "1e", "e5", "1 2",
        ] {
            assert!(Money::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rejects_overflow() {
        assert!(Money::parse("99999999999999999999").is_err());
    }

    #[test]
    fn sums_exactly() {
        let total = ["0.1", "0.2", "0.3"]
            .iter()
            .map(|s| Money::parse(s).unwrap())
            .try_fold(Money::ZERO, Money::checked_add);
        assert_eq!(total, Some(Money::parse("0.6").unwrap()));
    }

    #[test]
    fn arithmetic_ops() {
        let a = Money::from_micros(5);
        let b = Money::from_micros(3);
        assert_eq!(a.checked_add(b).unwrap().micros(), 8);
        assert_eq!(a.checked_sub(b).unwrap().micros(), 2);
        assert_eq!((-a).micros(), -5);
        assert_eq!(Money::from_micros(-4).abs().micros(), 4);
    }

    #[test]
    fn checked_arithmetic_refuses_instead_of_clamping() {
        let max = Money::from_micros(i64::MAX);
        let min = Money::from_micros(i64::MIN);
        assert_eq!(max.checked_add(Money::from_micros(1)), None);
        assert_eq!(min.checked_add(Money::from_micros(-1)), None);
        assert_eq!(min.checked_sub(Money::from_micros(1)), None);
        assert_eq!(max.checked_sub(Money::from_micros(-1)), None);
        assert_eq!(
            max.checked_add(Money::from_micros(-1)).unwrap().micros(),
            i64::MAX - 1
        );
    }

    #[test]
    fn display_has_two_decimals_and_separators() {
        assert_eq!(Money::parse("1234.5").unwrap().to_display(), "1,234.50");
        assert_eq!(Money::parse("0").unwrap().to_display(), "0.00");
        assert_eq!(
            Money::parse("1234567.891").unwrap().to_display(),
            "1,234,567.89"
        );
        assert_eq!(Money::parse("999.999").unwrap().to_display(), "1,000.00");
    }

    #[test]
    fn display_negative_and_rounding() {
        assert_eq!(Money::parse("-1234.565").unwrap().to_display(), "-1,234.57");
        assert_eq!(Money::parse("0.004").unwrap().to_display(), "0.00");
        assert_eq!(Money::parse("0.005").unwrap().to_display(), "0.01");
        assert_eq!(Money::parse("-0.001").unwrap().to_display(), "0.00");
    }

    #[test]
    fn plain_keeps_at_least_two_decimals() {
        assert_eq!(Money::parse("12").unwrap().to_plain(), "12.00");
        assert_eq!(Money::parse("12.5").unwrap().to_plain(), "12.50");
        assert_eq!(Money::parse("0.000123").unwrap().to_plain(), "0.000123");
        assert_eq!(Money::parse("-0.5").unwrap().to_plain(), "-0.50");
        assert_eq!(Money::parse("12.5").unwrap().to_string(), "12.50");
    }

    #[test]
    fn as_f64_is_close() {
        assert!((Money::parse("2.5").unwrap().as_f64() - 2.5).abs() < 1e-12);
    }
}
