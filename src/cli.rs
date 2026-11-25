//! Command line, input reading and the analysis pipeline.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::aggregate::{self, Dimension, Sort, select_months};
use crate::error::{Error, Result};
use crate::ingest::{Table, read_table};
use crate::month::Month;
use crate::report::{Format, Report, clean};

/// Top costs per SKU, project and label from a GCP billing-export CSV.
#[derive(Debug, Parser)]
#[command(name = "gcp-billing-top", version, about, long_about = None)]
pub struct Args {
    /// Billing-export CSV file, or `-` for standard input.
    pub file: PathBuf,

    /// Second CSV file that holds the previous month.
    #[arg(short, long, value_name = "FILE")]
    pub previous: Option<PathBuf>,

    /// Month to report, `YYYY-MM` (default: the latest month in the file).
    #[arg(long, value_name = "YYYY-MM", value_parser = Month::parse_cli)]
    pub month: Option<Month>,

    /// Month to compare with, `YYYY-MM` (default: the month before).
    #[arg(long, value_name = "YYYY-MM", value_parser = Month::parse_cli)]
    pub previous_month: Option<Month>,

    /// Which top lists to show; repeat for several (default: all the file supports).
    #[arg(short, long, value_enum)]
    pub by: Vec<Dimension>,

    /// Group the label list by the value of this label key.
    #[arg(long, value_name = "KEY")]
    pub label_key: Option<String>,

    /// Number of rows per list, 0 for all.
    #[arg(short = 'n', long, default_value_t = 10, value_name = "N")]
    pub top: usize,

    /// Rank by current cost or by the size of the change.
    #[arg(long, value_enum, default_value = "cost")]
    pub sort: Sort,

    /// Subtract credits from the cost.
    #[arg(long)]
    pub net: bool,

    /// Only count rows in this currency (for example USD).
    #[arg(long, value_name = "CODE")]
    pub currency: Option<String>,

    /// Output format.
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: Format,
}

fn open(path: &Path, stdin: &mut dyn Read) -> Result<Table> {
    if path == Path::new("-") {
        return read_table(stdin, "<stdin>");
    }
    let name = path.display().to_string();
    let file = File::open(path).map_err(|e| Error::Io {
        source_name: name.clone(),
        message: e.to_string(),
    })?;
    read_table(file, &name)
}

fn months(t: &Table) -> BTreeSet<Month> {
    t.rows.iter().map(|r| r.month).collect()
}

fn filter_currency(t: &mut Table, code: &str) -> Result<()> {
    if !t.has_currency {
        return Err(Error::Usage(format!(
            "--currency needs a currency column, but {} has none",
            t.source_name
        )));
    }
    t.rows.retain(|r| r.currency.as_deref() == Some(code));
    if t.rows.is_empty() {
        return Err(Error::Selection(format!(
            "{} has no rows in currency {code}",
            t.source_name
        )));
    }
    Ok(())
}

fn supports(t: &Table, d: Dimension) -> bool {
    match d {
        Dimension::Sku => t.has_sku,
        Dimension::Project => t.has_project,
        Dimension::Label => t.has_labels,
    }
}

fn column_hint(d: Dimension) -> &'static str {
    match d {
        Dimension::Sku => "a SKU column (\"sku.description\" or \"SKU description\")",
        Dimension::Project => "a project column (\"project.id\" or \"Project ID\")",
        Dimension::Label => "a labels column (\"labels\")",
    }
}

fn pick_dimensions(args: &Args, tables: &[&Table]) -> Result<Vec<Dimension>> {
    let all = [Dimension::Sku, Dimension::Project, Dimension::Label];
    let dims: Vec<Dimension> = if args.by.is_empty() {
        all.into_iter()
            .filter(|d| tables.iter().all(|t| supports(t, *d)))
            .collect()
    } else {
        let wanted: BTreeSet<Dimension> = args.by.iter().copied().collect();
        for d in &wanted {
            for t in tables {
                if !supports(t, *d) {
                    return Err(Error::Usage(format!(
                        "--by {} needs {}, but {} has none",
                        d.name(),
                        column_hint(*d),
                        t.source_name
                    )));
                }
            }
        }
        wanted.into_iter().collect()
    };
    if dims.is_empty() {
        return Err(Error::Usage(
            "the file has no SKU, project or labels column to group by".to_string(),
        ));
    }
    if args.label_key.is_some() && !dims.contains(&Dimension::Label) {
        return Err(Error::Usage(
            "--label-key needs the label list (use --by label)".to_string(),
        ));
    }
    Ok(dims)
}

