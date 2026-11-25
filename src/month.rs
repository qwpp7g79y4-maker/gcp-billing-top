//! Calendar months and the many ways an export writes them.

use std::fmt;

/// A calendar month, ordered chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    month: u8,
}

const NAMES: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

impl Month {
    /// Builds a month; `month` must be 1..=12 and `year` 1970..=9999.
    pub fn new(year: i32, month: u8) -> Option<Month> {
        ((1..=12).contains(&month) && (1970..=9999).contains(&year))
            .then_some(Month { year, month })
    }

    /// Year.
    pub fn year(self) -> i32 {
        self.year
    }

    /// Month number 1..=12.
    pub fn number(self) -> u8 {
        self.month
    }

    /// The month before this one (January wraps to December).
    pub fn previous(self) -> Month {
        if self.month == 1 {
            Month {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Month {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    /// Parses the `--month` option: exactly `YYYY-MM`.
    pub fn parse_cli(text: &str) -> Result<Month, String> {
        let bad = || format!("invalid month {text:?}, expected YYYY-MM (for example 2026-10)");
        let (y, m) = text.trim().split_once('-').ok_or_else(bad)?;
        if y.len() != 4 || m.len() != 2 || !y.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit()) {
            return Err(bad());
        }
        Month::new(y.parse().map_err(|_| bad())?, m.parse().map_err(|_| bad())?).ok_or_else(bad)
    }

    /// Reads the month out of an export field.
    ///
    /// Accepted: `2026-10-03T00:00:00Z`, `2026-10-03 00:00:00 UTC`,
    /// `2026-10-03`, `2026-10`, `202610`, `20261003`, `Oct 3, 2026`,
    /// `October 2026`.
    pub fn from_field(text: &str) -> Option<Month> {
        let t = text.trim();
        let b = t.as_bytes();
        if b.len() >= 7
            && b[..4].iter().all(u8::is_ascii_digit)
            && b[4] == b'-'
            && b[5..7].iter().all(u8::is_ascii_digit)
        {
            let rest = &b[7..];
            if rest.is_empty() || !rest[0].is_ascii_digit() {
                return Month::new(t[..4].parse().ok()?, t[5..7].parse().ok()?);
            }
            return None;
        }
        if (b.len() == 6 || b.len() == 8) && b.iter().all(u8::is_ascii_digit) {
            return Month::new(t[..4].parse().ok()?, t[4..6].parse().ok()?);
        }
        let mut month = None;
        let mut year = None;
        for token in t.split(|c: char| c.is_whitespace() || c == ',' || c == '.') {
            if token.is_empty() {
                continue;
            }
            if month.is_none() && token.len() >= 3 && token.bytes().all(|c| c.is_ascii_alphabetic())
            {
                let lower = token.to_ascii_lowercase();
                month = NAMES
                    .iter()
                    .position(|n| lower.starts_with(n))
                    .map(|i| i as u8 + 1);
            } else if year.is_none()
                && token.len() == 4
                && token.bytes().all(|c| c.is_ascii_digit())
            {
                year = token.parse::<i32>().ok();
            }
        }
        Month::new(year?, month?)
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(y: i32, mo: u8) -> Month {
        Month::new(y, mo).unwrap()
    }

    #[test]
    fn new_validates_range() {
        assert!(Month::new(2026, 0).is_none());
        assert!(Month::new(2026, 13).is_none());
        assert!(Month::new(1969, 1).is_none());
        assert!(Month::new(2026, 12).is_some());
    }

    #[test]
    fn display_is_zero_padded() {
        assert_eq!(m(2026, 3).to_string(), "2026-03");
        assert_eq!(m(2026, 12).to_string(), "2026-12");
    }

    #[test]
    fn ordering_is_chronological() {
        assert!(m(2025, 12) < m(2026, 1));
        assert!(m(2026, 2) < m(2026, 10));
    }

    #[test]
    fn previous_wraps_over_year_end() {
        assert_eq!(m(2026, 1).previous(), m(2025, 12));
        assert_eq!(m(2026, 10).previous(), m(2026, 9));
    }

    #[test]
    fn accessors() {
        assert_eq!(m(2026, 10).year(), 2026);
        assert_eq!(m(2026, 10).number(), 10);
    }

    #[test]
    fn cli_accepts_year_month() {
        assert_eq!(Month::parse_cli("2026-10").unwrap(), m(2026, 10));
        assert_eq!(Month::parse_cli(" 2026-01 ").unwrap(), m(2026, 1));
    }

    #[test]
    fn cli_rejects_other_shapes() {
        for bad in [
            "2026",
            "2026-1",
            "2026-13",
            "26-10",
            "2026/10",
            "2026-10-01",
            "oct",
            "",
        ] {
            assert!(Month::parse_cli(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn field_iso_timestamp_variants() {
        assert_eq!(Month::from_field("2026-10-03T00:00:00Z"), Some(m(2026, 10)));
        assert_eq!(
            Month::from_field("2026-10-03 00:00:00 UTC"),
            Some(m(2026, 10))
        );
        assert_eq!(
            Month::from_field("2026-10-03T23:59:59.999-07:00"),
            Some(m(2026, 10))
        );
        assert_eq!(Month::from_field("2026-10-03"), Some(m(2026, 10)));
        assert_eq!(Month::from_field("2026-10"), Some(m(2026, 10)));
    }

    #[test]
    fn field_invoice_month_digits() {
        assert_eq!(Month::from_field("202610"), Some(m(2026, 10)));
        assert_eq!(Month::from_field("20261003"), Some(m(2026, 10)));
    }

    #[test]
    fn field_month_names() {
        assert_eq!(Month::from_field("Oct 3, 2026"), Some(m(2026, 10)));
        assert_eq!(Month::from_field("October 2026"), Some(m(2026, 10)));
        assert_eq!(Month::from_field("sep 28, 2026"), Some(m(2026, 9)));
        assert_eq!(Month::from_field("Dec. 1, 2025"), Some(m(2025, 12)));
    }

    #[test]
    fn field_rejects_garbage() {
        for bad in [
            "",
            "hello",
            "2026-13-01",
            "2026-00",
            "12345",
            "Foo 3, 2026",
            "Oct 3",
            "2026-1003",
        ] {
            assert_eq!(Month::from_field(bad), None, "{bad:?}");
        }
    }
}
