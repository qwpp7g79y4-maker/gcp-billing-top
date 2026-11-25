# gcp-billing-top

A small command-line tool that reads a **GCP billing-export CSV** and shows
where the money goes: the top costs per **SKU**, **project** and **label**,
with the change against the previous month.

The tests and the measured numbers are in `QUALITY.md`. A Dutch version of
this page is in [README.nl.md](README.nl.md).

It works on a file you already have. It does not talk to BigQuery, to the
network or to any other service.

## Quick start

Build it, then run it on a fixture that ships with the repository:

```text
cargo build --release
target/release/gcp-billing-top tests/fixtures/two_months.csv
```

The report goes to standard output; the tool writes no files.

## Aan de slag in 5 minuten

1. Install it (see Installation below).
2. Take a billing-export CSV you already have.
3. Run `gcp-billing-top export.csv`.
4. Add `--by sku` or `--by project` to see one list.
5. Add `--sort delta` to see the biggest changes first.

## Usage

```text
gcp-billing-top export.csv                      # latest month, delta vs the month before
gcp-billing-top october.csv -p september.csv    # previous month in a second file
gcp-billing-top export.csv --by sku -n 20       # only the SKU list, 20 rows
gcp-billing-top export.csv --sort delta         # biggest changes first
gcp-billing-top export.csv --by label --label-key env
gcp-billing-top export.csv --net -f json        # net of credits, as JSON
cat export.csv | gcp-billing-top -              # standard input
```

## Example

Input `tests/fixtures/two_months.csv`, option `--by sku`:

```text
gcp-billing-top: 2026-10 vs 2026-09 (gross, USD)
Total: 300.00  (previous 250.00, +50.00, +20.0%)

Top 5 by SKU
  #  Name                                    Cost  Previous   Delta  Delta %
  1  N2 Instance Core running in Americas  200.00    150.00  +50.00   +33.3%
  2  Standard Storage US Multi-region       60.00     50.00  +10.00   +20.0%
  3  BigQuery Analysis                      25.00      0.00  +25.00      new
  4  Network Egress                         10.00     30.00  -20.00   -66.7%
  5  Support                                 5.00      0.00   +5.00      new
```

## What it does

Every claim below is covered by a test; [QUALITY.md](QUALITY.md) lists which.

- **[C1] Offline only.** It reads a CSV file or standard input. It has no
  network or BigQuery dependency.
- **[C2] Top costs.** Ranked lists per SKU, per project and per label
  (`--by sku|project|label`, default: every list the file has columns for).
- **[C3] Month delta in one export.** The latest month (or `--month`) is
  compared with the calendar month before it, when the export holds it.
- **[C4] Month delta from a second file.** `--previous FILE` takes the previous
  month from another file; the result equals the one-file report. Without
  `--previous-month` it uses the calendar month before, else the latest earlier
  month in that file.
- **[C5] Both export flavours.** BigQuery names (`project.id`,
  `sku.description`, `usage_start_time`, `invoice.month`, `cost`) and Cloud
  Console names (`Project ID`, `SKU description`, `Usage start date`,
  `Cost ($)`); dates as ISO timestamps, `YYYYMM` or `Oct 1, 2026`; a byte-order
  mark is ignored. The cost column is `cost` or `Cost (` and a currency or
  symbol and `)`, such as `Cost (EUR)`, and nothing else: `cost_at_list`,
  `cost center` and `Unrounded cost (EUR)` are other columns. Two columns
  of one kind (cost, month, credits, labels, SKU or currency) are refused with
  exit code 2; none is picked by position or by order of names, and the same project name twice
  (`project.id` and `Project ID`, or `project.name` and `Project name`) is refused too. That includes
  `usage_start_time` together with `invoice.month`, also when both name the same
  month. (`project.id` with `project.name` is normal and keeps `project.id`.)
- **[C6] Labels.** BigQuery JSON (`[{"key":"env","value":"prod"}]`), a JSON
  object, or `env=prod;team=web`. Without `--label-key` a row counts toward
  each of its labels, so label costs can exceed the total. With
  `--label-key env` the list is grouped by the value of `env` and adds up to
  the total (rows without that key fall in `(unlabeled)`). A label key that
  appears twice in one row, in JSON or in `env=prod;env=dev`, is refused with
  exit code 2: neither value is picked.
