# Quality

Tested code commit: a3d92b5 (measured 2026-11-25; the commit after it changes documents only).
Dutch version: [QUALITY.nl.md](QUALITY.nl.md).

Tests: 244

The number is the count of test functions in `src/` and `tests/`; the test
`quality_test_count_equals_suite` fails when it differs from the suite, and
`readme_test_count_equals_suite` does the same for the README.

| Suite | Where | Tests |
| --- | --- | --- |
| unit tests | `src/*.rs` | 158 |
| end-to-end (real binary, fixtures) | `tests/cli.rs` | 72 |
| docs, licenses, manifest | `tests/docs.rs` | 14 |

## Fixtures

`tests/fixtures/one_month.csv` (one month) and `two_months.csv` (two months)
carry the main scenarios; `previous_month.csv` plus `one_month.csv` must give the
same report as `two_months.csv`. Further fixtures cover the Cloud Console
layout, invoice months, JSON credits, mixed currencies, plain labels, many SKUs,
exotic amounts, a byte-order mark and the broken files (`bad_*`, `no_*`,
`empty`, `header_only`).

## README claims and the tests that cover them

| Claim | Tests |
| --- | --- |
| C1 offline only | `no_network_or_cloud_dependencies`, `sources_never_open_sockets`, `dash_reads_standard_input` |
| C2 top costs per SKU, project, label | `one_month_file_ranks_skus_by_cost`, `project_list_includes_the_no_project_bucket`, `label_list_counts_each_label_pair`, `two_months_csv_matches_golden_output` |
| C3 month delta in one export | `two_months_in_one_file_show_the_delta`, `current_defaults_to_latest_month`, `december_to_january_wraps_the_year`, `gap_in_same_export_means_no_previous` |
| C4 month delta from a second file | `separate_previous_file_gives_the_same_report_as_one_file`, `previous_month_option_selects_a_month_in_the_second_file`, `second_file_falls_back_to_latest_earlier_month`, `second_file_without_an_earlier_month_is_refused` |
| C5 BigQuery and Console layouts | `only_the_exact_cost_columns_count_as_the_cost_column`, `a_column_that_only_starts_with_cost_is_no_cost_column`, `console_style_headers_and_month_names_work`, `invoice_month_is_used_when_there_is_no_usage_date`, `field_month_names`, `normalize_header_variants`, `byte_order_mark_is_ignored`, `two_month_columns_exit_with_code_two`, `usage_start_time_with_invoice_month_exits_with_code_two_even_for_the_same_month`, `two_credits_columns_exit_with_code_two`, `two_labels_columns_exit_with_code_two`, `two_columns_that_normalize_to_the_same_sku_exit_with_code_two`, `two_currency_columns_exit_with_code_two`, `usage_start_with_invoice_month_is_ambiguous`, `two_project_id_columns_exit_with_code_two`, `two_project_name_columns_exit_with_code_two` |
| C6 labels | `plain_label_syntax_works`, `bigquery_json_array`, `labels_count_toward_each_pair`, `label_key_partitions_the_total` |
| C7 credits and `--net` | `net_subtracts_credits_from_both_months`, `json_credits_are_summed_for_net`, `net_without_credits_column_is_refused` |
| C8 exact sums | `twelve_rows_of_nine_hundred_billion_are_refused_not_clamped`, `eleven_rows_of_nine_hundred_billion_are_refused_too`, `ten_rows_of_nine_hundred_billion_still_add_up`, `a_partial_sum_that_overflows_is_refused_even_if_the_end_is_in_range`, `checked_arithmetic_refuses_instead_of_clamping`, `tiny_and_scientific_amounts_are_summed_exactly`, `sums_exactly`, `rounds_half_away_from_zero_at_the_sixth_decimal`, `parses_exponent_notation` |
| C9 table, JSON, CSV | `json_output_is_complete`, `csv_has_header_total_and_rows`, `table_columns_are_aligned`, `two_months_csv_matches_golden_output`, `csv_cells_starting_with_a_formula_character_get_a_quote`, `csv_text_quotes_a_leading_minus`, `csv_text_quotes_a_leading_tab`, `csv_text_quotes_a_leading_carriage_return`, `hostile_names_cannot_forge_rows_or_terminal_sequences` |
| C10 one currency at a time | `mixed_currencies_are_refused_until_one_is_chosen`, `currency_option_filters_rows` |
| C11 clear errors | `bad_cost_names_file_and_line`, `bad_labels_name_the_line`, `usage_errors_exit_with_code_two`, `missing_file_is_an_error`, `full_standard_error_gives_the_error_code_and_no_panic`, `amount_at_the_i128_limit_is_too_large_and_does_not_panic`, `crlf_files_give_the_same_line_number_as_lf_files`, `crlf_structure_error_names_line_three`, `byte_order_mark_with_crlf_keeps_the_line_number`, `a_field_with_a_line_break_inside_quotes_counts_its_physical_lines`, `two_cost_columns_exit_with_code_two`, `one_blank_line_counts_in_the_line_number`, `three_blank_lines_count_in_the_line_number`, `without_blank_lines_the_line_number_is_unchanged`, `a_repeated_key_in_credits_json_exits_with_code_two`, `a_repeated_label_key_exits_with_code_two_in_every_format`, `a_currency_code_with_a_line_break_and_escape_stays_off_standard_error_raw` |
| C12 standard input | `dash_reads_standard_input`, `stdin_can_hold_the_previous_month`, `run_refuses_stdin_twice` |
| C13 limits and ranking | `top_limits_each_list`, `default_top_is_ten_and_zero_means_all`, `sort_by_delta_includes_vanished_skus` |
| C14 one month is fine | `one_month_file_reports_total_without_delta`, `one_month_json_has_null_previous`, `single_month_has_no_previous` |

