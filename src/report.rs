//! The finished analysis and its three renderings.

use crate::aggregate::{Dimension, Line, Section, pct};
use crate::money::Money;
use crate::month::Month;

/// Longest group name shown in the table before it is cut with `…`.
pub const MAX_KEY_WIDTH: usize = 48;

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Table,
    Json,
    Csv,
}

/// Everything the renderers need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub current: Month,
    pub previous: Option<Month>,
    pub net: bool,
    pub currency: Option<String>,
    pub label_key: Option<String>,
    pub total_current: Money,
    pub total_previous: Option<Money>,
    pub sections: Vec<Section>,
}

impl Report {
    /// Total change, when there is a previous month.
    pub fn total_delta(&self) -> Option<Money> {
        self.total_previous
            .and_then(|p| self.total_current.checked_sub(p))
    }

    /// Total change in percent.
    pub fn total_pct(&self) -> Option<f64> {
        pct(self.total_current, self.total_previous?)
    }

    /// Renders in the requested format.
    pub fn render(&self, format: Format) -> String {
        match format {
            Format::Table => self.render_table(),
            Format::Json => self.render_json(),
            Format::Csv => self.render_csv(),
        }
    }

    fn section_title(&self, s: &Section) -> String {
        let what = match (s.dimension, &self.label_key) {
            (Dimension::Sku, _) => "SKU".to_string(),
            (Dimension::Project, _) => "project".to_string(),
            (Dimension::Label, Some(k)) => format!("label {}", clean(k)),
            (Dimension::Label, None) => "label".to_string(),
        };
        if s.groups > s.lines.len() {
            format!("Top {} of {} by {what}", s.lines.len(), s.groups)
        } else {
            format!("Top {} by {what}", s.lines.len())
        }
    }

    fn render_table(&self) -> String {
        let mut out = String::new();
        let basis = if self.net { "net of credits" } else { "gross" };
        let cur = self
            .currency
            .as_deref()
            .map(|c| format!(", {}", clean(c)))
            .unwrap_or_default();
        match self.previous {
            Some(p) => out.push_str(&format!(
                "gcp-billing-top: {} vs {p} ({basis}{cur})\n",
                self.current
            )),
            None => out.push_str(&format!(
                "gcp-billing-top: {} ({basis}{cur})\n",
                self.current
            )),
        }
        out.push_str(&format!("Total: {}", self.total_current.to_display()));
        if let (Some(prev), Some(delta)) = (self.total_previous, self.total_delta()) {
            out.push_str(&format!(
                "  (previous {}, {}, {})",
                prev.to_display(),
                signed(delta),
                pct_text(self.total_pct(), false)
            ));
        }
        out.push('\n');
        for s in &self.sections {
            out.push('\n');
            out.push_str(&self.section_title(s));
            out.push('\n');
            out.push_str(&render_section(s, self.previous.is_some()));
        }
        if self.previous.is_none() {
            out.push_str("\nNo previous month found: deltas are not shown. Give --previous FILE or an export with two months.\n");
        }
        if self
            .sections
            .iter()
            .any(|s| s.dimension == Dimension::Label)
            && self.label_key.is_none()
        {
            out.push_str("\nNote: a row counts toward each of its labels, so label costs can add up to more than the total.\n");
        }
        out
    }

    fn render_json(&self) -> String {
        let line_json = |l: &Line| {
            serde_json::json!({
                "key": l.key,
                "current": l.current.as_f64(),
                "previous": l.previous.map(Money::as_f64),
                "delta": l.delta().map(Money::as_f64),
                "delta_pct": l.delta_pct().map(round2),
            })
        };
        let sections: Vec<_> = self
            .sections
            .iter()
            .map(|s| {
                serde_json::json!({
                    "dimension": s.dimension.name(),
                    "groups": s.groups,
                    "top": s.lines.iter().map(line_json).collect::<Vec<_>>(),
                })
            })
            .collect();
        let doc = serde_json::json!({
            "current_month": self.current.to_string(),
            "previous_month": self.previous.map(|p| p.to_string()),
            "net": self.net,
            "currency": self.currency,
            "label_key": self.label_key,
            "total": {
                "current": self.total_current.as_f64(),
                "previous": self.total_previous.map(Money::as_f64),
                "delta": self.total_delta().map(Money::as_f64),
                "delta_pct": self.total_pct().map(round2),
            },
            "sections": sections,
        });
        let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
        text.push('\n');
        text
    }

