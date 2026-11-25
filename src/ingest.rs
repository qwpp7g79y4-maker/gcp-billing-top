//! Reads a billing-export CSV into typed rows.

use std::io::Read;

use crate::error::{Error, Fault, Result};
use crate::labels::parse_labels;
use crate::money::Money;
use crate::month::Month;

/// One line of the export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub month: Month,
    /// `None` when the export has no SKU column.
    pub sku: Option<String>,
    /// `None` when the export has no project column.
    pub project: Option<String>,
    /// `None` when the export has no labels column.
    pub labels: Option<Vec<(String, String)>>,
    pub cost: Money,
    /// `None` when the export has no credits column; credits are negative.
    pub credits: Option<Money>,
    pub currency: Option<String>,
}

/// All rows of one file plus which optional columns it had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub source_name: String,
    pub rows: Vec<Row>,
    pub has_sku: bool,
    pub has_project: bool,
    pub has_labels: bool,
    pub has_credits: bool,
    pub has_currency: bool,
}

const SKU: &[&str] = &["sku description", "sku"];
const PROJECT: &[&str] = &["project id", "project name"];
const LABELS: &[&str] = &["labels", "label"];
const CREDITS: &[&str] = &["credits", "credit"];
const MONTH: &[&str] = &[
    "usage start time",
    "usage start date",
    "usage start",
    "invoice month",
];
const CURRENCY: &[&str] = &["currency"];

/// Lower-cases and turns every run of non-alphanumerics into one space:
/// `Cost ($)` -> `cost`, `sku.description` -> `sku description`.
pub fn normalize_header(h: &str) -> String {
    let mut out = String::new();
    for c in h.trim_start_matches('\u{feff}').chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim_end().to_string()
}

/// The one column of a kind: every column whose normalized name is one of
/// `aliases`. Two of them is ambiguous: nothing is picked by position and
/// nothing by alias order, also not when both say the same thing.
fn find_unique(
    raw_headers: &[String],
    headers: &[String],
    aliases: &[&str],
    what: &str,
    source_name: &str,
) -> Result<Option<usize>> {
    let hits: Vec<usize> = (0..headers.len())
        .filter(|&i| aliases.contains(&headers[i].as_str()))
        .collect();
    match hits[..] {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        [first, second, ..] => Err(Error::Ambiguous {
            source_name: source_name.to_string(),
            line: None,
            message: format!(
                "two columns are the {what} column ({:?} and {:?}); keep one of them",
                raw_headers[first].trim_start_matches('\u{feff}'),
                raw_headers[second]
            ),
        }),
    }
}

/// The project column, where two different names are normal (a BigQuery
/// export has both `project.id` and `project.name`); `aliases` are in order
/// of preference. The same name twice is ambiguous, like any other kind.
fn find_preferred(
    raw_headers: &[String],
    headers: &[String],
    aliases: &[&str],
    what: &str,
    source_name: &str,
) -> Result<Option<usize>> {
    let mut found = Vec::new();
    for a in aliases {
        found.push(find_unique(raw_headers, headers, &[a], what, source_name)?);
    }
    Ok(found.into_iter().flatten().next())
}

/// Whether a raw header is the cost column: `cost`, or `Cost (` and a
/// currency or symbol `)`, as in `Cost ($)` or `Cost (EUR)`. Nothing else:
/// `cost_at_list`, `cost center` and `Unrounded cost (EUR)` are other columns.
fn is_cost_header(raw: &str) -> bool {
    let h = raw.trim_start_matches('\u{feff}').trim().to_lowercase();
    let Some(rest) = h.strip_prefix("cost") else {
        return false;
    };
    let rest = rest.trim_start();
    rest.is_empty()
        || rest
            .strip_prefix('(')
            .and_then(|r| r.strip_suffix(')'))
            .is_some_and(|inner| !inner.contains(['(', ')']))
}

/// The one cost column. Two of them is ambiguous: nothing picks one by position.
fn find_cost(raw_headers: &[String], source_name: &str) -> Result<Option<usize>> {
    let hits: Vec<usize> = (0..raw_headers.len())
        .filter(|&i| is_cost_header(&raw_headers[i]))
        .collect();
    match hits[..] {
        [] => Ok(None),
        [one] => Ok(Some(one)),
        [first, second, ..] => Err(Error::Ambiguous {
            source_name: source_name.to_string(),
            line: None,
            message: format!(
                "two columns are the cost column ({:?} and {:?}); keep one of them",
                raw_headers[first].trim_start_matches('\u{feff}'),
                raw_headers[second]
            ),
        }),
    }
}

