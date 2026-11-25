//! End-to-end tests: the real binary against the fixtures.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .display()
        .to_string()
}

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_with_stdin(args: &[&str], stdin: Option<&str>) -> Out {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gcp-billing-top"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary starts");
    {
        let mut pipe = child.stdin.take().expect("stdin pipe");
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).expect("write stdin");
        }
    }
    let out = child.wait_with_output().expect("binary ends");
    Out {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn run(args: &[&str]) -> Out {
    run_with_stdin(args, None)
}

fn ok(args: &[&str]) -> String {
    let o = run(args);
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(o.stderr.is_empty(), "stderr: {}", o.stderr);
    o.stdout
}

fn fail(args: &[&str]) -> String {
    let o = run(args);
    assert_eq!(o.code, 1, "stdout: {}", o.stdout);
    assert!(
        o.stdout.is_empty(),
        "stdout must stay empty on errors: {}",
        o.stdout
    );
    assert!(o.stderr.starts_with("error: "), "{}", o.stderr);
    o.stderr
}

const TWO_MONTHS_CSV: &str = "\
dimension,key,current,previous,delta,delta_pct
total,,300.00,250.00,50.00,20.0
sku,N2 Instance Core running in Americas,200.00,150.00,50.00,33.3
sku,Standard Storage US Multi-region,60.00,50.00,10.00,20.0
sku,BigQuery Analysis,25.00,0.00,25.00,
sku,Network Egress,10.00,30.00,-20.00,-66.7
sku,Support,5.00,0.00,5.00,
project,proj-a,245.00,190.00,55.00,28.9
project,proj-b,50.00,60.00,-10.00,-16.7
project,(no project),5.00,0.00,5.00,
label,env=prod,245.00,190.00,55.00,28.9
label,team=web,200.00,150.00,50.00,33.3
label,team=data,25.00,0.00,25.00,
label,(no labels),15.00,50.00,-35.00,-70.0
label,env=dev,15.00,10.00,5.00,50.0
";

// ---- one month -----------------------------------------------------------

#[test]
fn one_month_file_reports_total_without_delta() {
    let t = ok(&[&fixture("one_month.csv")]);
    assert!(
        t.starts_with("gcp-billing-top: 2026-10 (gross, USD)\n"),
        "{t}"
    );
    assert!(t.contains("Total: 300.00\n"), "{t}");
    assert!(!t.contains("Delta"), "{t}");
    assert!(t.contains("No previous month found"), "{t}");
}

#[test]
fn one_month_file_ranks_skus_by_cost() {
    let t = ok(&[&fixture("one_month.csv"), "--by", "sku"]);
    let a = t.find("N2 Instance Core").unwrap();
    let b = t.find("Standard Storage").unwrap();
    let c = t.find("BigQuery Analysis").unwrap();
    let d = t.find("Network Egress").unwrap();
    let e = t.find("Support").unwrap();
    assert!(a < b && b < c && c < d && d < e, "{t}");
}

#[test]
fn one_month_json_has_null_previous() {
    let t = ok(&[&fixture("one_month.csv"), "-f", "json"]);
    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert_eq!(v["current_month"], "2026-10");
    assert!(v["previous_month"].is_null());
    assert_eq!(v["total"]["current"], 300.0);
    assert!(v["total"]["delta"].is_null());
}

#[test]
fn single_month_file_asked_for_its_only_month() {
    let t = ok(&[&fixture("single_old_month.csv"), "--month", "2026-09"]);
    assert!(t.contains("Total: 250.00"), "{t}");
}

// ---- two months ----------------------------------------------------------

#[test]
fn two_months_in_one_file_show_the_delta() {
    let t = ok(&[&fixture("two_months.csv")]);
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
fn two_months_csv_matches_golden_output() {
    assert_eq!(
        ok(&[&fixture("two_months.csv"), "-f", "csv"]),
        TWO_MONTHS_CSV
    );
}

#[test]
fn separate_previous_file_gives_the_same_report_as_one_file() {
    let one = ok(&[&fixture("two_months.csv"), "-f", "csv"]);
    let two = ok(&[
        &fixture("one_month.csv"),
        "-p",
        &fixture("previous_month.csv"),
        "-f",
        "csv",
    ]);
    assert_eq!(one, two);
}

#[test]
fn table_rows_for_new_and_dropped_costs() {
    let t = ok(&[&fixture("two_months.csv"), "--by", "sku"]);
    let new_row = t.lines().find(|l| l.contains("BigQuery Analysis")).unwrap();
    assert!(new_row.ends_with("new"), "{new_row}");
    let down_row = t.lines().find(|l| l.contains("Network Egress")).unwrap();
    assert!(
        down_row.contains("-20.00") && down_row.ends_with("-66.7%"),
        "{down_row}"
    );
}

#[test]
fn project_list_includes_the_no_project_bucket() {
    let t = ok(&[&fixture("two_months.csv"), "--by", "project"]);
    assert!(t.contains("(no project)"), "{t}");
    assert!(t.contains("proj-a") && t.contains("+28.9%"), "{t}");
}

#[test]
fn label_list_counts_each_label_pair() {
    let t = ok(&[&fixture("two_months.csv"), "--by", "label"]);
    assert!(
        t.contains("env=prod") && t.contains("team=web") && t.contains("team=data"),
        "{t}"
    );
    assert!(t.contains("counts toward each of its labels"), "{t}");
}

#[test]
fn label_key_partitions_the_total() {
    let t = ok(&[
        &fixture("two_months.csv"),
        "--by",
        "label",
        "--label-key",
        "env",
        "-f",
        "csv",
    ]);
    let expected = "\
dimension,key,current,previous,delta,delta_pct
total,,300.00,250.00,50.00,20.0
label,prod,245.00,190.00,55.00,28.9
label,(unlabeled),40.00,50.00,-10.00,-20.0
label,dev,15.00,10.00,5.00,50.0
";
    assert_eq!(t, expected);
}

#[test]
fn explicit_months_pick_the_requested_pair() {
    let t = ok(&[&fixture("two_months.csv"), "--month", "2026-09"]);
    assert!(
        t.starts_with("gcp-billing-top: 2026-09 (gross, USD)"),
        "{t}"
    );
    assert!(t.contains("Total: 250.00"), "{t}");
}

#[test]
fn previous_month_option_selects_a_month_in_the_second_file() {
    let t = ok(&[
        &fixture("one_month.csv"),
        "-p",
        &fixture("two_months.csv"),
        "--previous-month",
        "2026-09",
        "-f",
        "json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert_eq!(v["previous_month"], "2026-09");
    assert_eq!(v["total"]["previous"], 250.0);
}

#[test]
fn second_file_without_an_earlier_month_is_refused() {
    let e = fail(&[
        &fixture("single_old_month.csv"),
        "-p",
        &fixture("one_month.csv"),
    ]);
    assert!(e.contains("no month before"), "{e}");
}

#[test]
fn unknown_month_lists_what_is_available() {
    let e = fail(&[&fixture("two_months.csv"), "--month", "2025-01"]);
    assert!(
        e.contains("2025-01") && e.contains("2026-09, 2026-10"),
        "{e}"
    );
}

#[test]
fn december_to_january_wraps_the_year() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let path = dir.join("wrap.csv");
    std::fs::write(
        &path,
        "sku.description,usage_start_time,cost\nA,2025-12-31T23:00:00Z,10\nA,2026-01-01T01:00:00Z,12\n",
    )
    .unwrap();
    let t = ok(&[path.to_str().unwrap()]);
    assert!(t.contains("2026-01 vs 2025-12"), "{t}");
}

// ---- ranking and limits --------------------------------------------------

#[test]
fn top_limits_each_list() {
    let t = ok(&[&fixture("two_months.csv"), "--by", "sku", "-n", "2"]);
    assert!(t.contains("Top 2 of 5 by SKU"), "{t}");
    assert!(!t.contains("Support"), "{t}");
}

#[test]
fn default_top_is_ten_and_zero_means_all() {
    let t = ok(&[&fixture("many_skus.csv")]);
    assert!(t.contains("Top 10 of 15 by SKU"), "{t}");
    let t = ok(&[&fixture("many_skus.csv"), "-n", "0"]);
    assert!(t.contains("Top 15 by SKU"), "{t}");
}

#[test]
fn sort_by_delta_includes_vanished_skus() {
    let t = ok(&[
        &fixture("two_months.csv"),
        "--by",
        "sku",
        "--sort",
        "delta",
        "-n",
        "0",
        "-f",
        "csv",
    ]);
    assert!(
        t.contains("sku,Old Legacy SKU,0.00,20.00,-20.00,-100.0"),
        "{t}"
    );
    let keys: Vec<&str> = t
        .lines()
        .skip(2)
        .map(|l| l.split(',').nth(1).unwrap())
        .collect();
    assert_eq!(keys[0], "N2 Instance Core running in Americas");
    assert_eq!(keys[1], "BigQuery Analysis");
    let default = ok(&[
        &fixture("two_months.csv"),
        "--by",
        "sku",
        "-n",
        "0",
        "-f",
        "csv",
    ]);
    assert!(!default.contains("Old Legacy SKU"), "{default}");
}

#[test]
fn by_selects_only_the_requested_lists() {
    let t = ok(&[&fixture("two_months.csv"), "--by", "project"]);
    assert!(
        t.contains("by project") && !t.contains("by SKU") && !t.contains("by label"),
        "{t}"
    );
}

// ---- credits, currency ---------------------------------------------------

#[test]
fn net_subtracts_credits_from_both_months() {
    let t = ok(&[&fixture("two_months.csv"), "--net"]);
    assert!(t.contains("(net of credits, USD)"), "{t}");
    assert!(
        t.contains("Total: 280.00  (previous 245.00, +35.00, +14.3%)"),
        "{t}"
    );
}

#[test]
fn json_credits_are_summed_for_net() {
    let gross = ok(&[&fixture("credits_json.csv")]);
    assert!(gross.contains("Total: 20.00"), "{gross}");
    let net = ok(&[&fixture("credits_json.csv"), "--net"]);
    assert!(net.contains("Total: 16.25"), "{net}");
}

#[test]
fn net_without_credits_column_is_refused() {
    let e = fail(&[&fixture("many_skus.csv"), "--net"]);
    assert!(e.contains("--net needs a credits column"), "{e}");
}

#[test]
fn mixed_currencies_are_refused_until_one_is_chosen() {
    let e = fail(&[&fixture("multi_currency.csv")]);
    assert!(e.contains("EUR, USD") && e.contains("--currency"), "{e}");
    let t = ok(&[&fixture("multi_currency.csv"), "--currency", "usd"]);
    assert!(t.contains("Total: 11.00") && t.contains("USD"), "{t}");
    let t = ok(&[&fixture("multi_currency.csv"), "--currency", "EUR"]);
    assert!(t.contains("Total: 9.00"), "{t}");
}

// ---- column conventions --------------------------------------------------

#[test]
fn console_style_headers_and_month_names_work() {
    let t = ok(&[&fixture("console_style.csv")]);
    assert!(t.contains("2026-10 vs 2026-09"), "{t}");
    assert!(t.contains("Total: 22.25  (previous 10.50"), "{t}");
    assert!(t.contains("web-1") && t.contains("data-1"), "{t}");
    assert!(!t.contains("by label"), "{t}");
}

#[test]
fn invoice_month_is_used_when_there_is_no_usage_date() {
    let t = ok(&[&fixture("invoice_month.csv")]);
    assert!(t.contains("2026-10 vs 2026-09"), "{t}");
    assert!(t.contains("Total: 15.00  (previous 10.00"), "{t}");
}

#[test]
fn plain_label_syntax_works() {
    let t = ok(&[&fixture("plain_labels.csv"), "-f", "csv"]);
    assert!(t.contains("label,env=prod,10.00,,,"), "{t}");
    assert!(t.contains("label,team=web,10.00,,,"), "{t}");
    assert!(t.contains("label,env=dev,4.00,,,"), "{t}");
    assert!(t.contains("label,(no labels),1.00,,,"), "{t}");
}

#[test]
fn byte_order_mark_is_ignored() {
    assert_eq!(
        ok(&[&fixture("bom.csv"), "-f", "csv"]),
        ok(&[&fixture("one_month.csv"), "-f", "csv"])
    );
}

#[test]
fn tiny_and_scientific_amounts_are_summed_exactly() {
    let t = ok(&[&fixture("exotic_amounts.csv"), "--by", "sku", "-f", "csv"]);
    assert!(t.contains("total,,1234567.892746,,,"), "{t}");
    assert!(t.contains("sku,Tiny,0.000246,,,"), "{t}");
    assert!(t.contains("sku,Sci,0.0015,,,"), "{t}");
    let table = ok(&[&fixture("exotic_amounts.csv"), "--by", "sku"]);
    assert!(table.contains("Total: 1,234,567.89"), "{table}");
}

// ---- formats -------------------------------------------------------------

#[test]
fn json_output_is_complete() {
    let t = ok(&[&fixture("two_months.csv"), "-f", "json"]);
    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert_eq!(v["previous_month"], "2026-09");
    assert_eq!(v["net"], false);
    assert_eq!(v["currency"], "USD");
    assert_eq!(v["total"]["delta_pct"], 20.0);
    let dims: Vec<&str> = v["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["dimension"].as_str().unwrap())
        .collect();
    assert_eq!(dims, ["sku", "project", "label"]);
    assert_eq!(
        v["sections"][0]["top"][0]["key"],
        "N2 Instance Core running in Americas"
    );
}

#[test]
fn output_is_deterministic() {
    let a = ok(&[&fixture("two_months.csv")]);
    let b = ok(&[&fixture("two_months.csv")]);
    assert_eq!(a, b);
}

// ---- stdin ---------------------------------------------------------------

#[test]
fn dash_reads_standard_input() {
    let text = std::fs::read_to_string(fixture("two_months.csv")).unwrap();
    let o = run_with_stdin(&["-", "-f", "csv"], Some(&text));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(o.stdout, TWO_MONTHS_CSV);
}

#[test]
fn stdin_can_hold_the_previous_month() {
    let prev = std::fs::read_to_string(fixture("previous_month.csv")).unwrap();
    let o = run_with_stdin(
        &[&fixture("one_month.csv"), "-p", "-", "-f", "csv"],
        Some(&prev),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(o.stdout, TWO_MONTHS_CSV);
}

// ---- errors --------------------------------------------------------------

#[test]
fn missing_file_is_an_error() {
    let e = fail(&["/nonexistent/none.csv"]);
    assert!(e.contains("cannot read /nonexistent/none.csv"), "{e}");
}

#[cfg(unix)]
#[test]
fn full_standard_error_gives_the_error_code_and_no_panic() {
    let Ok(full) = std::fs::OpenOptions::new().write(true).open("/dev/full") else {
        return;
    };
    let out = Command::new(env!("CARGO_BIN_EXE_gcp-billing-top"))
        .arg("/nonexistent/none.csv")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(full))
        .output()
        .expect("binary runs");
    assert_eq!(out.status.code(), Some(1), "{:?}", out.status);
}

#[test]
fn bad_cost_names_file_and_line() {
    let path = fixture("bad_cost.csv");
    let e = fail(&[&path]);
    assert!(
        e.contains("bad_cost.csv, line 3") && e.contains("\"abc\""),
        "{e}"
    );
}

#[test]
fn bad_labels_name_the_line() {
    let e = fail(&[&fixture("bad_labels.csv")]);
    assert!(
        e.contains("bad_labels.csv, line 2") && e.contains("labels"),
        "{e}"
    );
}

#[test]
fn missing_columns_are_explained() {
    let e = fail(&[&fixture("no_cost_column.csv")]);
    assert!(e.contains("no cost column"), "{e}");
    let e = fail(&[&fixture("no_month_column.csv")]);
    assert!(e.contains("no month column"), "{e}");
}

#[test]
fn empty_and_header_only_files_are_errors() {
    assert!(fail(&[&fixture("empty.csv")]).contains("empty"));
    assert!(fail(&[&fixture("header_only.csv")]).contains("no data rows"));
}

#[test]
fn bad_second_file_is_reported_by_name() {
    let e = fail(&[&fixture("one_month.csv"), "-p", &fixture("bad_cost.csv")]);
    assert!(e.contains("bad_cost.csv"), "{e}");
}

#[test]
fn usage_errors_exit_with_code_two() {
    let o = run(&[&fixture("one_month.csv"), "--month", "october"]);
    assert_eq!(o.code, 2);
    assert!(o.stdout.is_empty());
    let o = run(&[]);
    assert_eq!(o.code, 2);
    let o = run(&[&fixture("one_month.csv"), "--by", "service"]);
    assert_eq!(o.code, 2);
}

#[test]
fn requesting_a_missing_column_list_is_an_error() {
    let e = fail(&[&fixture("many_skus.csv"), "--by", "project"]);
    assert!(e.contains("--by project"), "{e}");
}

// ---- meta ----------------------------------------------------------------

#[test]
fn help_lists_every_option() {
    let t = ok(&["--help"]);
    for flag in [
        "--previous",
        "--month",
        "--previous-month",
        "--by",
        "--label-key",
        "--top",
        "--sort",
        "--net",
        "--currency",
        "--format",
    ] {
        assert!(t.contains(flag), "{flag} missing from --help");
    }
}

#[test]
fn version_matches_cargo_toml() {
    let t = ok(&["--version"]);
    assert_eq!(
        t.trim(),
        format!("gcp-billing-top {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn hostile_names_cannot_forge_rows_or_terminal_sequences() {
    let input = "project.id,sku.description,usage_start_time,cost\n\
                 \"p\nfake\",\"a\x1b[31mred\tx\",2026-10-03T00:00:00Z,5.00\n\
                 p2,=SUM(A1),2026-10-04T00:00:00Z,1.00\n";
    let o = run_with_stdin(&["-", "--by", "sku", "--by", "project"], Some(input));
    assert_eq!(o.code, 0, "stderr: {}", o.stderr);
    assert!(!o.stdout.contains('\x1b'), "{:?}", o.stdout);
    assert!(!o.stdout.contains('\t'), "{:?}", o.stdout);
    assert!(o.stdout.contains("a?[31mred?x"), "{:?}", o.stdout);
    assert!(o.stdout.contains("p?fake"), "{:?}", o.stdout);
    let csv = run_with_stdin(&["-", "--by", "sku", "--format", "csv"], Some(input));
    assert_eq!(csv.code, 0, "stderr: {}", csv.stderr);
    assert!(csv.stdout.contains("sku,'=SUM(A1),"), "{:?}", csv.stdout);
}

// ---- review gcp-eind-bc4169c: B-1 to B-4 ----------------------------------

fn stdin_run(csv: &str, extra: &[&str]) -> Out {
    let mut args = vec!["-"];
    args.extend_from_slice(extra);
    run_with_stdin(&args, Some(csv))
}

fn stdin_error(csv: &str) -> String {
    let o = stdin_run(csv, &[]);
    assert_eq!(o.code, 1, "stdout: {}", o.stdout);
    assert!(o.stdout.is_empty());
    o.stderr
}

const HEADER: &str = "sku.description,usage_start_time,cost";

/// A file whose line `bad` (1-based, header is line 1) holds a bad amount.
fn file_with_bad_line(bad: usize, eol: &str) -> String {
    let mut text = format!("{HEADER}{eol}");
    for line in 2..=bad.max(2) + 1 {
        let cost = if line == bad { "zz" } else { "1" };
        text.push_str(&format!("S{line},2026-10-01,{cost}{eol}"));
    }
    text
}

#[test]
fn crlf_files_give_the_same_line_number_as_lf_files() {
    for bad in [2, 3, 4, 10, 50] {
        let want = format!("<stdin>, line {bad}:");
        let lf = stdin_error(&file_with_bad_line(bad, "\n"));
        let crlf = stdin_error(&file_with_bad_line(bad, "\r\n"));
        assert!(lf.contains(&want), "LF {bad}: {lf}");
        assert!(crlf.contains(&want), "CRLF {bad}: {crlf}");
    }
}

#[test]
fn crlf_structure_error_names_line_three() {
    let e = stdin_error(&format!("{HEADER}\r\nA,2026-10-01,1\r\nB,2026-10-02\r\n"));
    assert!(e.contains("<stdin>, line 3:"), "{e}");
}

#[test]
fn byte_order_mark_with_crlf_keeps_the_line_number() {
    let e = stdin_error(&format!("\u{feff}{}", file_with_bad_line(3, "\r\n")));
    assert!(e.contains("<stdin>, line 3:"), "{e}");
}

#[test]
fn a_field_with_a_line_break_inside_quotes_counts_its_physical_lines() {
    let e = stdin_error(&format!(
        "{HEADER}\r\n\"two\r\nlines\",2026-10-01,1\r\nB,2026-10-02,zz\r\n"
    ));
    assert!(e.contains("<stdin>, line 4:"), "{e}");
}

fn rows_of(cost: &str, n: usize) -> String {
    let mut text = format!("{HEADER}\n");
    for i in 0..n {
        text.push_str(&format!("S{i},2026-10-01,{cost}\n"));
    }
    text
}

#[test]
fn twelve_rows_of_nine_hundred_billion_are_refused_not_clamped() {
    let o = stdin_run(&rows_of("900000000000", 12), &[]);
    assert_eq!(o.code, 1, "stdout: {}", o.stdout);
    assert!(o.stdout.is_empty(), "{}", o.stdout);
    assert!(o.stderr.contains("too large"), "{}", o.stderr);
    assert!(
        !o.stderr.contains("9223372036854.775807 \n"),
        "{}",
        o.stderr
    );
}

#[test]
fn eleven_rows_of_nine_hundred_billion_are_refused_too() {
    // 9.9e18 micro-units is above i64::MAX (9.22e18).
    let o = stdin_run(&rows_of("900000000000", 11), &[]);
    assert_eq!(o.code, 1);
    assert!(o.stderr.contains("too large"), "{}", o.stderr);
}

#[test]
fn ten_rows_of_nine_hundred_billion_still_add_up() {
    let o = stdin_run(&rows_of("900000000000", 10), &["-f", "csv"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.contains("total,,9000000000000.00"), "{}", o.stdout);
}

#[test]
fn a_partial_sum_that_overflows_is_refused_even_if_the_end_is_in_range() {
    let mut text = rows_of("900000000000", 12);
    for i in 0..3 {
        text.push_str(&format!("M{i},2026-10-01,-900000000000\n"));
    }
    let o = stdin_run(&text, &[]);
    assert_eq!(o.code, 1, "stdout: {}", o.stdout);
    assert!(o.stdout.is_empty());
    assert!(o.stderr.contains("too large"), "{}", o.stderr);
}

#[test]
fn only_the_exact_cost_columns_count_as_the_cost_column() {
    for header in [
        "cost_at_list,Cost (EUR),usage_start_time",
        "cost center,Cost (EUR),usage_start_time",
        "Unrounded cost (EUR),Cost (EUR),usage_start_time",
        "Cost (EUR),cost_at_list,usage_start_time",
    ] {
        let first = header.starts_with("Cost (EUR)");
        let row = if first {
            "5,999,2026-10-01\n"
        } else {
            "999,5,2026-10-01\n"
        };
        let o = stdin_run(
            &format!("sku.description,{header}\nA,{row}"),
            &["-f", "csv"],
        );
        assert_eq!(o.code, 0, "{header}: {}", o.stderr);
        assert!(o.stdout.contains("total,,5.00"), "{header}: {}", o.stdout);
    }
}

#[test]
fn a_column_that_only_starts_with_cost_is_no_cost_column() {
    for header in ["cost_at_list", "cost center", "cost type", "costly"] {
        let o = stdin_run(&format!("{header},usage_start_time\n77,2026-10-01\n"), &[]);
        assert_eq!(o.code, 1, "{header}: {}", o.stdout);
        assert!(o.stdout.is_empty());
        assert!(
            o.stderr.contains("no cost column"),
            "{header}: {}",
            o.stderr
        );
    }
}

#[test]
fn two_cost_columns_exit_with_code_two() {
    for header in [
        "sku.description,usage_start_time,cost,Cost ($)",
        "Cost (EUR),usage_start_time,Cost (USD)",
    ] {
        let o = stdin_run(&format!("{header}\nA,2026-10-01,1,100\n"), &[]);
        assert_eq!(o.code, 2, "{header}: {}", o.stderr);
        assert!(o.stdout.is_empty());
        assert!(
            o.stderr.contains("two columns are the cost column"),
            "{}",
            o.stderr
        );
    }
}

#[test]
fn a_repeated_key_in_credits_json_exits_with_code_two() {
    let csv =
        "usage_start_time,cost,credits\n2026-10-01,5,\"[{\"\"amount\"\":-1,\"\"amount\"\":-4}]\"\n";
    for extra in [&["--net"][..], &[][..]] {
        let o = stdin_run(csv, extra);
        assert_eq!(
            o.code, 2,
            "{extra:?}: stdout {} stderr {}",
            o.stdout, o.stderr
        );
        assert!(o.stdout.is_empty());
        assert!(
            o.stderr.contains("line 2") && o.stderr.contains("\"amount\""),
            "{}",
            o.stderr
        );
    }
}

#[test]
fn a_repeated_label_key_exits_with_code_two_in_every_format() {
    for labels in [
        "\"{\"\"env\"\":\"\"prod\"\",\"\"env\"\":\"\"dev\"\"}\"",
        "\"[{\"\"key\"\":\"\"env\"\",\"\"value\"\":\"\"prod\"\"},{\"\"key\"\":\"\"env\"\",\"\"value\"\":\"\"dev\"\"}]\"",
        "env=prod;env=dev",
    ] {
        let csv = format!("usage_start_time,cost,labels\n2026-10-01,5,{labels}\n");
        for extra in [
            &["--by", "label", "--label-key", "env"][..],
            &["--by", "label"][..],
        ] {
            let o = stdin_run(&csv, extra);
            assert_eq!(
                o.code, 2,
                "{labels} {extra:?}: stdout {} stderr {}",
                o.stdout, o.stderr
            );
            assert!(o.stdout.is_empty());
            assert!(o.stderr.contains("\"env\""), "{}", o.stderr);
        }
    }
}

#[test]
fn a_currency_code_with_a_line_break_and_escape_stays_off_standard_error_raw() {
    let csv = "sku.description,usage_start_time,cost,currency\n\
               A,2026-10-01,1,USD\n\
               B,2026-10-02,1,\"EUR\n\x1b[31merror: forged\"\n";
    let o = stdin_run(csv, &[]);
    assert_eq!(o.code, 1, "stdout: {}", o.stdout);
    assert!(o.stdout.is_empty());
    assert!(!o.stderr.contains('\x1b'), "{:?}", o.stderr);
    assert_eq!(o.stderr.lines().count(), 1, "a forged line: {:?}", o.stderr);
    assert!(o.stderr.contains("mix currencies"), "{:?}", o.stderr);
    assert!(o.stderr.contains("EUR?"), "{:?}", o.stderr);
}

/// Two columns of one kind: exit 2, nothing on stdout, nothing picked by position.
fn assert_two_columns_refused(csv: &str, extra: &[&str], words: &[&str]) {
    let o = stdin_run(csv, extra);
    assert_eq!(
        o.code, 2,
        "{csv:?}: stdout {} stderr {}",
        o.stdout, o.stderr
    );
    assert!(o.stdout.is_empty(), "{}", o.stdout);
    for w in words {
        assert!(o.stderr.contains(w), "{w}: {}", o.stderr);
    }
}

#[test]
fn two_month_columns_exit_with_code_two() {
    assert_two_columns_refused(
        "cost,usage_start_time,usage_start_time,sku.description\n5,2026-10-01,2026-09-01,A\n",
        &[],
        &["two columns are the month column"],
    );
}

#[test]
fn usage_start_time_with_invoice_month_exits_with_code_two_even_for_the_same_month() {
    for row in [
        "5,2026-10-01T00:00:00Z,202610,A",
        "5,2026-09-30T23:00:00Z,202610,A",
    ] {
        assert_two_columns_refused(
            &format!("cost,usage_start_time,invoice.month,sku\n{row}\n"),
            &[],
            &["two columns are the month column"],
        );
    }
}

#[test]
fn two_credits_columns_exit_with_code_two() {
    let csv = "cost,usage_start_time,sku,credits,credits\n5,2026-10-01,A,-1,-2\n";
    assert_two_columns_refused(csv, &["--net"], &["two columns are the credits column"]);
    assert_two_columns_refused(csv, &[], &["two columns are the credits column"]);
}

#[test]
fn two_labels_columns_exit_with_code_two() {
    assert_two_columns_refused(
        "cost,usage_start_time,labels,labels\n5,2026-10-01,env=a,env=b\n",
        &["--by", "label"],
        &["two columns are the labels column"],
    );
}

#[test]
fn two_columns_that_normalize_to_the_same_sku_exit_with_code_two() {
    assert_two_columns_refused(
        "cost,usage_start_time,sku.description,SKU_description\n5,2026-10-01,A,B\n",
        &[],
        &["two columns are the SKU column"],
    );
}

#[test]
fn two_currency_columns_exit_with_code_two() {
    assert_two_columns_refused(
        "cost,usage_start_time,sku,currency,Currency\n5,2026-10-01,A,USD,EUR\n",
        &[],
        &["two columns are the currency column"],
    );
}

#[test]
fn two_project_id_columns_exit_with_code_two() {
    assert_two_columns_refused(
        "project.id,Project ID,usage_start_time,cost\nfirst,second,2026-10-01,1\n",
        &["--by", "project"],
        &["two columns are the project column"],
    );
}

#[test]
fn two_project_name_columns_exit_with_code_two() {
    assert_two_columns_refused(
        "project.name,Project name,usage_start_time,cost\nfirst,second,2026-10-01,1\n",
        &["--by", "project"],
        &["two columns are the project column"],
    );
}

#[test]
fn one_blank_line_counts_in_the_line_number() {
    for eol in ["\n", "\r\n"] {
        let text =
            format!("cost,usage_start_time,sku{eol}1,2026-10-01,A{eol}{eol}x,2026-10-01,B{eol}");
        let e = stdin_error(&text);
        assert!(e.contains("<stdin>, line 4:"), "{eol:?}: {e}");
    }
}

#[test]
fn three_blank_lines_count_in_the_line_number() {
    for eol in ["\n", "\r\n"] {
        let text = format!("cost,usage_start_time,sku{eol}{eol}{eol}{eol}x,2026-10-01,A{eol}");
        let e = stdin_error(&text);
        assert!(e.contains("<stdin>, line 5:"), "{eol:?}: {e}");
    }
}

#[test]
fn without_blank_lines_the_line_number_is_unchanged() {
    for eol in ["\n", "\r\n"] {
        let text = format!("cost,usage_start_time,sku{eol}1,2026-10-01,A{eol}x,2026-10-01,B{eol}");
        let e = stdin_error(&text);
        assert!(e.contains("<stdin>, line 3:"), "{eol:?}: {e}");
    }
}

/// Starts the binary through `sh` with descriptor 1 given by `redirect` (`>&-` or `>/dev/null`).
fn run_with_stdout_redirect(redirect: &str) -> (i32, String) {
    let script = format!("exec \"$0\" \"$@\" {redirect}");
    let out = Command::new("sh")
        .arg("-c")
        .arg(script)
        .arg(env!("CARGO_BIN_EXE_gcp-billing-top"))
        .arg(fixture("bom.csv"))
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("sh starts");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn closed_standard_output_at_start_is_exit_2_with_a_message() {
    let (code, stderr) = run_with_stdout_redirect(">&-");
    assert_eq!(code, 2, "stderr: {stderr}");
    assert!(
        stderr.contains("standard output is closed"),
        "stderr: {stderr}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn standard_output_on_dev_null_stays_exit_0() {
    let (code, stderr) = run_with_stdout_redirect(">/dev/null");
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stderr.is_empty(), "stderr: {stderr}");
}