    fn render_csv(&self) -> String {
        let mut w = csv::Writer::from_writer(Vec::new());
        let opt = |m: Option<Money>| m.map(|m| m.to_plain()).unwrap_or_default();
        let optp = |p: Option<f64>| p.map(|p| format!("{p:.1}")).unwrap_or_default();
        let _ = w.write_record([
            "dimension",
            "key",
            "current",
            "previous",
            "delta",
            "delta_pct",
        ]);
        let _ = w.write_record([
            "total",
            "",
            &self.total_current.to_plain(),
            &opt(self.total_previous),
            &opt(self.total_delta()),
            &optp(self.total_pct()),
        ]);
        for s in &self.sections {
            for l in &s.lines {
                let _ = w.write_record([
                    &csv_text(s.dimension.name()),
                    &csv_text(&l.key),
                    &l.current.to_plain(),
                    &opt(l.previous),
                    &opt(l.delta()),
                    &optp(l.delta_pct()),
                ]);
            }
        }
        String::from_utf8(w.into_inner().unwrap_or_default()).unwrap_or_default()
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// `+50.00`, `-20.00`, `0.00`.
pub fn signed(m: Money) -> String {
    let text = m.to_display();
    if m > Money::ZERO && text != "0.00" {
        format!("+{text}")
    } else {
        text
    }
}

/// `+33.3%`, `-66.7%`, `new` or `-`.
pub fn pct_text(p: Option<f64>, is_new: bool) -> String {
    match p {
        Some(p) if p.abs() < 0.05 => "0.0%".to_string(),
        Some(p) => format!("{p:+.1}%"),
        None if is_new => "new".to_string(),
        None => "-".to_string(),
    }
}

/// Replaces every control character (newline, tab, escape, ...) and the
/// Unicode line and paragraph separators with `?`, so a value from the input
/// cannot start a fake table row or send a terminal sequence.
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || c == '\u{2028}' || c == '\u{2029}' {
                '?'
            } else {
                c
            }
        })
        .collect()
}

/// Makes a text cell safe for a spreadsheet: a cell that starts with `=`,
/// `+` or `@` gets a single quote in front, so it is read as text.
pub fn csv_text(text: &str) -> String {
    if text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{text}")
    } else {
        text.to_string()
    }
}

