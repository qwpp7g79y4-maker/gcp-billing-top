# Changelog

All notable changes to this project are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-11-26

### Changed

- Version number raised from 0.1.0 to 0.1.1. The program behaves the same as 0.1.0; only the version number in the manifest, the documentation and the tests changed.

## [0.1.0] - 2026-11-09

Changes made after 2026-11-09; the version number did not change.

### Fixed
- Table output replaces control characters in SKU, project and label names
  with `?`, so a name cannot forge a row or send a terminal sequence.
- CSV output puts a `'` before a name that starts with `=`, `+`, `-`, `@`, a tab
  or a carriage return.
- An amount at the limit of the 128-bit sum is refused as too large instead of
  panicking in a debug build.
- A full or closed standard error no longer makes the program panic; it exits
  with the normal error code.
- The line in an error message is counted by the program, so a CRLF file gives
  the same line as an LF file (it was one too low per line end).
- A sum that does not fit in 64 bits of millionths is refused as too large
  instead of being clamped silently to 9223372036854.775807.
- The cost column is `cost` or `Cost (...)` only; `cost_at_list` and
  `cost center` no longer count. Two cost columns, a repeated `amount` key in
  credits and a repeated label key now stop with exit code 2.
- Two month, credits, labels, SKU or currency columns now stop with exit code 2,
  like two cost columns; `usage_start_time` together with `invoice.month` does
  too. The same project name twice (`project.id` and `Project ID`, or
  `project.name` and `Project name`) stops with exit code 2 as well;
  `project.id` with `project.name` stays valid and `project.id` wins. Nothing
  is picked by position or by precedence any more.
- The line in an error message counts blank lines between records (it was too
  low by the number of blank lines).
- A currency code from the file is cleaned of control characters in the error
  text.
- `.gitattributes` keeps the checkout on LF.


First release.

### Added
- Reads a GCP billing-export CSV offline (BigQuery export and Cloud Console
  column names, `usage_start_time`, `Usage start date` and `invoice.month`).
- Top lists per SKU, project and label, ranked by cost or by size of change.
- Month-over-month delta against the previous month in the same export or in a
  second file (`--previous`), with `--month` and `--previous-month`.
- Labels as BigQuery JSON, JSON object or `key=value` pairs; `--label-key`
  groups by the value of one key.
- Credits (`--net`) as a number or as the BigQuery JSON array.
- Exact decimal sums in micro-units; scientific notation accepted.
- Table, JSON and CSV output; standard input via `-`.
- Refuses mixed currencies unless `--currency` picks one.