fn parse_credits(text: &str) -> std::result::Result<Money, Fault> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(Money::ZERO);
    }
    if t.starts_with('[') {
        let value: serde_json::Value =
            serde_json::from_str(t).map_err(|e| format!("credits are not valid JSON: {e}"))?;
        let items = value.as_array().ok_or("credits JSON must be an array")?;
        if let Some(key) = crate::json::repeated_key(t) {
            return Err(Fault::Ambiguous(format!(
                "the JSON key {key:?} appears twice in one credit"
            )));
        }
        let mut total = Money::ZERO;
        for item in items {
            let amount = item.get("amount").ok_or("a credit has no \"amount\"")?;
            let text = match amount {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => return Err("a credit amount must be a number".into()),
            };
            total = total
                .checked_add(Money::parse(&text)?)
                .ok_or_else(|| "the credits of one row are too large to add up".to_string())?;
        }
        return Ok(total);
    }
    Ok(Money::parse(t)?)
}

/// Turns byte offsets into line numbers by counting line ends itself, so a
/// file with `\r\n` line ends gets the same numbers as one with `\n`.
/// A line ends at `\n`, `\r\n` or a lone `\r`. Offsets must not go backwards.
struct Lines<'a> {
    bytes: &'a [u8],
    counted_to: usize,
    line: u64,
}

impl<'a> Lines<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Lines {
            bytes,
            counted_to: 0,
            line: 1,
        }
    }

    /// The 1-based line that holds the record the csv reader reports at `offset`.
    ///
    /// After `\r\n` the reader reports the offset of the `\n`, one byte before
    /// the record; that byte still ends the previous line. Blank lines between
    /// records are skipped by the reader and counted here.
    fn at(&mut self, offset: usize) -> u64 {
        let mut end = offset.min(self.bytes.len());
        if end > 0 && self.bytes.get(end) == Some(&b'\n') && self.bytes[end - 1] == b'\r' {
            end += 1;
        }
        // The reader reports the end of the previous record when blank lines
        // follow it; the record starts after them, and they are lines too.
        while matches!(self.bytes.get(end), Some(b'\n' | b'\r')) {
            end += 1;
        }
        for i in self.counted_to..end {
            let b = self.bytes[i];
            if b == b'\n' || (b == b'\r' && self.bytes.get(i + 1) != Some(&b'\n')) {
                self.line += 1;
            }
        }
        self.counted_to = self.counted_to.max(end);
        self.line
    }
}