/// Runs the analysis on already-read tables.
pub fn analyze(args: &Args, mut main: Table, mut second: Option<Table>) -> Result<Report> {
    if let Some(code) = args.currency.as_deref().map(str::to_uppercase) {
        filter_currency(&mut main, &code)?;
        if let Some(s) = second.as_mut() {
            filter_currency(s, &code)?;
        }
    }
    if args.net {
        for t in std::iter::once(&main).chain(second.iter()) {
            if !t.has_credits {
                return Err(Error::Usage(format!(
                    "--net needs a credits column, but {} has none",
                    t.source_name
                )));
            }
        }
    }
    let tables: Vec<&Table> = std::iter::once(&main).chain(second.iter()).collect();
    let dims = pick_dimensions(args, &tables)?;

    let main_months = months(&main);
    let second_months = second.as_ref().map(months);
    let sel = select_months(
        &main_months,
        second_months.as_ref(),
        args.month,
        args.previous_month,
    )?;
    let prev_rows = match &second {
        Some(s) => &s.rows,
        None => &main.rows,
    };

    let mut currencies: BTreeSet<&str> = main
        .rows
        .iter()
        .filter(|r| r.month == sel.current)
        .filter_map(|r| r.currency.as_deref())
        .collect();
    if let Some(p) = sel.previous {
        currencies.extend(
            prev_rows
                .iter()
                .filter(|r| r.month == p)
                .filter_map(|r| r.currency.as_deref()),
        );
    }
    if currencies.len() > 1 {
        return Err(Error::Usage(format!(
            "the selected months mix currencies ({}); pick one with --currency",
            currencies
                .into_iter()
                .map(clean)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let currency = currencies.into_iter().next().map(str::to_string);

    let label_key = args.label_key.as_deref();
    let mut sections = Vec::new();
    for dim in dims {
        let cur = aggregate::group(&main.rows, sel.current, dim, label_key, args.net)?;
        let prev = match sel.previous {
            Some(p) => Some(aggregate::group(prev_rows, p, dim, label_key, args.net)?),
            None => None,
        };
        if let Some(prev) = &prev {
            aggregate::ensure_deltas(&cur, prev)?;
        }
        sections.push(aggregate::rank(
            dim,
            &cur,
            prev.as_ref(),
            args.sort,
            args.top,
        ));
    }
    let total_current = aggregate::total(&main.rows, sel.current, args.net)?;
    let total_previous = match sel.previous {
        Some(p) => Some(aggregate::total(prev_rows, p, args.net)?),
        None => None,
    };
    if let Some(p) = total_previous {
        if total_current.checked_sub(p).is_none() {
            return Err(aggregate::too_large("the change of the total"));
        }
    }
    Ok(Report {
        current: sel.current,
        previous: sel.previous,
        net: args.net,
        currency,
        label_key: args.label_key.clone(),
        total_current,
        total_previous,
        sections,
    })
}

/// Reads the input files named in `args`, analyses them and renders the result.
pub fn run(args: &Args, stdin: &mut dyn Read) -> Result<String> {
    if args.file == Path::new("-") && args.previous.as_deref() == Some(Path::new("-")) {
        return Err(Error::Usage(
            "standard input can only be used for one file".to_string(),
        ));
    }
    let main = open(&args.file, stdin)?;
    let second = match &args.previous {
        Some(p) => Some(open(p, stdin)?),
        None => None,
    };
    Ok(analyze(args, main, second)?.render(args.format))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(extra: &[&str]) -> Args {
        let mut v = vec!["gcp-billing-top", "x.csv"];
        v.extend_from_slice(extra);
        Args::try_parse_from(v).unwrap()
    }

    fn table(csv: &str) -> Table {
        read_table(csv.as_bytes(), "t.csv").unwrap()
    }

    const TWO: &str = "project.id,sku.description,usage_start_time,labels,cost,credits,currency\n\
        p1,A,2026-09-01,env=prod,10,-1,USD\n\
        p1,A,2026-10-01,env=prod,30,-3,USD\n\
        p2,B,2026-10-02,env=dev,5,0,USD\n";

    #[test]
    fn defaults() {
        let a = args(&[]);
        assert_eq!(a.top, 10);
        assert_eq!(a.sort, Sort::Cost);
        assert_eq!(a.format, Format::Table);
        assert!(!a.net && a.by.is_empty() && a.previous.is_none());
    }

    #[test]
    fn parses_all_options() {
        let a = args(&[
            "-p",
            "old.csv",
            "--month",
            "2026-10",
            "--previous-month",
            "2026-08",
            "--by",
            "sku",
            "-b",
            "label",
            "--label-key",
            "env",
            "-n",
            "3",
            "--sort",
            "delta",
            "--net",
            "--currency",
            "usd",
            "-f",
            "json",
        ]);
        assert_eq!(a.previous, Some(PathBuf::from("old.csv")));
        assert_eq!(a.month, Month::new(2026, 10));
        assert_eq!(a.previous_month, Month::new(2026, 8));
        assert_eq!(a.by, vec![Dimension::Sku, Dimension::Label]);
        assert_eq!(a.label_key.as_deref(), Some("env"));
        assert_eq!(a.top, 3);
        assert_eq!(a.sort, Sort::Delta);
        assert!(a.net);
        assert_eq!(a.format, Format::Json);
    }

    #[test]
    fn rejects_bad_month_option() {
        assert!(Args::try_parse_from(["gcp-billing-top", "x.csv", "--month", "oct"]).is_err());
    }

    #[test]
    fn rejects_unknown_dimension_and_missing_file() {
        assert!(Args::try_parse_from(["gcp-billing-top", "x.csv", "--by", "service"]).is_err());
        assert!(Args::try_parse_from(["gcp-billing-top"]).is_err());
    }

    #[test]
    fn analyze_two_months_in_one_table() {
        let r = analyze(&args(&[]), table(TWO), None).unwrap();
        assert_eq!(r.current, Month::new(2026, 10).unwrap());
        assert_eq!(r.previous, Month::new(2026, 9));
        assert_eq!(r.total_current.to_plain(), "35.00");
        assert_eq!(r.total_previous.unwrap().to_plain(), "10.00");
        assert_eq!(r.sections.len(), 3);
        assert_eq!(r.currency.as_deref(), Some("USD"));
    }

    #[test]
    fn analyze_net_subtracts_credits() {
        let r = analyze(&args(&["--net"]), table(TWO), None).unwrap();
        assert_eq!(r.total_current.to_plain(), "32.00");
        assert_eq!(r.total_previous.unwrap().to_plain(), "9.00");
        assert!(r.net);
    }

    #[test]
    fn analyze_with_second_table() {
        let main = table("sku.description,usage_start_time,cost\nA,2026-10-01,30\n");
        let old = table("sku.description,usage_start_time,cost\nA,2026-09-01,10\n");
        let r = analyze(&args(&[]), main, Some(old)).unwrap();
        assert_eq!(r.total_delta().unwrap().to_plain(), "20.00");
        assert_eq!(r.sections.len(), 1);
    }

    #[test]
    fn default_dimensions_follow_the_columns() {
        let r = analyze(
            &args(&[]),
            table("sku.description,usage_start_time,cost\nA,2026-10-01,1\n"),
            None,
        )
        .unwrap();
        assert_eq!(r.sections.len(), 1);
        assert_eq!(r.sections[0].dimension, Dimension::Sku);
    }

    #[test]
    fn default_dimensions_need_the_column_in_both_files() {
        let main = table("sku.description,project.id,usage_start_time,cost\nA,p,2026-10-01,1\n");
        let old = table("sku.description,usage_start_time,cost\nA,2026-09-01,1\n");
        let r = analyze(&args(&[]), main, Some(old)).unwrap();
        assert_eq!(r.sections.len(), 1);
    }

    #[test]
    fn requested_dimension_without_column_is_an_error() {
        let e = analyze(
            &args(&["--by", "project"]),
            table("sku.description,usage_start_time,cost\nA,2026-10-01,1\n"),
            None,
        )
        .unwrap_err();
        assert!(matches!(e, Error::Usage(_)));
        assert!(e.to_string().contains("--by project"), "{e}");
    }

    #[test]
    fn no_groupable_column_is_an_error() {
        let e = analyze(
            &args(&[]),
            table("usage_start_time,cost\n2026-10-01,1\n"),
            None,
        )
        .unwrap_err();
        assert!(e.to_string().contains("no SKU, project or labels"), "{e}");
    }

    #[test]
    fn duplicate_by_is_deduplicated_and_ordered() {
        let r = analyze(
            &args(&["--by", "project", "--by", "sku", "--by", "project"]),
            table(TWO),
            None,
        )
        .unwrap();
        let dims: Vec<_> = r.sections.iter().map(|s| s.dimension).collect();
        assert_eq!(dims, vec![Dimension::Sku, Dimension::Project]);
    }

    #[test]
    fn label_key_requires_label_list() {
        let e = analyze(
            &args(&["--by", "sku", "--label-key", "env"]),
            table(TWO),
            None,
        )
        .unwrap_err();
        assert!(e.to_string().contains("--label-key"), "{e}");
    }

    #[test]
    fn net_requires_credits_column() {
        let e = analyze(
            &args(&["--net"]),
            table("sku.description,usage_start_time,cost\nA,2026-10-01,1\n"),
            None,
        )
        .unwrap_err();
        assert!(
            e.to_string().contains("--net needs a credits column"),
            "{e}"
        );
    }

    #[test]
    fn mixed_currencies_are_refused() {
        let t = table(
            "sku.description,usage_start_time,cost,currency\nA,2026-10-01,1,USD\nB,2026-10-01,2,EUR\n",
        );
        let e = analyze(&args(&[]), t, None).unwrap_err();
        assert!(e.to_string().contains("EUR, USD"), "{e}");
    }

    #[test]
    fn currency_option_filters_rows() {
        let t = table(
            "sku.description,usage_start_time,cost,currency\nA,2026-10-01,1,USD\nB,2026-10-01,2,EUR\n",
        );
        let r = analyze(&args(&["--currency", "eur"]), t, None).unwrap();
        assert_eq!(r.total_current.to_plain(), "2.00");
        assert_eq!(r.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn currency_option_without_column_or_rows_is_an_error() {
        let e = analyze(
            &args(&["--currency", "USD"]),
            table("sku.description,usage_start_time,cost\nA,2026-10-01,1\n"),
            None,
        )
        .unwrap_err();
        assert!(matches!(e, Error::Usage(_)));
        let t = table("sku.description,usage_start_time,cost,currency\nA,2026-10-01,1,USD\n");
        let e = analyze(&args(&["--currency", "JPY"]), t, None).unwrap_err();
        assert!(matches!(e, Error::Selection(_)));
    }

    #[test]
    fn run_reads_stdin() {
        let mut input = TWO.as_bytes();
        let mut a = args(&[]);
        a.file = PathBuf::from("-");
        let out = run(&a, &mut input).unwrap();
        assert!(out.contains("2026-10 vs 2026-09"), "{out}");
    }

    #[test]
    fn run_refuses_stdin_twice() {
        let mut a = args(&["-p", "-"]);
        a.file = PathBuf::from("-");
        let e = run(&a, &mut std::io::empty()).unwrap_err();
        assert!(e.to_string().contains("only be used for one file"));
    }

    #[test]
    fn run_reports_missing_file() {
        let mut a = args(&[]);
        a.file = PathBuf::from("/nonexistent/definitely-missing.csv");
        let e = run(&a, &mut std::io::empty()).unwrap_err();
        assert!(matches!(e, Error::Io { .. }));
    }
}
