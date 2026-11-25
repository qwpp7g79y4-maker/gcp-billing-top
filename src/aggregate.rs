//! Month selection, grouping and ranking.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Error, Result};
use crate::ingest::Row;
use crate::money::Money;
use crate::month::Month;

/// What to group the costs by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, clap::ValueEnum)]
pub enum Dimension {
    Sku,
    Project,
    Label,
}

impl Dimension {
    /// Name used in output (`sku`, `project`, `label`).
    pub fn name(self) -> &'static str {
        match self {
            Dimension::Sku => "sku",
            Dimension::Project => "project",
            Dimension::Label => "label",
        }
    }
}

/// How to rank groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Sort {
    /// Highest current cost first.
    Cost,
    /// Largest absolute change first (groups that vanished are included).
    Delta,
}

/// Group name for rows without a value.
pub const NO_SKU: &str = "(no sku)";
pub const NO_PROJECT: &str = "(no project)";
pub const NO_LABELS: &str = "(no labels)";
pub const UNLABELED: &str = "(unlabeled)";

/// The months to compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub current: Month,
    pub previous: Option<Month>,
}

fn list(months: &BTreeSet<Month>) -> String {
    months
        .iter()
        .map(Month::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Chooses the current month and the month to compare against.
///
/// * current: `want_current`, else the latest month of `main`.
/// * previous, same export: `want_previous`, else the calendar month before
///   `current` when present, else none.
/// * previous, second file: `want_previous`, else the calendar month before
///   `current` when present, else the latest earlier month of that file.
pub fn select_months(
    main: &BTreeSet<Month>,
    second: Option<&BTreeSet<Month>>,
    want_current: Option<Month>,
    want_previous: Option<Month>,
) -> Result<Selection> {
    let current = match want_current {
        Some(m) if main.contains(&m) => m,
        Some(m) => {
            return Err(Error::Selection(format!(
                "month {m} not found in the export (available: {})",
                list(main)
            )));
        }
        None => *main
            .iter()
            .next_back()
            .ok_or_else(|| Error::Selection("the export has no rows".to_string()))?,
    };
    if let Some(p) = want_previous {
        if p >= current {
            return Err(Error::Selection(format!(
                "previous month {p} must be earlier than the current month {current}"
            )));
        }
    }
    let pool = second.unwrap_or(main);
    let previous = match want_previous {
        Some(p) if pool.contains(&p) => Some(p),
        Some(p) => {
            return Err(Error::Selection(format!(
                "previous month {p} not found (available: {})",
                list(pool)
            )));
        }
        None => {
            let calendar = current.previous();
            if pool.contains(&calendar) {
                Some(calendar)
            } else if second.is_some() {
                let earlier = pool.iter().rev().find(|m| **m < current).copied();
                if earlier.is_none() {
                    return Err(Error::Selection(format!(
                        "the previous file has no month before {current} (available: {})",
                        list(pool)
                    )));
                }
                earlier
            } else {
                None
            }
        }
    };
    Ok(Selection { current, previous })
}

/// The refusal for a sum or difference outside the `i64` micro-unit range.
pub fn too_large(what: &str) -> Error {
    Error::Selection(format!(
        "{what} is too large: a sum must stay within {} to {}",
        Money::from_micros(i64::MIN).to_plain(),
        Money::from_micros(i64::MAX).to_plain()
    ))
}

/// The cost of a row, optionally net of credits.
pub fn row_cost(row: &Row, net: bool) -> Result<Money> {
    if net {
        row.cost
            .checked_add(row.credits.unwrap_or(Money::ZERO))
            .ok_or_else(|| too_large("the cost plus the credits of one row"))
    } else {
        Ok(row.cost)
    }
}

fn bump(out: &mut BTreeMap<String, Money>, key: String, cost: Money) -> Result<()> {
    let slot = out.entry(key).or_insert(Money::ZERO);
    match slot.checked_add(cost) {
        Some(sum) => {
            *slot = sum;
            Ok(())
        }
        None => Err(too_large("the total of a group")),
    }
}

/// Sums costs of `month` per group.
///
/// With `Dimension::Label` and no `label_key`, a row counts toward each of
/// its `key=value` labels, so label groups can add up to more than the total.
pub fn group(
    rows: &[Row],
    month: Month,
    dim: Dimension,
    label_key: Option<&str>,
    net: bool,
) -> Result<BTreeMap<String, Money>> {
    let mut out: BTreeMap<String, Money> = BTreeMap::new();
    for row in rows.iter().filter(|r| r.month == month) {
        let cost = row_cost(row, net)?;
        let mut add = |key: String| bump(&mut out, key, cost);
        match dim {
            Dimension::Sku => add(non_empty(row.sku.as_deref(), NO_SKU))?,
            Dimension::Project => add(non_empty(row.project.as_deref(), NO_PROJECT))?,
            Dimension::Label => {
                let labels = row.labels.as_deref().unwrap_or(&[]);
                match label_key {
                    Some(k) => {
                        let hit = labels.iter().find(|(lk, _)| lk == k);
                        add(match hit {
                            Some((_, v)) if !v.is_empty() => v.clone(),
                            Some(_) => "(empty)".to_string(),
                            None => UNLABELED.to_string(),
                        })?;
                    }
                    None if labels.is_empty() => add(NO_LABELS.to_string())?,
                    None => {
                        for (k, v) in labels {
                            add(format!("{k}={v}"))?;
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

fn non_empty(value: Option<&str>, fallback: &str) -> String {
    match value {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => fallback.to_string(),
    }
}

/// Sum of the costs of `month`.
///
/// Rows are added in file order and every partial sum must fit; a sum that
/// does not fit is an error, never a clamped number.
pub fn total(rows: &[Row], month: Month, net: bool) -> Result<Money> {
    let mut sum = Money::ZERO;
    for row in rows.iter().filter(|r| r.month == month) {
        sum = sum
            .checked_add(row_cost(row, net)?)
            .ok_or_else(|| too_large("the total"))?;
    }
    Ok(sum)
}

/// Refuses a month-over-month difference that does not fit, for every group
/// and for the totals, so `Line::delta` never has to hide an overflow.
pub fn ensure_deltas(
    current: &BTreeMap<String, Money>,
    previous: &BTreeMap<String, Money>,
) -> Result<()> {
    let keys: BTreeSet<&String> = current.keys().chain(previous.keys()).collect();
    for k in keys {
        let c = current.get(k).copied().unwrap_or(Money::ZERO);
        let p = previous.get(k).copied().unwrap_or(Money::ZERO);
        if c.checked_sub(p).is_none() {
            return Err(too_large("the change of a group"));
        }
    }
    Ok(())
}

/// One ranked group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub key: String,
    pub current: Money,
    /// `None` when there is no previous month at all.
    pub previous: Option<Money>,
}

impl Line {
    /// Current minus previous.
    pub fn delta(&self) -> Option<Money> {
        self.previous.and_then(|p| self.current.checked_sub(p))
    }

    /// Change in percent; `None` without a previous month or when it was zero.
    pub fn delta_pct(&self) -> Option<f64> {
        pct(self.current, self.previous?)
    }

    /// Whether the group has no cost in the previous month.
    pub fn is_new(&self) -> bool {
        self.previous == Some(Money::ZERO) && self.current != Money::ZERO
    }
}

/// Percent change from `previous` to `current`; `None` if `previous` is zero.
pub fn pct(current: Money, previous: Money) -> Option<f64> {
    if previous == Money::ZERO {
        return None;
    }
    let change = i128::from(current.micros()) - i128::from(previous.micros());
    Some(change as f64 / 1_000_000.0 / previous.as_f64().abs() * 100.0)
}

/// Ranked groups of one dimension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub dimension: Dimension,
    pub lines: Vec<Line>,
    /// Number of groups before cutting to the top N.
    pub groups: usize,
}

/// Ranks groups and keeps the first `top` (`0` keeps all).
pub fn rank(
    dimension: Dimension,
    current: &BTreeMap<String, Money>,
    previous: Option<&BTreeMap<String, Money>>,
    sort: Sort,
    top: usize,
) -> Section {
    let mut keys: BTreeSet<&String> = current.keys().collect();
    if sort == Sort::Delta {
        if let Some(p) = previous {
            keys.extend(p.keys());
        }
    }
    let mut lines: Vec<Line> = keys
        .into_iter()
        .map(|k| Line {
            key: k.clone(),
            current: current.get(k).copied().unwrap_or(Money::ZERO),
            previous: previous.map(|p| p.get(k).copied().unwrap_or(Money::ZERO)),
        })
        .collect();
    match sort {
        Sort::Cost => {
            lines.sort_by(|a, b| b.current.cmp(&a.current).then_with(|| a.key.cmp(&b.key)))
        }
        Sort::Delta => lines.sort_by(|a, b| {
            let da = a.delta().unwrap_or(a.current).abs();
            let db = b.delta().unwrap_or(b.current).abs();
            db.cmp(&da).then_with(|| a.key.cmp(&b.key))
        }),
    }
    let groups = lines.len();
    if top > 0 {
        lines.truncate(top);
    }
    Section {
        dimension,
        lines,
        groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(y: i32, mo: u8) -> Month {
        Month::new(y, mo).unwrap()
    }

    fn set(ms: &[(i32, u8)]) -> BTreeSet<Month> {
        ms.iter().map(|(y, mo)| m(*y, *mo)).collect()
    }

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn row(month: Month, sku: &str, project: &str, labels: &[(&str, &str)], cost: &str) -> Row {
        Row {
            month,
            sku: Some(sku.to_string()),
            project: Some(project.to_string()),
            labels: Some(
                labels
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            ),
            cost: money(cost),
            credits: None,
            currency: None,
        }
    }

    #[test]
    fn current_defaults_to_latest_month() {
        let s = select_months(&set(&[(2026, 8), (2026, 9), (2026, 10)]), None, None, None).unwrap();
        assert_eq!(s.current, m(2026, 10));
        assert_eq!(s.previous, Some(m(2026, 9)));
    }

    #[test]
    fn single_month_has_no_previous() {
        let s = select_months(&set(&[(2026, 10)]), None, None, None).unwrap();
        assert_eq!(s.previous, None);
    }

    #[test]
    fn gap_in_same_export_means_no_previous() {
        let s = select_months(&set(&[(2026, 8), (2026, 10)]), None, None, None).unwrap();
        assert_eq!(s.previous, None);
    }

    #[test]
    fn explicit_current_month() {
        let s =
            select_months(&set(&[(2026, 9), (2026, 10)]), None, Some(m(2026, 9)), None).unwrap();
        assert_eq!(s.current, m(2026, 9));
        assert_eq!(s.previous, None);
    }

    #[test]
    fn explicit_current_month_missing_lists_available() {
        let e = select_months(&set(&[(2026, 9), (2026, 10)]), None, Some(m(2026, 1)), None)
            .unwrap_err();
        let text = e.to_string();
        assert!(
            text.contains("2026-01") && text.contains("2026-09, 2026-10"),
            "{text}"
        );
    }

    #[test]
    fn explicit_previous_month_in_same_export() {
        let s =
            select_months(&set(&[(2026, 8), (2026, 10)]), None, None, Some(m(2026, 8))).unwrap();
        assert_eq!(s.previous, Some(m(2026, 8)));
    }

    #[test]
    fn explicit_previous_must_exist() {
        let e = select_months(&set(&[(2026, 10)]), None, None, Some(m(2026, 9))).unwrap_err();
        assert!(e.to_string().contains("not found"));
    }

    #[test]
    fn previous_must_be_earlier() {
        let e = select_months(
            &set(&[(2026, 9), (2026, 10)]),
            None,
            None,
            Some(m(2026, 10)),
        )
        .unwrap_err();
        assert!(e.to_string().contains("earlier"));
    }

    #[test]
    fn second_file_uses_calendar_previous() {
        let s = select_months(
            &set(&[(2026, 10)]),
            Some(&set(&[(2026, 8), (2026, 9)])),
            None,
            None,
        )
        .unwrap();
        assert_eq!(s.previous, Some(m(2026, 9)));
    }

    #[test]
    fn second_file_falls_back_to_latest_earlier_month() {
        let s = select_months(
            &set(&[(2026, 10)]),
            Some(&set(&[(2026, 6), (2026, 8)])),
            None,
            None,
        )
        .unwrap();
        assert_eq!(s.previous, Some(m(2026, 8)));
    }

    #[test]
    fn second_file_without_earlier_month_is_an_error() {
        let e = select_months(
            &set(&[(2026, 10)]),
            Some(&set(&[(2026, 10), (2026, 11)])),
            None,
            None,
        )
        .unwrap_err();
        assert!(e.to_string().contains("no month before"));
    }

    #[test]
    fn second_file_explicit_previous() {
        let s = select_months(
            &set(&[(2026, 10)]),
            Some(&set(&[(2026, 5), (2026, 9)])),
            None,
            Some(m(2026, 5)),
        )
        .unwrap();
        assert_eq!(s.previous, Some(m(2026, 5)));
    }

    #[test]
    fn dimension_names() {
        assert_eq!(Dimension::Sku.name(), "sku");
        assert_eq!(Dimension::Project.name(), "project");
        assert_eq!(Dimension::Label.name(), "label");
    }

    #[test]
    fn groups_by_sku_and_only_the_chosen_month() {
        let rows = vec![
            row(m(2026, 10), "A", "p", &[], "1"),
            row(m(2026, 10), "A", "p", &[], "2"),
            row(m(2026, 10), "B", "p", &[], "4"),
            row(m(2026, 9), "A", "p", &[], "100"),
        ];
        let g = group(&rows, m(2026, 10), Dimension::Sku, None, false).unwrap();
        assert_eq!(g.get("A"), Some(&money("3")));
        assert_eq!(g.get("B"), Some(&money("4")));
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn groups_by_project_with_fallback_name() {
        let rows = vec![
            row(m(2026, 10), "A", "", &[], "5"),
            row(m(2026, 10), "A", "p1", &[], "1"),
        ];
        let g = group(&rows, m(2026, 10), Dimension::Project, None, false).unwrap();
        assert_eq!(g.get(NO_PROJECT), Some(&money("5")));
        assert_eq!(g.get("p1"), Some(&money("1")));
    }

    #[test]
    fn empty_sku_gets_fallback_name() {
        let rows = vec![row(m(2026, 10), "", "p", &[], "5")];
        let g = group(&rows, m(2026, 10), Dimension::Sku, None, false).unwrap();
        assert_eq!(g.get(NO_SKU), Some(&money("5")));
    }

    #[test]
    fn labels_count_toward_each_pair() {
        let rows = vec![row(
            m(2026, 10),
            "A",
            "p",
            &[("env", "prod"), ("team", "web")],
            "10",
        )];
        let g = group(&rows, m(2026, 10), Dimension::Label, None, false).unwrap();
        assert_eq!(g.get("env=prod"), Some(&money("10")));
        assert_eq!(g.get("team=web"), Some(&money("10")));
    }

    #[test]
    fn rows_without_labels_are_grouped_together() {
        let rows = vec![
            row(m(2026, 10), "A", "p", &[], "2"),
            row(m(2026, 10), "B", "p", &[], "3"),
        ];
        let g = group(&rows, m(2026, 10), Dimension::Label, None, false).unwrap();
        assert_eq!(g.get(NO_LABELS), Some(&money("5")));
    }

    #[test]
    fn label_key_groups_by_value_and_partitions_the_total() {
        let rows = vec![
            row(m(2026, 10), "A", "p", &[("env", "prod")], "10"),
            row(m(2026, 10), "A", "p", &[("env", "dev"), ("team", "x")], "4"),
            row(m(2026, 10), "A", "p", &[("team", "x")], "1"),
        ];
        let g = group(&rows, m(2026, 10), Dimension::Label, Some("env"), false).unwrap();
        assert_eq!(g.get("prod"), Some(&money("10")));
        assert_eq!(g.get("dev"), Some(&money("4")));
        assert_eq!(g.get(UNLABELED), Some(&money("1")));
        assert_eq!(
            g.values()
                .copied()
                .try_fold(Money::ZERO, Money::checked_add)
                .unwrap(),
            total(&rows, m(2026, 10), false).unwrap()
        );
    }

    #[test]
    fn label_key_with_empty_value() {
        let rows = vec![row(m(2026, 10), "A", "p", &[("env", "")], "2")];
        let g = group(&rows, m(2026, 10), Dimension::Label, Some("env"), false).unwrap();
        assert_eq!(g.get("(empty)"), Some(&money("2")));
    }

    #[test]
    fn net_adds_credits() {
        let mut r = row(m(2026, 10), "A", "p", &[], "10");
        r.credits = Some(money("-3"));
        assert_eq!(row_cost(&r, false).unwrap(), money("10"));
        assert_eq!(row_cost(&r, true).unwrap(), money("7"));
        r.credits = None;
        assert_eq!(row_cost(&r, true).unwrap(), money("10"));
    }

    #[test]
    fn total_sums_one_month() {
        let rows = vec![
            row(m(2026, 10), "A", "p", &[], "1.5"),
            row(m(2026, 9), "A", "p", &[], "9"),
        ];
        assert_eq!(total(&rows, m(2026, 10), false).unwrap(), money("1.5"));
    }

    fn map(items: &[(&str, &str)]) -> BTreeMap<String, Money> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), money(v)))
            .collect()
    }

    #[test]
    fn rank_by_cost_descending_with_key_tiebreak() {
        let cur = map(&[("b", "5"), ("a", "5"), ("c", "9"), ("d", "1")]);
        let s = rank(Dimension::Sku, &cur, None, Sort::Cost, 0);
        let keys: Vec<_> = s.lines.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(keys, ["c", "a", "b", "d"]);
    }

    #[test]
    fn rank_truncates_but_remembers_group_count() {
        let cur = map(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let s = rank(Dimension::Sku, &cur, None, Sort::Cost, 2);
        assert_eq!(s.lines.len(), 2);
        assert_eq!(s.groups, 3);
    }

    #[test]
    fn top_zero_keeps_everything() {
        let cur = map(&[("a", "1"), ("b", "2"), ("c", "3")]);
        assert_eq!(
            rank(Dimension::Sku, &cur, None, Sort::Cost, 0).lines.len(),
            3
        );
    }

    #[test]
    fn rank_without_previous_has_no_delta() {
        let cur = map(&[("a", "1")]);
        let s = rank(Dimension::Sku, &cur, None, Sort::Cost, 5);
        assert_eq!(s.lines[0].previous, None);
        assert_eq!(s.lines[0].delta(), None);
        assert_eq!(s.lines[0].delta_pct(), None);
    }

    #[test]
    fn delta_and_percent() {
        let cur = map(&[("a", "150")]);
        let prev = map(&[("a", "100")]);
        let l = &rank(Dimension::Sku, &cur, Some(&prev), Sort::Cost, 5).lines[0];
        assert_eq!(l.delta(), Some(money("50")));
        assert!((l.delta_pct().unwrap() - 50.0).abs() < 1e-9);
        assert!(!l.is_new());
    }

    #[test]
    fn negative_delta() {
        let cur = map(&[("a", "10")]);
        let prev = map(&[("a", "30")]);
        let l = &rank(Dimension::Sku, &cur, Some(&prev), Sort::Cost, 5).lines[0];
        assert_eq!(l.delta(), Some(money("-20")));
        assert!((l.delta_pct().unwrap() + 66.666_666_666).abs() < 1e-6);
    }

    #[test]
    fn new_group_has_no_percent_and_is_new() {
        let cur = map(&[("a", "10")]);
        let prev = map(&[]);
        let l = &rank(Dimension::Sku, &cur, Some(&prev), Sort::Cost, 5).lines[0];
        assert_eq!(l.previous, Some(Money::ZERO));
        assert_eq!(l.delta_pct(), None);
        assert!(l.is_new());
    }

    #[test]
    fn cost_sort_ignores_vanished_groups() {
        let cur = map(&[("a", "10")]);
        let prev = map(&[("gone", "50")]);
        let s = rank(Dimension::Sku, &cur, Some(&prev), Sort::Cost, 0);
        assert_eq!(s.lines.len(), 1);
    }

    #[test]
    fn delta_sort_includes_vanished_groups_and_orders_by_magnitude() {
        let cur = map(&[("a", "110"), ("b", "10")]);
        let prev = map(&[("a", "100"), ("b", "40"), ("gone", "50")]);
        let s = rank(Dimension::Sku, &cur, Some(&prev), Sort::Delta, 0);
        let keys: Vec<_> = s.lines.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(keys, ["gone", "b", "a"]);
        assert_eq!(s.lines[0].current, Money::ZERO);
        assert_eq!(s.lines[0].delta(), Some(money("-50")));
    }

    #[test]
    fn delta_sort_without_previous_falls_back_to_cost_magnitude() {
        let cur = map(&[("a", "1"), ("b", "9")]);
        let s = rank(Dimension::Sku, &cur, None, Sort::Delta, 0);
        assert_eq!(s.lines[0].key, "b");
    }

    #[test]
    fn pct_of_zero_previous_is_none() {
        assert_eq!(pct(money("5"), Money::ZERO), None);
        assert!((pct(money("5"), money("4")).unwrap() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn sums_past_the_range_are_refused_not_clamped() {
        let rows: Vec<Row> = (0..12)
            .map(|_| row(m(2026, 10), "a", "p", &[], "900000000000"))
            .collect();
        for e in [
            total(&rows, m(2026, 10), false).unwrap_err(),
            group(&rows, m(2026, 10), Dimension::Sku, None, false).unwrap_err(),
        ] {
            assert!(e.to_string().contains("too large"), "{e}");
            assert!(e.to_string().contains("9223372036854.775807"), "{e}");
        }
    }

    #[test]
    fn cost_plus_credits_past_the_range_is_refused() {
        let mut r = row(m(2026, 10), "a", "p", &[], "1");
        r.cost = Money::from_micros(i64::MAX);
        r.credits = Some(Money::from_micros(1));
        assert!(row_cost(&r, true).is_err());
        assert!(row_cost(&r, false).is_ok());
    }

    #[test]
    fn a_change_past_the_range_is_refused() {
        let cur = BTreeMap::from([("a".to_string(), Money::from_micros(i64::MAX))]);
        let prev = BTreeMap::from([("a".to_string(), Money::from_micros(-1))]);
        assert!(ensure_deltas(&cur, &prev).is_err());
        assert!(ensure_deltas(&cur, &cur).is_ok());
    }
}
