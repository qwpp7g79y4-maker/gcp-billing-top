# Security policy

## Reporting a problem

Send security reports by e-mail to pep_ai@icloud.com. Please do not open a
public issue for a vulnerability.

Say which version you used (`gcp-billing-top --version`), what you did and what
you saw. If you can, attach a small CSV that shows it, with the real amounts and
project names removed.

## Scope

The tool reads a billing-export CSV from a file or standard input and prints a
report. It opens no network connection and writes no files. Reports about a
crafted CSV that crashes it, uses a lot of memory or prints wrong totals are
welcome.

## Supported versions

Only the latest release (0.1.1 at the time of writing) is supported.
