//! Keeps README, QUALITY, CHANGELOG, licenses and manifest honest.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::CommandFactory;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn all_sources() -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    rust_files(&root().join("tests"), &mut files);
    files.sort();
    files
        .iter()
        .map(|f| fs::read_to_string(f).unwrap())
        .collect()
}

/// Number of test functions in `src/` and `tests/`: lines that are exactly the test attribute.
fn suite_size() -> usize {
    let attribute = ["#[", "test]"].concat();
    all_sources()
        .iter()
        .map(|s| s.lines().filter(|l| l.trim() == attribute).count())
        .sum()
}

fn tests_line(text: &str) -> usize {
    text.lines()
        .find_map(|l| l.strip_prefix("Tests: "))
        .unwrap_or_else(|| panic!("no 'Tests: N' line"))
        .trim()
        .parse()
        .expect("'Tests: N' holds a number")
}

fn claim_ids(text: &str, marker: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for part in text.split(marker).skip(1) {
        let id: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() {
            ids.insert(format!("C{id}"));
        }
    }
    ids
}

#[test]
fn suite_has_at_least_sixty_tests() {
    assert!(suite_size() >= 60, "only {} tests", suite_size());
}

#[test]
fn readme_test_count_equals_suite() {
    assert_eq!(tests_line(&read("README.md")), suite_size());
}

#[test]
fn quality_test_count_equals_suite() {
    assert_eq!(tests_line(&read("QUALITY.md")), suite_size());
}

#[test]
fn every_readme_claim_is_in_quality_and_back() {
    let readme = claim_ids(&read("README.md"), "**[C");
    let quality = claim_ids(&read("QUALITY.md"), "| C");
    assert!(readme.len() >= 10, "{readme:?}");
    assert_eq!(readme, quality);
}

#[test]
fn every_test_named_in_quality_exists() {
    let quality = read("QUALITY.md");
    let sources = all_sources().join("\n");
    let mut seen = 0;
    for line in quality
        .lines()
        .filter(|l| l.starts_with("| C") && l[3..].starts_with(|c: char| c.is_ascii_digit()))
    {
        let tests = line.rsplit('|').nth(1).unwrap_or("");
        let names: Vec<&str> = tests.split('`').skip(1).step_by(2).collect();
        assert!(!names.is_empty(), "claim without tests: {line}");
        for name in names {
            assert!(
                sources.contains(&format!("fn {name}(")),
                "QUALITY names a missing test: {name}"
            );
            seen += 1;
        }
    }
    assert!(seen >= 20, "{seen}");
}

#[test]
fn readme_documents_every_option() {
    let readme = read("README.md");
    let cmd = gcp_billing_top::cli::Args::command();
    for arg in cmd.get_arguments() {
        if let Some(long) = arg.get_long() {
            if long == "help" || long == "version" {
                continue;
            }
            assert!(
                readme.contains(&format!("--{long}")),
                "README lacks --{long}"
            );
        }
    }
}

#[test]
fn readme_example_is_the_real_output() {
    let out = Command::new(env!("CARGO_BIN_EXE_gcp-billing-top"))
        .arg(root().join("tests/fixtures/two_months.csv"))
        .args(["--by", "sku"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let real = String::from_utf8(out.stdout).unwrap();
    assert!(
        read("README.md").contains(real.trim_end()),
        "README example differs from real output:\n{real}"
    );
}

#[test]
fn fixture_paths_in_readme_exist() {
    let readme = read("README.md");
    let mut checked = 0;
    for part in readme.split("tests/fixtures/").skip(1) {
        let name: String = part
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        assert!(
            root().join("tests/fixtures").join(&name).is_file(),
            "missing fixture {name}"
        );
        checked += 1;
    }
    assert!(checked >= 1);
}

#[test]
fn fixtures_cover_one_and_two_months() {
    let months = |name: &str| -> BTreeSet<String> {
        read(&format!("tests/fixtures/{name}"))
            .lines()
            .filter_map(|l| {
                l.split(',')
                    .nth(2)
                    .map(|d| d.chars().take(7).collect::<String>())
            })
            .filter(|d| d.starts_with("2026-"))
            .collect()
    };
    assert_eq!(months("one_month.csv").len(), 1);
    assert_eq!(months("two_months.csv").len(), 2);
}

#[test]
fn no_network_or_cloud_dependencies() {
    let manifest = read("Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    for banned in [
        "reqwest",
        "hyper",
        "ureq",
        "curl",
        "tokio",
        "async",
        "google",
        "gcp-bigquery",
        "tonic",
        "openssl",
        "rustls",
        "isahc",
        "surf",
    ] {
        assert!(!deps.contains(banned), "dependency on {banned}");
    }
    let lock = read("Cargo.lock");
    for banned in ["reqwest", "hyper", "ureq", "tokio", "rustls", "openssl"] {
        assert!(
            !lock.contains(&format!("name = \"{banned}\"")),
            "{banned} in Cargo.lock"
        );
    }
}

#[test]
fn sources_never_open_sockets() {
    let sources = all_sources();
    let banned = ["std::net", "TcpStream", "UdpSocket"];
    // The docs tests themselves name these words, so skip tests/docs.rs by content marker.
    for (i, s) in sources.iter().enumerate() {
        if s.contains("Keeps README, QUALITY") {
            continue;
        }
        for b in banned {
            assert!(!s.contains(b), "source #{i} uses {b}");
        }
    }
}

#[test]
fn changelog_has_release_0_1_0() {
    let log = read("CHANGELOG.md");
    assert!(log.contains("## [0.1.0] - 2026-11-09"));
    assert!(read("Cargo.toml").contains("version = \"0.1.0\""));
}

#[test]
fn both_licenses_are_present_and_declared() {
    let manifest = read("Cargo.toml");
    assert!(manifest.contains("license = \"MIT OR Apache-2.0\""));
    assert!(read("LICENSE-MIT").starts_with("MIT License"));
    assert!(read("LICENSE-APACHE").contains("Apache License"));
    assert!(read("LICENSE-APACHE").contains("Version 2.0, January 2004"));
    assert!(
        read("README.md").contains("LICENSE-MIT") && read("README.md").contains("LICENSE-APACHE")
    );
}

#[test]
fn gitattributes_pins_lf_so_a_crlf_checkout_cannot_break_the_tests() {
    let attributes = read(".gitattributes");
    assert!(attributes.contains("text=auto"), "{attributes}");
    assert!(attributes.contains("eol=lf"), "{attributes}");
}