- **[C7] Credits.** `--net` adds the credits column (a number or the BigQuery
  JSON array of `{"amount": ...}`) to the cost, in both months. A credit with
  the key `"amount"` twice is refused with exit code 2.
- **[C8] Exact sums.** Amounts are parsed into millionths and summed as
  integers (half away from zero past the sixth decimal), so totals do not drift.
  Scientific notation such as `1.5e-3` is accepted. Range: one amount must be
  below 1e12 in size; a sum (a total, a group, cost plus credits, a change
  between months) must stay within -9223372036854.775808 to
  9223372036854.775807, checked after every row in file order. A sum outside
  that range is refused with an error that says it is too large; it is never
  clamped to the limit.
- **[C9] Three formats.** `-f table` (default), `-f json`, `-f csv`. In the table, control characters in names
  (newline, tab, escape) are shown as `?`. In CSV, a name that starts with `=`,
  `+` or `@` gets a leading `'` so a spreadsheet reads it as text.
- **[C10] One currency at a time.** If the compared months mix currencies it
  stops and asks for `--currency CODE`.
- **[C11] Clear errors.** Problems name the file and line; the line is counted
  by the program itself, so a file with CRLF line ends gets the same line as the
  same file with LF. Exit code 1 for data errors, 2 for wrong options and for
  input that says two things (see C5 to C7). Nothing goes to stdout on an
  error. A currency code from the file is shown with control characters as `?`
  in the error text, like names in the table.
- **[C12] Standard input.** `-` reads the main file (or the `--previous` file)
  from stdin.
- **[C13] Limits and ranking.** `-n N` rows per list (default 10, `0` = all);
  `--sort delta` ranks by size of change and also shows groups that vanished.
- **[C14] One month is fine.** A file with a single month still gives the
  top lists; it says that no delta is available.

## Options

| Option | Meaning |
| --- | --- |
| `FILE` | export CSV, or `-` for stdin |
| `-p`, `--previous FILE` | second file with the previous month |
| `--month YYYY-MM` | month to report (default: latest) |
| `--previous-month YYYY-MM` | month to compare with |
| `-b`, `--by sku\|project\|label` | which lists (repeatable) |
| `--label-key KEY` | group the label list by the value of KEY |
| `-n`, `--top N` | rows per list, `0` = all |
| `--sort cost\|delta` | ranking |
| `--net` | subtract credits |
| `--currency CODE` | only rows in this currency |
| `-f`, `--format table\|json\|csv` | output format |

Rows without a SKU, project or labels are grouped as `(no sku)`,
`(no project)` and `(no labels)`.

## Installation

Version 0.1.0. From crates.io, once 0.1.0 is published there:

```sh
cargo install gcp-billing-top
```

Or build from source.

```text
cargo install --path .
cargo test
```

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | the report was written |
| 1 | a data error: unreadable file, bad amount or label, a sum that is too large, mixed currencies, missing month |
| 2 | wrong options (reported by the argument parser), or ambiguous input: two cost columns, a repeated `amount` or label key |

On an error nothing goes to standard output; the message goes to standard
error and names the file and line.

## Limitations

- It reads one CSV export (plus one optional second file for the previous
  month). It does not fetch data from BigQuery or from anywhere else.
- It compares calendar months only.
- It works in one currency at a time; mixed currencies stop the run until
  `--currency CODE` picks one.
- Without `--label-key` a row counts toward each of its labels, so label costs
  can add up to more than the total.
- It shows costs from the file as they are. It does not check them against an
  invoice.

## Maintenance

Changes go through Issues, see [CONTRIBUTING.md](CONTRIBUTING.md). Report
security problems as described in [SECURITY.md](SECURITY.md). Every change to
behaviour comes with a test, and the test count in this file and in
`QUALITY.md` is checked by the test suite.

## Quality

Tests: 242

Details and the claim-to-test map are in [QUALITY.md](QUALITY.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

The licenses of the third-party crates built into the binary are listed in [THIRD-PARTY-LICENSES.html](THIRD-PARTY-LICENSES.html).