Also guarded: the checkout stays on LF (`gitattributes_pins_lf_so_a_crlf_checkout_cannot_break_the_tests`),
every CLI option is in the README (`readme_documents_every_option`),
the README example is the real output (`readme_example_is_the_real_output`),
licenses and CHANGELOG 0.1.0 (`both_licenses_are_present_and_declared`,
`changelog_has_release_0_1_0`).

## Gates

Run from the repository root:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo publish --dry-run
```

The measured result of these gates is the next section. The sections after it
are earlier measurements, kept as they were written.

## Measured 2026-11-25, fix a3d92b5 (clippy on macOS and Windows)

Date: 2026-11-25. Fix commit: a3d92b5. Machine: Linux 7.2.15. Clean worktree of a3d92b5.

Gap: CI run 45698473602 failed on clippy, not on the tests, on macos-latest,
macos-15-intel and windows-latest: `tests/cli.rs:925`, `function
run_with_stdout_redirect is never used`. The two tests that call it are under
`cfg(target_os = "linux")`; the helper was not. The helper is now under the same
`cfg`, with no `allow(dead_code)` and no `expect`. The two Linux tests are
unchanged (exit 2 with standard output closed, exit 0 for `/dev/null`).

| Command | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo test --locked` | exit 0: 158 unit tests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 244 passed, 0 failed |
| `cargo test --release --locked` | exit 0: the same 244 passed, 0 failed |
| `cargo +1.85 fmt --check` | exit 0 (rustc 1.85.1, measured 2026-11-25) |
| `cargo +1.85 clippy --all-targets --locked -- -D warnings` | exit 0, no warnings (rustc 1.85.1, measured 2026-11-25) |
| `cargo +1.85 test --locked` | exit 0: 158 unit tests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 244 passed, 0 failed (rustc 1.85.1, measured 2026-11-25) |

The 1.85 rows are the declared `rust-version`, run on the unchanged sources;
`cargo +1.85 test --release --locked` exit 0: 158 unit tests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 244 passed, 0 failed (rustc 1.85.1, measured 2026-11-25, isolated target directory).

Not run: clippy and tests on macos-latest, macos-15-intel and windows-latest
(not run; the CI run above is the failing one, no new run exists), `cargo deny`,
`cargo audit`, `cargo publish --dry-run`, fuzzing, mutation testing. Nothing
was pushed.

## Measured 2026-11-25, fix a7b3b5c (closed standard output)

Date: 2026-11-25. Fix commit: a7b3b5c. Machine: Linux 7.2.15. Clean worktree of 3bfff1c plus the fix.

Gap: with standard output closed at start (`>&-`) the program exited 0 and
printed nothing, because Rust reopens a closed descriptor 1 as `/dev/null`
before `main`. It now exits 2 with `standard output is closed`; `>/dev/null`
stays exit 0. The check is a constructor in `.init_array` (`src/stdout_probe.rs`,
the only `unsafe` in the binary), Linux only.

Red, before the fix, `cargo test --locked --test cli standard_output`:
1 passed (`/dev/null`), 1 failed (`closed_standard_output_at_start_is_exit_2_with_a_message`, left 0, right 2).

| Command | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo test --locked` | exit 0: 158 unit tests, 0 in `src/main.rs`, 72 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 244 passed, 0 failed |
| `cargo test --release --locked` | exit 0: the same 244 passed, 0 failed |

Checked and not a gap: there is no `--output` option and no `build.rs` in this
tree, so there is no symlink or special-file write and no `.git/HEAD` read.
Not run: `cargo deny`, `cargo audit`, `cargo publish --dry-run`, a GitHub
Actions run, a Windows or macOS run (the probe does nothing there), fuzzing,
mutation testing.

## Measured 2026-11-25, commit c837bdb

Date: 2026-11-25. Commit: c837bdb. Machine: Linux 7.2.15, cargo 1.99.0, rustc 1.99.0.
A clean worktree of c837bdb, nothing changed in the sources.

| Command | Result |
| --- | --- |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked -- -D warnings` | exit 0, no warnings |
| `cargo test --locked` | exit 0: 158 unit tests, 0 in `src/main.rs`, 70 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 242 passed, 0 failed |
| `cargo test --release --locked` | exit 0: the same 242 passed, 0 failed |

Not run in this measurement: `cargo deny`, `cargo audit`,
`cargo publish --dry-run`, a GitHub Actions run, a Windows or macOS run,
fuzzing, mutation testing.