/// Reads one export. `source_name` is only used in error messages.
pub fn read_table<R: Read>(mut reader: R, source_name: &str) -> Result<Table> {
    let schema_err = |message: String| Error::Schema {
        source_name: source_name.to_string(),
        message,
    };
    let mut raw = Vec::new();
    reader.read_to_end(&mut raw).map_err(|e| Error::Io {
        source_name: source_name.to_string(),
        message: e.to_string(),
    })?;
    // The byte-order mark is no part of the first header, and the line count
    // below starts after it.
    let bytes = raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw);
    let mut lines = Lines::new(bytes);
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let raw_headers: Vec<String> = rdr
        .headers()
        .map_err(|e| schema_err(format!("cannot read the header line: {e}")))?
        .iter()
        .map(str::to_string)
        .collect();
    let headers: Vec<String> = raw_headers.iter().map(|h| normalize_header(h)).collect();
    if headers.iter().all(String::is_empty) {
        return Err(schema_err("the file is empty".to_string()));
    }
    let cost_col = find_cost(&raw_headers, source_name)?.ok_or_else(|| {
        schema_err(
            "no cost column found (expected a header such as \"cost\" or \"Cost ($)\")".to_string(),
        )
    })?;
    let month_col = find_unique(&raw_headers, &headers, MONTH, "month", source_name)?
        .ok_or_else(|| {
        schema_err(
            "no month column found (expected \"usage_start_time\", \"Usage start date\" or \"invoice.month\")".to_string(),
        )
    })?;
    let sku_col = find_unique(&raw_headers, &headers, SKU, "SKU", source_name)?;
    let project_col = find_preferred(&raw_headers, &headers, PROJECT, "project", source_name)?;
    let labels_col = find_unique(&raw_headers, &headers, LABELS, "labels", source_name)?;
    let credits_col = find_unique(&raw_headers, &headers, CREDITS, "credits", source_name)?;
    let currency_col = find_unique(&raw_headers, &headers, CURRENCY, "currency", source_name)?;

    let mut rows = Vec::new();
    for record in rdr.records() {
        let record = record.map_err(|e| {
            let line = e.position().map_or(0, |p| {
                lines.at(usize::try_from(p.byte()).unwrap_or(usize::MAX))
            });
            // The csv crate puts its own (CRLF-blind) line number in its text,
            // so the message is built here from the kind of error.
            let message = match e.kind() {
                csv::ErrorKind::UnequalLengths {
                    expected_len, len, ..
                } => format!(
                    "found a record with {len} fields, but the header has {expected_len} fields"
                ),
                csv::ErrorKind::Utf8 { err, .. } => format!("invalid UTF-8: {err}"),
                _ => e.to_string(),
            };
            Error::Row {
                source_name: source_name.to_string(),
                line,
                message,
            }
        })?;
        let line = record.position().map_or(0, |p| {
            lines.at(usize::try_from(p.byte()).unwrap_or(usize::MAX))
        });
        let fault = |what: &str, f: Fault| match f {
            Fault::Invalid(message) => Error::Row {
                source_name: source_name.to_string(),
                line,
                message: format!("{what}: {message}"),
            },
            Fault::Ambiguous(message) => Error::Ambiguous {
                source_name: source_name.to_string(),
                line: Some(line),
                message: format!("{what}: {message}"),
            },
        };
        let row_err = |message: String| Error::Row {
            source_name: source_name.to_string(),
            line,
            message,
        };
        let field = |i: usize| record.get(i).unwrap_or("");
        let month = Month::from_field(field(month_col))
            .ok_or_else(|| row_err(format!("cannot read a month from {:?}", field(month_col))))?;
        let cost = Money::parse(field(cost_col)).map_err(|e| row_err(format!("cost: {e}")))?;
        let credits = match credits_col {
            Some(i) => Some(parse_credits(field(i)).map_err(|f| fault("credits", f))?),
            None => None,
        };
        let labels = match labels_col {
            Some(i) => Some(parse_labels(field(i)).map_err(|f| fault("labels", f))?),
            None => None,
        };
        rows.push(Row {
            month,
            sku: sku_col.map(|i| field(i).to_string()),
            project: project_col.map(|i| field(i).to_string()),
            labels,
            cost,
            credits,
            currency: currency_col
                .map(|i| field(i).to_uppercase())
                .filter(|c| !c.is_empty()),
        });
    }
    if rows.is_empty() {
        return Err(schema_err(
            "the file has a header but no data rows".to_string(),
        ));
    }
    Ok(Table {
        source_name: source_name.to_string(),
        rows,
        has_sku: sku_col.is_some(),
        has_project: project_col.is_some(),
        has_labels: labels_col.is_some(),
        has_credits: credits_col.is_some(),
        has_currency: currency_col.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(s: &str) -> Result<Table> {
        read_table(s.as_bytes(), "t.csv")
    }

    const BASIC: &str = "project.id,sku.description,usage_start_time,cost\n\
                         p1,Core,2026-10-03T00:00:00Z,1.50\n\
                         p2,RAM,2026-10-04T00:00:00Z,2.25\n";

    #[test]
    fn normalize_header_variants() {
        assert_eq!(normalize_header("Cost ($)"), "cost");
        assert_eq!(normalize_header("sku.description"), "sku description");
        assert_eq!(normalize_header("SKU_description"), "sku description");
        assert_eq!(normalize_header("  Project ID "), "project id");
        assert_eq!(normalize_header("\u{feff}cost"), "cost");
        assert_eq!(normalize_header("invoice.month"), "invoice month");
    }

    #[test]
    fn reads_basic_bigquery_columns() {
        let t = read(BASIC).unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[0].sku.as_deref(), Some("Core"));
        assert_eq!(t.rows[0].project.as_deref(), Some("p1"));
        assert_eq!(t.rows[1].cost, Money::parse("2.25").unwrap());
        assert_eq!(t.rows[0].month, Month::new(2026, 10).unwrap());
        assert!(t.has_sku && t.has_project);
        assert!(!t.has_labels && !t.has_credits && !t.has_currency);
    }

    #[test]
    fn reads_console_style_headers() {
        let t = read("Project ID,SKU description,Usage start date,Cost ($),Currency\np1,Core,\"Oct 1, 2026\",3.00,usd\n").unwrap();
        assert_eq!(t.rows[0].month, Month::new(2026, 10).unwrap());
        assert_eq!(t.rows[0].currency.as_deref(), Some("USD"));
        assert!(t.has_currency);
    }

    #[test]
    fn project_name_is_a_fallback_for_project_id() {
        let t = read("Project name,usage_start_time,cost\nMy Project,2026-10-01,1\n").unwrap();
        assert_eq!(t.rows[0].project.as_deref(), Some("My Project"));
    }

    #[test]
    fn project_id_beats_project_name() {
        let t = read("Project name,Project ID,usage_start_time,cost\nName,id-1,2026-10-01,1\n")
            .unwrap();
        assert_eq!(t.rows[0].project.as_deref(), Some("id-1"));
    }

    #[test]
    fn cost_column_with_currency_suffix() {
        let t = read("usage_start_time,Cost (USD)\n2026-10-01,4\n").unwrap();
        assert_eq!(t.rows[0].cost, Money::parse("4").unwrap());
    }

    #[test]
    fn exact_cost_header_beats_prefix_match() {
        let t = read("cost type,cost,usage_start_time\nregular,7,2026-10-01\n").unwrap();
        assert_eq!(t.rows[0].cost, Money::parse("7").unwrap());
    }

    #[test]
    fn usage_start_with_invoice_month_is_ambiguous() {
        let e = read("invoice.month,usage_start_time,cost\n202611,2026-10-31T23:00:00Z,1\n")
            .unwrap_err();
        assert!(matches!(e, Error::Ambiguous { line: None, .. }), "{e:?}");
        assert!(
            e.to_string().contains("\"invoice.month\"")
                && e.to_string().contains("\"usage_start_time\"")
        );
    }

    #[test]
    fn invoice_month_alone_is_enough() {
        let t = read("invoice.month,cost\n202610,1\n").unwrap();
        assert_eq!(t.rows[0].month, Month::new(2026, 10).unwrap());
    }

    #[test]
    fn bom_on_first_header_is_ignored() {
        let t = read("\u{feff}usage_start_time,cost\n2026-10-01,1\n").unwrap();
        assert_eq!(t.rows.len(), 1);
    }

    #[test]
    fn reads_json_labels_with_quotes() {
        let t = read("usage_start_time,cost,labels\n2026-10-01,1,\"[{\"\"key\"\":\"\"env\"\",\"\"value\"\":\"\"prod\"\"}]\"\n").unwrap();
        assert_eq!(
            t.rows[0].labels,
            Some(vec![("env".to_string(), "prod".to_string())])
        );
        assert!(t.has_labels);
    }

    #[test]
    fn numeric_credits() {
        let t =
            read("usage_start_time,cost,credits\n2026-10-01,10,-2.5\n2026-10-02,10,\n").unwrap();
        assert_eq!(t.rows[0].credits, Some(Money::parse("-2.5").unwrap()));
        assert_eq!(t.rows[1].credits, Some(Money::ZERO));
    }

    #[test]
    fn json_credits_are_summed() {
        let c =
            r#""[{""name"":""SUD"",""amount"":-1.5},{""name"":""Promo"",""amount"":""-0.25""}]""#;
        let t = read(&format!(
            "usage_start_time,cost,credits\n2026-10-01,10,{c}\n"
        ))
        .unwrap();
        assert_eq!(t.rows[0].credits, Some(Money::parse("-1.75").unwrap()));
    }

    #[test]
    fn empty_json_credits_array_is_zero() {
        let t = read("usage_start_time,cost,credits\n2026-10-01,10,[]\n").unwrap();
        assert_eq!(t.rows[0].credits, Some(Money::ZERO));
    }

    #[test]
    fn bad_credits_report_the_line() {
        let e = read("usage_start_time,cost,credits\n2026-10-01,10,oops\n").unwrap_err();
        assert!(matches!(e, Error::Row { line: 2, .. }), "{e:?}");
        assert!(e.to_string().contains("credits"));
    }

    #[test]
    fn credit_without_amount_is_an_error() {
        let e =
            read("usage_start_time,cost,credits\n2026-10-01,10,\"[{\"\"name\"\":\"\"x\"\"}]\"\n")
                .unwrap_err();
        assert!(e.to_string().contains("amount"), "{e}");
    }

    #[test]
    fn missing_cost_column() {
        let e = read("usage_start_time,sku\n2026-10-01,x\n").unwrap_err();
        assert!(matches!(e, Error::Schema { .. }));
        assert!(e.to_string().contains("no cost column"));
    }

    #[test]
    fn missing_month_column() {
        let e = read("cost,sku\n1,x\n").unwrap_err();
        assert!(e.to_string().contains("no month column"));
    }

    #[test]
    fn bad_cost_names_file_and_line() {
        let e = read("usage_start_time,cost\n2026-10-01,1\n2026-10-02,abc\n").unwrap_err();
        assert_eq!(e.to_string(), "t.csv, line 3: cost: invalid amount \"abc\"");
    }

    #[test]
    fn bad_month_names_the_line() {
        let e = read("usage_start_time,cost\nnot a date,1\n").unwrap_err();
        assert!(matches!(e, Error::Row { line: 2, .. }));
    }

    #[test]
    fn bad_labels_name_the_line() {
        let e = read("usage_start_time,cost,labels\n2026-10-01,1,novalue\n").unwrap_err();
        assert!(e.to_string().contains("labels"), "{e}");
    }

    #[test]
    fn ragged_row_is_an_error() {
        let e = read("usage_start_time,cost\n2026-10-01,1,extra\n").unwrap_err();
        assert!(matches!(e, Error::Row { .. }), "{e:?}");
    }

    #[test]
    fn empty_input() {
        let e = read("").unwrap_err();
        assert!(matches!(e, Error::Schema { .. }));
    }

    #[test]
    fn header_only() {
        let e = read("usage_start_time,cost\n").unwrap_err();
        assert!(e.to_string().contains("no data rows"));
    }

    #[test]
    fn whitespace_around_fields_is_trimmed() {
        let t = read("usage_start_time , cost\n 2026-10-01 , 1.5 \n").unwrap();
        assert_eq!(t.rows[0].cost, Money::parse("1.5").unwrap());
    }

    #[test]
    fn empty_currency_cell_is_none() {
        let t = read("usage_start_time,cost,currency\n2026-10-01,1,\n").unwrap();
        assert_eq!(t.rows[0].currency, None);
        assert!(t.has_currency);
    }

    #[test]
    fn lines_end_at_lf_crlf_or_a_lone_cr() {
        for eol in ["\n", "\r\n", "\r"] {
            let e = read(&format!(
                "usage_start_time,cost{eol}2026-10-01,1{eol}2026-10-02,1{eol}x,zz{eol}"
            ))
            .unwrap_err();
            assert!(matches!(e, Error::Row { line: 4, .. }), "{eol:?}: {e:?}");
        }
    }

    #[test]
    fn the_cost_header_is_cost_or_cost_in_brackets_and_nothing_else() {
        for yes in [
            "cost",
            "Cost",
            " COST ",
            "Cost ($)",
            "Cost (EUR)",
            "\u{feff}cost",
            "cost(USD)",
        ] {
            assert!(is_cost_header(yes), "{yes:?}");
        }
        for no in [
            "cost_at_list",
            "cost center",
            "cost type",
            "costly",
            "Unrounded cost (EUR)",
            "Cost (EUR) (x)",
            "Cost (",
            "",
        ] {
            assert!(!is_cost_header(no), "{no:?}");
        }
    }

    #[test]
    fn two_cost_columns_are_ambiguous_and_name_both() {
        let e = read("a,cost,Cost ($),usage_start_time\nx,1,2,2026-10-01\n").unwrap_err();
        assert!(matches!(e, Error::Ambiguous { line: None, .. }), "{e:?}");
        assert!(e.to_string().contains("\"cost\"") && e.to_string().contains("\"Cost ($)\""));
    }

    #[test]
    fn credits_that_add_up_past_the_range_are_refused() {
        let big = "900000000000";
        let text = format!("[{{\"amount\":{big}}},{{\"amount\":{big}}}]");
        let mut csv = String::from("usage_start_time,cost,credits\n");
        let many = (0..11)
            .map(|_| format!("{{\"amount\":{big}}}"))
            .collect::<Vec<_>>()
            .join(",");
        csv.push_str(&format!(
            "2026-10-01,1,\"[{}]\"\n",
            many.replace('"', "\"\"")
        ));
        assert!(parse_credits(&text).is_ok());
        let e = read(&csv).unwrap_err();
        assert!(e.to_string().contains("too large"), "{e}");
    }
}