/// Cuts `text` to `max` characters, ending in `…` when cut.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn render_section(s: &Section, with_delta: bool) -> String {
    let width = s
        .lines
        .iter()
        .map(|l| truncate(&clean(&l.key), MAX_KEY_WIDTH).chars().count())
        .max()
        .unwrap_or(0)
        .max(4);
    let mut table: Vec<Vec<String>> = Vec::new();
    for (i, l) in s.lines.iter().enumerate() {
        let mut row = vec![
            (i + 1).to_string(),
            truncate(&clean(&l.key), MAX_KEY_WIDTH),
            l.current.to_display(),
        ];
        if with_delta {
            row.push(l.previous.map(Money::to_display).unwrap_or_default());
            row.push(l.delta().map(signed).unwrap_or_default());
            row.push(pct_text(l.delta_pct(), l.is_new()));
        }
        table.push(row);
    }
    let mut headers = vec!["#", "Name", "Cost"];
    if with_delta {
        headers.extend(["Previous", "Delta", "Delta %"]);
    }
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    widths[1] = width;
    for row in &table {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let fmt_row = |cells: &[&str]| {
        let mut line = String::from("  ");
        for i in 0..cols {
            if i > 0 {
                line.push_str("  ");
            }
            if i == 1 {
                line.push_str(&format!("{:<w$}", cells[i], w = widths[i]));
            } else {
                line.push_str(&format!("{:>w$}", cells[i], w = widths[i]));
            }
        }
        line.trim_end().to_string() + "\n"
    };
    let mut out = fmt_row(&headers);
    for row in &table {
        let cells: Vec<&str> = row.iter().map(String::as_str).collect();
        out.push_str(&fmt_row(&cells));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn line(key: &str, cur: &str, prev: Option<&str>) -> Line {
        Line {
            key: key.to_string(),
            current: money(cur),
            previous: prev.map(money),
        }
    }

    fn report(with_prev: bool) -> Report {
        let prev = |s: &'static str| if with_prev { Some(s) } else { None };
        Report {
            current: Month::new(2026, 10).unwrap(),
            previous: if with_prev { Month::new(2026, 9) } else { None },
            net: false,
            currency: Some("USD".to_string()),
            label_key: None,
            total_current: money("300"),
            total_previous: prev("250").map(money),
            sections: vec![Section {
                dimension: Dimension::Sku,
                groups: 3,
                lines: vec![
                    line("N2 Core", "200", prev("150")),
                    line("Brand new", "25", prev("0")),
                ],
            }],
        }
    }

    #[test]
    fn signed_formats() {
        assert_eq!(signed(money("50")), "+50.00");
        assert_eq!(signed(money("-20")), "-20.00");
        assert_eq!(signed(Money::ZERO), "0.00");
        assert_eq!(signed(money("0.001")), "0.00");
        assert_eq!(signed(money("1234.5")), "+1,234.50");
    }

    #[test]
    fn pct_text_formats() {
        assert_eq!(pct_text(Some(33.333), false), "+33.3%");
        assert_eq!(pct_text(Some(-66.666), false), "-66.7%");
        assert_eq!(pct_text(Some(0.0), false), "0.0%");
        assert_eq!(pct_text(Some(0.01), false), "0.0%");
        assert_eq!(pct_text(None, true), "new");
        assert_eq!(pct_text(None, false), "-");
    }

    #[test]
    fn truncate_cuts_with_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 10), "abcdefghij");
        assert_eq!(truncate("abcdefghijk", 10), "abcdefghi…");
        assert_eq!(truncate("ééééééé", 4), "ééé…");
    }

    #[test]
    fn table_header_and_total_with_previous() {
        let t = report(true).render(Format::Table);
        assert!(
            t.starts_with("gcp-billing-top: 2026-10 vs 2026-09 (gross, USD)\n"),
            "{t}"
        );
        assert!(
            t.contains("Total: 300.00  (previous 250.00, +50.00, +20.0%)"),
            "{t}"
        );
    }

    #[test]
    fn table_rows_show_delta_and_new() {
        let t = report(true).render(Format::Table);
        assert!(t.contains("Top 2 of 3 by SKU"), "{t}");
        assert!(
            t.contains("N2 Core") && t.contains("+50.00") && t.contains("+33.3%"),
            "{t}"
        );
        assert!(t.contains("Brand new") && t.contains("new"), "{t}");
        assert!(t.contains("Delta %"), "{t}");
    }

    #[test]
    fn table_without_previous_has_no_delta_columns_and_a_note() {
        let t = report(false).render(Format::Table);
        assert!(!t.contains("Delta"), "{t}");
        assert!(t.contains("No previous month found"), "{t}");
        assert!(
            t.starts_with("gcp-billing-top: 2026-10 (gross, USD)\n"),
            "{t}"
        );
    }

    #[test]
    fn table_net_label_and_no_currency() {
        let mut r = report(true);
        r.net = true;
        r.currency = None;
        let t = r.render(Format::Table);
        assert!(
            t.starts_with("gcp-billing-top: 2026-10 vs 2026-09 (net of credits)\n"),
            "{t}"
        );
    }

    #[test]
    fn table_title_without_cut_says_just_top_n() {
        let mut r = report(true);
        r.sections[0].groups = 2;
        assert!(r.render(Format::Table).contains("Top 2 by SKU"));
    }

    #[test]
    fn label_title_with_key_and_note_without_key() {
        let mut r = report(true);
        r.sections[0].dimension = Dimension::Label;
        let t = r.render(Format::Table);
        assert!(t.contains("by label\n"), "{t}");
        assert!(t.contains("counts toward each of its labels"), "{t}");
        r.label_key = Some("env".to_string());
        let t = r.render(Format::Table);
        assert!(t.contains("by label env\n"), "{t}");
        assert!(!t.contains("counts toward each"), "{t}");
    }

    #[test]
    fn long_names_are_truncated_in_the_table() {
        let mut r = report(true);
        r.sections[0].lines[0].key = "x".repeat(80);
        let t = r.render(Format::Table);
        assert!(
            t.contains(&format!("{}…", "x".repeat(MAX_KEY_WIDTH - 1))),
            "{t}"
        );
        assert!(!t.contains(&"x".repeat(MAX_KEY_WIDTH + 1)), "{t}");
    }

    #[test]
    fn table_columns_are_aligned() {
        let t = report(true).render(Format::Table);
        let rows: Vec<&str> = t.lines().filter(|l| l.starts_with("  ")).collect();
        assert!(rows.len() >= 3);
        let end = rows[0].chars().count();
        assert!(rows.iter().all(|r| r.chars().count() == end), "{t}");
    }

    #[test]
    fn json_is_valid_and_complete() {
        let v: serde_json::Value =
            serde_json::from_str(&report(true).render(Format::Json)).unwrap();
        assert_eq!(v["current_month"], "2026-10");
        assert_eq!(v["previous_month"], "2026-09");
        assert_eq!(v["currency"], "USD");
        assert_eq!(v["total"]["delta"], 50.0);
        assert_eq!(v["total"]["delta_pct"], 20.0);
        assert_eq!(v["sections"][0]["dimension"], "sku");
        assert_eq!(v["sections"][0]["groups"], 3);
        assert_eq!(v["sections"][0]["top"][0]["key"], "N2 Core");
        assert_eq!(v["sections"][0]["top"][0]["delta_pct"], 33.33);
        assert!(v["sections"][0]["top"][1]["delta_pct"].is_null());
    }

    #[test]
    fn json_without_previous_uses_null() {
        let v: serde_json::Value =
            serde_json::from_str(&report(false).render(Format::Json)).unwrap();
        assert!(v["previous_month"].is_null());
        assert!(v["total"]["previous"].is_null());
        assert!(v["sections"][0]["top"][0]["delta"].is_null());
    }

    #[test]
    fn csv_has_header_total_and_rows() {
        let c = report(true).render(Format::Csv);
        let lines: Vec<&str> = c.lines().collect();
        assert_eq!(lines[0], "dimension,key,current,previous,delta,delta_pct");
        assert_eq!(lines[1], "total,,300.00,250.00,50.00,20.0");
        assert_eq!(lines[2], "sku,N2 Core,200.00,150.00,50.00,33.3");
        assert_eq!(lines[3], "sku,Brand new,25.00,0.00,25.00,");
    }

    #[test]
    fn csv_quotes_keys_with_commas_and_leaves_previous_empty_without_one() {
        let mut r = report(false);
        r.sections[0].lines[0] = line("a, \"b\"", "1", None);
        let c = r.render(Format::Csv);
        assert!(c.contains("sku,\"a, \"\"b\"\"\",1.00,,,"), "{c}");
        assert!(c.contains("total,,300.00,,,"), "{c}");
    }

    #[test]
    fn table_replaces_control_characters_in_names() {
        let mut r = report(true);
        r.sections[0].lines[0].key = "a\nfake  row\tb\x1b[31mred\r\u{2028}".to_string();
        r.label_key = Some("k\x1b]0;x\x07".to_string());
        r.currency = Some("U\nSD".to_string());
        let t = r.render(Format::Table);
        assert!(!t.chars().any(|c| c.is_control() && c != '\n'), "{t:?}");
        assert!(t.contains("a?fake  row?b?[31mred??"), "{t:?}");
        assert!(t.contains("(gross, U?SD)"), "{t:?}");
        let rows = t.lines().filter(|l| l.starts_with("  ")).count();
        assert_eq!(rows, 3, "{t:?}");
    }

    #[test]
    fn json_keeps_control_characters_escaped() {
        let mut r = report(true);
        r.sections[0].lines[0].key = "a\nb\x1b".to_string();
        let j = r.render(Format::Json);
        assert!(!j.contains('\x1b'), "{j:?}");
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["sections"][0]["top"][0]["key"], "a\nb\x1b");
    }

    #[test]
    fn csv_cells_starting_with_a_formula_character_get_a_quote() {
        let mut r = report(true);
        r.sections[0].lines[0] = line("=SUM(A1)", "-5", Some("1"));
        r.sections[0].lines[1] = line("@cmd", "2", None);
        r.sections.push(Section {
            dimension: Dimension::Project,
            groups: 1,
            lines: vec![line("+1+1", "3", None)],
        });
        let c = r.render(Format::Csv);
        assert!(c.contains("sku,'=SUM(A1),-5.00,"), "{c}");
        assert!(c.contains("sku,'@cmd,2.00,"), "{c}");
        assert!(c.contains("project,'+1+1,3.00,"), "{c}");
        assert!(!c.contains(",=SUM"), "{c}");
    }

    #[test]
    fn csv_text_quotes_a_leading_minus() {
        assert_eq!(csv_text("-2+3"), "'-2+3");
        assert_eq!(csv_text("-cmd|' /C calc'!A0"), "'-cmd|' /C calc'!A0");
    }

    #[test]
    fn csv_text_quotes_a_leading_tab() {
        assert_eq!(csv_text("\t=1+1"), "'\t=1+1");
    }

    #[test]
    fn csv_text_quotes_a_leading_carriage_return() {
        assert_eq!(csv_text("\r=1+1"), "'\r=1+1");
    }

    #[test]
    fn csv_keeps_negative_amounts_and_plain_names_as_they_are() {
        let mut r = report(true);
        r.sections[0].lines[0] = line("N2 = Core", "-5", Some("10"));
        let c = r.render(Format::Csv);
        assert!(c.contains("sku,N2 = Core,-5.00,10.00,-15.00,-150.0"), "{c}");
    }

    #[test]
    fn total_delta_and_pct() {
        let r = report(true);
        assert_eq!(r.total_delta(), Some(money("50")));
        assert!((r.total_pct().unwrap() - 20.0).abs() < 1e-9);
        let r = report(false);
        assert_eq!(r.total_delta(), None);
        assert_eq!(r.total_pct(), None);
    }
}