## What was not in the short note

This page records checks. It is not a guarantee and it is not a fuzz report.

On 2026-11-18, on this machine, Linux, with cargo -j 2, these commands finished with exit 0: cargo fmt --all -- --check, cargo clippy --all-targets --locked -- -D warnings, cargo test --locked, and cargo test --release --locked. Debug and release passed the same tests. 0 failed.

Not run that day: cargo deny, cargo audit, a GitHub Actions run, fuzzing, and mutation testing. The CI file lists deny and audit. Those jobs have not run on GitHub for this tree, because this tree is not pushed.

The Dutch pages are README.nl.md and QUALITY.nl.md. The local notes task.md and blueprint.md are kept out of git on purpose (see `.gitignore`).

## Measured 2026-11-24, review gcp-eind-ad79106 (B-1)

Command: `cargo test --locked --offline --no-fail-fast`, run on 2026-11-24.

Red, on ad79106 with the new tests copied in and the source not fixed: the cargo
line was `test result: FAILED. 68 passed; 2 failed` for `tests/cli.rs` (the two
project-column tests) and `test result: FAILED. 12 passed; 2 failed` for
`tests/docs.rs` (the two count tests).

Green, on the new tip (code commit 7f1a4d8 plus the document commit): 158 unit
tests, 0 in `src/main.rs`, 70 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0
doc-tests: 242 passed, 0 failed. The commit after 7f1a4d8 changes documents only.
The first commit, 7f1a4d8, is itself red on the two count tests, because the
counts are in documents.

The same project name twice (`project.id` and `Project ID`, or `project.name`
and `Project name`) now exits with code 2. `project.id` with `project.name`
stays valid and `project.id` wins. The refusal of `usage_start_time` together
with `invoice.month` is unchanged; there is no month-column option.

Not run: a GitHub Actions run (nothing was pushed). Open and unchanged: N1 to
N11 of the review gcp-eind-ad79106.

STAND: groen

## Measured 2026-11-24, review gcp-eind-4bd85a5 (B-1 and B-2)

Command: `cargo test --locked --offline --no-fail-fast`, run on 2026-11-24.

Red, on 4bd85a5 with the new tests copied in and the sources not fixed: the cargo
line was `test result: FAILED. 60 passed; 8 failed` for `tests/cli.rs` (the
six column tests and the two blank-line tests; the test without blank lines
passed, as it should) and `test result: FAILED. 12 passed; 2 failed` for
`tests/docs.rs` (the two count tests).

Green, on the new tip (code commit b4b3011 plus the document commit): 158 unit tests, 0
in `src/main.rs`, 68 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0 doc-tests: 240
passed, 0 failed. The commit after b4b3011 changes documents only. The first
commit, b4b3011, is itself red on the two count tests, because the counts are in
documents.

Changed on purpose: the unit test `usage_start_beats_invoice_month` is replaced
by `usage_start_with_invoice_month_is_ambiguous`, because the precedence it
tested is the silent choice B-1 forbids. `project.id` together with
`project.name` is not refused (a BigQuery export has both); the project column
is not part of B-1.

Not run: a GitHub Actions run (nothing was pushed).
Open and unchanged: N1 to N14 of the review gcp-eind-4bd85a5.

STAND: groen

## Measured 2026-11-24, review gcp-eind-bc4169c (B-1 to B-5)

Command: `cargo test --locked --no-fail-fast`, run on 2026-11-24.

Red, on bc4169c with the new `tests/cli.rs` and `tests/docs.rs` copied in and
the sources not fixed (B-1 to B-5 of review gcp-eind-bc4169c): the cargo line
was `test result: FAILED. 46 passed; 13 failed` for `tests/cli.rs` and
`test result: FAILED. 11 passed; 3 failed` for `tests/docs.rs` (the two count
tests, and the `.gitattributes` test). The 13 failing end-to-end tests are the
CRLF line numbers, the overflowing sums, the cost columns, the repeated keys and
the currency code. The new unit tests in `src/` do not compile on the old
sources, so they are not in the red count.

Green, on the new tip (code commit 9580598 plus the document commit): 158 unit
tests, 0 in `src/main.rs`, 59 in `tests/cli.rs`, 14 in `tests/docs.rs`, 0
doc-tests: 231 passed, 0 failed. The commit after 9580598 changes documents only.
The first commit, 9580598, is itself red on the two count tests
(`readme_test_count_equals_suite`, `quality_test_count_equals_suite`), because
the counts are in documents.

STAND: groen

## Not run

On 2026-11-24 `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`
also finished with exit 0 on 9580598, and `cargo test --release --locked` passed the same 231 tests. Not run: `cargo deny`,
`cargo audit`, `cargo publish --dry-run`, a GitHub Actions run (the CRLF problem
is fixed in the sources and by `.gitattributes`, but no run on GitHub or Windows
has happened), fuzzing and mutation testing. Open and unchanged: findings N-a,
N-b and N-c of the review (N-a: the words "built-in prices" in this file, which
the sources do not have; N-b: memory use; N-c: the range of years), and the
other N findings.
