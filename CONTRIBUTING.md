# Contributing

Contributions come in through Issues: open an issue that describes the
problem or the change, and attach the patch there.

## A patch needs a test

A patch has a test. A fix has a test that fails without it; a new option or
behaviour has a test that shows it, and the README says so. The tests live in
`src/` (unit tests) and `tests/` (end-to-end, with CSV files in
`tests/fixtures/`).

## Before you send it

Run from the repository root:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The suite checks that the test count written in `README.md` and `QUALITY.md`
equals the real count, and that every option is in the README. If you add or
remove tests, update both numbers in the same patch.

## Rules

- The tool stays offline: no network or cloud dependencies.
- Keep a change small and say in the issue what it changes for the user.
- Security problems go to the address in [SECURITY.md](SECURITY.md), not to
  Issues.

By contributing you agree that your work is licensed as in the README:
Apache-2.0 or MIT, at the user's option.
